use postgres::{Client, NoTls};

use crate::hexagon::models::{
    IdentificationStatus, PlantIdentity, PlantIdentityId, ReproductiveRole, Tree,
};
use crate::hexagon::ports::{OrchardTransaction, OrchardTransactionError, OrchardUnitOfWork};

/// PostgreSQL/PostGIS implementation of the orchard unit of work.
/// Each transaction owns its database connection.
pub struct PostgresOrchardStorage {
    database_url: String,
}

pub struct PostgresOrchardTransaction {
    client: Client,
    completed: bool,
}

impl PostgresOrchardStorage {
    pub fn connect(database_url: &str) -> Result<Self, OrchardTransactionError> {
        Client::connect(database_url, NoTls).map_err(|_| OrchardTransactionError::CouldNotBegin)?;
        Ok(Self {
            database_url: database_url.into(),
        })
    }
}

impl OrchardUnitOfWork for PostgresOrchardStorage {
    type Transaction = PostgresOrchardTransaction;

    fn begin(&mut self) -> Result<Self::Transaction, OrchardTransactionError> {
        let mut client = Client::connect(&self.database_url, NoTls)
            .map_err(|_| OrchardTransactionError::CouldNotBegin)?;
        client
            .batch_execute("BEGIN")
            .map_err(|_| OrchardTransactionError::CouldNotBegin)?;
        Ok(PostgresOrchardTransaction {
            client,
            completed: false,
        })
    }
}

impl OrchardTransaction for PostgresOrchardTransaction {
    fn is_legacy_tree_already_imported(
        &mut self,
        legacy_feature_id: u32,
    ) -> Result<bool, OrchardTransactionError> {
        is_legacy_tree_already_imported(&mut self.client, legacy_feature_id)
            .map_err(|_| OrchardTransactionError::CouldNotCheckExistingLegacyTree)
    }

    fn find_or_create_plant_identity(
        &mut self,
        plant_identity: PlantIdentity,
    ) -> Result<PlantIdentityId, OrchardTransactionError> {
        find_or_create_plant_identity(&mut self.client, plant_identity)
    }

    fn save_tree(&mut self, tree: Tree) -> Result<(), OrchardTransactionError> {
        save_tree(&mut self.client, tree).map_err(|_| OrchardTransactionError::TreeCouldNotBeSaved)
    }

    fn commit(mut self) -> Result<(), OrchardTransactionError> {
        match self.client.batch_execute("COMMIT") {
            Ok(()) => {
                self.completed = true;
                Ok(())
            }
            Err(_) => {
                let _ = self.client.batch_execute("ROLLBACK");
                self.completed = true;
                Err(OrchardTransactionError::CouldNotCommit)
            }
        }
    }

    fn rollback(mut self) {
        let _ = self.client.batch_execute("ROLLBACK");
        self.completed = true;
    }
}

impl Drop for PostgresOrchardTransaction {
    fn drop(&mut self) {
        if !self.completed {
            let _ = self.client.batch_execute("ROLLBACK");
        }
    }
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
) -> Result<PlantIdentityId, OrchardTransactionError> {
    let botanical_taxon = serde_json::to_string(&plant_identity.botanical_taxon)
        .map_err(|_| OrchardTransactionError::PlantIdentityCouldNotBeResolved)?;
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
        .map_err(|_| OrchardTransactionError::PlantIdentityCouldNotBeResolved)?
        .map(|row| row.get::<_, i64>(0));
    let id = match returned_id {
        Some(id) => id,
        None => client
            .query_one(
                "SELECT id FROM plant_identities WHERE identity_key = $1",
                &[&identity_key],
            )
            .map_err(|_| OrchardTransactionError::PlantIdentityCouldNotBeResolved)?
            .get::<_, i64>(0),
    };
    let id =
        u64::try_from(id).map_err(|_| OrchardTransactionError::PlantIdentityCouldNotBeResolved)?;
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
