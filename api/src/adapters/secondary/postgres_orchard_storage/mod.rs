use postgres::{Client, NoTls};

use crate::hexagon::models::Tree;
use crate::hexagon::ports::{
    OrchardTransaction, OrchardTransactionError, OrchardUnitOfWork, TreeRepository,
    TreeRepositoryError,
};

/// PostgreSQL/PostGIS storage family. Ordinary repository calls and import
/// transactions address the same database; a transaction owns its connection.
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

impl TreeRepository for PostgresOrchardStorage {
    fn has_legacy_feature_id(
        &mut self,
        legacy_feature_id: u32,
    ) -> Result<bool, TreeRepositoryError> {
        let mut client = Client::connect(&self.database_url, NoTls)
            .map_err(|_| TreeRepositoryError::CouldNotCheckExistingLegacyFeature)?;
        has_legacy_feature_id(&mut client, legacy_feature_id)
    }

    fn save(&mut self, tree: Tree) -> Result<(), TreeRepositoryError> {
        let mut client = Client::connect(&self.database_url, NoTls)
            .map_err(|_| TreeRepositoryError::TreeCouldNotBeSaved)?;
        save_tree(&mut client, tree)
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

impl TreeRepository for PostgresOrchardTransaction {
    fn has_legacy_feature_id(
        &mut self,
        legacy_feature_id: u32,
    ) -> Result<bool, TreeRepositoryError> {
        has_legacy_feature_id(&mut self.client, legacy_feature_id)
    }

    fn save(&mut self, tree: Tree) -> Result<(), TreeRepositoryError> {
        save_tree(&mut self.client, tree)
    }
}

impl OrchardTransaction for PostgresOrchardTransaction {
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

fn has_legacy_feature_id(
    client: &mut Client,
    legacy_feature_id: u32,
) -> Result<bool, TreeRepositoryError> {
    client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM trees WHERE legacy_feature_id = $1)",
            &[&(legacy_feature_id as i32)],
        )
        .map(|row| row.get(0))
        .map_err(|_| TreeRepositoryError::CouldNotCheckExistingLegacyFeature)
}

fn save_tree(client: &mut Client, tree: Tree) -> Result<(), TreeRepositoryError> {
    let legacy_feature_id = tree.legacy_feature_id.map(|id| id as i32);
    let harvest_start_day = tree.harvest_start_day.map(|day| day as i16);
    let harvest_end_day = tree.harvest_end_day.map(|day| day as i16);
    client
        .execute(
            "INSERT INTO trees (
                legacy_feature_id, location, name, latin_name, planted_on, row_name,
                roles, is_alive, harvest_start_day, harvest_end_day,
                adult_height_meters, adult_width_meters
            ) VALUES (
                $1, ST_SetSRID(ST_MakePoint($2, $3), 4326), $4, $5, $6::TEXT::DATE, $7,
                $8, $9, $10, $11, $12, $13
            )",
            &[
                &legacy_feature_id,
                &tree.longitude,
                &tree.latitude,
                &tree.name,
                &tree.latin_name,
                &tree.planted_on,
                &tree.row_name,
                &tree.roles,
                &tree.is_alive,
                &harvest_start_day,
                &harvest_end_day,
                &tree.adult_height_meters,
                &tree.adult_width_meters,
            ],
        )
        .map(|_| ())
        .map_err(|_| TreeRepositoryError::TreeCouldNotBeSaved)
}
