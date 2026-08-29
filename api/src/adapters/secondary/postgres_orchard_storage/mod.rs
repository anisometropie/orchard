use postgres::{Client, NoTls};

use crate::hexagon::models::{
    IdentificationStatus, LegacyPlantIdentification, LegacyTreeSource, OrchardTree, PlantIdentity,
    PlantIdentityId, ReproductiveRole, Tree,
};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

/// PostgreSQL/PostGIS implementation of orchard storage.
pub struct PostgresOrchardStorage {
    client: Client,
}

impl PostgresOrchardStorage {
    pub fn connect(database_url: &str) -> Result<Self, OrchardStorageError> {
        let client = Client::connect(database_url, NoTls)
            .map_err(|_| OrchardStorageError::AtomicOperationCouldNotBegin)?;
        Ok(Self { client })
    }
}

impl OrchardStorage for PostgresOrchardStorage {
    fn transaction<T, E>(
        &mut self,
        operation: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E>
    where
        E: From<OrchardStorageError>,
    {
        self.client
            .batch_execute("BEGIN")
            .map_err(|_| E::from(OrchardStorageError::AtomicOperationCouldNotBegin))?;
        let result = operation(self);
        match result {
            Err(error) => {
                let _ = self.client.batch_execute("ROLLBACK");
                Err(error)
            }
            Ok(value) => match self.client.batch_execute("COMMIT") {
                Ok(()) => Ok(value),
                Err(_) => {
                    let _ = self.client.batch_execute("ROLLBACK");
                    Err(E::from(OrchardStorageError::AtomicOperationCouldNotCommit))
                }
            },
        }
    }

    fn is_legacy_tree_already_imported(
        &mut self,
        legacy_feature_id: u32,
    ) -> Result<bool, OrchardStorageError> {
        is_legacy_tree_already_imported(&mut self.client, legacy_feature_id)
            .map_err(|_| OrchardStorageError::ExistingLegacyTreeCouldNotBeChecked)
    }

    fn find_or_create_plant_identity(
        &mut self,
        plant_identity: PlantIdentity,
    ) -> Result<PlantIdentityId, OrchardStorageError> {
        find_or_create_plant_identity(&mut self.client, plant_identity)
    }

    fn save_tree(&mut self, tree: Tree) -> Result<(), OrchardStorageError> {
        save_tree(&mut self.client, tree).map_err(|_| OrchardStorageError::TreeCouldNotBeSaved)
    }

    fn trees(&mut self) -> Result<Vec<OrchardTree>, OrchardStorageError> {
        self.client
            .query(
                "SELECT
                    t.legacy_feature_id, t.plant_identity_id,
                    ST_X(t.location), ST_Y(t.location),
                    t.legacy_name, t.legacy_latin_name, t.legacy_source_url,
                    t.legacy_identification_name, t.legacy_identification_latin_name,
                    t.planted_on::text, t.row_name, t.roles, t.is_alive,
                    t.reproductive_role, t.harvest_start_day, t.harvest_end_day,
                    t.adult_height_meters, t.adult_width_meters,
                    p.common_name, p.botanical_taxon::text, p.cultivar, p.trade_name,
                    p.identification_status
                 FROM trees t
                 JOIN plant_identities p ON p.id = t.plant_identity_id
                 ORDER BY t.id",
                &[],
            )
            .map_err(|_| OrchardStorageError::TreesCouldNotBeRead)?
            .into_iter()
            .map(|row| orchard_tree_from_row(&row))
            .collect()
    }
}

fn orchard_tree_from_row(row: &postgres::Row) -> Result<OrchardTree, OrchardStorageError> {
    let legacy_feature_id = row.get::<_, Option<i32>>(0);
    let legacy_name = row.get::<_, Option<String>>(4);
    let legacy_latin_name = row.get::<_, Option<String>>(5);
    let legacy_identification_name = row.get::<_, Option<String>>(7);
    let legacy_identification_latin_name = row.get::<_, Option<String>>(8);
    let legacy_source = match (legacy_feature_id, legacy_name, legacy_latin_name) {
        (Some(feature_id), Some(name), Some(latin_name)) => Some(LegacyTreeSource {
            feature_id: u32::try_from(feature_id)
                .map_err(|_| OrchardStorageError::TreesCouldNotBeRead)?,
            name,
            latin_name,
            legacy_identification: match (
                legacy_identification_name,
                legacy_identification_latin_name,
            ) {
                (Some(name), Some(latin_name)) => {
                    Some(LegacyPlantIdentification { name, latin_name })
                }
                _ => None,
            },
            source_url: row.get(6),
        }),
        (None, None, None) => None,
        _ => return Err(OrchardStorageError::TreesCouldNotBeRead),
    };
    let reproductive_role = match row.get::<_, Option<&str>>(13) {
        Some("female") => Some(ReproductiveRole::Female),
        Some("male") => Some(ReproductiveRole::Male),
        Some("self_fertile") => Some(ReproductiveRole::SelfFertile),
        Some("parthenocarpic") => Some(ReproductiveRole::Parthenocarpic),
        None => None,
        Some(_) => return Err(OrchardStorageError::TreesCouldNotBeRead),
    };
    let identification_status = match row.get::<_, &str>(22) {
        "confirmed" => IdentificationStatus::Confirmed,
        "uncertain" => IdentificationStatus::Uncertain,
        _ => return Err(OrchardStorageError::TreesCouldNotBeRead),
    };
    let botanical_taxon = serde_json::from_str(&row.get::<_, String>(19))
        .map_err(|_| OrchardStorageError::TreesCouldNotBeRead)?;

    Ok(OrchardTree {
        tree: Tree {
            legacy_source,
            plant_identity_id: PlantIdentityId(
                u64::try_from(row.get::<_, i64>(1))
                    .map_err(|_| OrchardStorageError::TreesCouldNotBeRead)?,
            ),
            longitude: row.get(2),
            latitude: row.get(3),
            planted_on: row.get(9),
            row_name: row.get(10),
            roles: row.get(11),
            is_alive: row.get(12),
            reproductive_role,
            harvest_start_day: optional_u16(row.get(14))?,
            harvest_end_day: optional_u16(row.get(15))?,
            adult_height_meters: row.get(16),
            adult_width_meters: row.get(17),
        },
        plant_identity: PlantIdentity {
            common_name: row.get(18),
            botanical_taxon,
            cultivar: row.get(20),
            trade_name: row.get(21),
            identification_status,
        },
    })
}

fn optional_u16(value: Option<i16>) -> Result<Option<u16>, OrchardStorageError> {
    value
        .map(|value| u16::try_from(value).map_err(|_| OrchardStorageError::TreesCouldNotBeRead))
        .transpose()
}

fn is_legacy_tree_already_imported(
    client: &mut Client,
    legacy_feature_id: u32,
) -> Result<bool, postgres::Error> {
    client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM trees WHERE legacy_feature_id = $1)",
            &[&(legacy_feature_id as i32)],
        )
        .map(|row| row.get(0))
}

fn find_or_create_plant_identity(
    client: &mut Client,
    plant_identity: PlantIdentity,
) -> Result<PlantIdentityId, OrchardStorageError> {
    let botanical_taxon = serde_json::to_string(&plant_identity.botanical_taxon)
        .map_err(|_| OrchardStorageError::PlantIdentityCouldNotBeResolved)?;
    let identification_status = match &plant_identity.identification_status {
        IdentificationStatus::Confirmed => "confirmed",
        IdentificationStatus::Uncertain => "uncertain",
    };
    let identity_key = plant_identity.catalog_key();
    let returned_id = client
        .query_opt(
            "INSERT INTO plant_identities (
                common_name, botanical_taxon, cultivar, trade_name,
                identification_status, identity_key
            ) VALUES ($1, $2::TEXT::jsonb, $3, $4, $5, $6)
            ON CONFLICT (identity_key) DO NOTHING
            RETURNING id",
            &[
                &plant_identity.common_name,
                &botanical_taxon,
                &plant_identity.cultivar,
                &plant_identity.trade_name,
                &identification_status,
                &identity_key,
            ],
        )
        .map_err(|_| OrchardStorageError::PlantIdentityCouldNotBeResolved)?
        .map(|row| row.get::<_, i64>(0));
    let id = match returned_id {
        Some(id) => id,
        None => client
            .query_one(
                "SELECT id FROM plant_identities WHERE identity_key = $1",
                &[&identity_key],
            )
            .map_err(|_| OrchardStorageError::PlantIdentityCouldNotBeResolved)?
            .get::<_, i64>(0),
    };
    let id = u64::try_from(id).map_err(|_| OrchardStorageError::PlantIdentityCouldNotBeResolved)?;
    Ok(PlantIdentityId(id))
}

fn save_tree(client: &mut Client, tree: Tree) -> Result<(), postgres::Error> {
    let legacy_feature_id = tree
        .legacy_source
        .as_ref()
        .map(|source| source.feature_id as i32);
    let legacy_name = tree
        .legacy_source
        .as_ref()
        .map(|source| source.name.as_str());
    let legacy_latin_name = tree
        .legacy_source
        .as_ref()
        .map(|source| source.latin_name.as_str());
    let legacy_source_url = tree
        .legacy_source
        .as_ref()
        .and_then(|source| source.source_url.as_deref());
    let legacy_identification_name = tree.legacy_source.as_ref().and_then(|source| {
        source
            .legacy_identification
            .as_ref()
            .map(|identification| identification.name.as_str())
    });
    let legacy_identification_latin_name = tree.legacy_source.as_ref().and_then(|source| {
        source
            .legacy_identification
            .as_ref()
            .map(|identification| identification.latin_name.as_str())
    });
    let plant_identity_id = tree.plant_identity_id.0 as i64;
    let reproductive_role = tree.reproductive_role.map(|role| match role {
        ReproductiveRole::Female => "female",
        ReproductiveRole::Male => "male",
        ReproductiveRole::SelfFertile => "self_fertile",
        ReproductiveRole::Parthenocarpic => "parthenocarpic",
    });
    let harvest_start_day = tree.harvest_start_day.map(|day| day as i16);
    let harvest_end_day = tree.harvest_end_day.map(|day| day as i16);
    client
        .execute(
            "INSERT INTO trees (
                legacy_feature_id, plant_identity_id, location,
                legacy_name, legacy_latin_name,
                legacy_source_url,
                legacy_identification_name, legacy_identification_latin_name,
                planted_on, row_name, roles, is_alive, reproductive_role,
                harvest_start_day, harvest_end_day,
                adult_height_meters, adult_width_meters
            ) VALUES (
                $1, $2, ST_SetSRID(ST_MakePoint($3, $4), 4326),
                $5, $6, $7, $8, $9, $10::TEXT::DATE, $11,
                $12, $13, $14, $15, $16, $17, $18
            )",
            &[
                &legacy_feature_id,
                &plant_identity_id,
                &tree.longitude,
                &tree.latitude,
                &legacy_name,
                &legacy_latin_name,
                &legacy_source_url,
                &legacy_identification_name,
                &legacy_identification_latin_name,
                &tree.planted_on,
                &tree.row_name,
                &tree.roles,
                &tree.is_alive,
                &reproductive_role,
                &harvest_start_day,
                &harvest_end_day,
                &tree.adult_height_meters,
                &tree.adult_width_meters,
            ],
        )
        .map(|_| ())
}
