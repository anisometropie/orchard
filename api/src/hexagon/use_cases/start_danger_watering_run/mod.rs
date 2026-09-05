use crate::hexagon::models::{OrchardId, OrchardTree, TreeId, WateringRun, WateringRunTarget};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

use super::start_watering_run::{WateringProgress, watering_progress};

const EXACT_ROUTE_TREE_LIMIT: usize = 20;

pub struct DangerWateringRunStartRequested {
    pub orchard_id: OrchardId,
}

#[derive(Debug, PartialEq)]
pub enum DangerWateringRunStartError {
    NoDangerTrees,
    AnotherWateringRunIsActive,
    WateringRunCouldNotBeStarted,
}

impl From<OrchardStorageError> for DangerWateringRunStartError {
    fn from(_: OrchardStorageError) -> Self {
        Self::WateringRunCouldNotBeStarted
    }
}

pub fn start_danger_watering_run(
    event: DangerWateringRunStartRequested,
    storage: &mut impl OrchardStorage,
) -> Result<WateringProgress, DangerWateringRunStartError> {
    storage.transaction(|orchard| {
        let orchard_trees = orchard
            .trees_in_orchard(event.orchard_id)
            .map_err(|_| DangerWateringRunStartError::WateringRunCouldNotBeStarted)?;
        if let Some(active_run) = orchard
            .active_watering_run(event.orchard_id)
            .map_err(|_| DangerWateringRunStartError::WateringRunCouldNotBeStarted)?
        {
            if active_run.target != WateringRunTarget::DangerTrees {
                return Err(DangerWateringRunStartError::AnotherWateringRunIsActive);
            }
            return watering_progress(&active_run, &orchard_trees)
                .ok_or(DangerWateringRunStartError::WateringRunCouldNotBeStarted);
        }

        let danger_trees = orchard_trees
            .iter()
            .filter(|tree| tree.tree.is_alive && tree.tree.is_in_danger)
            .collect::<Vec<_>>();
        if danger_trees.is_empty() {
            return Err(DangerWateringRunStartError::NoDangerTrees);
        }
        let ordered_tree_ids = shortest_danger_route(danger_trees);
        let target = WateringRunTarget::DangerTrees;
        let run_id = orchard
            .create_watering_run(event.orchard_id, &target, &ordered_tree_ids)
            .map_err(|_| DangerWateringRunStartError::WateringRunCouldNotBeStarted)?;
        let run = WateringRun {
            id: run_id,
            orchard_id: event.orchard_id,
            target,
            ordered_tree_ids,
            watered_tree_ids: vec![],
            completed: false,
        };
        watering_progress(&run, &orchard_trees)
            .ok_or(DangerWateringRunStartError::WateringRunCouldNotBeStarted)
    })
}

fn shortest_danger_route(mut trees: Vec<&OrchardTree>) -> Vec<TreeId> {
    trees.sort_by(|left, right| {
        right
            .tree
            .latitude
            .total_cmp(&left.tree.latitude)
            .then_with(|| left.id.0.cmp(&right.id.0))
    });
    let start = trees.remove(0);
    trees.sort_by_key(|tree| tree.id.0);
    trees.insert(0, start);

    let route = if trees.len() <= EXACT_ROUTE_TREE_LIMIT {
        exact_open_route(&trees)
    } else {
        optimized_open_route(&trees)
    };
    route.into_iter().map(|index| trees[index].id).collect()
}

fn exact_open_route(trees: &[&OrchardTree]) -> Vec<usize> {
    if trees.len() == 1 {
        return vec![0];
    }
    let remaining_count = trees.len() - 1;
    let state_count = 1_usize << remaining_count;
    let distances = distance_matrix(trees);
    let mut costs = vec![f64::INFINITY; state_count * remaining_count];
    let mut predecessors = vec![u8::MAX; state_count * remaining_count];

    for endpoint in 0..remaining_count {
        costs[((1 << endpoint) * remaining_count) + endpoint] = distances[0][endpoint + 1];
    }
    for visited in 1..state_count {
        for endpoint in 0..remaining_count {
            let endpoint_bit = 1 << endpoint;
            if visited & endpoint_bit == 0 {
                continue;
            }
            let previous_visited = visited ^ endpoint_bit;
            if previous_visited == 0 {
                continue;
            }
            let state_index = visited * remaining_count + endpoint;
            for previous in 0..remaining_count {
                if previous_visited & (1 << previous) == 0 {
                    continue;
                }
                let candidate = costs[previous_visited * remaining_count + previous]
                    + distances[previous + 1][endpoint + 1];
                if candidate < costs[state_index] {
                    costs[state_index] = candidate;
                    predecessors[state_index] = previous as u8;
                }
            }
        }
    }

    let all_visited = state_count - 1;
    let mut endpoint = (0..remaining_count)
        .min_by(|left, right| {
            costs[all_visited * remaining_count + *left]
                .total_cmp(&costs[all_visited * remaining_count + *right])
                .then_with(|| trees[*left + 1].id.0.cmp(&trees[*right + 1].id.0))
        })
        .expect("a route with at least two trees has an endpoint");
    let mut visited = all_visited;
    let mut reverse_route = Vec::with_capacity(remaining_count);
    loop {
        reverse_route.push(endpoint + 1);
        let state_index = visited * remaining_count + endpoint;
        visited ^= 1 << endpoint;
        if visited == 0 {
            break;
        }
        endpoint = usize::from(predecessors[state_index]);
    }
    reverse_route.reverse();
    std::iter::once(0).chain(reverse_route).collect()
}

fn optimized_open_route(trees: &[&OrchardTree]) -> Vec<usize> {
    let distances = distance_matrix(trees);
    let mut remaining = (1..trees.len()).collect::<Vec<_>>();
    let mut route = vec![0];
    while !remaining.is_empty() {
        let current = *route.last().expect("the route always contains its start");
        let next_index = (0..remaining.len())
            .min_by(|left, right| {
                distances[current][remaining[*left]]
                    .total_cmp(&distances[current][remaining[*right]])
                    .then_with(|| {
                        trees[remaining[*left]]
                            .id
                            .0
                            .cmp(&trees[remaining[*right]].id.0)
                    })
            })
            .expect("a non-empty remainder has a nearest tree");
        route.push(remaining.remove(next_index));
    }

    loop {
        let mut improved = false;
        for first in 1..route.len().saturating_sub(1) {
            for last in first + 1..route.len() {
                let previous = route[first - 1];
                let old_distance = distances[previous][route[first]]
                    + route
                        .get(last + 1)
                        .map_or(0.0, |next| distances[route[last]][*next]);
                let new_distance = distances[previous][route[last]]
                    + route
                        .get(last + 1)
                        .map_or(0.0, |next| distances[route[first]][*next]);
                if new_distance + f64::EPSILON < old_distance {
                    route[first..=last].reverse();
                    improved = true;
                }
            }
        }
        if !improved {
            return route;
        }
    }
}

fn distance_matrix(trees: &[&OrchardTree]) -> Vec<Vec<f64>> {
    trees
        .iter()
        .map(|from| {
            trees
                .iter()
                .map(|to| {
                    let mean_latitude =
                        ((from.tree.latitude + to.tree.latitude) / 2.0).to_radians();
                    let longitude = (to.tree.longitude - from.tree.longitude).to_radians()
                        * mean_latitude.cos();
                    let latitude = (to.tree.latitude - from.tree.latitude).to_radians();
                    longitude.hypot(latitude)
                })
                .collect()
        })
        .collect()
}
