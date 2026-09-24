use orchard_api::hexagon::models::{
    AnnualDate, AnnualHarvestWindow, BotanicalTaxon, HarvestDataOrigin, HarvestScheduleOwner,
    HarvestedPart, IdentificationStatus, NamedTaxon, OrchardId, PlantCultivar,
    PlantIdentification, PlantIdentity,
};
use orchard_api::hexagon::ports::{OrchardStorage, OrchardStorageError};
use orchard_api::hexagon::use_cases::replace_plant_harvest_windows::{
    AnnualHarvestWindowChanged, OrchardHarvestWindowsReplaced, replace_orchard_harvest_windows,
};

pub fn assert_harvest_window_edits_preserve_sources(storage: &mut impl OrchardStorage) {
    let identity = storage.transaction(|storage| storage.resolve_plant_identification(
        PlantIdentification {
            plant_identity: PlantIdentity {
                common_name: "Apple".into(),
                botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
                    genus: "Malus".into(), species: Some("domestica".into()),
                    species_is_hybrid: false, infraspecific: None,
                    is_aggregate: false, cultivar_group: None,
                }),
            },
            plant_cultivar: Some(PlantCultivar { cultivar: "Boskoop".into(), trade_name: None }),
            identification_status: IdentificationStatus::Confirmed,
        },
    )).unwrap();
    let owners = [HarvestScheduleOwner::PlantIdentity(identity.plant_identity_id),
        HarvestScheduleOwner::PlantCultivar(identity.cultivar_id.unwrap())];
    let originals = [6, 9].map(|month| AnnualHarvestWindow {
        start: AnnualDate { month, day: 1 }, end: AnnualDate { month, day: 20 },
        reference_region: Some("Sapporo, Japan".into()), harvested_part: HarvestedPart::Fruit,
        data_origin: HarvestDataOrigin::ExternalReference,
        source_url: Some(format!("https://example.com/harvest/{month}")),
    }).to_vec();

    for owner in owners {
        storage.transaction(|storage| {
            storage.replace_harvest_windows(owner, originals.clone())?;
            assert_eq!(storage.harvest_windows(owner)?, originals);
            storage.replace_harvest_windows(owner, vec![])?;
            assert!(storage.harvest_windows(owner)?.is_empty());
            Ok::<_, OrchardStorageError>(())
        }).unwrap();
        for orchard_id in [OrchardId(1), OrchardId(2)] {
            assert!(storage.orchard_harvest_windows(orchard_id, owner).unwrap().is_empty());
            storage.transaction(|storage| {
                storage.replace_orchard_harvest_windows(orchard_id, owner, originals.clone())?;
                assert_eq!(storage.orchard_harvest_windows(orchard_id, owner)?, originals);
                Ok::<_, OrchardStorageError>(())
            }).unwrap();
        }
    }

    for owner in owners {
        let failed: Result<(), OrchardStorageError> = storage.transaction(|storage| {
            storage.replace_orchard_harvest_windows(OrchardId(1), owner, vec![])?;
            assert!(storage.orchard_harvest_windows(OrchardId(1), owner)?.is_empty());
            Err(OrchardStorageError::HarvestWindowsCouldNotBeReplaced)
        });
        assert!(failed.is_err());
        assert_eq!(storage.orchard_harvest_windows(OrchardId(1), owner).unwrap(), originals);

        replace_orchard_harvest_windows(OrchardHarvestWindowsReplaced {
            orchard_id: OrchardId(1), owner, reference_region: "Sapporo, Japan".into(),
            windows: vec![
                AnnualHarvestWindowChanged {
                    start_month: 9, start_day: 1, end_month: 9, end_day: 20,
                    harvested_part: HarvestedPart::Fruit,
                },
                AnnualHarvestWindowChanged {
                    start_month: 6, start_day: 1, end_month: 6, end_day: 25,
                    harvested_part: HarvestedPart::Fruit,
                },
            ],
        }, storage).unwrap();
        assert_eq!(storage.orchard_harvest_windows(OrchardId(1), owner).unwrap(), vec![
            originals[1].clone(),
            AnnualHarvestWindow {
                end: AnnualDate { month: 6, day: 25 },
                data_origin: HarvestDataOrigin::FieldObservation, source_url: None,
                ..originals[0].clone()
            },
        ]);
        assert_eq!(storage.orchard_harvest_windows(OrchardId(2), owner).unwrap(), originals);
    }
}
