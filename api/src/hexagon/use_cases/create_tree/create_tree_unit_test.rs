use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::Tree;
use orchard_api::hexagon::use_cases::create_tree::{TreeCreationRequested, create_tree};

#[test]
fn when_an_orchardist_creates_a_fruit_tree_it_is_saved_with_its_orchard_details() {
    let (mut trees, observed_trees) = InMemoryOrchardStorage::new();

    let created_tree = create_tree(
        TreeCreationRequested {
            longitude: 0.72,
            latitude: 0.24,
            name: "Pommier".into(),
            latin_name: Some("Malus domestica".into()),
            roles: vec!["fruit".into()],
            harvest_start_day: Some(210),
            harvest_end_day: Some(260),
        },
        &mut trees,
    );

    let expected_tree = Tree {
        legacy_feature_id: None,
        longitude: 0.72,
        latitude: 0.24,
        name: "Pommier".into(),
        latin_name: Some("Malus domestica".into()),
        planted_on: None,
        row_name: None,
        roles: vec!["fruit".into()],
        is_alive: true,
        harvest_start_day: Some(210),
        harvest_end_day: Some(260),
        adult_height_meters: None,
        adult_width_meters: None,
    };
    assert_eq!(created_tree, expected_tree);
    assert_eq!(observed_trees.trees(), vec![expected_tree]);
}
