use crate::adapters::secondary::InMemoryOrchardStorage;
use crate::hexagon::models::{
    AnnualDate, AnnualHarvestWindow, BotanicalTaxon, HarvestDataOrigin, HarvestScheduleOwner,
    HarvestedPart, NamedTaxon, PlantIdentity, PlantIdentityId,
};

use super::{
    AnnualHarvestWindowChanged, PlantHarvestWindowsReplaced, PlantHarvestWindowsReplacementError,
    replace_plant_harvest_windows,
};

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
