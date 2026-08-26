use postgres::{Client, NoTls};

use crate::hexagon::models::Tree;
use crate::hexagon::ports::{OrchardImportTransaction, OrchardTransactionError, OrchardUnitOfWork};

pub struct PostgresOrchardUnitOfWork {
    database_url: String,
}

pub struct PostgresOrchardImportTransaction {
    client: Client,
    completed: bool,
}

impl PostgresOrchardUnitOfWork {
    pub fn connect(database_url: &str) -> Result<Self, OrchardTransactionError> {
        Client::connect(database_url, NoTls).map_err(|_| OrchardTransactionError::CouldNotBegin)?;
        Ok(Self {
            database_url: database_url.into(),
        })
    }
}

impl OrchardUnitOfWork for PostgresOrchardUnitOfWork {
    type Transaction = PostgresOrchardImportTransaction;

    fn begin(&mut self) -> Result<Self::Transaction, OrchardTransactionError> {
        let mut client = Client::connect(&self.database_url, NoTls)
            .map_err(|_| OrchardTransactionError::CouldNotBegin)?;
        client
            .batch_execute("BEGIN")
            .map_err(|_| OrchardTransactionError::CouldNotBegin)?;
        Ok(PostgresOrchardImportTransaction {
            client,
            completed: false,
        })
    }
}

impl OrchardImportTransaction for PostgresOrchardImportTransaction {
    fn has_legacy_feature_id(
        &mut self,
        legacy_feature_id: u32,
    ) -> Result<bool, OrchardTransactionError> {
        self.client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM trees WHERE legacy_feature_id = $1)",
                &[&(legacy_feature_id as i32)],
            )
            .map(|row| row.get(0))
            .map_err(|_| OrchardTransactionError::CouldNotCheckExistingLegacyFeature)
    }

    fn save_tree(&mut self, tree: Tree) -> Result<(), OrchardTransactionError> {
        let legacy_feature_id = tree.legacy_feature_id.map(|id| id as i32);
        let harvest_start_day = tree.harvest_start_day.map(|day| day as i16);
        let harvest_end_day = tree.harvest_end_day.map(|day| day as i16);
        self.client
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
            .map_err(|_| OrchardTransactionError::TreeCouldNotBeSaved)
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

impl Drop for PostgresOrchardImportTransaction {
    fn drop(&mut self) {
        if !self.completed {
            let _ = self.client.batch_execute("ROLLBACK");
        }
    }
}
