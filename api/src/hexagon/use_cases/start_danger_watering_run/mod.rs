use crate::hexagon::models::{
    GeoPoint, OrchardId, OrchardTree, TreeId, WateringRun, WateringRunTarget,
};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

use super::start_watering_run::{WateringProgress, watering_progress};

const EXACT_ROUTE_TREE_LIMIT: usize = 20;

pub struct DangerWateringRunStartRequested {
    pub orchard_id: OrchardId,
    pub water_source: GeoPoint,
    pub carry_capacity: u32,
}

#[derive(Debug, PartialEq)]
pub enum DangerWateringRunStartError {
    InvalidWaterSource,
    InvalidCarryCapacity,
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
    if !event.water_source.longitude.is_finite()
        || !event.water_source.latitude.is_finite()
        || !(-180.0..=180.0).contains(&event.water_source.longitude)
        || !(-90.0..=90.0).contains(&event.water_source.latitude)
    {
        return Err(DangerWateringRunStartError::InvalidWaterSource);
    }
    if event.carry_capacity == 0 || i32::try_from(event.carry_capacity).is_err() {
        return Err(DangerWateringRunStartError::InvalidCarryCapacity);
    }
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
        let ordered_tree_ids = shortest_danger_route(
            danger_trees,
            event.water_source,
            event.carry_capacity as usize,
        );
        let target = WateringRunTarget::DangerTrees;
        let run_id = orchard
            .create_watering_run(
                event.orchard_id,
                &target,
                Some(event.water_source),
                Some(event.carry_capacity),
                &ordered_tree_ids,
            )
            .map_err(|_| DangerWateringRunStartError::WateringRunCouldNotBeStarted)?;
        let run = WateringRun {
            id: run_id,
            orchard_id: event.orchard_id,
            target,
            water_source: Some(event.water_source),
            carry_capacity: Some(event.carry_capacity),
            ordered_tree_ids,
            watered_tree_ids: vec![],
            completed: false,
        };
        watering_progress(&run, &orchard_trees)
            .ok_or(DangerWateringRunStartError::WateringRunCouldNotBeStarted)
    })
}

fn shortest_danger_route(
    mut trees: Vec<&OrchardTree>,
    water_source: GeoPoint,
    carry_capacity: usize,
) -> Vec<TreeId> {
    trees.sort_by_key(|tree| tree.id.0);
    if carry_capacity == 1 {
        return optimized_single_can_route(&trees, water_source)
            .into_iter()
            .map(|index| trees[index].id)
            .collect();
    }
    let mut trips = if carry_capacity == 2 && trees.len() <= EXACT_ROUTE_TREE_LIMIT {
        exact_capacity_two_trips(&trees, water_source)
    } else if carry_capacity == 2 {
        optimized_capacity_two_trips(&trees, water_source)
    } else {
        optimized_capacity_trips(&trees, water_source, carry_capacity)
    };
    if carry_capacity == 2 {
        order_trips_from_source(&mut trips, &trees, water_source);
    } else {
        order_capacity_trips_from_source(&mut trips, &trees, water_source);
    }
    trips
        .into_iter()
        .flatten()
        .map(|index| trees[index].id)
        .collect()
}

fn optimized_single_can_route(trees: &[&OrchardTree], water_source: GeoPoint) -> Vec<usize> {
    let tree_distances = distance_matrix(trees);
    let source_distances = trees
        .iter()
        .map(|tree| distance_to_source(tree, water_source))
        .collect::<Vec<_>>();
    let first = closest_tree_index(trees, &source_distances);
    let mut remaining = (0..trees.len())
        .filter(|index| *index != first)
        .collect::<Vec<_>>();
    let mut route = vec![first];
    while !remaining.is_empty() {
        let previous = *route.last().expect("a watering route has a first tree");
        let next_position = remaining
            .iter()
            .enumerate()
            .min_by(|(_, left), (_, right)| {
                tree_distances[previous][**left]
                    .total_cmp(&tree_distances[previous][**right])
                    .then_with(|| trees[**left].id.0.cmp(&trees[**right].id.0))
            })
            .map(|(position, _)| position)
            .expect("a non-empty remainder has a nearest tree");
        route.push(remaining.remove(next_position));
    }
    improve_open_route(&mut route, &tree_distances);
    route
}

fn improve_open_route(route: &mut [usize], distances: &[Vec<f64>]) {
    loop {
        let mut improved = false;
        for start in 1..route.len().saturating_sub(1) {
            for end in start + 1..route.len() {
                let old_distance = distances[route[start - 1]][route[start]]
                    + route
                        .get(end + 1)
                        .map_or(0.0, |next| distances[route[end]][*next]);
                let new_distance = distances[route[start - 1]][route[end]]
                    + route
                        .get(end + 1)
                        .map_or(0.0, |next| distances[route[start]][*next]);
                if new_distance < old_distance {
                    route[start..=end].reverse();
                    improved = true;
                }
            }
        }
        if !improved {
            return;
        }
    }
}

fn exact_capacity_two_trips(trees: &[&OrchardTree], water_source: GeoPoint) -> Vec<Vec<usize>> {
    let tree_distances = distance_matrix(trees);
    let source_distances = trees
        .iter()
        .map(|tree| distance_to_source(tree, water_source))
        .collect::<Vec<_>>();
    let state_count = 1_usize << trees.len();
    let mut costs = vec![f64::NAN; state_count];
    let mut choices = vec![i8::MIN; state_count];
    costs[0] = 0.0;
    let all_trees = state_count - 1;
    let mut trips = Vec::with_capacity(trees.len().div_ceil(2));
    let remaining = if trees.len() > 1 && trees.len() % 2 == 1 {
        let closest = closest_tree_index(trees, &source_distances);
        let mut best_partner = None;
        let mut best_remaining = 0;
        let mut best_cost = f64::INFINITY;
        for partner in 0..trees.len() {
            if partner == closest {
                continue;
            }
            let remaining = all_trees ^ (1 << closest) ^ (1 << partner);
            let candidate = source_distances[closest]
                + tree_distances[closest][partner]
                + source_distances[partner]
                + minimum_excursion_cost(
                    remaining,
                    &tree_distances,
                    &source_distances,
                    &mut costs,
                    &mut choices,
                );
            if candidate < best_cost {
                best_cost = candidate;
                best_partner = Some(partner);
                best_remaining = remaining;
            }
        }
        trips.push(vec![closest, best_partner.expect("at least two trees")]);
        best_remaining
    } else {
        minimum_excursion_cost(
            all_trees,
            &tree_distances,
            &source_distances,
            &mut costs,
            &mut choices,
        );
        all_trees
    };
    trips.extend(reconstruct_trips(remaining, &choices));
    trips
}

fn reconstruct_trips(mut remaining: usize, choices: &[i8]) -> Vec<Vec<usize>> {
    let mut trips = Vec::with_capacity((remaining.count_ones() as usize).div_ceil(2));
    while remaining != 0 {
        let first = remaining.trailing_zeros() as usize;
        remaining ^= 1 << first;
        let second = choices[remaining | (1 << first)];
        if second < 0 {
            trips.push(vec![first]);
        } else {
            let second = second as usize;
            remaining ^= 1 << second;
            trips.push(vec![first, second]);
        }
    }
    trips
}

fn closest_tree_index(trees: &[&OrchardTree], source_distances: &[f64]) -> usize {
    source_distances
        .iter()
        .enumerate()
        .min_by(|(left_index, left), (right_index, right)| {
            left.total_cmp(right)
                .then_with(|| trees[*left_index].id.0.cmp(&trees[*right_index].id.0))
        })
        .map(|(index, _)| index)
        .expect("a danger route contains at least one tree")
}

fn minimum_excursion_cost(
    remaining: usize,
    tree_distances: &[Vec<f64>],
    source_distances: &[f64],
    costs: &mut [f64],
    choices: &mut [i8],
) -> f64 {
    if !costs[remaining].is_nan() {
        return costs[remaining];
    }
    let first = remaining.trailing_zeros() as usize;
    let without_first = remaining ^ (1 << first);
    let mut best = if remaining.count_ones() % 2 == 1 {
        choices[remaining] = -1;
        2.0 * source_distances[first]
            + minimum_excursion_cost(
                without_first,
                tree_distances,
                source_distances,
                costs,
                choices,
            )
    } else {
        f64::INFINITY
    };
    for second in first + 1..tree_distances.len() {
        if without_first & (1 << second) == 0 {
            continue;
        }
        let candidate = source_distances[first]
            + tree_distances[first][second]
            + source_distances[second]
            + minimum_excursion_cost(
                without_first ^ (1 << second),
                tree_distances,
                source_distances,
                costs,
                choices,
            );
        if candidate < best {
            best = candidate;
            choices[remaining] = second as i8;
        }
    }
    costs[remaining] = best;
    best
}

fn optimized_capacity_two_trips(trees: &[&OrchardTree], water_source: GeoPoint) -> Vec<Vec<usize>> {
    let tree_distances = distance_matrix(trees);
    let source_distances = trees
        .iter()
        .map(|tree| distance_to_source(tree, water_source))
        .collect::<Vec<_>>();
    let mut candidates = Vec::new();
    for first in 0..trees.len() {
        for second in first + 1..trees.len() {
            let saving =
                source_distances[first] + source_distances[second] - tree_distances[first][second];
            candidates.push((saving, first, second));
        }
    }
    candidates.sort_by(|left, right| {
        right
            .0
            .total_cmp(&left.0)
            .then_with(|| trees[left.1].id.0.cmp(&trees[right.1].id.0))
            .then_with(|| trees[left.2].id.0.cmp(&trees[right.2].id.0))
    });
    let mut paired = vec![false; trees.len()];
    let mut trips = Vec::with_capacity(trees.len().div_ceil(2));
    if trees.len() > 1 && trees.len() % 2 == 1 {
        let closest = closest_tree_index(trees, &source_distances);
        let (_, first, second) = candidates
            .iter()
            .copied()
            .find(|(_, first, second)| *first == closest || *second == closest)
            .expect("the closest tree has a possible partner");
        paired[first] = true;
        paired[second] = true;
        trips.push(vec![first, second]);
    }
    for (_, first, second) in candidates {
        if paired[first] || paired[second] {
            continue;
        }
        paired[first] = true;
        paired[second] = true;
        trips.push(vec![first, second]);
    }
    for (index, is_paired) in paired.into_iter().enumerate() {
        if !is_paired {
            trips.push(vec![index]);
        }
    }
    trips
}

fn optimized_capacity_trips(
    trees: &[&OrchardTree],
    water_source: GeoPoint,
    carry_capacity: usize,
) -> Vec<Vec<usize>> {
    let tree_distances = distance_matrix(trees);
    let source_distances = trees
        .iter()
        .map(|tree| distance_to_source(tree, water_source))
        .collect::<Vec<_>>();
    let mut savings = Vec::new();
    for first in 0..trees.len() {
        for second in first + 1..trees.len() {
            savings.push((
                source_distances[first] + source_distances[second] - tree_distances[first][second],
                first,
                second,
            ));
        }
    }
    savings.sort_by(|left, right| {
        right
            .0
            .total_cmp(&left.0)
            .then_with(|| trees[left.1].id.0.cmp(&trees[right.1].id.0))
            .then_with(|| trees[left.2].id.0.cmp(&trees[right.2].id.0))
    });

    let mut trips = (0..trees.len())
        .map(|index| vec![index])
        .collect::<Vec<_>>();
    for (_, first, second) in savings {
        let Some(first_trip_index) = trips.iter().position(|trip| trip.contains(&first)) else {
            continue;
        };
        let Some(second_trip_index) = trips.iter().position(|trip| trip.contains(&second)) else {
            continue;
        };
        if first_trip_index == second_trip_index
            || trips[first_trip_index].len() + trips[second_trip_index].len() > carry_capacity
        {
            continue;
        }
        let mut first_trip = trips[first_trip_index].clone();
        let mut second_trip = trips[second_trip_index].clone();
        if !orient_trip_endpoint(&mut first_trip, first, true)
            || !orient_trip_endpoint(&mut second_trip, second, false)
        {
            continue;
        }
        first_trip.extend(second_trip);
        trips[first_trip_index] = first_trip;
        trips.remove(second_trip_index);
    }
    trips
}

fn orient_trip_endpoint(trip: &mut [usize], tree_index: usize, at_end: bool) -> bool {
    let already_oriented = if at_end {
        trip.last() == Some(&tree_index)
    } else {
        trip.first() == Some(&tree_index)
    };
    if already_oriented {
        return true;
    }
    let can_reverse = if at_end {
        trip.first() == Some(&tree_index)
    } else {
        trip.last() == Some(&tree_index)
    };
    if can_reverse {
        trip.reverse();
    }
    can_reverse
}

fn order_capacity_trips_from_source(
    trips: &mut [Vec<usize>],
    trees: &[&OrchardTree],
    water_source: GeoPoint,
) {
    let tree_distances = distance_matrix(trees);
    let source_distances = trees
        .iter()
        .map(|tree| distance_to_source(tree, water_source))
        .collect::<Vec<_>>();
    for trip in trips.iter_mut() {
        let first = trip
            .iter()
            .copied()
            .min_by(|left, right| {
                source_distances[*left]
                    .total_cmp(&source_distances[*right])
                    .then_with(|| trees[*left].id.0.cmp(&trees[*right].id.0))
            })
            .expect("a watering trip contains at least one tree");
        let mut remaining = trip.clone();
        remaining.retain(|tree_index| *tree_index != first);
        let mut ordered = vec![first];
        while !remaining.is_empty() {
            let previous = *ordered.last().expect("an ordered trip has a first tree");
            let next_position = remaining
                .iter()
                .enumerate()
                .min_by(|(_, left), (_, right)| {
                    tree_distances[previous][**left]
                        .total_cmp(&tree_distances[previous][**right])
                        .then_with(|| trees[**left].id.0.cmp(&trees[**right].id.0))
                })
                .map(|(position, _)| position)
                .expect("a non-empty remainder has a nearest tree");
            ordered.push(remaining.remove(next_position));
        }
        *trip = ordered;
    }
    trips.sort_by(|left, right| {
        source_distances[left[0]]
            .total_cmp(&source_distances[right[0]])
            .then_with(|| trees[left[0]].id.0.cmp(&trees[right[0]].id.0))
    });
}

fn order_trips_from_source(
    trips: &mut [Vec<usize>],
    trees: &[&OrchardTree],
    water_source: GeoPoint,
) {
    for trip in trips.iter_mut() {
        trip.sort_by(|left, right| {
            distance_to_source(trees[*left], water_source)
                .total_cmp(&distance_to_source(trees[*right], water_source))
                .then_with(|| trees[*left].id.0.cmp(&trees[*right].id.0))
        });
    }
    trips.sort_by(|left, right| match (left.len() == 1, right.len() == 1) {
        (true, false) => std::cmp::Ordering::Greater,
        (false, true) => std::cmp::Ordering::Less,
        _ => distance_to_source(trees[left[0]], water_source)
            .total_cmp(&distance_to_source(trees[right[0]], water_source))
            .then_with(|| trees[left[0]].id.0.cmp(&trees[right[0]].id.0)),
    });
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

fn distance_to_source(tree: &OrchardTree, water_source: GeoPoint) -> f64 {
    let mean_latitude = ((tree.tree.latitude + water_source.latitude) / 2.0).to_radians();
    let longitude =
        (water_source.longitude - tree.tree.longitude).to_radians() * mean_latitude.cos();
    let latitude = (water_source.latitude - tree.tree.latitude).to_radians();
    longitude.hypot(latitude)
}
