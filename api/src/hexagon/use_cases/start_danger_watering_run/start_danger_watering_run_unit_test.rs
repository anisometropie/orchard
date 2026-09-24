use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, GeoPoint, HarvestRunTarget, HarvestScheduleOwner, HarvestedPart,
    IdentificationStatus, NamedTaxon, Orchard, OrchardId, PlantIdentity, PlantIdentityId, Tree,
    TreeId, WateringRunTarget,
};
use orchard_api::hexagon::use_cases::replace_plant_harvest_windows::{
    AnnualHarvestWindowChanged, OrchardHarvestWindowsReplaced, replace_orchard_harvest_windows,
};
use orchard_api::hexagon::use_cases::start_danger_watering_run::{
    DangerWateringRunStartError, DangerWateringRunStartRequested, start_danger_watering_run,
};
use orchard_api::hexagon::use_cases::start_harvest_run::{
    HarvestRunStartRequested, start_harvest_run,
};

#[test]
fn exclude_flagged_danger_trees_from_a_new_route() {
    let mut excluded = tree(-73.5, 12.25, true, true);
    excluded.is_excluded_from_watering = true;
    let (mut storage, _) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        orchard(),
        vec![apple_identity()],
        vec![excluded, tree(-73.49, 12.26, true, true)],
    );
    let progress = start_danger_watering_run(
        DangerWateringRunStartRequested {
            orchard_id: OrchardId(7),
            water_source: GeoPoint {
                longitude: -73.5,
                latitude: 12.25,
            },
            carry_capacity: 2,
        },
        &mut storage,
    )
    .unwrap();
    assert_eq!(progress.total_tree_count, 1);
    assert_eq!(progress.next_tree.unwrap().id, TreeId(2));
    assert_eq!(
        progress
            .route
            .iter()
            .map(|tree| tree.id)
            .collect::<Vec<_>>(),
        vec![TreeId(2)]
    );
}

#[test]
fn start_at_the_danger_tree_closest_to_the_source_then_minimize_all_two_can_trips() {
    let trees = vec![
        tree(-73.5, 12.2952, true, true),
        tree(-73.4909, 12.25, true, true),
        tree(-73.4727, 12.27825, true, true),
        tree(-73.4545, 12.25565, true, true),
        tree(-73.4818, 12.2726, true, false),
        tree(-73.4636, 12.2839, false, true),
    ];
    let danger_coordinates = trees[..4]
        .iter()
        .map(|tree| (tree.longitude, tree.latitude))
        .collect::<Vec<_>>();
    let water_source = GeoPoint {
        longitude: -73.4909,
        latitude: 12.25113,
    };
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
            water_source,
            carry_capacity: 2,
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(progress.target, WateringRunTarget::DangerTrees);
    assert_eq!(progress.water_source, Some(water_source));
    assert_eq!(progress.carry_capacity, Some(2));
    let route = observer.active_watering_run_tree_ids(OrchardId(7));
    assert_eq!(
        progress
            .route
            .iter()
            .map(|tree| tree.id)
            .collect::<Vec<_>>(),
        route
    );
    assert_eq!(progress.next_tree.unwrap().id, TreeId(2));
    assert_eq!(route.len(), 4);
    assert_eq!(route[0], TreeId(2));
    assert_eq!(
        capacity_two_route_distance(&route, &danger_coordinates, water_source),
        shortest_capacity_two_route_distance(&danger_coordinates, water_source),
    );
}

#[test]
fn persist_an_arbitrary_capacity_and_start_at_the_tree_closest_to_the_source() {
    let trees = vec![
        tree(-73.4636, 12.2952, true, true),
        tree(-73.49909, 12.25113, true, true),
        tree(-73.4818, 12.2726, true, true),
        tree(-73.5091, 12.2387, true, true),
        tree(-73.4727, 12.2839, true, true),
    ];
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
            water_source: GeoPoint {
                longitude: -73.5,
                latitude: 12.25,
            },
            carry_capacity: 3,
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(progress.carry_capacity, Some(3));
    assert_eq!(progress.next_tree.unwrap().id, TreeId(2));
    let route = observer.active_watering_run_tree_ids(OrchardId(7));
    assert_eq!(route.len(), 5);
    let mut sorted_route = route;
    sorted_route.sort_by_key(|tree_id| tree_id.0);
    assert_eq!(
        sorted_route,
        vec![TreeId(1), TreeId(2), TreeId(3), TreeId(4), TreeId(5)]
    );
}

#[test]
fn refuse_a_zero_carry_capacity_before_creating_a_run() {
    let (mut storage, observer) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        orchard(),
        vec![apple_identity()],
        vec![tree(-73.5, 12.25, true, true)],
    );

    let result = start_danger_watering_run(
        DangerWateringRunStartRequested {
            orchard_id: OrchardId(7),
            water_source: GeoPoint {
                longitude: -73.5,
                latitude: 12.2613,
            },
            carry_capacity: 0,
        },
        &mut storage,
    );

    assert_eq!(
        result,
        Err(DangerWateringRunStartError::InvalidCarryCapacity)
    );
    assert!(
        observer
            .active_watering_run_tree_ids(OrchardId(7))
            .is_empty()
    );
}

#[test]
fn refuse_to_start_danger_watering_while_a_harvest_tour_is_active() {
    let (mut storage, observer) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        orchard(),
        vec![apple_identity()],
        vec![tree(-73.5, 12.25, true, true)],
    );
    replace_orchard_harvest_windows(
        OrchardHarvestWindowsReplaced {
            orchard_id: OrchardId(7),
            owner: HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
            reference_region: "Example Region, France".into(),
            windows: vec![AnnualHarvestWindowChanged {
                start_month: 8,
                start_day: 1,
                end_month: 9,
                end_day: 30,
                harvested_part: HarvestedPart::Fruit,
            }],
        },
        &mut storage,
    )
    .unwrap();
    start_harvest_run(
        HarvestRunStartRequested {
            orchard_id: OrchardId(7),
            target: HarvestRunTarget::All,
            harvested_parts: vec![HarvestedPart::Fruit],
            action_date: "2026-09-18".into(),
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(
        start_danger_watering_run(
            DangerWateringRunStartRequested {
                orchard_id: OrchardId(7),
                water_source: GeoPoint {
                    longitude: -73.5,
                    latitude: 12.2613,
                },
                carry_capacity: 2,
            },
            &mut storage,
        ),
        Err(DangerWateringRunStartError::HarvestRunIsActive)
    );
    assert!(
        observer
            .active_watering_run_tree_ids(OrchardId(7))
            .is_empty()
    );
}

#[test]
fn with_one_can_start_closest_to_the_source_then_follow_one_short_route() {
    let (mut storage, observer) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        orchard(),
        vec![apple_identity()],
        vec![
            tree(-73.4999909, 12.25, true, true),
            tree(-73.5000182, 12.25, true, true),
            tree(-73.4999727, 12.25, true, true),
        ],
    );

    let progress = start_danger_watering_run(
        DangerWateringRunStartRequested {
            orchard_id: OrchardId(7),
            water_source: GeoPoint {
                longitude: -73.5,
                latitude: 12.25,
            },
            carry_capacity: 1,
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(progress.carry_capacity, Some(1));
    assert_eq!(progress.next_tree.unwrap().id, TreeId(1));
    assert_eq!(
        observer.active_watering_run_tree_ids(OrchardId(7)),
        vec![TreeId(1), TreeId(3), TreeId(2)]
    );
}

#[test]
fn refuse_to_start_when_no_living_tree_is_currently_in_danger() {
    let (mut storage, observer) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        orchard(),
        vec![apple_identity()],
        vec![tree(-73.5, 12.25, true, false)],
    );

    let result = start_danger_watering_run(
        DangerWateringRunStartRequested {
            orchard_id: OrchardId(7),
            water_source: GeoPoint {
                longitude: -73.5,
                latitude: 12.2613,
            },
            carry_capacity: 2,
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
        .map(|index| tree(-73.5, 12.25 + 1.13 * f64::from(index) / 1_000.0, true, true))
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
            water_source: GeoPoint {
                longitude: -73.5,
                latitude: 12.2726,
            },
            carry_capacity: 2,
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(
        observer.active_watering_run_tree_ids(OrchardId(7)),
        vec![
            TreeId(19),
            TreeId(17),
            TreeId(16),
            TreeId(15),
            TreeId(14),
            TreeId(13),
            TreeId(12),
            TreeId(11),
            TreeId(10),
            TreeId(9),
            TreeId(8),
            TreeId(7),
            TreeId(6),
            TreeId(5),
            TreeId(4),
            TreeId(3),
            TreeId(2),
            TreeId(1),
            TreeId(18),
        ]
    );
}

#[test]
fn keep_the_single_tree_trip_last_so_every_visible_pair_uses_the_same_two_cans() {
    let trees = vec![
        tree(-73.5, 12.2500113, true, true),
        tree(-73.5, 12.249887, true, true),
        tree(-73.5, 12.2498757, true, true),
        tree(-73.4998635, 12.25, true, true),
        tree(-73.4998544, 12.25, true, true),
    ];
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
            water_source: GeoPoint {
                longitude: -73.5,
                latitude: 12.25,
            },
            carry_capacity: 2,
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(
        observer.active_watering_run_tree_ids(OrchardId(7)),
        vec![TreeId(1), TreeId(2), TreeId(4), TreeId(5), TreeId(3)]
    );
}

fn shortest_capacity_two_route_distance(coordinates: &[(f64, f64)], water_source: GeoPoint) -> u64 {
    shortest_excursions((1_u64 << coordinates.len()) - 1, coordinates, water_source)
}

fn shortest_excursions(mask: u64, coordinates: &[(f64, f64)], water_source: GeoPoint) -> u64 {
    if mask == 0 {
        return 0;
    }
    let first = mask.trailing_zeros() as usize;
    let without_first = mask ^ (1 << first);
    let mut shortest = if mask.count_ones() % 2 == 1 {
        2 * distance_to_source(coordinates[first], water_source)
            + shortest_excursions(without_first, coordinates, water_source)
    } else {
        u64::MAX
    };
    for second in first + 1..coordinates.len() {
        if without_first & (1 << second) == 0 {
            continue;
        }
        let excursion = distance_to_source(coordinates[first], water_source)
            + point_distance(coordinates[first], coordinates[second])
            + distance_to_source(coordinates[second], water_source);
        shortest = shortest.min(
            excursion
                + shortest_excursions(without_first ^ (1 << second), coordinates, water_source),
        );
    }
    shortest
}

fn capacity_two_route_distance(
    route: &[TreeId],
    coordinates: &[(f64, f64)],
    water_source: GeoPoint,
) -> u64 {
    route
        .chunks(2)
        .map(|trip| {
            let first = coordinates[trip[0].0 as usize - 1];
            let second = trip
                .get(1)
                .map(|tree_id| coordinates[tree_id.0 as usize - 1]);
            distance_to_source(first, water_source)
                + second.map_or(0, |second| point_distance(first, second))
                + distance_to_source(second.unwrap_or(first), water_source)
        })
        .sum()
}

fn distance_to_source(point: (f64, f64), source: GeoPoint) -> u64 {
    point_distance(point, (source.longitude, source.latitude))
}

fn point_distance(from: (f64, f64), to: (f64, f64)) -> u64 {
    let longitude = (to.0 - from.0) * 108_900.0;
    let latitude = (to.1 - from.1) * 111_320.0;
    (longitude.hypot(latitude) * 1_000.0).round() as u64
}

fn orchard() -> Orchard {
    Orchard {
        id: OrchardId(7),
        name: "My orchard".into(),
        longitude: -73.5,
        latitude: 12.25,
        reference_region: "Example Region, France".into(),
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
        is_excluded_from_watering: false,
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
