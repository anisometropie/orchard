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
        PlantIdentification {
            plant_identity: PlantIdentity {
                common_name: "Brugnon blanc".into(),
                botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
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
            },
            plant_cultivar: Some(PlantCultivar {
                cultivar: "John Rivers".into(),
                trade_name: None,
            }),
            identification_status: IdentificationStatus::Confirmed,
        }
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
        PlantIdentification {
            plant_identity: PlantIdentity {
                common_name: "Goji".into(),
                botanical_taxon: BotanicalTaxon::Named(named_taxon("Lycium", Some("barbarum"),)),
            },
            plant_cultivar: Some(PlantCultivar {
                cultivar: "SMNDSL".into(),
                trade_name: Some("Sweet Lifeberry".into()),
            }),
            identification_status: IdentificationStatus::Confirmed,
        }
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
        PlantIdentification {
            plant_identity: PlantIdentity {
                common_name: "Albizia".into(),
                botanical_taxon: BotanicalTaxon::Named(
                    named_taxon("Albizia", Some("julibrissin"),)
                ),
            },
            plant_cultivar: Some(PlantCultivar {
                cultivar: "Boubri".into(),
                trade_name: Some("Ombrella".into()),
            }),
            identification_status: IdentificationStatus::Confirmed,
        }
    );
}

#[test]
fn thornfree() {
    assert_eq!(
        parse(
            "Mûre sans épines ‘Thornfree’",
            "Rubus fruticosus agg. ‘Thornfree’"
        ),
        PlantIdentification {
            plant_identity: PlantIdentity {
                common_name: "Mûre sans épines".into(),
                botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
                    genus: "Rubus".into(),
                    species: Some("fruticosus".into()),
                    species_is_hybrid: false,
                    infraspecific: None,
                    is_aggregate: true,
                    cultivar_group: None,
                }),
            },
            plant_cultivar: Some(PlantCultivar {
                cultivar: "Thornfree".into(),
                trade_name: None,
            }),
            identification_status: IdentificationStatus::Confirmed,
        }
    );
}

#[test]
fn emil() {
    assert_eq!(
        parse(
            "Myrtillier ‘Emil’",
            "Vaccinium (Angustifolium Group) ‘Emil’"
        ),
        PlantIdentification {
            plant_identity: PlantIdentity {
                common_name: "Myrtillier".into(),
                botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
                    genus: "Vaccinium".into(),
                    species: None,
                    species_is_hybrid: false,
                    infraspecific: None,
                    is_aggregate: false,
                    cultivar_group: Some("Angustifolium".into()),
                }),
            },
            plant_cultivar: Some(PlantCultivar {
                cultivar: "Emil".into(),
                trade_name: None,
            }),
            identification_status: IdentificationStatus::Confirmed,
        }
    );
}

#[test]
fn medana_tayberry() {
    assert_eq!(
        parse(
            "Mûre-framboise ‘Medana Tayberry’",
            "Rubus (Tayberry Group) ‘Medana Tayberry’",
        ),
        PlantIdentification {
            plant_identity: PlantIdentity {
                common_name: "Mûre-framboise".into(),
                botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
                    genus: "Rubus".into(),
                    species: None,
                    species_is_hybrid: false,
                    infraspecific: None,
                    is_aggregate: false,
                    cultivar_group: Some("Tayberry".into()),
                }),
            },
            plant_cultivar: Some(PlantCultivar {
                cultivar: "Medana Tayberry".into(),
                trade_name: None,
            }),
            identification_status: IdentificationStatus::Confirmed,
        }
    );
}

#[test]
fn bredon_springs() {
    assert_eq!(
        parse(
            "Lavatère arbustive ‘Bredon Springs’",
            "Malva × clementii ‘Bredon Springs’",
        ),
        PlantIdentification {
            plant_identity: PlantIdentity {
                common_name: "Lavatère arbustive".into(),
                botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
                    genus: "Malva".into(),
                    species: Some("clementii".into()),
                    species_is_hybrid: true,
                    infraspecific: None,
                    is_aggregate: false,
                    cultivar_group: None,
                }),
            },
            plant_cultivar: Some(PlantCultivar {
                cultivar: "Bredon Springs".into(),
                trade_name: None,
            }),
            identification_status: IdentificationStatus::Confirmed,
        }
    );
}

#[test]
fn baco_noir() {
    assert_eq!(
        parse("Baco Noir", "Vitis vinifera × Vitis riparia ‘Baco Noir’"),
        PlantIdentification {
            plant_identity: PlantIdentity {
                common_name: "Baco Noir".into(),
                botanical_taxon: BotanicalTaxon::HybridFormula {
                    parents: [
                        named_taxon("Vitis", Some("vinifera")),
                        named_taxon("Vitis", Some("riparia")),
                    ],
                },
            },
            plant_cultivar: Some(PlantCultivar {
                cultivar: "Baco Noir".into(),
                trade_name: None,
            }),
            identification_status: IdentificationStatus::Confirmed,
        }
    );
}

#[test]
fn worcesterberry() {
    assert_eq!(
        parse("Groseillier ‘Worcesterberry’", "Ribes ‘Worcesterberry’"),
        PlantIdentification {
            plant_identity: PlantIdentity {
                common_name: "Groseillier".into(),
                botanical_taxon: BotanicalTaxon::Named(named_taxon("Ribes", None)),
            },
            plant_cultivar: Some(PlantCultivar {
                cultivar: "Worcesterberry".into(),
                trade_name: None,
            }),
            identification_status: IdentificationStatus::Confirmed,
        }
    );
}

#[test]
fn prune_totoche() {
    assert_eq!(
        parse("Prune totoche", "Prunus domestica ?"),
        PlantIdentification {
            plant_identity: PlantIdentity {
                common_name: "Prune totoche".into(),
                botanical_taxon: BotanicalTaxon::Named(named_taxon("Prunus", Some("domestica"),)),
            },
            plant_cultivar: None,
            identification_status: IdentificationStatus::Uncertain,
        }
    );
}

#[test]
fn pin_noir() {
    assert_eq!(
        parse("Pin noir d’Autriche", "Pinus nigra subsp. nigra"),
        PlantIdentification {
            plant_identity: PlantIdentity {
                common_name: "Pin noir d’Autriche".into(),
                botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
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
            },
            plant_cultivar: None,
            identification_status: IdentificationStatus::Confirmed,
        }
    );
}

#[test]
fn goutte_dor() {
    assert_eq!(
        parse("Figuier ‘Goutte d’Or’", "Ficus carica ‘Goutte d’Or’"),
        PlantIdentification {
            plant_identity: PlantIdentity {
                common_name: "Figuier".into(),
                botanical_taxon: BotanicalTaxon::Named(named_taxon("Ficus", Some("carica"))),
            },
            plant_cultivar: Some(PlantCultivar {
                cultivar: "Goutte d’Or".into(),
                trade_name: None,
            }),
            identification_status: IdentificationStatus::Confirmed,
        }
    );
}

#[test]
fn pistachier_terebinthe() {
    assert_eq!(
        parse("Pistachier térébinthe", "Pistacia terebinthus"),
        PlantIdentification {
            plant_identity: PlantIdentity {
                common_name: "Pistachier térébinthe".into(),
                botanical_taxon: BotanicalTaxon::Named(named_taxon(
                    "Pistacia",
                    Some("terebinthus"),
                )),
            },
            plant_cultivar: None,
            identification_status: IdentificationStatus::Confirmed,
        }
    );
}

#[test]
fn kamtschatica() {
    assert_eq!(
        parse(
            "Camérisier du Kamtchatka",
            "Lonicera caerulea var. kamtschatica",
        ),
        PlantIdentification {
            plant_identity: PlantIdentity {
                common_name: "Camérisier du Kamtchatka".into(),
                botanical_taxon: BotanicalTaxon::Named(NamedTaxon {
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
            },
            plant_cultivar: None,
            identification_status: IdentificationStatus::Confirmed,
        }
    );
}

fn parse(name: &str, latin_name: &str) -> PlantIdentification {
    parse_legacy_tree_identity(name, latin_name, None).unwrap()
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
