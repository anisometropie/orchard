use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, HarvestDate, HarvestPeriod, HarvestRunTarget, HarvestRunTree,
    HarvestTreeOutcome, HarvestedPart, IdentificationStatus, LegacyTreeSource, NamedTaxon,
    OrchardId, PlantCultivar, PlantCultivarId, PlantIdentification, PlantIdentity, PlantIdentityId,
    PlantIdentityReference, Tree, TreeId,
};
use orchard_api::hexagon::ports::{OrchardStorage, OrchardStorageError};

#[path = "../../../../tests/support/watering_run_fixture.rs"]
mod watering_run_fixture;

#[test]
fn harvest_window_edits_honor_the_source_preservation_contract() {
    #[path = "../../../../tests/support/harvest_window_edit_contract.rs"]
    mod harvest_window_edit_contract;
    let (mut storage, _) = InMemoryOrchardStorage::new();
    harvest_window_edit_contract::assert_harvest_window_edits_preserve_sources(&mut storage);
}

#[test]
fn skipped_watering_progress_honors_the_storage_contract() {
    #[path = "../../../../tests/support/watering_skip_contract.rs"]
    mod watering_skip_contract;
    let (mut storage, _) = watering_run_fixture::storage();
    let run_id = orchard_api::hexagon::models::WateringRunId(1);
    storage
        .transaction(|orchard| orchard.mark_watering_tree_watered(run_id, TreeId(1)))
        .unwrap();
    watering_skip_contract::assert_skips_are_atomic_and_distinct_from_watering(
        &mut storage,
        run_id,
        TreeId(1),
        TreeId(2),
    );
}

#[test]
fn paused_watering_progress_honors_the_storage_contract() {
    #[path = "../../../../tests/support/watering_pause_contract.rs"]
    mod watering_pause_contract;
    let (mut storage, _) = watering_run_fixture::storage();
    let run_id = orchard_api::hexagon::models::WateringRunId(1);
    storage
        .transaction(|orchard| orchard.mark_watering_tree_watered(run_id, TreeId(1)))
        .unwrap();
    watering_pause_contract::assert_pause_preserves_progress_and_rolls_back(
        &mut storage,
        OrchardId(7),
        run_id,
    );
}

#[test]
fn reject_missing_identity() {
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::new();
    let save_result = orchard_storage.transaction(|orchard| {
        orchard.save_tree(tree(
            PlantIdentityReference {
                plant_identity_id: PlantIdentityId(1),
                cultivar_id: None,
            },
            64,
        ))
    });

    assert_eq!(save_result, Err(OrchardStorageError::TreeCouldNotBeSaved));
    assert_eq!(observed_orchard.plant_identities(), vec![]);
    assert_eq!(observed_orchard.trees(), vec![]);
}

#[test]
fn reject_duplicate_feature() {
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::new();
    let boskoop = plant_identity();

    orchard_storage
        .transaction(|orchard| {
            let plant_identity_id = orchard.resolve_plant_identification(boskoop.clone())?;
            orchard.save_tree(tree(plant_identity_id, 64))
        })
        .unwrap();

    let save_result = orchard_storage.transaction(|orchard| {
        let plant_identity_id = orchard.resolve_plant_identification(boskoop)?;
        orchard.save_tree(tree(plant_identity_id, 64))
    });

    assert_eq!(save_result, Err(OrchardStorageError::TreeCouldNotBeSaved));
    assert_eq!(observed_orchard.plant_identities().len(), 1);
    assert_eq!(observed_orchard.trees().len(), 1);
}

#[test]
fn reject_nested_transaction() {
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::new();
    let result = orchard_storage
        .transaction(|orchard| orchard.transaction::<_, OrchardStorageError>(|_| Ok(())));

    assert_eq!(
        result,
        Err(OrchardStorageError::AtomicOperationCouldNotBegin)
    );
    assert_eq!(observed_orchard.plant_identities(), vec![]);
    assert_eq!(observed_orchard.trees(), vec![]);
}

#[test]
fn staged_tree_is_visible_inside_transaction_but_not_to_observers() {
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::new();

    orchard_storage
        .transaction(|orchard| {
            let plant_identity_id = orchard.resolve_plant_identification(plant_identity())?;
            orchard.save_tree(tree(plant_identity_id, 64))?;

            assert!(orchard.is_legacy_tree_already_imported(64)?);
            assert_eq!(observed_orchard.trees(), vec![]);
            Ok::<_, OrchardStorageError>(())
        })
        .unwrap();

    assert_eq!(
        observed_orchard.trees(),
        vec![tree(
            PlantIdentityReference {
                plant_identity_id: PlantIdentityId(1),
                cultivar_id: Some(PlantCultivarId(1)),
            },
            64,
        )]
    );
    assert_eq!(
        orchard_storage.trees().unwrap()[0].plant_cultivar,
        Some(PlantCultivar {
            cultivar: "Boskoop".into(),
            trade_name: None,
        })
    );
}

#[test]
fn keep_completed_and_resolved_harvest_history_immutable() {
    let (mut orchard_storage, _) = InMemoryOrchardStorage::new();
    let started_on = HarvestDate::new(2026, 9, 17).unwrap();
    let period = HarvestPeriod {
        start: HarvestDate::new(2026, 9, 1).unwrap(),
        end: HarvestDate::new(2026, 9, 24).unwrap(),
    };
    let run_id = orchard_storage
        .transaction(|orchard| {
            orchard.create_harvest_run(
                OrchardId(1),
                HarvestRunTarget::All,
                &[HarvestedPart::Fruit],
                started_on,
                &[
                    HarvestRunTree {
                        tree_id: TreeId(1),
                        harvested_parts: vec![HarvestedPart::Fruit],
                        period,
                        outcome: None,
                    },
                    HarvestRunTree {
                        tree_id: TreeId(2),
                        harvested_parts: vec![HarvestedPart::Fruit],
                        period,
                        outcome: None,
                    },
                ],
            )
        })
        .unwrap();

    assert_eq!(
        orchard_storage.transaction(|orchard| {
            orchard.extend_harvest_run_tree_period(
                run_id,
                TreeId(1),
                HarvestDate::new(2026, 9, 23).unwrap(),
                started_on,
            )
        }),
        Err(OrchardStorageError::HarvestRunCouldNotBeChanged)
    );

    let outside_window = HarvestTreeOutcome::HarvestedEverything {
        harvested_on: HarvestDate::new(2026, 8, 31).unwrap(),
    };
    assert_eq!(
        orchard_storage.transaction(|orchard| {
            orchard.record_harvest_tree_outcome(run_id, TreeId(1), outside_window)
        }),
        Err(OrchardStorageError::HarvestRunCouldNotBeChanged)
    );
    let retry_after_window = HarvestTreeOutcome::Deferred {
        deferred_on: started_on,
        retry_on: HarvestDate::new(2026, 10, 1).unwrap(),
    };
    assert_eq!(
        orchard_storage.transaction(|orchard| {
            orchard.record_harvest_tree_outcome(run_id, TreeId(1), retry_after_window)
        }),
        Err(OrchardStorageError::HarvestRunCouldNotBeChanged)
    );

    let harvested = HarvestTreeOutcome::HarvestedEverything {
        harvested_on: started_on,
    };
    orchard_storage
        .transaction(|orchard| orchard.record_harvest_tree_outcome(run_id, TreeId(1), harvested))
        .unwrap();
    assert_eq!(
        orchard_storage.transaction(|orchard| orchard.delete_harvest_run(run_id)),
        Err(OrchardStorageError::HarvestRunCouldNotBeDeleted)
    );

    orchard_storage
        .transaction(|orchard| orchard.complete_harvest_run(run_id))
        .unwrap();
    assert_eq!(
        orchard_storage.transaction(|orchard| {
            orchard.record_harvest_tree_outcome(run_id, TreeId(2), harvested)
        }),
        Err(OrchardStorageError::HarvestRunCouldNotBeChanged)
    );
    assert_eq!(
        orchard_storage.transaction(|orchard| {
            orchard.extend_harvest_run_tree_period(
                run_id,
                TreeId(2),
                HarvestDate::new(2026, 10, 1).unwrap(),
                started_on,
            )
        }),
        Err(OrchardStorageError::HarvestRunCouldNotBeChanged)
    );
    assert_eq!(
        orchard_storage.transaction(|orchard| orchard.delete_harvest_run(run_id)),
        Err(OrchardStorageError::HarvestRunCouldNotBeDeleted)
    );

    let stored = orchard_storage.harvest_run(run_id).unwrap().unwrap();
    assert!(stored.completed);
    assert_eq!(stored.ordered_trees[0].outcome, Some(harvested));
    assert_eq!(stored.ordered_trees[1].outcome, None);
    assert_eq!(stored.ordered_trees[0].period, period);
    assert_eq!(stored.ordered_trees[1].period, period);
}

#[test]
fn allocate_harvest_run_ids_above_surviving_runs_after_deletion() {
    let (mut orchard_storage, _) = InMemoryOrchardStorage::new();
    let started_on = HarvestDate::new(2026, 9, 17).unwrap();
    let first_run_id = orchard_storage
        .transaction(|orchard| {
            orchard.create_harvest_run(
                OrchardId(1),
                HarvestRunTarget::All,
                &[HarvestedPart::Fruit],
                started_on,
                &[],
            )
        })
        .unwrap();
    let surviving_run_id = orchard_storage
        .transaction(|orchard| {
            orchard.create_harvest_run(
                OrchardId(2),
                HarvestRunTarget::All,
                &[HarvestedPart::Fruit],
                started_on,
                &[],
            )
        })
        .unwrap();
    orchard_storage
        .transaction(|orchard| orchard.complete_harvest_run(surviving_run_id))
        .unwrap();
    orchard_storage
        .transaction(|orchard| orchard.delete_harvest_run(first_run_id))
        .unwrap();

    let replacement_run_id = orchard_storage
        .transaction(|orchard| {
            orchard.create_harvest_run(
                OrchardId(1),
                HarvestRunTarget::All,
                &[HarvestedPart::Fruit],
                started_on,
                &[],
            )
        })
        .unwrap();

    assert_eq!(first_run_id.0, 1);
    assert_eq!(surviving_run_id.0, 2);
    assert_eq!(replacement_run_id.0, 3);
    assert_eq!(
        orchard_storage
            .harvest_run(surviving_run_id)
            .unwrap()
            .unwrap()
            .orchard_id,
        OrchardId(2)
    );
    assert_eq!(
        orchard_storage
            .harvest_run(replacement_run_id)
            .unwrap()
            .unwrap()
            .orchard_id,
        OrchardId(1)
    );
}

#[test]
fn use_the_latest_staged_period_extension_for_outcome_validation() {
    let (mut orchard_storage, _) = InMemoryOrchardStorage::new();
    let started_on = HarvestDate::new(2026, 9, 25).unwrap();
    let original_period = HarvestPeriod {
        start: HarvestDate::new(2026, 9, 1).unwrap(),
        end: HarvestDate::new(2026, 9, 30).unwrap(),
    };
    let extended_end = HarvestDate::new(2026, 10, 7).unwrap();
    let run_id = orchard_storage
        .transaction(|orchard| {
            orchard.create_harvest_run(
                OrchardId(1),
                HarvestRunTarget::All,
                &[HarvestedPart::Fruit],
                started_on,
                &[HarvestRunTree {
                    tree_id: TreeId(1),
                    harvested_parts: vec![HarvestedPart::Fruit],
                    period: original_period,
                    outcome: None,
                }],
            )
        })
        .unwrap();

    assert_eq!(
        orchard_storage.transaction(|orchard| {
            orchard.extend_harvest_run_tree_period(run_id, TreeId(1), extended_end, started_on)?;
            orchard.extend_harvest_run_tree_period(
                run_id,
                TreeId(1),
                HarvestDate::new(2026, 10, 6).unwrap(),
                started_on,
            )
        }),
        Err(OrchardStorageError::HarvestRunCouldNotBeChanged)
    );
    assert_eq!(
        orchard_storage
            .harvest_run(run_id)
            .unwrap()
            .unwrap()
            .ordered_trees[0]
            .period,
        original_period
    );

    let deferred = HarvestTreeOutcome::Deferred {
        deferred_on: started_on,
        retry_on: HarvestDate::new(2026, 10, 2).unwrap(),
    };
    orchard_storage
        .transaction(|orchard| {
            orchard.extend_harvest_run_tree_period(run_id, TreeId(1), extended_end, started_on)?;
            orchard.record_harvest_tree_outcome(run_id, TreeId(1), deferred)
        })
        .unwrap();

    let stored_tree = orchard_storage
        .harvest_run(run_id)
        .unwrap()
        .unwrap()
        .ordered_trees[0]
        .clone();
    assert_eq!(stored_tree.period.end, extended_end);
    assert_eq!(stored_tree.outcome, Some(deferred));

    let revisited = HarvestTreeOutcome::HarvestedEverything {
        harvested_on: HarvestDate::new(2026, 10, 2).unwrap(),
    };
    orchard_storage
        .transaction(|orchard| orchard.record_harvest_tree_outcome(run_id, TreeId(1), revisited))
        .unwrap();
    assert_eq!(
        orchard_storage
            .harvest_run(run_id)
            .unwrap()
            .unwrap()
            .ordered_trees[0]
            .outcome,
        Some(revisited)
    );
}

fn plant_identity() -> PlantIdentification {
    PlantIdentification {
        plant_identity: PlantIdentity {
            common_name: "Kiwi".into(),
            botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
                genus: "Actinidia".into(),
                species: Some("deliciosa".into()),
                species_is_hybrid: false,
                infraspecific: None,
                is_aggregate: false,
                cultivar_group: None,
            }),
        },
        plant_cultivar: Some(PlantCultivar {
            cultivar: "Boskoop".into(),
            trade_name: None,
        }),
        identification_status: IdentificationStatus::Confirmed,
    }
}

fn tree(plant_identity: PlantIdentityReference, legacy_feature_id: u32) -> Tree {
    Tree {
        legacy_source: Some(LegacyTreeSource {
            feature_id: legacy_feature_id,
            name: "Kiwi ‘Boskoop’".into(),
            latin_name: "Actinidia deliciosa ‘Boskoop’".into(),
            legacy_identification: None,
            source_url: None,
        }),
        plant_identity_id: plant_identity.plant_identity_id,
        cultivar_id: plant_identity.cultivar_id,
        identification_status: IdentificationStatus::Confirmed,
        longitude: 0.81,
        latitude: 0.68,
        planted_on: None,
        row_name: None,
        roles: vec!["fruit".into()],
        is_alive: true,
        is_in_danger: false,
        is_excluded_from_watering: false,
        reproductive_role: None,
        adult_height_meters: None,
        adult_width_meters: None,
    }
}
