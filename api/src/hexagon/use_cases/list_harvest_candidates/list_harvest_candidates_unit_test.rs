use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, HarvestRunTarget, HarvestScheduleOwner, HarvestedPart, IdentificationStatus,
    NamedTaxon, Orchard, OrchardId, PlantIdentity, PlantIdentityId, Tree, TreeId,
};
use orchard_api::hexagon::use_cases::defer_harvest_tree::{
    HarvestTreeDeferred, defer_harvest_tree,
};
use orchard_api::hexagon::use_cases::list_harvest_candidates::{
    HarvestCandidate, HarvestCandidatesError, HarvestCandidatesRequested, list_harvest_candidates,
};
use orchard_api::hexagon::use_cases::load_active_harvest_run::load_active_harvest_run;
use orchard_api::hexagon::use_cases::record_tree_harvested::{
    TreeHarvestedEverything, record_tree_harvested,
};
use orchard_api::hexagon::use_cases::replace_plant_harvest_windows::{
    AnnualHarvestWindowChanged, OrchardHarvestWindowsReplaced, replace_orchard_harvest_windows,
};
use orchard_api::hexagon::use_cases::start_harvest_run::{
    HarvestRunStartRequested, start_harvest_run,
};

#[test]
fn list_candidates_from_prior_outcomes_without_changing_the_active_run() {
    let mut storage = storage_with_trees(vec![
        tree(PlantIdentityId(1), 5.0, true),
        tree(PlantIdentityId(1), 5.1, true),
        tree(PlantIdentityId(1), 5.2, true),
    ]);
    configure_window(
        &mut storage,
        PlantIdentityId(1),
        HarvestedPart::Fruit,
        9,
        1,
        10,
        30,
    );
    let started = start_harvest_run(
        HarvestRunStartRequested {
            orchard_id: OrchardId(7),
            target: HarvestRunTarget::All,
            harvested_parts: vec![HarvestedPart::Fruit],
            action_date: "2026-09-17".into(),
        },
        &mut storage,
    )
    .unwrap();
    record_tree_harvested(
        TreeHarvestedEverything {
            orchard_id: OrchardId(7),
            harvest_run_id: started.run_id,
            tree_id: TreeId(1),
            action_date: "2026-09-17".into(),
        },
        &mut storage,
    )
    .unwrap();
    defer_harvest_tree(
        HarvestTreeDeferred {
            orchard_id: OrchardId(7),
            harvest_run_id: started.run_id,
            tree_id: TreeId(2),
            action_date: "2026-09-17".into(),
            extend_window: None,
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(
        list(&mut storage, "2026-09-17").unwrap(),
        vec![candidate(3, 1)]
    );
    let active = load_active_harvest_run(OrchardId(7), "2026-09-17", &mut storage)
        .unwrap()
        .unwrap();
    assert_eq!(active.run_id, started.run_id);
    assert_eq!(active.current_tree.unwrap().id, TreeId(3));
    assert_eq!(
        list(&mut storage, "2026-09-24").unwrap(),
        vec![candidate(2, 1), candidate(3, 1)]
    );
}

#[test]
fn list_only_living_trees_in_a_current_fruit_window_and_validate_the_date() {
    let mut storage = storage_with_trees(vec![
        tree(PlantIdentityId(1), 5.0, true),
        tree(PlantIdentityId(1), 5.1, false),
        tree(PlantIdentityId(2), 5.2, true),
        tree(PlantIdentityId(3), 5.3, true),
    ]);
    configure_window(
        &mut storage,
        PlantIdentityId(1),
        HarvestedPart::Fruit,
        9,
        1,
        9,
        30,
    );
    configure_window(
        &mut storage,
        PlantIdentityId(2),
        HarvestedPart::Nut,
        9,
        1,
        9,
        30,
    );
    configure_window(
        &mut storage,
        PlantIdentityId(3),
        HarvestedPart::Fruit,
        10,
        1,
        10,
        30,
    );

    assert_eq!(
        list(&mut storage, "2026-09-17").unwrap(),
        vec![candidate(1, 1)]
    );
    assert_eq!(
        list(&mut storage, "2026-02-29"),
        Err(HarvestCandidatesError::InvalidActionDate)
    );
}

#[test]
fn harvesting_selected_parts_leaves_unselected_parts_available() {
    let mut storage = storage_with_trees(vec![tree(PlantIdentityId(1), 5.0, true)]);
    replace_orchard_harvest_windows(
        OrchardHarvestWindowsReplaced {
            orchard_id: OrchardId(7),
            owner: HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
            reference_region: "Example Region, France".into(),
            windows: vec![
                AnnualHarvestWindowChanged {
                    start_month: 9,
                    start_day: 1,
                    end_month: 9,
                    end_day: 30,
                    harvested_part: HarvestedPart::Flower,
                },
                AnnualHarvestWindowChanged {
                    start_month: 9,
                    start_day: 1,
                    end_month: 9,
                    end_day: 30,
                    harvested_part: HarvestedPart::Fruit,
                },
            ],
        },
        &mut storage,
    )
    .unwrap();
    let started = start_harvest_run(
        HarvestRunStartRequested {
            orchard_id: OrchardId(7),
            target: HarvestRunTarget::All,
            harvested_parts: vec![HarvestedPart::Flower],
            action_date: "2026-09-17".into(),
        },
        &mut storage,
    )
    .unwrap();
    record_tree_harvested(
        TreeHarvestedEverything {
            orchard_id: OrchardId(7),
            harvest_run_id: started.run_id,
            tree_id: TreeId(1),
            action_date: "2026-09-17".into(),
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(
        list_with_parts(&mut storage, "2026-09-17", vec![HarvestedPart::Flower]).unwrap(),
        vec![]
    );
    assert_eq!(
        list_with_parts(&mut storage, "2026-09-17", vec![HarvestedPart::Fruit]).unwrap(),
        vec![candidate(1, 1)]
    );
}

fn list(
    storage: &mut InMemoryOrchardStorage,
    action_date: &str,
) -> Result<Vec<HarvestCandidate>, HarvestCandidatesError> {
    list_with_parts(storage, action_date, vec![HarvestedPart::Fruit])
}

fn list_with_parts(
    storage: &mut InMemoryOrchardStorage,
    action_date: &str,
    harvested_parts: Vec<HarvestedPart>,
) -> Result<Vec<HarvestCandidate>, HarvestCandidatesError> {
    list_harvest_candidates(
        HarvestCandidatesRequested {
            orchard_id: OrchardId(7),
            harvested_parts,
            action_date: action_date.into(),
        },
        storage,
    )
}

fn candidate(tree_id: u64, plant_identity_id: u64) -> HarvestCandidate {
    HarvestCandidate {
        tree_id: TreeId(tree_id),
        plant_identity_id: PlantIdentityId(plant_identity_id),
    }
}

fn storage_with_trees(trees: Vec<Tree>) -> InMemoryOrchardStorage {
    InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        Orchard {
            id: OrchardId(7),
            name: "My orchard".into(),
            longitude: -73.5,
            latitude: 12.25,
            reference_region: "Example Region, France".into(),
        },
        vec![identity("Apple"), identity("Pear"), identity("Plum")],
        trees,
    )
    .0
}

fn configure_window(
    storage: &mut InMemoryOrchardStorage,
    plant_identity_id: PlantIdentityId,
    harvested_part: HarvestedPart,
    start_month: u8,
    start_day: u8,
    end_month: u8,
    end_day: u8,
) {
    replace_orchard_harvest_windows(
        OrchardHarvestWindowsReplaced {
            orchard_id: OrchardId(7),
            owner: HarvestScheduleOwner::PlantIdentity(plant_identity_id),
            reference_region: "Example Region, France".into(),
            windows: vec![AnnualHarvestWindowChanged {
                start_month,
                start_day,
                end_month,
                end_day,
                harvested_part,
            }],
        },
        storage,
    )
    .unwrap();
}

fn tree(plant_identity_id: PlantIdentityId, longitude: f64, is_alive: bool) -> Tree {
    Tree {
        legacy_source: None,
        plant_identity_id,
        cultivar_id: None,
        identification_status: IdentificationStatus::Confirmed,
        longitude,
        latitude: 12.25,
        planted_on: None,
        row_name: None,
        roles: vec!["fruit".into()],
        is_alive,
        is_in_danger: false,
        is_excluded_from_watering: false,
        reproductive_role: None,
        adult_height_meters: None,
        adult_width_meters: None,
    }
}

fn identity(name: &str) -> PlantIdentity {
    PlantIdentity {
        common_name: name.into(),
        botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
            genus: name.into(),
            species: Some("domestica".into()),
            species_is_hybrid: false,
            infraspecific: None,
            is_aggregate: false,
            cultivar_group: None,
        }),
    }
}
