use crate::hexagon::models::{
    BotanicalTaxon, IdentificationStatus, InfraspecificRank, InfraspecificTaxon, NamedTaxon,
    PlantCultivar, PlantIdentification, PlantIdentity,
};

use super::parse_legacy_tree_identity;

#[test]
fn john_rivers() {
    assert_eq!(
        parse(
            "Brugnon blanc ‘John Rivers’",
            "Prunus persica var. nucipersica ‘John Rivers’"
        ),
        identity(
            "Brugnon blanc",
            BotanicalTaxon::Named(NamedTaxon {
                genus: "Prunus".into(),
                species: Some("persica".into()),
                species_is_hybrid: false,
                infraspecific: Some(InfraspecificTaxon {
                    rank: InfraspecificRank::Variety,
                    name: "nucipersica".into(),
                }),
                is_aggregate: false,
                cultivar_group: None,
            }),
            Some("John Rivers"),
            None,
            IdentificationStatus::Confirmed,
        )
    );
}

#[test]
fn sweet_lifeberry() {
    let parsed = parse_legacy_tree_identity(
        "Goji ‘Sweet Lifeberry’",
        "Lycium barbarum ‘SMNDSL’",
        Some("Sweet Lifeberry".into()),
    )
    .unwrap();

    assert_eq!(
        parsed,
        identity(
            "Goji",
            BotanicalTaxon::Named(named_taxon("Lycium", Some("barbarum"))),
            Some("SMNDSL"),
            Some("Sweet Lifeberry"),
            IdentificationStatus::Confirmed,
        )
    );
}

#[test]
fn ombrella() {
    let parsed = parse_legacy_tree_identity(
        "Albizia Ombrella",
        "Albizia julibrissin ‘Boubri’",
        Some("Ombrella".into()),
    )
    .unwrap();

    assert_eq!(
        parsed,
        identity(
            "Albizia",
            BotanicalTaxon::Named(named_taxon("Albizia", Some("julibrissin"))),
            Some("Boubri"),
            Some("Ombrella"),
            IdentificationStatus::Confirmed,
        )
    );
}

#[test]
fn thornfree() {
    assert_eq!(
        parse(
            "Mûre sans épines ‘Thornfree’",
            "Rubus fruticosus agg. ‘Thornfree’"
        ),
        identity(
            "Mûre sans épines",
            BotanicalTaxon::Named(NamedTaxon {
                genus: "Rubus".into(),
                species: Some("fruticosus".into()),
                species_is_hybrid: false,
                infraspecific: None,
                is_aggregate: true,
                cultivar_group: None,
            }),
            Some("Thornfree"),
            None,
            IdentificationStatus::Confirmed,
        )
    );
}

#[test]
fn emil() {
    assert_eq!(
        parse(
            "Myrtillier ‘Emil’",
            "Vaccinium (Angustifolium Group) ‘Emil’"
        ),
        identity(
            "Myrtillier",
            BotanicalTaxon::Named(NamedTaxon {
                genus: "Vaccinium".into(),
                species: None,
                species_is_hybrid: false,
                infraspecific: None,
                is_aggregate: false,
                cultivar_group: Some("Angustifolium".into()),
            }),
            Some("Emil"),
            None,
            IdentificationStatus::Confirmed,
        )
    );
}

#[test]
fn medana_tayberry() {
    assert_eq!(
        parse(
            "Mûre-framboise ‘Medana Tayberry’",
            "Rubus (Tayberry Group) ‘Medana Tayberry’",
        ),
        identity(
            "Mûre-framboise",
            BotanicalTaxon::Named(NamedTaxon {
                genus: "Rubus".into(),
                species: None,
                species_is_hybrid: false,
                infraspecific: None,
                is_aggregate: false,
                cultivar_group: Some("Tayberry".into()),
            }),
            Some("Medana Tayberry"),
            None,
            IdentificationStatus::Confirmed,
        )
    );
}

#[test]
fn bredon_springs() {
    assert_eq!(
        parse(
            "Lavatère arbustive ‘Bredon Springs’",
            "Malva × clementii ‘Bredon Springs’",
        ),
        identity(
            "Lavatère arbustive",
            BotanicalTaxon::Named(NamedTaxon {
                genus: "Malva".into(),
                species: Some("clementii".into()),
                species_is_hybrid: true,
                infraspecific: None,
                is_aggregate: false,
                cultivar_group: None,
            }),
            Some("Bredon Springs"),
            None,
            IdentificationStatus::Confirmed,
        )
    );
}

#[test]
fn baco_noir() {
    assert_eq!(
        parse("Baco Noir", "Vitis vinifera × Vitis riparia ‘Baco Noir’"),
        identity(
            "Baco Noir",
            BotanicalTaxon::HybridFormula {
                parents: [
                    named_taxon("Vitis", Some("vinifera")),
                    named_taxon("Vitis", Some("riparia")),
                ],
            },
            Some("Baco Noir"),
            None,
            IdentificationStatus::Confirmed,
        )
    );
}

#[test]
fn worcesterberry() {
    assert_eq!(
        parse("Groseillier ‘Worcesterberry’", "Ribes ‘Worcesterberry’"),
        identity(
            "Groseillier",
            BotanicalTaxon::Named(named_taxon("Ribes", None)),
            Some("Worcesterberry"),
            None,
            IdentificationStatus::Confirmed,
        )
    );
}

#[test]
fn prune_totoche() {
    assert_eq!(
        parse("Prune totoche", "Prunus domestica ?"),
        identity(
            "Prune totoche",
            BotanicalTaxon::Named(named_taxon("Prunus", Some("domestica"))),
            None,
            None,
            IdentificationStatus::Uncertain,
        )
    );
}

#[test]
fn pin_noir() {
    assert_eq!(
        parse("Pin noir d’Autriche", "Pinus nigra subsp. nigra"),
        identity(
            "Pin noir d’Autriche",
            BotanicalTaxon::Named(NamedTaxon {
                genus: "Pinus".into(),
                species: Some("nigra".into()),
                species_is_hybrid: false,
                infraspecific: Some(InfraspecificTaxon {
                    rank: InfraspecificRank::Subspecies,
                    name: "nigra".into(),
                }),
                is_aggregate: false,
                cultivar_group: None,
            }),
            None,
            None,
            IdentificationStatus::Confirmed,
        )
    );
}

#[test]
fn goutte_dor() {
    assert_eq!(
        parse("Figuier ‘Goutte d’Or’", "Ficus carica ‘Goutte d’Or’"),
        identity(
            "Figuier",
            BotanicalTaxon::Named(named_taxon("Ficus", Some("carica"))),
            Some("Goutte d’Or"),
            None,
            IdentificationStatus::Confirmed,
        )
    );
}

#[test]
fn pistachier_terebinthe() {
    assert_eq!(
        parse("Pistachier térébinthe", "Pistacia terebinthus"),
        identity(
            "Pistachier térébinthe",
            BotanicalTaxon::Named(named_taxon("Pistacia", Some("terebinthus"))),
            None,
            None,
            IdentificationStatus::Confirmed,
        )
    );
}

#[test]
fn kamtschatica() {
    assert_eq!(
        parse(
            "Camérisier du Kamtchatka",
            "Lonicera caerulea var. kamtschatica",
        ),
        identity(
            "Camérisier du Kamtchatka",
            BotanicalTaxon::Named(NamedTaxon {
                genus: "Lonicera".into(),
                species: Some("caerulea".into()),
                species_is_hybrid: false,
                infraspecific: Some(InfraspecificTaxon {
                    rank: InfraspecificRank::Variety,
                    name: "kamtschatica".into(),
                }),
                is_aggregate: false,
                cultivar_group: None,
            }),
            None,
            None,
            IdentificationStatus::Confirmed,
        )
    );
}

fn parse(name: &str, latin_name: &str) -> PlantIdentification {
    parse_legacy_tree_identity(name, latin_name, None).unwrap()
}

fn identity(
    common_name: &str,
    botanical_taxon: BotanicalTaxon,
    cultivar: Option<&str>,
    trade_name: Option<&str>,
    identification_status: IdentificationStatus,
) -> PlantIdentification {
    PlantIdentification {
        plant_identity: PlantIdentity {
            common_name: common_name.into(),
            botanical_taxon,
        },
        plant_cultivar: cultivar.map(|cultivar| PlantCultivar {
            cultivar: cultivar.into(),
            trade_name: trade_name.map(str::to_owned),
        }),
        identification_status,
    }
}

fn named_taxon(genus: &str, species: Option<&str>) -> NamedTaxon {
    NamedTaxon {
        genus: genus.into(),
        species: species.map(str::to_owned),
        species_is_hybrid: false,
        infraspecific: None,
        is_aggregate: false,
        cultivar_group: None,
    }
}
