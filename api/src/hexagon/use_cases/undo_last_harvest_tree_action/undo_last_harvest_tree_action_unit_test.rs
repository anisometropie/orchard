use orchard_api::adapters::secondary::{InMemoryOrchardObserver, InMemoryOrchardStorage};
use orchard_api::hexagon::models::{
    AnnualDate, BotanicalTaxon, HarvestRunTarget, HarvestScheduleOwner, HarvestedPart,
    IdentificationStatus, NamedTaxon, Orchard, OrchardId, PlantIdentity, PlantIdentityId, Tree,
    TreeId,
};
use orchard_api::hexagon::use_cases::defer_harvest_tree::{
    HarvestTreeDeferred, defer_harvest_tree,
};
use orchard_api::hexagon::use_cases::record_tree_harvested::{
    TreeHarvestedEverything, record_tree_harvested,
};
use orchard_api::hexagon::use_cases::replace_plant_harvest_windows::{
    AnnualHarvestWindowChanged, OrchardHarvestWindowsReplaced, replace_orchard_harvest_windows,
};
use orchard_api::hexagon::use_cases::start_harvest_run::{
    HarvestRunStartRequested, start_harvest_run,
};
use orchard_api::hexagon::use_cases::undo_last_harvest_tree_action::{
    LastHarvestTreeActionUndoError, LastHarvestTreeActionUndone, undo_last_harvest_tree_action,
};

#[test]
fn undo_the_previous_tree_and_choose_its_outcome_again() {
    let (mut storage, _) = harvest_storage(30, 2);
    let started = start(&mut storage);
    let advanced =
        record_tree_harvested(harvested(started.run_id, TreeId(1)), &mut storage).unwrap();
    assert_eq!(advanced.current_tree.unwrap().id, TreeId(2));

    let restored = undo(started.run_id, &mut storage).unwrap();
    assert_eq!(restored.current_tree.unwrap().id, TreeId(1));
    assert_eq!(restored.handled_tree_count, 0);

    let corrected = defer_harvest_tree(
        HarvestTreeDeferred {
            orchard_id: OrchardId(7),
            harvest_run_id: started.run_id,
            tree_id: TreeId(1),
            action_date: "2026-09-17".into(),
            extend_window: None,
        },
        &mut storage,
    )
    .unwrap();
    assert_eq!(corrected.current_tree.unwrap().id, TreeId(2));
    assert_eq!(corrected.deferred_tree_count, 1);
}

#[test]
fn undo_the_last_tree_reopens_the_tour_and_reverses_its_shared_extension() {
    let (mut storage, observer) = harvest_storage(20, 1);
    let started = start(&mut storage);
    let completed = defer_harvest_tree(
        HarvestTreeDeferred {
            orchard_id: OrchardId(7),
            harvest_run_id: started.run_id,
            tree_id: TreeId(1),
            action_date: "2026-09-17".into(),
            extend_window: Some(true),
        },
        &mut storage,
    )
    .unwrap();
    assert!(completed.current_tree.is_none());
    assert_eq!(window_end(&observer), AnnualDate { month: 9, day: 27 });

    let restored = undo(started.run_id, &mut storage).unwrap();
    assert_eq!(restored.current_tree.unwrap().id, TreeId(1));
    assert_eq!(restored.route[0].period.end.to_string(), "2026-09-20");
    assert_eq!(window_end(&observer), AnnualDate { month: 9, day: 20 });

    let corrected =
        record_tree_harvested(harvested(started.run_id, TreeId(1)), &mut storage).unwrap();
    assert!(corrected.current_tree.is_none());
    assert_eq!(corrected.harvested_tree_count, 1);
}

#[test]
fn undo_every_shared_part_extension_from_the_previous_tree_action() {
    let (mut storage, observer) = harvest_storage(20, 1);
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
                    end_day: 20,
                    harvested_part: HarvestedPart::Cone,
                },
                AnnualHarvestWindowChanged {
                    start_month: 9,
                    start_day: 1,
                    end_month: 9,
                    end_day: 22,
                    harvested_part: HarvestedPart::Flower,
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
            harvested_parts: vec![HarvestedPart::Cone, HarvestedPart::Flower],
            action_date: "2026-09-17".into(),
        },
        &mut storage,
    )
    .unwrap();
    defer_harvest_tree(
        HarvestTreeDeferred {
            orchard_id: OrchardId(7),
            harvest_run_id: started.run_id,
            tree_id: TreeId(1),
            action_date: "2026-09-17".into(),
            extend_window: Some(true),
        },
        &mut storage,
    )
    .unwrap();

    undo(started.run_id, &mut storage).unwrap();

    let windows = observer.orchard_harvest_windows(
        OrchardId(7),
        HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
    );
    assert_eq!(windows[0].end, AnnualDate { month: 9, day: 20 });
    assert_eq!(windows[1].end, AnnualDate { month: 9, day: 22 });
}

#[test]
fn undo_only_the_previous_tree_and_keep_an_earlier_tree_extension() {
    let (mut storage, observer) = harvest_storage(20, 2);
    let started = start(&mut storage);
    let after_first = defer_harvest_tree(
        HarvestTreeDeferred {
            orchard_id: OrchardId(7),
            harvest_run_id: started.run_id,
            tree_id: TreeId(1),
            action_date: "2026-09-17".into(),
            extend_window: Some(true),
        },
        &mut storage,
    )
    .unwrap();
    assert_eq!(after_first.current_tree.unwrap().id, TreeId(2));

    let completed =
        record_tree_harvested(harvested(started.run_id, TreeId(2)), &mut storage).unwrap();
    assert!(completed.current_tree.is_none());

    let restored = undo(started.run_id, &mut storage).unwrap();
    assert_eq!(restored.current_tree.unwrap().id, TreeId(2));
    assert_eq!(restored.deferred_tree_count, 1);
    assert_eq!(window_end(&observer), AnnualDate { month: 9, day: 27 });
}

#[test]
fn declining_the_shared_extension_finishes_the_tree_for_the_window_and_can_be_undone() {
    let (mut storage, observer) = harvest_storage(20, 2);
    let started = start(&mut storage);

    let advanced = defer_harvest_tree(
        HarvestTreeDeferred {
            orchard_id: OrchardId(7),
            harvest_run_id: started.run_id,
            tree_id: TreeId(1),
            action_date: "2026-09-17".into(),
            extend_window: Some(false),
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(advanced.current_tree.unwrap().id, TreeId(2));
    assert_eq!(advanced.handled_tree_count, 1);
    assert_eq!(advanced.done_for_window_tree_count, 1);
    assert_eq!(advanced.deferred_tree_count, 0);
    assert_eq!(advanced.route[0].period.end.to_string(), "2026-09-20");
    assert_eq!(window_end(&observer), AnnualDate { month: 9, day: 20 });

    let restored = undo(started.run_id, &mut storage).unwrap();
    assert_eq!(restored.current_tree.unwrap().id, TreeId(1));
    assert_eq!(restored.route[0].period.end.to_string(), "2026-09-20");
    assert_eq!(restored.done_for_window_tree_count, 0);
    assert_eq!(restored.deferred_tree_count, 0);
    assert_eq!(window_end(&observer), AnnualDate { month: 9, day: 20 });
}

#[test]
fn reject_a_second_undo_without_a_new_tree_action() {
    let (mut storage, _) = harvest_storage(30, 2);
    let started = start(&mut storage);
    record_tree_harvested(harvested(started.run_id, TreeId(1)), &mut storage).unwrap();
    undo(started.run_id, &mut storage).unwrap();

    assert_eq!(
        undo(started.run_id, &mut storage),
        Err(LastHarvestTreeActionUndoError::NoHarvestTreeActionToUndo)
    );
}

fn harvested(
    run_id: orchard_api::hexagon::models::HarvestRunId,
    tree_id: TreeId,
) -> TreeHarvestedEverything {
    TreeHarvestedEverything {
        orchard_id: OrchardId(7),
        harvest_run_id: run_id,
        tree_id,
        action_date: "2026-09-17".into(),
    }
}

fn undo(
    run_id: orchard_api::hexagon::models::HarvestRunId,
    storage: &mut InMemoryOrchardStorage,
) -> Result<
    orchard_api::hexagon::use_cases::start_harvest_run::HarvestProgress,
    LastHarvestTreeActionUndoError,
> {
    undo_last_harvest_tree_action(
        LastHarvestTreeActionUndone {
            orchard_id: OrchardId(7),
            harvest_run_id: run_id,
            action_date: "2026-09-17".into(),
        },
        storage,
    )
}

fn start(
    storage: &mut InMemoryOrchardStorage,
) -> orchard_api::hexagon::use_cases::start_harvest_run::HarvestProgress {
    start_harvest_run(
        HarvestRunStartRequested {
            orchard_id: OrchardId(7),
            target: HarvestRunTarget::All,
            harvested_parts: vec![HarvestedPart::Fruit],
            action_date: "2026-09-17".into(),
        },
        storage,
    )
    .unwrap()
}

fn window_end(observer: &InMemoryOrchardObserver) -> AnnualDate {
    observer.orchard_harvest_windows(
        OrchardId(7),
        HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
    )[0]
    .end
}

fn harvest_storage(
    end_day: u8,
    tree_count: u64,
) -> (InMemoryOrchardStorage, InMemoryOrchardObserver) {
    let (mut storage, observer) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        Orchard {
            id: OrchardId(7),
            name: "Example orchard".into(),
            longitude: -73.5,
            latitude: 12.25,
            reference_region: "Example Region, France".into(),
        },
        vec![PlantIdentity {
            common_name: "Apple".into(),
            botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
                genus: "Malus".into(),
                species: Some("domestica".into()),
                species_is_hybrid: false,
                infraspecific: None,
                is_aggregate: false,
                cultivar_group: None,
            }),
        }],
        (0..tree_count)
            .map(|index| Tree {
                legacy_source: None,
                plant_identity_id: PlantIdentityId(1),
                cultivar_id: None,
                identification_status: IdentificationStatus::Confirmed,
                longitude: -73.5 + index as f64 / 100.0,
                latitude: 12.25,
                planted_on: None,
                row_name: None,
                roles: vec!["fruit".into()],
                is_alive: true,
                is_in_danger: false,
                is_excluded_from_watering: false,
                reproductive_role: None,
                adult_height_meters: None,
                adult_width_meters: None,
            })
            .collect(),
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
                end_day,
                harvested_part: HarvestedPart::Fruit,
            }],
        },
        &mut storage,
    )
    .unwrap();
    (storage, observer)
}
