use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, HarvestRunTarget, HarvestScheduleOwner, HarvestedPart, IdentificationStatus,
    NamedTaxon, Orchard, OrchardId, PlantIdentity, PlantIdentityId, Tree, TreeId,
};
use orchard_api::hexagon::use_cases::record_tree_harvested::{
    TreeHarvestedEverything, TreeHarvestedEverythingError, record_tree_harvested,
};
use orchard_api::hexagon::use_cases::replace_plant_harvest_windows::{
    AnnualHarvestWindowChanged, OrchardHarvestWindowsReplaced, replace_orchard_harvest_windows,
};
use orchard_api::hexagon::use_cases::start_harvest_run::{
    HarvestRunStartError, HarvestRunStartRequested, start_harvest_run,
};

#[test]
fn harvest_each_current_tree_then_exclude_that_period_but_allow_next_year() {
    let mut storage = storage_with_fruit_window();
    let started = start(&mut storage, "2026-09-17").unwrap();

    let after_first = record_tree_harvested(
        TreeHarvestedEverything {
            orchard_id: OrchardId(7),
            harvest_run_id: started.run_id,
            tree_id: TreeId(1),
            action_date: "2026-09-17".into(),
        },
        &mut storage,
    )
    .unwrap();
    assert_eq!(after_first.harvested_tree_count, 1);
    assert_eq!(after_first.handled_tree_count, 1);
    assert_eq!(after_first.current_tree.unwrap().id, TreeId(2));

    let completed = record_tree_harvested(
        TreeHarvestedEverything {
            orchard_id: OrchardId(7),
            harvest_run_id: started.run_id,
            tree_id: TreeId(2),
            action_date: "2026-09-17".into(),
        },
        &mut storage,
    )
    .unwrap();
    assert_eq!(completed.harvested_tree_count, 2);
    assert_eq!(completed.current_tree, None);

    replace_orchard_harvest_windows(
        OrchardHarvestWindowsReplaced {
            orchard_id: OrchardId(7),
            owner: HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
            reference_region: "Example Region, France".into(),
            windows: vec![AnnualHarvestWindowChanged {
                start_month: 9,
                start_day: 2,
                end_month: 9,
                end_day: 30,
                harvested_part: HarvestedPart::Fruit,
            }],
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(
        start(&mut storage, "2026-09-20"),
        Err(HarvestRunStartError::NoTreesCurrentlyInFruit)
    );
    assert_eq!(
        start(&mut storage, "2027-09-17").unwrap().total_tree_count,
        2
    );
}

#[test]
fn reject_harvest_when_the_live_harvest_occurrence_changed() {
    let mut storage = storage_with_fruit_window();
    let started = start(&mut storage, "2026-09-17").unwrap();
    replace_orchard_harvest_windows(
        OrchardHarvestWindowsReplaced {
            orchard_id: OrchardId(7),
            owner: HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
            reference_region: "Example Region, France".into(),
            windows: vec![AnnualHarvestWindowChanged {
                start_month: 9,
                start_day: 2,
                end_month: 9,
                end_day: 30,
                harvested_part: HarvestedPart::Fruit,
            }],
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(
        record_tree_harvested(
            TreeHarvestedEverything {
                orchard_id: OrchardId(7),
                harvest_run_id: started.run_id,
                tree_id: TreeId(1),
                action_date: "2026-09-17".into(),
            },
            &mut storage,
        ),
        Err(TreeHarvestedEverythingError::HarvestWindowChanged)
    );
}

#[test]
fn refuse_to_skip_the_current_tree_without_changing_progress() {
    let mut storage = storage_with_fruit_window();
    let started = start(&mut storage, "2026-09-17").unwrap();

    let result = record_tree_harvested(
        TreeHarvestedEverything {
            orchard_id: OrchardId(7),
            harvest_run_id: started.run_id,
            tree_id: TreeId(2),
            action_date: "2026-09-17".into(),
        },
        &mut storage,
    );

    assert_eq!(result, Err(TreeHarvestedEverythingError::TreeIsNotCurrent));
    assert_eq!(
        start(&mut storage, "2026-09-17")
            .unwrap()
            .current_tree
            .unwrap()
            .id,
        TreeId(1)
    );
}

#[test]
fn reject_retroactive_and_out_of_period_harvest_dates() {
    let mut storage = storage_with_fruit_window();
    let started = start(&mut storage, "2026-09-17").unwrap();

    for action_date in ["2026-09-16", "2026-10-01"] {
        assert_eq!(
            record_tree_harvested(
                TreeHarvestedEverything {
                    orchard_id: OrchardId(7),
                    harvest_run_id: started.run_id,
                    tree_id: TreeId(1),
                    action_date: action_date.into(),
                },
                &mut storage,
            ),
            Err(TreeHarvestedEverythingError::ActionDateOutsideRunPeriod),
            "{action_date}"
        );
    }

    assert_eq!(
        start(&mut storage, "2026-09-17")
            .unwrap()
            .current_tree
            .unwrap()
            .id,
        TreeId(1)
    );
}

fn start(
    storage: &mut InMemoryOrchardStorage,
    action_date: &str,
) -> Result<orchard_api::hexagon::use_cases::start_harvest_run::HarvestProgress, HarvestRunStartError>
{
    start_harvest_run(
        HarvestRunStartRequested {
            orchard_id: OrchardId(7),
            target: HarvestRunTarget::All,
            action_date: action_date.into(),
        },
        storage,
    )
}

fn storage_with_fruit_window() -> InMemoryOrchardStorage {
    let (mut storage, _) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        orchard(),
        vec![apple_identity()],
        vec![apple_tree(5.0), apple_tree(5.1)],
    );
    replace_orchard_harvest_windows(
        OrchardHarvestWindowsReplaced {
            orchard_id: OrchardId(7),
            owner: HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
            reference_region: "Example Region, France".into(),
            windows: vec![AnnualHarvestWindowChanged {
                start_month: 9,
                start_day: 1,
                end_month: 9,
                end_day: 30,
                harvested_part: HarvestedPart::Fruit,
            }],
        },
        &mut storage,
    )
    .unwrap();
    storage
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

fn apple_tree(longitude: f64) -> Tree {
    Tree {
        legacy_source: None,
        plant_identity_id: PlantIdentityId(1),
        cultivar_id: None,
        identification_status: IdentificationStatus::Confirmed,
        longitude,
        latitude: 12.25,
        planted_on: None,
        row_name: None,
        roles: vec!["fruit".into()],
        is_alive: true,
        is_in_danger: false,
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
