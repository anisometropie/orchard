use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, HarvestDate, HarvestPeriod, HarvestRunTarget, HarvestRunTree,
    HarvestTreeOutcome, HarvestedPart, IdentificationStatus, NamedTaxon, Orchard, OrchardId,
    PlantIdentity, PlantIdentityId, Tree, TreeId, WateringRunTarget,
};
use orchard_api::hexagon::ports::{AccessControl, OrchardStorage};
use orchard_api::hexagon::use_cases::list_orchard_run_history::{
    OrchardRunHistoryRequested, list_orchard_run_history,
};

#[test]
fn list_completed_watering_and_partial_harvest_runs_with_ordered_tree_details() {
    let (mut storage, _) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        Orchard {
            id: OrchardId(7),
            name: "My orchard".into(),
            longitude: 0.5,
            latitude: 0.5,
            reference_region: "Example Region".into(),
        },
        vec![apple_identity()],
        vec![apple_tree("North"), apple_tree("North")],
    );
    let owner = storage
        .verify_credentials("owner", "password")
        .unwrap()
        .unwrap();
    let session_token = storage.create_session(owner.id).unwrap();

    let watering_run_id = storage
        .transaction(|orchard| {
            orchard.create_watering_run(
                OrchardId(7),
                &WateringRunTarget::Row("North".into()),
                None,
                None,
                &[TreeId(2), TreeId(1)],
            )
        })
        .unwrap();
    storage
        .transaction(|orchard| {
            orchard.mark_watering_tree_watered(watering_run_id, TreeId(2))?;
            orchard.mark_watering_tree_watered(watering_run_id, TreeId(1))?;
            orchard.complete_watering_run(watering_run_id)
        })
        .unwrap();

    let harvest_run_id = storage
        .transaction(|orchard| {
            orchard.create_harvest_run(
                OrchardId(7),
                HarvestRunTarget::Species(PlantIdentityId(1)),
                &[HarvestedPart::Fruit],
                date("2026-09-18"),
                &[harvest_tree(TreeId(1)), harvest_tree(TreeId(2))],
            )
        })
        .unwrap();
    storage
        .transaction(|orchard| {
            orchard.record_harvest_tree_outcome(
                harvest_run_id,
                TreeId(1),
                HarvestTreeOutcome::HarvestedEverything {
                    harvested_on: date("2026-09-18"),
                },
            )?;
            orchard.complete_harvest_run(harvest_run_id)
        })
        .unwrap();

    let history = list_orchard_run_history(
        OrchardRunHistoryRequested {
            orchard_id: OrchardId(7),
            session_token,
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(history.watering_runs.len(), 1);
    assert_eq!(history.watering_runs[0].target_label, "North");
    assert_eq!(history.watering_runs[0].trees[0].tree_id, TreeId(2));
    assert_eq!(history.watering_runs[0].trees[0].name, "Apple");
    assert!(
        history.watering_runs[0].trees[0]
            .watered_at_unix_seconds
            .is_some()
    );

    assert_eq!(history.harvest_runs.len(), 1);
    assert_eq!(history.harvest_runs[0].target_label, "Apple");
    assert_eq!(
        history.harvest_runs[0].harvested_parts,
        vec![HarvestedPart::Fruit]
    );
    assert_eq!(history.harvest_runs[0].total_tree_count, 2);
    assert_eq!(history.harvest_runs[0].trees.len(), 1);
    assert_eq!(history.harvest_runs[0].trees[0].tree_id, TreeId(1));
    assert!(matches!(
        history.harvest_runs[0].trees[0].outcome,
        HarvestTreeOutcome::HarvestedEverything { .. }
    ));
}

fn harvest_tree(tree_id: TreeId) -> HarvestRunTree {
    HarvestRunTree {
        tree_id,
        harvested_parts: vec![HarvestedPart::Fruit],
        period: HarvestPeriod {
            start: date("2026-09-01"),
            end: date("2026-09-30"),
        },
        outcome: None,
    }
}

fn date(value: &str) -> HarvestDate {
    HarvestDate::parse_iso(value).unwrap()
}

fn apple_tree(row_name: &str) -> Tree {
    Tree {
        legacy_source: None,
        plant_identity_id: PlantIdentityId(1),
        cultivar_id: None,
        identification_status: IdentificationStatus::Confirmed,
        longitude: 0.5,
        latitude: 0.5,
        planted_on: None,
        row_name: Some(row_name.into()),
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
