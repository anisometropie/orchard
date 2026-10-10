use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, HarvestRunTarget, HarvestScheduleOwner, HarvestedPart, IdentificationStatus,
    NamedTaxon, Orchard, OrchardId, PlantIdentity, PlantIdentityId, Tree,
};
use orchard_api::hexagon::use_cases::replace_plant_harvest_windows::{
    AnnualHarvestWindowChanged, OrchardHarvestWindowsReplaced, replace_orchard_harvest_windows,
};
use orchard_api::hexagon::use_cases::start_harvest_run::{
    HarvestRunStartError, HarvestRunStartRequested, start_harvest_run,
};

pub fn start(
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

pub fn storage_with_fruit_window() -> InMemoryOrchardStorage {
    storage(false)
}
pub fn storage_with_cultivar_window() -> InMemoryOrchardStorage {
    storage(true)
}

fn storage(with_cultivar: bool) -> InMemoryOrchardStorage {
    use orchard_api::hexagon::models::{PlantCultivar, PlantCultivarId, PlantIdentification};
    use orchard_api::hexagon::ports::{OrchardStorage, OrchardStorageError};
    let mut second_tree = apple_tree(5.1);
    if with_cultivar {
        second_tree.cultivar_id = Some(PlantCultivarId(1));
    }
    let (mut storage, _) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        orchard(),
        vec![apple_identity()],
        vec![apple_tree(5.0), second_tree],
    );
    if with_cultivar {
        storage
            .transaction(|storage| {
                storage.resolve_plant_identification(PlantIdentification {
                    plant_identity: apple_identity(),
                    plant_cultivar: Some(PlantCultivar {
                        cultivar: "Boskoop".into(),
                        trade_name: None,
                    }),
                    identification_status: IdentificationStatus::Confirmed,
                })?;
                Ok::<_, OrchardStorageError>(())
            })
            .unwrap();
    }
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
    if with_cultivar {
        let windows = storage
            .orchard_harvest_windows(
                OrchardId(7),
                HarvestScheduleOwner::PlantIdentity(PlantIdentityId(1)),
            )
            .unwrap();
        storage
            .transaction(|storage| {
                storage.replace_orchard_harvest_windows(
                    OrchardId(7),
                    HarvestScheduleOwner::PlantCultivar(PlantCultivarId(1)),
                    windows,
                )
            })
            .unwrap();
    }
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
