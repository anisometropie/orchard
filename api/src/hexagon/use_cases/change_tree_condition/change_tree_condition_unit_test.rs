use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    BotanicalTaxon, IdentificationStatus, NamedTaxon, PlantIdentity, PlantIdentityId, Tree, TreeId,
};
use orchard_api::hexagon::use_cases::change_tree_condition::{
    TreeConditionChangeError, TreeConditionChanged, change_tree_condition,
};

#[test]
fn mark_one_tree_dead_with_a_partial_change_and_clear_its_danger() {
    let first_tree = apple_tree(0.64, true, false);
    let second_tree = apple_tree(0.22, true, true);
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::with_existing_orchard(
        vec![apple_identity()],
        vec![first_tree, second_tree],
    );

    let result = change_tree_condition(
        TreeConditionChanged {
            tree_id: TreeId(2),
            is_alive: Some(false),
            is_in_danger: None,
        },
        &mut orchard_storage,
    );

    assert_eq!(result, Ok(()));
    let trees = observed_orchard.trees();
    assert!(trees[0].is_alive);
    assert!(!trees[1].is_alive);
    assert!(!trees[1].is_in_danger);
}

#[test]
fn change_only_the_danger_field() {
    let tree = apple_tree(0.64, true, false);
    let (mut orchard_storage, observed_orchard) =
        InMemoryOrchardStorage::with_existing_orchard(vec![apple_identity()], vec![tree]);

    let result = change_tree_condition(
        TreeConditionChanged {
            tree_id: TreeId(1),
            is_alive: None,
            is_in_danger: Some(true),
        },
        &mut orchard_storage,
    );

    assert_eq!(result, Ok(()));
    assert!(observed_orchard.trees()[0].is_alive);
    assert!(observed_orchard.trees()[0].is_in_danger);
}

#[test]
fn revive_a_tree_and_mark_it_in_danger_in_one_change() {
    let tree = apple_tree(0.64, false, false);
    let (mut orchard_storage, observed_orchard) =
        InMemoryOrchardStorage::with_existing_orchard(vec![apple_identity()], vec![tree]);

    let result = change_tree_condition(
        TreeConditionChanged {
            tree_id: TreeId(1),
            is_alive: Some(true),
            is_in_danger: Some(true),
        },
        &mut orchard_storage,
    );

    assert_eq!(result, Ok(()));
    assert!(observed_orchard.trees()[0].is_alive);
    assert!(observed_orchard.trees()[0].is_in_danger);
}

#[test]
fn reject_a_change_that_would_leave_a_dead_tree_in_danger() {
    let tree = apple_tree(0.64, true, false);
    let (mut orchard_storage, observed_orchard) =
        InMemoryOrchardStorage::with_existing_orchard(vec![apple_identity()], vec![tree]);

    let result = change_tree_condition(
        TreeConditionChanged {
            tree_id: TreeId(1),
            is_alive: Some(false),
            is_in_danger: Some(true),
        },
        &mut orchard_storage,
    );

    assert_eq!(
        result,
        Err(TreeConditionChangeError::DeadTreeCannotBeInDanger)
    );
    assert!(observed_orchard.trees()[0].is_alive);
    assert!(!observed_orchard.trees()[0].is_in_danger);
}

#[test]
fn reject_an_empty_condition_change() {
    let tree = apple_tree(0.64, true, false);
    let (mut orchard_storage, observed_orchard) =
        InMemoryOrchardStorage::with_existing_orchard(vec![apple_identity()], vec![tree]);

    let result = change_tree_condition(
        TreeConditionChanged {
            tree_id: TreeId(1),
            is_alive: None,
            is_in_danger: None,
        },
        &mut orchard_storage,
    );

    assert_eq!(result, Err(TreeConditionChangeError::NoChangesRequested));
    assert!(observed_orchard.trees()[0].is_alive);
}

#[test]
fn report_when_tree_does_not_exist() {
    let tree = apple_tree(0.64, true, false);
    let (mut orchard_storage, observed_orchard) =
        InMemoryOrchardStorage::with_existing_orchard(vec![apple_identity()], vec![tree]);

    let result = change_tree_condition(
        TreeConditionChanged {
            tree_id: TreeId(2),
            is_alive: Some(false),
            is_in_danger: None,
        },
        &mut orchard_storage,
    );

    assert_eq!(result, Err(TreeConditionChangeError::TreeNotFound));
    assert!(observed_orchard.trees()[0].is_alive);
}

fn apple_tree(longitude: f64, is_alive: bool, is_in_danger: bool) -> Tree {
    Tree {
        legacy_source: None,
        plant_identity_id: PlantIdentityId(1),
        longitude,
        latitude: 0.68,
        planted_on: Some("2024-02-03".into()),
        row_name: Some("1. Haut haut haut".into()),
        roles: vec!["fruit".into()],
        is_alive,
        is_in_danger,
        reproductive_role: None,
        harvest_start_day: Some(210),
        harvest_end_day: Some(260),
        adult_height_meters: Some(4.0),
        adult_width_meters: Some(3.0),
    }
}

fn apple_identity() -> PlantIdentity {
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
        cultivar: None,
        trade_name: None,
        identification_status: IdentificationStatus::Confirmed,
    }
}
