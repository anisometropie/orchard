use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, IdentificationStatus, NamedTaxon, OrchardTree, PlantIdentity, PlantIdentityId,
    Tree,
};
use orchard_api::hexagon::ports::OrchardReadError;
use orchard_api::hexagon::use_cases::list_orchard_trees::list_orchard_trees;

#[test]
fn list_a_stored_tree_with_its_plant_identity() {
    let apple = PlantIdentity {
        common_name: "Pommier".into(),
        botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
            genus: "Malus".into(),
            species: Some("domestica".into()),
            species_is_hybrid: false,
            infraspecific: None,
            is_aggregate: false,
            cultivar_group: None,
        }),
        cultivar: None,
        trade_name: None,
        identification_status: IdentificationStatus::Confirmed,
    };
    let tree = Tree {
        legacy_source: None,
        plant_identity_id: PlantIdentityId(1),
        longitude: 0.64,
        latitude: 0.68,
        planted_on: Some("2024-02-03".into()),
        row_name: Some("1. Haut haut haut".into()),
        roles: vec!["fruit".into()],
        is_alive: true,
        reproductive_role: None,
        harvest_start_day: Some(210),
        harvest_end_day: Some(260),
        adult_height_meters: Some(4.0),
        adult_width_meters: Some(3.0),
    };
    let (mut orchard_storage, _) =
        InMemoryOrchardStorage::with_existing_orchard(vec![apple.clone()], vec![tree.clone()]);

    let listed_trees = list_orchard_trees(&mut orchard_storage);

    assert_eq!(
        listed_trees,
        Ok(vec![OrchardTree {
            tree,
            plant_identity: apple,
        }])
    );
}

#[test]
fn report_when_stored_trees_cannot_be_read() {
    let mut orchard_storage = InMemoryOrchardStorage::failing_when_reading_trees();

    assert_eq!(
        list_orchard_trees(&mut orchard_storage),
        Err(OrchardReadError::TreesCouldNotBeRead)
    );
}
