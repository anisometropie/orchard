use std::path::Path;

use orchard_api::adapters::primary::import_legacy_geojson_file;
use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{LegacyPlantIdentification, ReproductiveRole};

#[test]
fn import_geojson_file() {
    let (mut orchard_unit_of_work, observed_orchard) = InMemoryOrchardStorage::new();

    let import_result = import_legacy_geojson_file(
        Path::new("../data/trees-wgs84.geojson"),
        &mut orchard_unit_of_work,
    );

    let plant_identities = observed_orchard.plant_identities();
    let trees = observed_orchard.trees();

    assert_eq!(import_result, Ok(278));
    assert_eq!(plant_identities.len(), 152);
    assert_eq!(trees.len(), 278);
    assert!(trees.iter().all(|tree| {
        tree.plant_identity_id.0 >= 1
            && (tree.plant_identity_id.0 as usize) <= plant_identities.len()
    }));
    assert_eq!(
        trees
            .iter()
            .find(|tree| {
                tree.legacy_source
                    .as_ref()
                    .is_some_and(|source| source.feature_id == 64)
            })
            .unwrap()
            .reproductive_role,
        Some(ReproductiveRole::SelfFertile)
    );
    assert_eq!(
        trees
            .iter()
            .find(|tree| {
                tree.legacy_source
                    .as_ref()
                    .is_some_and(|source| source.feature_id == 166)
            })
            .unwrap()
            .legacy_source
            .as_ref()
            .unwrap()
            .legacy_identification,
        Some(LegacyPlantIdentification {
            name: "Cranberry oxycoccos".into(),
            latin_name: "Vaccinium macrocarpon ‘Howes’".into(),
        })
    );
    assert_eq!(
        trees
            .iter()
            .find(|tree| {
                tree.legacy_source
                    .as_ref()
                    .is_some_and(|source| source.feature_id == 157)
            })
            .unwrap()
            .legacy_source
            .as_ref()
            .unwrap()
            .source_url,
        Some("https://www.promessedefleurs.com/fruitiers/petits-fruits/petits-fruits-de-a-a-z/lonicera-kamtschatica-eisbar-baie-de-mai.html".into())
    );
}

#[test]
fn import_trade_names() {
    let (mut orchard_unit_of_work, observed_orchard) = InMemoryOrchardStorage::new();

    assert_eq!(
        import_legacy_geojson_file(
            Path::new("../data/trees-wgs84.geojson"),
            &mut orchard_unit_of_work,
        ),
        Ok(278)
    );

    let plant_identities = observed_orchard.plant_identities();
    let trees = observed_orchard.trees();

    for (legacy_feature_id, common_name, cultivar, trade_name) in [
        (75, "Albizia", "Boubri", "Ombrella"),
        (80, "Albizia", "Boubri", "Ombrella"),
        (151, "Myrtillier", "VacBri1", "Cabernet Splash"),
        (155, "Myrtillier", "VacBri1", "Cabernet Splash"),
        (
            159,
            "Aronie à fruits noirs",
            "UCONNAM165",
            "Revontuli Mound",
        ),
        (170, "Goji", "Huang Se Medlar", "Amber Sweet"),
        (208, "Framboisier remontant", "MA 2920", "Marastar"),
        (245, "Albizia", "Boubri", "Ombrella"),
        (252, "Cassissier", "Chereshneva", "Neva"),
        (271, "Framboisier remontant", "EMR 201201", "Autumn Amber"),
        (277, "Framboisier", "Jdeboer005", "Groovy"),
    ] {
        let tree = trees
            .iter()
            .find(|tree| {
                tree.legacy_source
                    .as_ref()
                    .is_some_and(|source| source.feature_id == legacy_feature_id)
            })
            .unwrap();
        let plant_identity = &plant_identities[tree.plant_identity_id.0 as usize - 1];

        assert_eq!(
            plant_identity.common_name, common_name,
            "feature {legacy_feature_id}"
        );
        assert_eq!(
            plant_identity.cultivar.as_deref(),
            Some(cultivar),
            "feature {legacy_feature_id}"
        );
        assert_eq!(
            plant_identity.trade_name.as_deref(),
            Some(trade_name),
            "feature {legacy_feature_id}"
        );
    }
}
