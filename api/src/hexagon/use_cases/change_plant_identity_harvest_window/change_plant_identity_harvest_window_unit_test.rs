use crate::adapters::secondary::InMemoryOrchardStorage;
use crate::hexagon::models::{
    AnnualDate, AnnualHarvestWindow, BotanicalTaxon, NamedTaxon, PlantIdentity, PlantIdentityId,
};

use super::{
    PlantIdentityHarvestWindowChangeError, PlantIdentityHarvestWindowChanged,
    change_plant_identity_harvest_window,
};

#[test]
fn change_the_recurring_harvest_window_for_a_plant_identity() {
    let (mut orchard, observer) =
        InMemoryOrchardStorage::with_existing_orchard(vec![apple()], vec![]);

    let result = change_plant_identity_harvest_window(
        PlantIdentityHarvestWindowChanged {
            plant_identity_id: PlantIdentityId(1),
            start_month: 8,
            start_day: 20,
            end_month: 10,
            end_day: 5,
        },
        &mut orchard,
    );

    assert_eq!(result, Ok(()));
    assert_eq!(
        observer.harvest_window(PlantIdentityId(1)),
        Some(AnnualHarvestWindow {
            start: AnnualDate { month: 8, day: 20 },
            end: AnnualDate { month: 10, day: 5 },
        })
    );
}

#[test]
fn reject_an_impossible_annual_date_without_changing_the_identity() {
    let (mut orchard, observer) =
        InMemoryOrchardStorage::with_existing_orchard(vec![apple()], vec![]);

    let result = change_plant_identity_harvest_window(
        PlantIdentityHarvestWindowChanged {
            plant_identity_id: PlantIdentityId(1),
            start_month: 2,
            start_day: 30,
            end_month: 3,
            end_day: 5,
        },
        &mut orchard,
    );

    assert_eq!(
        result,
        Err(PlantIdentityHarvestWindowChangeError::InvalidAnnualDate)
    );
    assert_eq!(observer.harvest_window(PlantIdentityId(1)), None);
}

#[test]
fn report_a_missing_plant_identity() {
    let (mut orchard, _) = InMemoryOrchardStorage::with_existing_orchard(vec![apple()], vec![]);

    let result = change_plant_identity_harvest_window(
        PlantIdentityHarvestWindowChanged {
            plant_identity_id: PlantIdentityId(2),
            start_month: 8,
            start_day: 20,
            end_month: 10,
            end_day: 5,
        },
        &mut orchard,
    );

    assert_eq!(
        result,
        Err(PlantIdentityHarvestWindowChangeError::PlantIdentityNotFound)
    );
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
