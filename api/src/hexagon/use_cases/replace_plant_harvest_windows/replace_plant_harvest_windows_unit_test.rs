use crate::adapters::secondary::InMemoryOrchardStorage;
use crate::hexagon::models::{
    AnnualDate, AnnualHarvestWindow, BotanicalTaxon, HarvestDataOrigin, HarvestScheduleOwner,
    HarvestedPart, IdentificationStatus, NamedTaxon, OrchardId, PlantCultivar, PlantIdentification,
    PlantIdentity, PlantIdentityId,
};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

use super::{
    AnnualHarvestWindowChanged, OrchardHarvestWindowsReplaced, PlantHarvestWindowsReplaced,
    PlantHarvestWindowsReplacementError, replace_orchard_harvest_windows,
    replace_plant_harvest_windows,
};

#[test]
fn replace_windows_for_only_the_current_orchard() {
    let (mut orchard, observer) =
        InMemoryOrchardStorage::with_existing_orchard(vec![apple()], vec![]);
    let owner = HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1));

    assert_eq!(
        replace_orchard_harvest_windows(
            OrchardHarvestWindowsReplaced {
                orchard_id: OrchardId(7),
                owner,
                reference_region: "Sapporo, Japan".into(),
                windows: vec![window(8, 10, 10, 20)],
            },
            &mut orchard,
        ),
        Ok(())
    );
    assert_eq!(
        observer.orchard_harvest_windows(OrchardId(7), owner).len(),
        1
    );
    assert!(
        observer
            .orchard_harvest_windows(OrchardId(8), owner)
            .is_empty()
    );
}

#[test]
fn replace_multiple_windows_and_clear_them() {
    let (mut orchard, observer) =
        InMemoryOrchardStorage::with_existing_orchard(vec![apple()], vec![]);
    let owner = HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1));

    assert_eq!(
        replace_plant_harvest_windows(
            PlantHarvestWindowsReplaced {
                owner,
                reference_region: "Sapporo, Japan".into(),
                windows: vec![window(6, 15, 7, 5), window(8, 10, 10, 20)],
            },
            &mut orchard,
        ),
        Ok(())
    );
    assert_eq!(
        observer.harvest_windows(owner),
        vec![
            AnnualHarvestWindow {
                start: AnnualDate { month: 6, day: 15 },
                end: AnnualDate { month: 7, day: 5 },
                reference_region: Some("Sapporo, Japan".into()),
                harvested_part: HarvestedPart::Fruit,
                data_origin: HarvestDataOrigin::FieldObservation,
                source_url: None,
            },
            AnnualHarvestWindow {
                start: AnnualDate { month: 8, day: 10 },
                end: AnnualDate { month: 10, day: 20 },
                reference_region: Some("Sapporo, Japan".into()),
                harvested_part: HarvestedPart::Fruit,
                data_origin: HarvestDataOrigin::FieldObservation,
                source_url: None,
            },
        ]
    );

    assert_eq!(
        replace_plant_harvest_windows(
            PlantHarvestWindowsReplaced {
                owner,
                reference_region: "Sapporo, Japan".into(),
                windows: vec![],
            },
            &mut orchard,
        ),
        Ok(())
    );
    assert_eq!(observer.harvest_windows(owner), vec![]);
}

#[test]
fn reject_an_impossible_date_without_changing_the_schedule() {
    let (mut orchard, observer) =
        InMemoryOrchardStorage::with_existing_orchard(vec![apple()], vec![]);
    let owner = HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1));

    let result = replace_plant_harvest_windows(
        PlantHarvestWindowsReplaced {
            owner,
            reference_region: "Sapporo, Japan".into(),
            windows: vec![window(2, 30, 3, 5)],
        },
        &mut orchard,
    );

    assert_eq!(
        result,
        Err(PlantHarvestWindowsReplacementError::InvalidAnnualDate)
    );
    assert_eq!(observer.harvest_windows(owner), vec![]);
}

#[test]
fn require_a_reference_region_for_observed_windows() {
    let (mut orchard, observer) =
        InMemoryOrchardStorage::with_existing_orchard(vec![apple()], vec![]);
    let owner = HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1));

    let result = replace_plant_harvest_windows(
        PlantHarvestWindowsReplaced {
            owner,
            reference_region: "  ".into(),
            windows: vec![window(8, 20, 10, 5)],
        },
        &mut orchard,
    );

    assert_eq!(
        result,
        Err(PlantHarvestWindowsReplacementError::MissingReferenceRegion)
    );
    assert_eq!(observer.harvest_windows(owner), vec![]);
}

#[test]
fn report_a_missing_owner() {
    let (mut orchard, _) = InMemoryOrchardStorage::with_existing_orchard(vec![apple()], vec![]);

    let result = replace_plant_harvest_windows(
        PlantHarvestWindowsReplaced {
            owner: HarvestScheduleOwner::PlantIdentity(PlantIdentityId(2)),
            reference_region: "Sapporo, Japan".into(),
            windows: vec![window(8, 20, 10, 5)],
        },
        &mut orchard,
    );

    assert_eq!(
        result,
        Err(PlantHarvestWindowsReplacementError::OwnerNotFound)
    );
}

#[test]
fn editing_one_window_preserves_other_sources_even_after_reordering_or_removing_windows() {
    let (mut orchard, observer) =
        InMemoryOrchardStorage::with_existing_orchard(vec![apple()], vec![]);
    let owner = HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1));
    let unchanged = reference_window(8, 10, 10, 20);
    orchard
        .transaction(|storage| {
            storage.replace_harvest_windows(
                owner,
                vec![
                    reference_window(4, 1, 5, 1),
                    reference_window(6, 15, 7, 5),
                    unchanged.clone(),
                ],
            )
        })
        .unwrap();

    replace_plant_harvest_windows(
        PlantHarvestWindowsReplaced {
            owner,
            reference_region: " Sapporo, Japan ".into(),
            windows: vec![
                window(8, 10, 10, 20),
                window(6, 15, 7, 10),
                window(11, 1, 11, 20),
            ],
        },
        &mut orchard,
    )
    .unwrap();

    let saved = observer.harvest_windows(owner);
    assert_eq!(saved.len(), 3);
    assert_eq!(saved[0], unchanged);
    for edited in &saved[1..] {
        assert_eq!(edited.data_origin, HarvestDataOrigin::FieldObservation);
        assert_eq!(edited.source_url, None);
    }
}

#[test]
fn preserve_unedited_windows_for_both_species_and_cultivars_without_crossing_orchards() {
    let (mut orchard, observer) =
        InMemoryOrchardStorage::with_existing_orchard(vec![apple()], vec![]);
    let cultivar = orchard
        .transaction(|storage| {
            storage.resolve_plant_identification(PlantIdentification {
                identification_status: IdentificationStatus::Confirmed,
                plant_identity: apple(),
                plant_cultivar: Some(PlantCultivar {
                    cultivar: "Boskoop".into(),
                    trade_name: None,
                }),
            })
        })
        .unwrap();
    let owners = [
        HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
        HarvestScheduleOwner::PlantCultivar(cultivar.cultivar_id.unwrap()),
    ];
    let original = vec![
        reference_window(6, 15, 7, 5),
        reference_window(8, 10, 10, 20),
    ];
    orchard
        .transaction(|storage| {
            for owner in owners {
                for orchard_id in [OrchardId(7), OrchardId(8)] {
                    storage.replace_orchard_harvest_windows(orchard_id, owner, original.clone())?;
                }
            }
            Ok::<_, OrchardStorageError>(())
        })
        .unwrap();

    for owner in owners {
        replace_orchard_harvest_windows(
            OrchardHarvestWindowsReplaced {
                orchard_id: OrchardId(7),
                owner,
                reference_region: "Sapporo, Japan".into(),
                windows: vec![window(6, 15, 7, 10), window(8, 10, 10, 20)],
            },
            &mut orchard,
        )
        .unwrap();
        let saved = observer.orchard_harvest_windows(OrchardId(7), owner);
        assert_eq!(saved[0].data_origin, HarvestDataOrigin::FieldObservation);
        assert_eq!(saved[0].source_url, None);
        assert_eq!(saved[1], original[1]);
        assert_eq!(
            observer.orchard_harvest_windows(OrchardId(8), owner),
            original
        );
    }
}

#[test]
fn saving_unchanged_windows_preserves_metadata_but_a_part_or_region_change_is_an_observation() {
    let (mut orchard, observer) =
        InMemoryOrchardStorage::with_existing_orchard(vec![apple()], vec![]);
    let owner = HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1));
    let original = reference_window(6, 15, 7, 5);
    orchard
        .transaction(|storage| {
            storage.replace_orchard_harvest_windows(OrchardId(7), owner, vec![original.clone()])
        })
        .unwrap();
    for (region, part, expected) in [
        ("Sapporo, Japan", HarvestedPart::Fruit, original.clone()),
        (
            "Sapporo, Japan",
            HarvestedPart::Flower,
            AnnualHarvestWindow {
                harvested_part: HarvestedPart::Flower,
                data_origin: HarvestDataOrigin::FieldObservation,
                source_url: None,
                ..original.clone()
            },
        ),
        (
            "Paris, France",
            HarvestedPart::Fruit,
            AnnualHarvestWindow {
                reference_region: Some("Paris, France".into()),
                data_origin: HarvestDataOrigin::FieldObservation,
                source_url: None,
                ..original.clone()
            },
        ),
    ] {
        orchard
            .transaction(|storage| {
                storage.replace_orchard_harvest_windows(OrchardId(7), owner, vec![original.clone()])
            })
            .unwrap();
        replace_orchard_harvest_windows(
            OrchardHarvestWindowsReplaced {
                orchard_id: OrchardId(7),
                owner,
                reference_region: region.into(),
                windows: vec![AnnualHarvestWindowChanged {
                    harvested_part: part,
                    ..window(6, 15, 7, 5)
                }],
            },
            &mut orchard,
        )
        .unwrap();
        assert_eq!(
            observer.orchard_harvest_windows(OrchardId(7), owner),
            vec![expected]
        );
    }
}

#[test]
fn failed_save_preserves_original_dates_and_sources() {
    let (mut orchard, observer) =
        InMemoryOrchardStorage::with_existing_orchard(vec![apple()], vec![]);
    let owner = HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1));
    let original = vec![
        reference_window(6, 15, 7, 5),
        reference_window(8, 10, 10, 20),
    ];
    orchard
        .transaction(|storage| {
            storage.replace_orchard_harvest_windows(OrchardId(7), owner, original.clone())
        })
        .unwrap();
    orchard = orchard.with_commit_failure();
    let result = replace_orchard_harvest_windows(
        OrchardHarvestWindowsReplaced {
            orchard_id: OrchardId(7),
            owner,
            reference_region: "Sapporo, Japan".into(),
            windows: vec![window(6, 15, 7, 10), window(8, 10, 10, 20)],
        },
        &mut orchard,
    );
    assert_eq!(
        result,
        Err(PlantHarvestWindowsReplacementError::TransactionCouldNotCommit)
    );
    assert_eq!(
        observer.orchard_harvest_windows(OrchardId(7), owner),
        original
    );
}

fn reference_window(
    start_month: u8,
    start_day: u8,
    end_month: u8,
    end_day: u8,
) -> AnnualHarvestWindow {
    AnnualHarvestWindow {
        start: AnnualDate {
            month: start_month,
            day: start_day,
        },
        end: AnnualDate {
            month: end_month,
            day: end_day,
        },
        reference_region: Some("Sapporo, Japan".into()),
        harvested_part: HarvestedPart::Fruit,
        data_origin: HarvestDataOrigin::ExternalReference,
        source_url: Some(format!("https://example.com/harvest/{start_month}")),
    }
}

fn window(
    start_month: u8,
    start_day: u8,
    end_month: u8,
    end_day: u8,
) -> AnnualHarvestWindowChanged {
    AnnualHarvestWindowChanged {
        start_month,
        start_day,
        end_month,
        end_day,
        harvested_part: HarvestedPart::Fruit,
    }
}

fn apple() -> PlantIdentity {
    PlantIdentity {
        common_name: "Pommier".into(),
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
