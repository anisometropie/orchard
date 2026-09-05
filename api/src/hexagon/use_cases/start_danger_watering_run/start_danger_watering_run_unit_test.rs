use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, IdentificationStatus, NamedTaxon, Orchard, OrchardId, PlantIdentity,
    PlantIdentityId, Tree, TreeId, WateringRunTarget,
};
use orchard_api::hexagon::use_cases::start_danger_watering_run::{
    DangerWateringRunStartError, DangerWateringRunStartRequested, start_danger_watering_run,
};

#[test]
fn start_at_the_northernmost_danger_tree_then_take_the_shortest_route_through_all_of_them() {
    let trees = vec![
        tree(5.000, 45.040, true, true),
        tree(5.010, 45.000, true, true),
        tree(5.030, 45.025, true, true),
        tree(5.050, 45.005, true, true),
        tree(5.020, 45.020, true, false),
        tree(5.040, 45.030, false, true),
    ];
    let danger_coordinates = trees[..4]
        .iter()
        .map(|tree| (tree.longitude, tree.latitude))
        .collect::<Vec<_>>();
    let (mut storage, observer) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        orchard(),
        vec![apple_identity()],
        trees,
    );

    let progress = start_danger_watering_run(
        DangerWateringRunStartRequested {
            orchard_id: OrchardId(7),
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(progress.target, WateringRunTarget::DangerTrees);
    assert_eq!(progress.next_tree.unwrap().id, TreeId(1));
    let route = observer.active_watering_run_tree_ids(OrchardId(7));
    assert_eq!(route.len(), 4);
    assert_eq!(route[0], TreeId(1));
    assert_eq!(
        route_distance(&route, &danger_coordinates),
        shortest_route_distance_from(TreeId(1), &danger_coordinates),
    );
}

#[test]
fn refuse_to_start_when_no_living_tree_is_currently_in_danger() {
    let (mut storage, observer) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        orchard(),
        vec![apple_identity()],
        vec![tree(5.0, 45.0, true, false)],
    );

    let result = start_danger_watering_run(
        DangerWateringRunStartRequested {
            orchard_id: OrchardId(7),
        },
        &mut storage,
    );

    assert_eq!(result, Err(DangerWateringRunStartError::NoDangerTrees));
    assert!(
        observer
            .active_watering_run_tree_ids(OrchardId(7))
            .is_empty()
    );
}

#[test]
fn compute_an_exact_route_for_the_orchards_current_danger_tree_scale() {
    let trees = (0..19)
        .map(|index| tree(5.0, 45.0 + f64::from(index) / 1_000.0, true, true))
        .collect::<Vec<_>>();
    let (mut storage, observer) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        orchard(),
        vec![apple_identity()],
        trees,
    );

    start_danger_watering_run(
        DangerWateringRunStartRequested {
            orchard_id: OrchardId(7),
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(
        observer.active_watering_run_tree_ids(OrchardId(7)),
        (1..=19).rev().map(TreeId).collect::<Vec<_>>()
    );
}

fn shortest_route_distance_from(start: TreeId, coordinates: &[(f64, f64)]) -> u64 {
    let mut remaining = (1..=coordinates.len())
        .map(|id| TreeId(id as u64))
        .filter(|tree_id| *tree_id != start)
        .collect::<Vec<_>>();
    let mut shortest = u64::MAX;
    visit_permutations(&mut remaining, 0, &mut |tail| {
        let route = std::iter::once(start)
            .chain(tail.iter().copied())
            .collect::<Vec<_>>();
        shortest = shortest.min(route_distance(&route, coordinates));
    });
    shortest
}

fn visit_permutations(values: &mut [TreeId], index: usize, visit: &mut impl FnMut(&[TreeId])) {
    if index == values.len() {
        visit(values);
        return;
    }
    for next in index..values.len() {
        values.swap(index, next);
        visit_permutations(values, index + 1, visit);
        values.swap(index, next);
    }
}

fn route_distance(route: &[TreeId], coordinates: &[(f64, f64)]) -> u64 {
    route
        .windows(2)
        .map(|pair| {
            let from = coordinates[pair[0].0 as usize - 1];
            let to = coordinates[pair[1].0 as usize - 1];
            let longitude = (to.0 - from.0) * 78_715.0;
            let latitude = (to.1 - from.1) * 111_320.0;
            (longitude.hypot(latitude) * 1_000.0).round() as u64
        })
        .sum()
}

fn orchard() -> Orchard {
    Orchard {
        id: OrchardId(7),
        name: "My orchard".into(),
        longitude: 5.0,
        latitude: 45.0,
        reference_region: "Drôme, France".into(),
    }
}

fn tree(longitude: f64, latitude: f64, is_alive: bool, is_in_danger: bool) -> Tree {
    Tree {
        legacy_source: None,
        plant_identity_id: PlantIdentityId(1),
        cultivar_id: None,
        identification_status: IdentificationStatus::Confirmed,
        longitude,
        latitude,
        planted_on: None,
        row_name: Some("North".into()),
        roles: vec!["fruit".into()],
        is_alive,
        is_in_danger,
        reproductive_role: None,
        adult_height_meters: None,
        adult_width_meters: None,
    }
}

fn apple_identity() -> PlantIdentity {
    PlantIdentity {
        common_name: "Apple".into(),
        botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
            genus: "Malus".into(),
            species: Some("domestica".into()),
            species_is_hybrid: false,
            infraspecific: None,
            is_aggregate: false,
            cultivar_group: None,
        }),
    }
}
