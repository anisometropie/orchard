use std::{collections::BTreeMap, error::Error, fmt};

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};

const MIGRATION_ADVISORY_LOCK_ID: i64 = 6_741_903_771;
const LEGACY_BASELINE_LAST_VERSION: u32 = 10;

#[derive(Clone, Copy)]
struct Migration {
    version: u32,
    name: &'static str,
    sql: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/embedded_migrations.rs"));

#[derive(Debug, PartialEq)]
pub struct MigrationReport {
    pub applied_versions: Vec<u32>,
    pub adopted_legacy_schema: bool,
}

#[derive(Debug, PartialEq)]
pub enum MigrationError {
    CouldNotConnect,
    CouldNotAcquireLock,
    CouldNotReleaseLock,
    MigrationLedgerCouldNotBeRead,
    UnexpectedLegacySchema,
    UnknownAppliedMigration { version: u32 },
    AppliedMigrationNameChanged { version: u32 },
    AppliedMigrationChecksumChanged { version: u32 },
    AppliedMigrationHistoryHasGap { version: u32 },
    InvalidMigrationTransactionWrapper { version: u32 },
    MigrationCouldNotBeApplied { version: u32, reason: String },
}

impl fmt::Display for MigrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CouldNotConnect => write!(formatter, "could not connect to the database"),
            Self::CouldNotAcquireLock => write!(formatter, "could not acquire the migration lock"),
            Self::CouldNotReleaseLock => write!(formatter, "could not release the migration lock"),
            Self::MigrationLedgerCouldNotBeRead => {
                write!(formatter, "could not read or update the migration ledger")
            }
            Self::UnexpectedLegacySchema => write!(
                formatter,
                "the untracked database is neither empty nor the expected version-10 schema"
            ),
            Self::UnknownAppliedMigration { version } => {
                write!(
                    formatter,
                    "applied migration {version} is unknown to this binary"
                )
            }
            Self::AppliedMigrationNameChanged { version } => {
                write!(formatter, "applied migration {version} has been renamed")
            }
            Self::AppliedMigrationChecksumChanged { version } => {
                write!(formatter, "applied migration {version} has been modified")
            }
            Self::AppliedMigrationHistoryHasGap { version } => write!(
                formatter,
                "migration {version} is applied after a missing earlier migration"
            ),
            Self::InvalidMigrationTransactionWrapper { version } => write!(
                formatter,
                "migration {version} has an invalid BEGIN/COMMIT wrapper"
            ),
            Self::MigrationCouldNotBeApplied { version, reason } => {
                write!(
                    formatter,
                    "migration {version} could not be applied: {reason}"
                )
            }
        }
    }
}

impl Error for MigrationError {}

pub struct PostgresMigrator {
    client: Client,
}

impl PostgresMigrator {
    pub fn connect(database_url: &str) -> Result<Self, MigrationError> {
        Client::connect(database_url, NoTls)
            .map(|client| Self { client })
            .map_err(|_| MigrationError::CouldNotConnect)
    }

    pub fn from_client(client: Client) -> Self {
        Self { client }
    }

    pub fn client(&mut self) -> &mut Client {
        &mut self.client
    }

    pub fn migrate(&mut self) -> Result<MigrationReport, MigrationError> {
        self.client
            .query_one(
                "SELECT pg_advisory_lock($1)",
                &[&MIGRATION_ADVISORY_LOCK_ID],
            )
            .map_err(|_| MigrationError::CouldNotAcquireLock)?;

        let migration_result = self.migrate_while_locked(MIGRATIONS);
        let unlock_result = self
            .client
            .query_one(
                "SELECT pg_advisory_unlock($1)",
                &[&MIGRATION_ADVISORY_LOCK_ID],
            )
            .map_err(|_| MigrationError::CouldNotReleaseLock)
            .and_then(|row| {
                if row.get::<_, bool>(0) {
                    Ok(())
                } else {
                    Err(MigrationError::CouldNotReleaseLock)
                }
            });

        match (migration_result, unlock_result) {
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
            (Ok(report), Ok(())) => Ok(report),
        }
    }

    fn migrate_while_locked(
        &mut self,
        migrations: &[Migration],
    ) -> Result<MigrationReport, MigrationError> {
        self.create_ledger()?;
        let mut applied = self.applied_migrations()?;
        let adopted_legacy_schema = if applied.is_empty() && !self.database_is_empty()? {
            if !self.database_is_expected_legacy_v10()? {
                return Err(MigrationError::UnexpectedLegacySchema);
            }
            self.adopt_legacy_v10(migrations)?;
            applied = self.applied_migrations()?;
            true
        } else {
            false
        };
        validate_history(migrations, &applied)?;

        let mut applied_versions = Vec::new();
        for migration in migrations {
            if applied.contains_key(&migration.version) {
                continue;
            }
            self.apply_migration(*migration)?;
            applied_versions.push(migration.version);
        }
        Ok(MigrationReport {
            applied_versions,
            adopted_legacy_schema,
        })
    }

    fn create_ledger(&mut self) -> Result<(), MigrationError> {
        self.client
            .batch_execute(
                "CREATE TABLE IF NOT EXISTS orchard_schema_migrations (
                    version INTEGER PRIMARY KEY CHECK (version > 0),
                    name TEXT NOT NULL,
                    checksum TEXT NOT NULL,
                    applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
                 )",
            )
            .map_err(|_| MigrationError::MigrationLedgerCouldNotBeRead)
    }

    fn applied_migrations(&mut self) -> Result<BTreeMap<u32, AppliedMigration>, MigrationError> {
        self.client
            .query(
                "SELECT version, name, checksum
                 FROM orchard_schema_migrations
                 ORDER BY version",
                &[],
            )
            .map_err(|_| MigrationError::MigrationLedgerCouldNotBeRead)?
            .into_iter()
            .map(|row| {
                let version = u32::try_from(row.get::<_, i32>(0))
                    .map_err(|_| MigrationError::MigrationLedgerCouldNotBeRead)?;
                Ok((
                    version,
                    AppliedMigration {
                        name: row.get(1),
                        checksum: row.get(2),
                    },
                ))
            })
            .collect()
    }

    fn database_is_empty(&mut self) -> Result<bool, MigrationError> {
        self.client
            .query_one(
                "SELECT
                    NOT EXISTS (
                        SELECT 1
                        FROM pg_class relation
                        JOIN pg_namespace namespace
                          ON namespace.oid = relation.relnamespace
                        WHERE namespace.nspname = current_schema()
                          AND relation.relkind IN ('r', 'p', 'v', 'm', 'S', 'f')
                          AND relation.relname <> 'orchard_schema_migrations'
                          AND NOT EXISTS (
                              SELECT 1
                              FROM pg_depend dependency
                              WHERE dependency.classid = 'pg_class'::regclass
                                AND dependency.objid = relation.oid
                                AND dependency.deptype = 'e'
                          )
                    )
                    AND NOT EXISTS (
                        SELECT 1
                        FROM pg_type data_type
                        JOIN pg_namespace namespace
                          ON namespace.oid = data_type.typnamespace
                        WHERE namespace.nspname = current_schema()
                          AND data_type.typtype IN ('d', 'e')
                          AND NOT EXISTS (
                              SELECT 1
                              FROM pg_depend dependency
                              WHERE dependency.classid = 'pg_type'::regclass
                                AND dependency.objid = data_type.oid
                                AND dependency.deptype = 'e'
                          )
                    )",
                &[],
            )
            .map(|row| row.get(0))
            .map_err(|_| MigrationError::MigrationLedgerCouldNotBeRead)
    }

    fn database_is_expected_legacy_v10(&mut self) -> Result<bool, MigrationError> {
        self.client
            .query_one(
                "SELECT
                    to_regclass(format('%I.trees', current_schema())) IS NOT NULL
                    AND to_regclass(format('%I.plant_identities', current_schema())) IS NOT NULL
                    AND to_regclass(format('%I.plant_cultivars', current_schema())) IS NOT NULL
                    AND to_regclass(format('%I.plant_harvest_windows', current_schema())) IS NOT NULL
                    AND to_regclass(format('%I.users', current_schema())) IS NOT NULL
                    AND to_regclass(format('%I.aerial_overlays', current_schema())) IS NOT NULL
                    AND to_regclass(format('%I.orchards', current_schema())) IS NULL
                    AND to_regtype(format('%I.harvested_part', current_schema())) IS NOT NULL
                    AND to_regtype(format('%I.harvest_data_origin', current_schema())) IS NOT NULL
                    AND EXISTS (
                        SELECT 1 FROM information_schema.columns
                        WHERE table_schema = current_schema()
                          AND table_name = 'plant_harvest_windows'
                          AND column_name = 'reference_region'
                    )
                    AND EXISTS (
                        SELECT 1 FROM information_schema.columns
                        WHERE table_schema = current_schema()
                          AND table_name = 'plant_harvest_windows'
                          AND column_name = 'source_url'
                    )
                    AND NOT EXISTS (
                        SELECT 1 FROM information_schema.columns
                        WHERE table_schema = current_schema()
                          AND table_name = 'plant_identities'
                          AND column_name = 'harvest_start_month'
                    )",
                &[],
            )
            .map(|row| row.get(0))
            .map_err(|_| MigrationError::MigrationLedgerCouldNotBeRead)
    }

    fn adopt_legacy_v10(&mut self, migrations: &[Migration]) -> Result<(), MigrationError> {
        let mut transaction = self
            .client
            .transaction()
            .map_err(|_| MigrationError::MigrationLedgerCouldNotBeRead)?;
        for migration in migrations
            .iter()
            .filter(|migration| migration.version <= LEGACY_BASELINE_LAST_VERSION)
        {
            transaction
                .execute(
                    "INSERT INTO orchard_schema_migrations (version, name, checksum)
                     VALUES ($1, $2, $3)",
                    &[
                        &(migration.version as i32),
                        &migration.name,
                        &checksum(migration.sql),
                    ],
                )
                .map_err(|_| MigrationError::MigrationLedgerCouldNotBeRead)?;
        }
        transaction
            .commit()
            .map_err(|_| MigrationError::MigrationLedgerCouldNotBeRead)
    }

    fn apply_migration(&mut self, migration: Migration) -> Result<(), MigrationError> {
        let sql = migration_body(migration)?;
        let mut transaction = self
            .client
            .transaction()
            .map_err(|error| migration_failure(migration.version, error))?;
        transaction
            .batch_execute(sql)
            .map_err(|error| migration_failure(migration.version, error))?;
        transaction
            .execute(
                "INSERT INTO orchard_schema_migrations (version, name, checksum)
                 VALUES ($1, $2, $3)",
                &[
                    &(migration.version as i32),
                    &migration.name,
                    &checksum(migration.sql),
                ],
            )
            .map_err(|error| migration_failure(migration.version, error))?;
        transaction
            .commit()
            .map_err(|error| migration_failure(migration.version, error))
    }
}

#[derive(Debug)]
struct AppliedMigration {
    name: String,
    checksum: String,
}

fn validate_history(
    migrations: &[Migration],
    applied: &BTreeMap<u32, AppliedMigration>,
) -> Result<(), MigrationError> {
    for version in applied.keys() {
        if !migrations
            .iter()
            .any(|migration| migration.version == *version)
        {
            return Err(MigrationError::UnknownAppliedMigration { version: *version });
        }
    }

    let mut missing_earlier_migration = false;
    for migration in migrations {
        match applied.get(&migration.version) {
            None => missing_earlier_migration = true,
            Some(_) if missing_earlier_migration => {
                return Err(MigrationError::AppliedMigrationHistoryHasGap {
                    version: migration.version,
                });
            }
            Some(applied_migration) => {
                if applied_migration.name != migration.name {
                    return Err(MigrationError::AppliedMigrationNameChanged {
                        version: migration.version,
                    });
                }
                if applied_migration.checksum != checksum(migration.sql) {
                    return Err(MigrationError::AppliedMigrationChecksumChanged {
                        version: migration.version,
                    });
                }
            }
        }
    }
    Ok(())
}

fn migration_body(migration: Migration) -> Result<&'static str, MigrationError> {
    let sql = migration.sql.trim();
    let begins_transaction = sql.starts_with("BEGIN;");
    let commits_transaction = sql.ends_with("COMMIT;");
    match (begins_transaction, commits_transaction) {
        (true, true) => Ok(sql
            .strip_prefix("BEGIN;")
            .and_then(|sql| sql.strip_suffix("COMMIT;"))
            .expect("a checked transaction wrapper should be removable")
            .trim()),
        (false, false) => Ok(sql),
        _ => Err(MigrationError::InvalidMigrationTransactionWrapper {
            version: migration.version,
        }),
    }
}

fn checksum(sql: &str) -> String {
    Sha256::digest(sql.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn migration_failure(version: u32, error: postgres::Error) -> MigrationError {
    MigrationError::MigrationCouldNotBeApplied {
        version,
        reason: error.to_string(),
    }
}

#[cfg(all(test, feature = "postgres-integration"))]
mod postgres_migrator_integration_test;
