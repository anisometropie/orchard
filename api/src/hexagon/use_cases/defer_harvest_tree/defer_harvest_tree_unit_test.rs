use orchard_api::adapters::secondary::{InMemoryOrchardObserver, InMemoryOrchardStorage};
use orchard_api::hexagon::models::{
    AnnualDate, BotanicalTaxon, HarvestRunTarget, HarvestScheduleOwner, HarvestedPart,
    IdentificationStatus, NamedTaxon, Orchard, OrchardId, PlantIdentity, PlantIdentityId, Tree,
    TreeId,
};
use orchard_api::hexagon::use_cases::defer_harvest_tree::{
    HarvestTreeDeferralError, HarvestTreeDeferred, HarvestWindowExtensionProposal,
    defer_harvest_tree,
};
use orchard_api::hexagon::use_cases::record_tree_harvested::{
    TreeHarvestedEverything, record_tree_harvested,
};
use orchard_api::hexagon::use_cases::replace_plant_harvest_windows::{
    AnnualHarvestWindowChanged, OrchardHarvestWindowsReplaced, replace_orchard_harvest_windows,
};
use orchard_api::hexagon::use_cases::start_harvest_run::{
    HarvestRunStartError, HarvestRunStartRequested, start_harvest_run,
};

#[test]
fn defer_for_seven_days_then_propose_the_tree_again() {
    let (mut storage, _) = harvest_storage(30);
    let started = start(&mut storage, "2026-09-17").unwrap();

    let deferred = defer_harvest_tree(
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

    assert_eq!(deferred.deferred_tree_count, 1);
    assert_eq!(deferred.current_tree, None);
    assert_eq!(
        start(&mut storage, "2026-09-23"),
        Err(HarvestRunStartError::NoTreesCurrentlyAvailable)
    );
    assert_eq!(
        start(&mut storage, "2026-09-24").unwrap().total_tree_count,
        1
    );
}

#[test]
fn re_propose_a_due_tree_while_the_original_tour_is_still_active() {
    let (mut storage, _) = harvest_storage_with_tree_count(30, 2);
    let started = start(&mut storage, "2026-09-17").unwrap();

    let deferred = defer_harvest_tree(
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
    assert_eq!(deferred.current_tree.unwrap().id, TreeId(2));

    let resumed = start(&mut storage, "2026-09-24").unwrap();
    assert_eq!(resumed.run_id, started.run_id);
    assert_eq!(resumed.current_tree.as_ref().unwrap().id, TreeId(1));
    assert_eq!(resumed.deferred_tree_count, 0);

    assert_eq!(
        defer_harvest_tree(
            HarvestTreeDeferred {
                orchard_id: OrchardId(7),
                harvest_run_id: started.run_id,
                tree_id: TreeId(1),
                action_date: "2026-09-24".into(),
                extend_window: None,
            },
            &mut storage,
        ),
        Err(HarvestTreeDeferralError::WindowExtensionRequired(
            HarvestWindowExtensionProposal {
                current_end: orchard_api::hexagon::models::HarvestDate::new(2026, 9, 30).unwrap(),
                proposed_end: orchard_api::hexagon::models::HarvestDate::new(2026, 10, 7).unwrap(),
            }
        ))
    );
    let deferred_again = defer_harvest_tree(
        HarvestTreeDeferred {
            orchard_id: OrchardId(7),
            harvest_run_id: started.run_id,
            tree_id: TreeId(1),
            action_date: "2026-09-24".into(),
            extend_window: Some(true),
        },
        &mut storage,
    )
    .unwrap();
    assert_eq!(deferred_again.current_tree.unwrap().id, TreeId(2));
    assert_eq!(deferred_again.deferred_tree_count, 1);
    assert_eq!(deferred_again.route[0].period.end.to_string(), "2026-10-07");
}

#[test]
fn require_confirmation_then_extend_the_shared_window_and_defer_atomically() {
    let (mut storage, observer) = harvest_storage(24);
    let started = start(&mut storage, "2026-09-17").unwrap();
    let request = || HarvestTreeDeferred {
        orchard_id: OrchardId(7),
        harvest_run_id: started.run_id,
        tree_id: TreeId(1),
        action_date: "2026-09-17".into(),
        extend_window: None,
    };

    assert_eq!(
        defer_harvest_tree(request(), &mut storage),
        Err(HarvestTreeDeferralError::WindowExtensionRequired(
            HarvestWindowExtensionProposal {
                current_end: orchard_api::hexagon::models::HarvestDate::new(2026, 9, 24).unwrap(),
                proposed_end: orchard_api::hexagon::models::HarvestDate::new(2026, 10, 1).unwrap(),
            }
        ))
    );

    let deferred = defer_harvest_tree(
        HarvestTreeDeferred {
            extend_window: Some(true),
            ..request()
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(deferred.deferred_tree_count, 1);
    assert_eq!(deferred.route[0].period.end.to_string(), "2026-10-01");
    assert_eq!(
        observer.orchard_harvest_windows(
            OrchardId(7),
            HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
        )[0]
        .end,
        AnnualDate { month: 10, day: 1 }
    );
}

#[test]
fn declining_the_extension_finishes_the_tree_for_this_window_and_returns_it_next_year() {
    let (mut storage, observer) = harvest_storage(20);
    let started = start(&mut storage, "2026-09-17").unwrap();
    let finished_for_window = defer_harvest_tree(
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
    assert!(finished_for_window.current_tree.is_none());
    assert_eq!(finished_for_window.handled_tree_count, 1);
    assert_eq!(finished_for_window.done_for_window_tree_count, 1);
    assert_eq!(finished_for_window.deferred_tree_count, 0);
    assert_eq!(
        finished_for_window.route[0].period.end.to_string(),
        "2026-09-20"
    );
    assert_eq!(
        start(&mut storage, "2026-09-18"),
        Err(HarvestRunStartError::NoTreesCurrentlyAvailable)
    );
    let next_year = start(&mut storage, "2027-09-17").unwrap();
    assert_eq!(next_year.current_tree.as_ref().unwrap().id, TreeId(1));
    assert_eq!(
        observer.orchard_harvest_windows(
            OrchardId(7),
            HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
        )[0]
        .end,
        AnnualDate { month: 9, day: 20 }
    );
}

#[test]
fn reconcile_the_shared_extension_when_later_trees_are_reached() {
    let (mut storage, _) = harvest_storage_with_tree_count(24, 2);
    let started = start(&mut storage, "2026-09-17").unwrap();
    let deferred = defer_harvest_tree(
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
    assert_eq!(deferred.current_tree.unwrap().id, TreeId(2));

    let revisited = record_tree_harvested(
        TreeHarvestedEverything {
            orchard_id: OrchardId(7),
            harvest_run_id: started.run_id,
            tree_id: TreeId(1),
            action_date: "2026-09-25".into(),
        },
        &mut storage,
    )
    .unwrap();
    assert_eq!(revisited.current_tree.unwrap().id, TreeId(2));

    let completed = record_tree_harvested(
        TreeHarvestedEverything {
            orchard_id: OrchardId(7),
            harvest_run_id: started.run_id,
            tree_id: TreeId(2),
            action_date: "2026-10-01".into(),
        },
        &mut storage,
    )
    .unwrap();
    assert_eq!(completed.current_tree, None);
    assert_eq!(completed.harvested_tree_count, 2);
    assert_eq!(completed.route[1].period.end.to_string(), "2026-10-01");
}

#[test]
fn use_the_shortened_live_window_when_deciding_whether_to_extend() {
    let (mut storage, observer) = harvest_storage(30);
    let started = start(&mut storage, "2026-09-17").unwrap();
    replace_orchard_harvest_windows(
        OrchardHarvestWindowsReplaced {
            orchard_id: OrchardId(7),
            owner: HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
            reference_region: "Example Region, France".into(),
            windows: vec![AnnualHarvestWindowChanged {
                start_month: 9,
                start_day: 1,
                end_month: 9,
                end_day: 20,
                harvested_part: HarvestedPart::Fruit,
            }],
        },
        &mut storage,
    )
    .unwrap();
    let request = || HarvestTreeDeferred {
        orchard_id: OrchardId(7),
        harvest_run_id: started.run_id,
        tree_id: TreeId(1),
        action_date: "2026-09-17".into(),
        extend_window: None,
    };

    assert_eq!(
        defer_harvest_tree(request(), &mut storage),
        Err(HarvestTreeDeferralError::WindowExtensionRequired(
            HarvestWindowExtensionProposal {
                current_end: orchard_api::hexagon::models::HarvestDate::new(2026, 9, 20).unwrap(),
                proposed_end: orchard_api::hexagon::models::HarvestDate::new(2026, 9, 27).unwrap(),
            }
        ))
    );

    let progress = defer_harvest_tree(
        HarvestTreeDeferred {
            extend_window: Some(true),
            ..request()
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(progress.deferred_tree_count, 1);
    assert_eq!(progress.route[0].period.end.to_string(), "2026-09-30");
    assert_eq!(
        observer.orchard_harvest_windows(
            OrchardId(7),
            HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
        )[0]
        .end,
        AnnualDate { month: 9, day: 27 }
    );
}

#[test]
fn reject_deferral_when_the_live_harvest_occurrence_changed() {
    let (mut storage, _) = harvest_storage(30);
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
        defer_harvest_tree(
            HarvestTreeDeferred {
                orchard_id: OrchardId(7),
                harvest_run_id: started.run_id,
                tree_id: TreeId(1),
                action_date: "2026-09-17".into(),
                extend_window: None,
            },
            &mut storage,
        ),
        Err(HarvestTreeDeferralError::HarvestWindowChanged)
    );
}

#[test]
fn reject_deferral_when_the_live_harvest_occurrence_was_removed() {
    let (mut storage, _) = harvest_storage(30);
    let started = start(&mut storage, "2026-09-17").unwrap();
    replace_orchard_harvest_windows(
        OrchardHarvestWindowsReplaced {
            orchard_id: OrchardId(7),
            owner: HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
            reference_region: "Example Region, France".into(),
            windows: vec![],
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(
        defer_harvest_tree(
            HarvestTreeDeferred {
                orchard_id: OrchardId(7),
                harvest_run_id: started.run_id,
                tree_id: TreeId(1),
                action_date: "2026-09-17".into(),
                extend_window: None,
            },
            &mut storage,
        ),
        Err(HarvestTreeDeferralError::HarvestWindowChanged)
    );
}

#[test]
fn reject_retroactive_and_out_of_period_deferral_dates() {
    let (mut storage, _) = harvest_storage(30);
    let started = start(&mut storage, "2026-09-17").unwrap();

    for action_date in ["2026-09-16", "2026-10-01"] {
        assert_eq!(
            defer_harvest_tree(
                HarvestTreeDeferred {
                    orchard_id: OrchardId(7),
                    harvest_run_id: started.run_id,
                    tree_id: TreeId(1),
                    action_date: action_date.into(),
                    extend_window: None,
                },
                &mut storage,
            ),
            Err(HarvestTreeDeferralError::ActionDateOutsideRunPeriod),
            "{action_date}"
        );
    }

    assert_eq!(
        start(&mut storage, "2026-09-17").unwrap().run_id,
        started.run_id
    );
}

#[test]
fn reject_an_extension_that_the_annual_window_cannot_represent() {
    let (mut storage, observer) = harvest_storage(30);
    replace_orchard_harvest_windows(
        OrchardHarvestWindowsReplaced {
            orchard_id: OrchardId(7),
            owner: HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
            reference_region: "Example Region, France".into(),
            windows: vec![AnnualHarvestWindowChanged {
                start_month: 1,
                start_day: 1,
                end_month: 12,
                end_day: 30,
                harvested_part: HarvestedPart::Fruit,
            }],
        },
        &mut storage,
    )
    .unwrap();
    let started = start(&mut storage, "2026-12-29").unwrap();

    assert_eq!(
        defer_harvest_tree(
            HarvestTreeDeferred {
                orchard_id: OrchardId(7),
                harvest_run_id: started.run_id,
                tree_id: TreeId(1),
                action_date: "2026-12-29".into(),
                extend_window: Some(true),
            },
            &mut storage,
        ),
        Err(HarvestTreeDeferralError::HarvestWindowCouldNotBeExtended)
    );
    assert_eq!(
        observer.orchard_harvest_windows(
            OrchardId(7),
            HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
        )[0]
        .end,
        AnnualDate { month: 12, day: 30 }
    );
}

#[test]
fn defer_all_selected_parts_together_and_extend_each_short_window() {
    let (mut storage, observer) = harvest_storage(30);
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
    let request = |extend_window| HarvestTreeDeferred {
        orchard_id: OrchardId(7),
        harvest_run_id: started.run_id,
        tree_id: TreeId(1),
        action_date: "2026-09-17".into(),
        extend_window,
    };

    assert_eq!(
        defer_harvest_tree(request(None), &mut storage),
        Err(HarvestTreeDeferralError::WindowExtensionRequired(
            HarvestWindowExtensionProposal {
                current_end: orchard_api::hexagon::models::HarvestDate::new(2026, 9, 20).unwrap(),
                proposed_end: orchard_api::hexagon::models::HarvestDate::new(2026, 9, 27).unwrap(),
            }
        ))
    );
    let deferred = defer_harvest_tree(request(Some(true)), &mut storage).unwrap();

    assert_eq!(deferred.route[0].period.end.to_string(), "2026-09-29");
    let windows = observer.orchard_harvest_windows(
        OrchardId(7),
        HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
    );
    assert_eq!(windows[0].end, AnnualDate { month: 9, day: 27 });
    assert_eq!(windows[1].end, AnnualDate { month: 9, day: 29 });
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
            harvested_parts: vec![HarvestedPart::Fruit],
            action_date: action_date.into(),
        },
        storage,
    )
}

fn harvest_storage(end_day: u8) -> (InMemoryOrchardStorage, InMemoryOrchardObserver) {
    harvest_storage_with_tree_count(end_day, 1)
}

fn harvest_storage_with_tree_count(
    end_day: u8,
    tree_count: u64,
) -> (InMemoryOrchardStorage, InMemoryOrchardObserver) {
    let (mut storage, observer) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        Orchard {
            id: OrchardId(7),
            name: "My orchard".into(),
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
