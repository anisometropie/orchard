use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng as PasswordOsRng},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use postgres::{Client, NoTls, Row};
use rand::{RngCore, rngs::OsRng};
use sha2::{Digest, Sha256};

use crate::hexagon::models::{
    AerialOverlay, AerialOverlayId, AerialOverlayImage, AnnualDate, AnnualHarvestWindow, GeoPoint,
    HarvestDataOrigin, HarvestScheduleOwner, HarvestedPart, IdentificationStatus,
    LegacyPlantIdentification, LegacyTreeSource, MapConfiguration, Orchard, OrchardId,
    OrchardShareAccess, OrchardSharePermission, OrchardTree, PlantCultivar, PlantCultivarId,
    PlantIdentification, PlantIdentity, PlantIdentityId, PlantIdentityReference, ReproductiveRole,
    Tree, TreeId, User, UserId, WateringRun, WateringRunId, WateringRunTarget,
};
use crate::hexagon::ports::{
    AccessControl, AccessControlError, MapConfigurationStorage, MapConfigurationStorageError,
    OrchardStorage, OrchardStorageError,
};

/// PostgreSQL/PostGIS implementation of orchard storage.
pub struct PostgresOrchardStorage {
    client: Client,
}

impl AccessControl for PostgresOrchardStorage {
    fn verify_credentials(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<Option<User>, AccessControlError> {
        let user = self
            .client
            .query_opt(
                "SELECT id, username, password_hash FROM users WHERE username = $1",
                &[&username],
            )
            .map_err(|_| AccessControlError::CredentialsCouldNotBeChecked)?;
        let Some(user) = user else {
            return Ok(None);
        };
        let Some(password_hash) = user.get::<_, Option<String>>(2) else {
            return Ok(None);
        };
        let parsed_hash = PasswordHash::new(&password_hash)
            .map_err(|_| AccessControlError::CredentialsCouldNotBeChecked)?;
        if Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_err()
        {
            return Ok(None);
        }
        Ok(Some(User {
            id: UserId(
                u64::try_from(user.get::<_, i64>(0))
                    .map_err(|_| AccessControlError::CredentialsCouldNotBeChecked)?,
            ),
            username: user.get(1),
        }))
    }

    fn create_session(&mut self, user_id: UserId) -> Result<String, AccessControlError> {
        let user_id =
            i64::try_from(user_id.0).map_err(|_| AccessControlError::SessionCouldNotBeCreated)?;
        let token = random_access_token();
        let token_hash = access_token_hash(&token);
        self.client
            .execute(
                "INSERT INTO user_sessions (token_hash, user_id, expires_at)
                 VALUES ($1, $2, now() + interval '30 days')",
                &[&token_hash, &user_id],
            )
            .map_err(|_| AccessControlError::SessionCouldNotBeCreated)?;
        Ok(token)
    }

    fn orchards_owned_by(&mut self, user_id: UserId) -> Result<Vec<Orchard>, AccessControlError> {
        let user_id =
            i64::try_from(user_id.0).map_err(|_| AccessControlError::OrchardsCouldNotBeRead)?;
        self.client
            .query(
                "SELECT id, name, ST_X(center), ST_Y(center), reference_region
                 FROM orchards
                 WHERE owner_user_id = $1
                 ORDER BY id",
                &[&user_id],
            )
            .map_err(|_| AccessControlError::OrchardsCouldNotBeRead)?
            .into_iter()
            .map(|row| {
                orchard_from_row(&row).map_err(|_| AccessControlError::OrchardsCouldNotBeRead)
            })
            .collect()
    }

    fn user_for_session(&mut self, token: &str) -> Result<Option<User>, AccessControlError> {
        let token_hash = access_token_hash(token);
        self.client
            .query_opt(
                "SELECT users.id, users.username
                 FROM user_sessions
                 JOIN users ON users.id = user_sessions.user_id
                 WHERE user_sessions.token_hash = $1
                   AND user_sessions.expires_at > now()",
                &[&token_hash],
            )
            .map_err(|_| AccessControlError::SessionCouldNotBeRead)?
            .map(|row| {
                Ok(User {
                    id: UserId(
                        u64::try_from(row.get::<_, i64>(0))
                            .map_err(|_| AccessControlError::SessionCouldNotBeRead)?,
                    ),
                    username: row.get(1),
                })
            })
            .transpose()
    }

    fn user_owns_orchard(
        &mut self,
        user_id: UserId,
        orchard_id: OrchardId,
    ) -> Result<bool, AccessControlError> {
        let user_id = i64::try_from(user_id.0)
            .map_err(|_| AccessControlError::OrchardOwnershipCouldNotBeRead)?;
        let orchard_id = i64::try_from(orchard_id.0)
            .map_err(|_| AccessControlError::OrchardOwnershipCouldNotBeRead)?;
        self.client
            .query_one(
                "SELECT EXISTS (
                    SELECT 1 FROM orchards WHERE id = $1 AND owner_user_id = $2
                 )",
                &[&orchard_id, &user_id],
            )
            .map(|row| row.get(0))
            .map_err(|_| AccessControlError::OrchardOwnershipCouldNotBeRead)
    }

    fn create_share_token(
        &mut self,
        user_id: UserId,
        orchard_id: OrchardId,
        permission: OrchardSharePermission,
    ) -> Result<String, AccessControlError> {
        let user_id = i64::try_from(user_id.0)
            .map_err(|_| AccessControlError::ShareTokenCouldNotBeCreated)?;
        let orchard_id = i64::try_from(orchard_id.0)
            .map_err(|_| AccessControlError::ShareTokenCouldNotBeCreated)?;
        let token = random_access_token();
        let token_hash = access_token_hash(&token);
        let permission = match permission {
            OrchardSharePermission::View => "view",
            OrchardSharePermission::Watering => "watering",
        };
        let changed = self
            .client
            .execute(
                "INSERT INTO orchard_share_tokens (orchard_id, permission, token_hash)
                 SELECT id, $3, $4
                 FROM orchards
                 WHERE id = $1 AND owner_user_id = $2",
                &[&orchard_id, &user_id, &permission, &token_hash],
            )
            .map_err(|_| AccessControlError::ShareTokenCouldNotBeCreated)?;
        if changed != 1 {
            return Err(AccessControlError::ShareTokenCouldNotBeCreated);
        }
        Ok(token)
    }

    fn orchard_share_for_token(
        &mut self,
        token: &str,
    ) -> Result<Option<OrchardShareAccess>, AccessControlError> {
        let token_hash = access_token_hash(token);
        let access = self
            .client
            .query_opt(
                "SELECT orchard_id, permission
                 FROM orchard_share_tokens
                 WHERE token_hash = $1",
                &[&token_hash],
            )
            .map_err(|_| AccessControlError::ShareTokenCouldNotBeRead)?;
        access
            .map(|row| {
                let orchard_id = u64::try_from(row.get::<_, i64>(0))
                    .map(OrchardId)
                    .map_err(|_| AccessControlError::ShareTokenCouldNotBeRead)?;
                let permission = match row.get::<_, String>(1).as_str() {
                    "view" => OrchardSharePermission::View,
                    "watering" => OrchardSharePermission::Watering,
                    _ => return Err(AccessControlError::ShareTokenCouldNotBeRead),
                };
                Ok(OrchardShareAccess {
                    orchard_id,
                    permission,
                })
            })
            .transpose()
    }

    fn delete_session(&mut self, token: &str) -> Result<(), AccessControlError> {
        let token_hash = access_token_hash(token);
        self.client
            .execute(
                "DELETE FROM user_sessions WHERE token_hash = $1",
                &[&token_hash],
            )
            .map(|_| ())
            .map_err(|_| AccessControlError::SessionCouldNotBeDeleted)
    }

    fn set_user_password(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<bool, AccessControlError> {
        let password_hash = Argon2::default()
            .hash_password(
                password.as_bytes(),
                &SaltString::generate(&mut PasswordOsRng),
            )
            .map_err(|_| AccessControlError::PasswordCouldNotBeChanged)?
            .to_string();
        self.client
            .query_one(
                "WITH changed_user AS (
                    UPDATE users
                    SET password_hash = $2
                    WHERE username = $1
                    RETURNING id
                 ), revoked_sessions AS (
                    DELETE FROM user_sessions
                    USING changed_user
                    WHERE user_sessions.user_id = changed_user.id
                 )
                 SELECT EXISTS (SELECT 1 FROM changed_user)",
                &[&username, &password_hash],
            )
            .map(|row| row.get(0))
            .map_err(|_| AccessControlError::PasswordCouldNotBeChanged)
    }
}

fn random_access_token() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn access_token_hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
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

    fn resolve_plant_identification(
        &mut self,
        plant_identification: PlantIdentification,
    ) -> Result<PlantIdentityReference, OrchardStorageError> {
        resolve_plant_identification(&mut self.client, plant_identification)
    }

    fn replace_harvest_windows(
        &mut self,
        owner: HarvestScheduleOwner,
        harvest_windows: Vec<AnnualHarvestWindow>,
    ) -> Result<bool, OrchardStorageError> {
        let (plant_identity_id, cultivar_id) = match owner {
            HarvestScheduleOwner::PlantIdentity(id) => {
                let id = i64::try_from(id.0)
                    .map_err(|_| OrchardStorageError::HarvestWindowsCouldNotBeReplaced)?;
                let exists = self
                    .client
                    .query_opt("SELECT id FROM plant_identities WHERE id = $1", &[&id])
                    .map_err(|_| OrchardStorageError::HarvestWindowsCouldNotBeReplaced)?
                    .is_some();
                if !exists {
                    return Ok(false);
                }
                (id, None)
            }
            HarvestScheduleOwner::PlantCultivar(id) => {
                let id = i64::try_from(id.0)
                    .map_err(|_| OrchardStorageError::HarvestWindowsCouldNotBeReplaced)?;
                let identity = self
                    .client
                    .query_opt(
                        "SELECT plant_identity_id FROM plant_cultivars WHERE id = $1",
                        &[&id],
                    )
                    .map_err(|_| OrchardStorageError::HarvestWindowsCouldNotBeReplaced)?;
                let Some(identity) = identity else {
                    return Ok(false);
                };
                (identity.get::<_, i64>(0), Some(id))
            }
        };

        self.client
            .execute(
                "DELETE FROM plant_harvest_windows
                 WHERE plant_identity_id = $1
                   AND cultivar_id IS NOT DISTINCT FROM $2",
                &[&plant_identity_id, &cultivar_id],
            )
            .map_err(|_| OrchardStorageError::HarvestWindowsCouldNotBeReplaced)?;
        for window in harvest_windows {
            self.client
                .execute(
                    "INSERT INTO plant_harvest_windows (
                        plant_identity_id, cultivar_id,
                        start_month, start_day, end_month, end_day,
                        reference_region, harvested_part, data_origin, source_url
                     ) VALUES ($1, $2, $3, $4, $5, $6, $7,
                               $8::text::harvested_part,
                               $9::text::harvest_data_origin, $10)",
                    &[
                        &plant_identity_id,
                        &cultivar_id,
                        &i16::from(window.start.month),
                        &i16::from(window.start.day),
                        &i16::from(window.end.month),
                        &i16::from(window.end.day),
                        &window.reference_region,
                        &window.harvested_part.as_str(),
                        &window.data_origin.as_str(),
                        &window.source_url,
                    ],
                )
                .map_err(|_| OrchardStorageError::HarvestWindowsCouldNotBeReplaced)?;
        }
        Ok(true)
    }

    fn replace_orchard_harvest_windows(
        &mut self,
        orchard_id: OrchardId,
        owner: HarvestScheduleOwner,
        harvest_windows: Vec<AnnualHarvestWindow>,
    ) -> Result<bool, OrchardStorageError> {
        let orchard_id = i64::try_from(orchard_id.0)
            .map_err(|_| OrchardStorageError::HarvestWindowsCouldNotBeReplaced)?;
        let (plant_identity_id, cultivar_id) = match owner {
            HarvestScheduleOwner::PlantIdentity(id) => {
                let id = i64::try_from(id.0)
                    .map_err(|_| OrchardStorageError::HarvestWindowsCouldNotBeReplaced)?;
                let exists = self
                    .client
                    .query_opt("SELECT id FROM plant_identities WHERE id = $1", &[&id])
                    .map_err(|_| OrchardStorageError::HarvestWindowsCouldNotBeReplaced)?
                    .is_some();
                if !exists {
                    return Ok(false);
                }
                (id, None)
            }
            HarvestScheduleOwner::PlantCultivar(id) => {
                let id = i64::try_from(id.0)
                    .map_err(|_| OrchardStorageError::HarvestWindowsCouldNotBeReplaced)?;
                let identity = self
                    .client
                    .query_opt(
                        "SELECT plant_identity_id FROM plant_cultivars WHERE id = $1",
                        &[&id],
                    )
                    .map_err(|_| OrchardStorageError::HarvestWindowsCouldNotBeReplaced)?;
                let Some(identity) = identity else {
                    return Ok(false);
                };
                (identity.get::<_, i64>(0), Some(id))
            }
        };

        self.client
            .execute(
                "DELETE FROM plant_harvest_windows
                 WHERE orchard_id = $1
                   AND plant_identity_id = $2
                   AND cultivar_id IS NOT DISTINCT FROM $3",
                &[&orchard_id, &plant_identity_id, &cultivar_id],
            )
            .map_err(|_| OrchardStorageError::HarvestWindowsCouldNotBeReplaced)?;
        for window in harvest_windows {
            self.client
                .execute(
                    "INSERT INTO plant_harvest_windows (
                        orchard_id, plant_identity_id, cultivar_id,
                        start_month, start_day, end_month, end_day,
                        reference_region, harvested_part, data_origin, source_url
                     ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8,
                               $9::text::harvested_part,
                               $10::text::harvest_data_origin, $11)",
                    &[
                        &orchard_id,
                        &plant_identity_id,
                        &cultivar_id,
                        &i16::from(window.start.month),
                        &i16::from(window.start.day),
                        &i16::from(window.end.month),
                        &i16::from(window.end.day),
                        &window.reference_region,
                        &window.harvested_part.as_str(),
                        &window.data_origin.as_str(),
                        &window.source_url,
                    ],
                )
                .map_err(|_| OrchardStorageError::HarvestWindowsCouldNotBeReplaced)?;
        }
        Ok(true)
    }

    fn save_tree(&mut self, tree: Tree) -> Result<(), OrchardStorageError> {
        save_tree(&mut self.client, tree).map_err(|_| OrchardStorageError::TreeCouldNotBeSaved)
    }

    fn tree_is_alive(&mut self, tree_id: TreeId) -> Result<Option<bool>, OrchardStorageError> {
        let tree_id =
            i64::try_from(tree_id.0).map_err(|_| OrchardStorageError::TreeCouldNotBeRead)?;
        self.client
            .query_opt("SELECT is_alive FROM trees WHERE id = $1", &[&tree_id])
            .map(|row| row.map(|row| row.get(0)))
            .map_err(|_| OrchardStorageError::TreeCouldNotBeRead)
    }

    fn change_tree_danger(
        &mut self,
        tree_id: TreeId,
        is_in_danger: bool,
    ) -> Result<(), OrchardStorageError> {
        let tree_id = i64::try_from(tree_id.0)
            .map_err(|_| OrchardStorageError::TreeDangerCouldNotBeChanged)?;
        match self.client.execute(
            "UPDATE trees SET is_in_danger = $2 WHERE id = $1",
            &[&tree_id, &is_in_danger],
        ) {
            Ok(1) => Ok(()),
            _ => Err(OrchardStorageError::TreeDangerCouldNotBeChanged),
        }
    }

    fn change_tree_life_status(
        &mut self,
        tree_id: TreeId,
        is_alive: bool,
    ) -> Result<(), OrchardStorageError> {
        let tree_id = i64::try_from(tree_id.0)
            .map_err(|_| OrchardStorageError::TreeLifeStatusCouldNotBeChanged)?;
        match self.client.execute(
            "UPDATE trees SET is_alive = $2 WHERE id = $1",
            &[&tree_id, &is_alive],
        ) {
            Ok(1) => Ok(()),
            _ => Err(OrchardStorageError::TreeLifeStatusCouldNotBeChanged),
        }
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
                    t.reproductive_role, t.adult_height_meters, t.adult_width_meters,
                    t.is_in_danger, t.cultivar_id, t.identification_status,
                    p.common_name, p.botanical_taxon::text, c.cultivar, c.trade_name,
                    t.id, COALESCE(harvest.windows, '[]'), t.row_rank
                 FROM trees t
                 JOIN plant_identities p ON p.id = t.plant_identity_id
                 LEFT JOIN plant_cultivars c ON c.id = t.cultivar_id
                 LEFT JOIN LATERAL (
                    SELECT json_agg(
                        json_build_object(
                            'start_month', w.start_month,
                            'start_day', w.start_day,
                            'end_month', w.end_month,
                            'end_day', w.end_day,
                            'reference_region', w.reference_region,
                            'harvested_part', w.harvested_part,
                            'data_origin', w.data_origin,
                            'source_url', w.source_url
                        ) ORDER BY w.start_month, w.start_day, w.end_month, w.end_day, w.id
                    )::text AS windows
                    FROM plant_harvest_windows w
                    WHERE w.plant_identity_id = t.plant_identity_id
                      AND w.cultivar_id IS NOT DISTINCT FROM t.cultivar_id
                 ) harvest ON TRUE
                 ORDER BY t.id",
                &[],
            )
            .map_err(|_| OrchardStorageError::TreesCouldNotBeRead)?
            .into_iter()
            .map(|row| orchard_tree_from_row(&row))
            .collect()
    }

    fn trees_in_orchard(
        &mut self,
        orchard_id: OrchardId,
    ) -> Result<Vec<OrchardTree>, OrchardStorageError> {
        let orchard_id =
            i64::try_from(orchard_id.0).map_err(|_| OrchardStorageError::TreesCouldNotBeRead)?;
        self.client
            .query(
                "SELECT
                    t.legacy_feature_id, t.plant_identity_id,
                    ST_X(t.location), ST_Y(t.location),
                    t.legacy_name, t.legacy_latin_name, t.legacy_source_url,
                    t.legacy_identification_name, t.legacy_identification_latin_name,
                    t.planted_on::text, t.row_name, t.roles, t.is_alive,
                    t.reproductive_role, t.adult_height_meters, t.adult_width_meters,
                    t.is_in_danger, t.cultivar_id, t.identification_status,
                    p.common_name, p.botanical_taxon::text, c.cultivar, c.trade_name,
                    t.id, COALESCE(harvest.windows, '[]'), t.row_rank
                 FROM trees t
                 JOIN plant_identities p ON p.id = t.plant_identity_id
                 LEFT JOIN plant_cultivars c ON c.id = t.cultivar_id
                 LEFT JOIN LATERAL (
                    SELECT json_agg(
                        json_build_object(
                            'start_month', w.start_month,
                            'start_day', w.start_day,
                            'end_month', w.end_month,
                            'end_day', w.end_day,
                            'reference_region', w.reference_region,
                            'harvested_part', w.harvested_part,
                            'data_origin', w.data_origin,
                            'source_url', w.source_url
                        ) ORDER BY w.start_month, w.start_day, w.end_month, w.end_day, w.id
                    )::text AS windows
                    FROM plant_harvest_windows w
                    WHERE w.plant_identity_id = t.plant_identity_id
                      AND w.cultivar_id IS NOT DISTINCT FROM t.cultivar_id
                      AND w.orchard_id IS NOT DISTINCT FROM t.orchard_id
                 ) harvest ON TRUE
                 WHERE t.orchard_id = $1
                 ORDER BY t.id",
                &[&orchard_id],
            )
            .map_err(|_| OrchardStorageError::TreesCouldNotBeRead)?
            .into_iter()
            .map(|row| orchard_tree_from_row(&row))
            .collect()
    }

    fn tree_belongs_to_orchard(
        &mut self,
        tree_id: TreeId,
        orchard_id: OrchardId,
    ) -> Result<bool, OrchardStorageError> {
        let tree_id =
            i64::try_from(tree_id.0).map_err(|_| OrchardStorageError::TreeCouldNotBeRead)?;
        let orchard_id =
            i64::try_from(orchard_id.0).map_err(|_| OrchardStorageError::TreeCouldNotBeRead)?;
        self.client
            .query_opt(
                "SELECT 1 FROM trees WHERE id = $1 AND orchard_id = $2",
                &[&tree_id, &orchard_id],
            )
            .map(|row| row.is_some())
            .map_err(|_| OrchardStorageError::TreeCouldNotBeRead)
    }

    fn replace_row_order(
        &mut self,
        orchard_id: OrchardId,
        row_name: &str,
        ordered_tree_ids: &[TreeId],
    ) -> Result<(), OrchardStorageError> {
        let orchard_id = i64::try_from(orchard_id.0)
            .map_err(|_| OrchardStorageError::RowOrderCouldNotBeSaved)?;
        self.client
            .execute(
                "UPDATE trees SET row_rank = NULL WHERE orchard_id = $1 AND row_name = $2",
                &[&orchard_id, &row_name],
            )
            .map_err(|_| OrchardStorageError::RowOrderCouldNotBeSaved)?;
        for (index, tree_id) in ordered_tree_ids.iter().enumerate() {
            let tree_id = i64::try_from(tree_id.0)
                .map_err(|_| OrchardStorageError::RowOrderCouldNotBeSaved)?;
            let rank = i32::try_from(index + 1)
                .map_err(|_| OrchardStorageError::RowOrderCouldNotBeSaved)?;
            let changed = self
                .client
                .execute(
                    "UPDATE trees
                     SET row_rank = $4
                     WHERE id = $1 AND orchard_id = $2 AND row_name = $3",
                    &[&tree_id, &orchard_id, &row_name, &rank],
                )
                .map_err(|_| OrchardStorageError::RowOrderCouldNotBeSaved)?;
            if changed != 1 {
                return Err(OrchardStorageError::RowOrderCouldNotBeSaved);
            }
        }
        Ok(())
    }

    fn active_watering_run(
        &mut self,
        orchard_id: OrchardId,
    ) -> Result<Option<WateringRun>, OrchardStorageError> {
        let orchard_id = i64::try_from(orchard_id.0)
            .map_err(|_| OrchardStorageError::WateringRunCouldNotBeRead)?;
        let run = self
            .client
            .query_opt(
                "SELECT id, orchard_id, target_kind, row_name, completed_at IS NOT NULL,
                        ST_X(water_source), ST_Y(water_source)
                 FROM watering_runs
                 WHERE orchard_id = $1 AND completed_at IS NULL",
                &[&orchard_id],
            )
            .map_err(|_| OrchardStorageError::WateringRunCouldNotBeRead)?;
        run.map(|row| watering_run_from_row(&mut self.client, &row))
            .transpose()
    }

    fn watering_run(
        &mut self,
        watering_run_id: WateringRunId,
    ) -> Result<Option<WateringRun>, OrchardStorageError> {
        let watering_run_id = i64::try_from(watering_run_id.0)
            .map_err(|_| OrchardStorageError::WateringRunCouldNotBeRead)?;
        let run = self
            .client
            .query_opt(
                "SELECT id, orchard_id, target_kind, row_name, completed_at IS NOT NULL,
                        ST_X(water_source), ST_Y(water_source)
                 FROM watering_runs
                 WHERE id = $1",
                &[&watering_run_id],
            )
            .map_err(|_| OrchardStorageError::WateringRunCouldNotBeRead)?;
        run.map(|row| watering_run_from_row(&mut self.client, &row))
            .transpose()
    }

    fn create_watering_run(
        &mut self,
        orchard_id: OrchardId,
        target: &WateringRunTarget,
        water_source: Option<GeoPoint>,
        ordered_tree_ids: &[TreeId],
    ) -> Result<WateringRunId, OrchardStorageError> {
        let orchard_id = i64::try_from(orchard_id.0)
            .map_err(|_| OrchardStorageError::WateringRunCouldNotBeCreated)?;
        let (target_kind, row_name) = match target {
            WateringRunTarget::Row(row_name) => ("row", Some(row_name.as_str())),
            WateringRunTarget::DangerTrees => ("danger", None),
        };
        let water_source_longitude = water_source.map(|source| source.longitude);
        let water_source_latitude = water_source.map(|source| source.latitude);
        let run_id = self
            .client
            .query_one(
                "INSERT INTO watering_runs (orchard_id, target_kind, row_name, water_source)
                 VALUES (
                    $1, $2, $3,
                    CASE WHEN $4::DOUBLE PRECISION IS NULL THEN NULL
                         ELSE ST_SetSRID(ST_MakePoint($4, $5), 4326)
                    END
                 )
                 RETURNING id",
                &[
                    &orchard_id,
                    &target_kind,
                    &row_name,
                    &water_source_longitude,
                    &water_source_latitude,
                ],
            )
            .map_err(|_| OrchardStorageError::WateringRunCouldNotBeCreated)?
            .get::<_, i64>(0);
        for (index, tree_id) in ordered_tree_ids.iter().enumerate() {
            let tree_id = i64::try_from(tree_id.0)
                .map_err(|_| OrchardStorageError::WateringRunCouldNotBeCreated)?;
            let row_rank = i32::try_from(index + 1)
                .map_err(|_| OrchardStorageError::WateringRunCouldNotBeCreated)?;
            self.client
                .execute(
                    "INSERT INTO watering_run_trees (watering_run_id, tree_id, row_rank)
                     VALUES ($1, $2, $3)",
                    &[&run_id, &tree_id, &row_rank],
                )
                .map_err(|_| OrchardStorageError::WateringRunCouldNotBeCreated)?;
        }
        u64::try_from(run_id)
            .map(WateringRunId)
            .map_err(|_| OrchardStorageError::WateringRunCouldNotBeCreated)
    }

    fn mark_watering_tree_watered(
        &mut self,
        watering_run_id: WateringRunId,
        tree_id: TreeId,
    ) -> Result<(), OrchardStorageError> {
        let watering_run_id = i64::try_from(watering_run_id.0)
            .map_err(|_| OrchardStorageError::WateringRunCouldNotBeChanged)?;
        let tree_id = i64::try_from(tree_id.0)
            .map_err(|_| OrchardStorageError::WateringRunCouldNotBeChanged)?;
        match self.client.execute(
            "UPDATE watering_run_trees
             SET watered_at = now()
             WHERE watering_run_id = $1 AND tree_id = $2 AND watered_at IS NULL",
            &[&watering_run_id, &tree_id],
        ) {
            Ok(1) => Ok(()),
            _ => Err(OrchardStorageError::WateringRunCouldNotBeChanged),
        }
    }

    fn complete_watering_run(
        &mut self,
        watering_run_id: WateringRunId,
    ) -> Result<(), OrchardStorageError> {
        let watering_run_id = i64::try_from(watering_run_id.0)
            .map_err(|_| OrchardStorageError::WateringRunCouldNotBeChanged)?;
        match self.client.execute(
            "UPDATE watering_runs
             SET completed_at = now()
             WHERE id = $1 AND completed_at IS NULL",
            &[&watering_run_id],
        ) {
            Ok(1) => Ok(()),
            _ => Err(OrchardStorageError::WateringRunCouldNotBeChanged),
        }
    }
}

impl MapConfigurationStorage for PostgresOrchardStorage {
    fn map_configuration(
        &mut self,
    ) -> Result<Option<MapConfiguration>, MapConfigurationStorageError> {
        let default_user = self
            .client
            .query_opt(
                "SELECT id, ST_X(default_center), ST_Y(default_center)
                 FROM users
                 WHERE is_default = TRUE
                 ORDER BY id
                 LIMIT 1",
                &[],
            )
            .map_err(|_| MapConfigurationStorageError::ConfigurationCouldNotBeRead)?;
        let Some(default_user) = default_user else {
            return Ok(None);
        };
        let user_id = default_user.get::<_, i64>(0);
        let aerial_overlays = self
            .client
            .query(
                "SELECT
                    id, name,
                    ST_X(top_left), ST_Y(top_left),
                    ST_X(top_right), ST_Y(top_right),
                    ST_X(bottom_right), ST_Y(bottom_right),
                    ST_X(bottom_left), ST_Y(bottom_left)
                 FROM aerial_overlays
                 WHERE user_id = $1
                 ORDER BY sort_order, id",
                &[&user_id],
            )
            .map_err(|_| MapConfigurationStorageError::ConfigurationCouldNotBeRead)?
            .into_iter()
            .map(|row| {
                let id = u64::try_from(row.get::<_, i64>(0))
                    .map_err(|_| MapConfigurationStorageError::ConfigurationCouldNotBeRead)?;
                Ok(AerialOverlay {
                    id: AerialOverlayId(id),
                    name: row.get(1),
                    corners: [
                        GeoPoint {
                            longitude: row.get(2),
                            latitude: row.get(3),
                        },
                        GeoPoint {
                            longitude: row.get(4),
                            latitude: row.get(5),
                        },
                        GeoPoint {
                            longitude: row.get(6),
                            latitude: row.get(7),
                        },
                        GeoPoint {
                            longitude: row.get(8),
                            latitude: row.get(9),
                        },
                    ],
                })
            })
            .collect::<Result<Vec<_>, MapConfigurationStorageError>>()?;

        Ok(Some(MapConfiguration {
            default_center: GeoPoint {
                longitude: default_user.get(1),
                latitude: default_user.get(2),
            },
            aerial_overlays,
        }))
    }

    fn aerial_overlay_image(
        &mut self,
        overlay_id: AerialOverlayId,
    ) -> Result<Option<AerialOverlayImage>, MapConfigurationStorageError> {
        let overlay_id = i64::try_from(overlay_id.0)
            .map_err(|_| MapConfigurationStorageError::AerialOverlayImageCouldNotBeRead)?;
        self.client
            .query_opt(
                "SELECT media_type, image_bytes FROM aerial_overlays WHERE id = $1",
                &[&overlay_id],
            )
            .map(|row| {
                row.map(|row| AerialOverlayImage {
                    media_type: row.get(0),
                    bytes: row.get(1),
                })
            })
            .map_err(|_| MapConfigurationStorageError::AerialOverlayImageCouldNotBeRead)
    }

    fn map_configuration_for_orchard(
        &mut self,
        orchard_id: OrchardId,
    ) -> Result<Option<MapConfiguration>, MapConfigurationStorageError> {
        let orchard_id = i64::try_from(orchard_id.0)
            .map_err(|_| MapConfigurationStorageError::ConfigurationCouldNotBeRead)?;
        let orchard = self
            .client
            .query_opt(
                "SELECT ST_X(center), ST_Y(center) FROM orchards WHERE id = $1",
                &[&orchard_id],
            )
            .map_err(|_| MapConfigurationStorageError::ConfigurationCouldNotBeRead)?;
        let Some(orchard) = orchard else {
            return Ok(None);
        };
        let aerial_overlays =
            self.client
                .query(
                    "SELECT
                    id, name,
                    ST_X(top_left), ST_Y(top_left),
                    ST_X(top_right), ST_Y(top_right),
                    ST_X(bottom_right), ST_Y(bottom_right),
                    ST_X(bottom_left), ST_Y(bottom_left)
                 FROM aerial_overlays
                 WHERE orchard_id = $1
                 ORDER BY sort_order, id",
                    &[&orchard_id],
                )
                .map_err(|_| MapConfigurationStorageError::ConfigurationCouldNotBeRead)?
                .into_iter()
                .map(|row| {
                    Ok(AerialOverlay {
                        id: AerialOverlayId(u64::try_from(row.get::<_, i64>(0)).map_err(|_| {
                            MapConfigurationStorageError::ConfigurationCouldNotBeRead
                        })?),
                        name: row.get(1),
                        corners: [
                            GeoPoint {
                                longitude: row.get(2),
                                latitude: row.get(3),
                            },
                            GeoPoint {
                                longitude: row.get(4),
                                latitude: row.get(5),
                            },
                            GeoPoint {
                                longitude: row.get(6),
                                latitude: row.get(7),
                            },
                            GeoPoint {
                                longitude: row.get(8),
                                latitude: row.get(9),
                            },
                        ],
                    })
                })
                .collect::<Result<Vec<_>, MapConfigurationStorageError>>()?;
        Ok(Some(MapConfiguration {
            default_center: GeoPoint {
                longitude: orchard.get(0),
                latitude: orchard.get(1),
            },
            aerial_overlays,
        }))
    }

    fn aerial_overlay_image_for_orchard(
        &mut self,
        orchard_id: OrchardId,
        overlay_id: AerialOverlayId,
    ) -> Result<Option<AerialOverlayImage>, MapConfigurationStorageError> {
        let orchard_id = i64::try_from(orchard_id.0)
            .map_err(|_| MapConfigurationStorageError::AerialOverlayImageCouldNotBeRead)?;
        let overlay_id = i64::try_from(overlay_id.0)
            .map_err(|_| MapConfigurationStorageError::AerialOverlayImageCouldNotBeRead)?;
        self.client
            .query_opt(
                "SELECT media_type, image_bytes
                 FROM aerial_overlays
                 WHERE id = $1 AND orchard_id = $2",
                &[&overlay_id, &orchard_id],
            )
            .map(|row| {
                row.map(|row| AerialOverlayImage {
                    media_type: row.get(0),
                    bytes: row.get(1),
                })
            })
            .map_err(|_| MapConfigurationStorageError::AerialOverlayImageCouldNotBeRead)
    }
}

fn orchard_from_row(row: &Row) -> Result<Orchard, OrchardStorageError> {
    Ok(Orchard {
        id: OrchardId(
            u64::try_from(row.get::<_, i64>(0))
                .map_err(|_| OrchardStorageError::TreesCouldNotBeRead)?,
        ),
        name: row.get(1),
        longitude: row.get(2),
        latitude: row.get(3),
        reference_region: row.get(4),
    })
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
    let identification_status = match row.get::<_, &str>(18) {
        "confirmed" => IdentificationStatus::Confirmed,
        "uncertain" => IdentificationStatus::Uncertain,
        _ => return Err(OrchardStorageError::TreesCouldNotBeRead),
    };
    let botanical_taxon = serde_json::from_str(&row.get::<_, String>(20))
        .map_err(|_| OrchardStorageError::TreesCouldNotBeRead)?;
    let harvest_windows = harvest_windows_from_row(row)?;

    Ok(OrchardTree {
        id: TreeId(
            u64::try_from(row.get::<_, i64>(23))
                .map_err(|_| OrchardStorageError::TreesCouldNotBeRead)?,
        ),
        row_rank: row
            .get::<_, Option<i32>>(25)
            .map(|rank| u32::try_from(rank).map_err(|_| OrchardStorageError::TreesCouldNotBeRead))
            .transpose()?,
        tree: Tree {
            legacy_source,
            plant_identity_id: PlantIdentityId(
                u64::try_from(row.get::<_, i64>(1))
                    .map_err(|_| OrchardStorageError::TreesCouldNotBeRead)?,
            ),
            cultivar_id: row
                .get::<_, Option<i64>>(17)
                .map(|id| {
                    u64::try_from(id)
                        .map(PlantCultivarId)
                        .map_err(|_| OrchardStorageError::TreesCouldNotBeRead)
                })
                .transpose()?,
            identification_status,
            longitude: row.get(2),
            latitude: row.get(3),
            planted_on: row.get(9),
            row_name: row.get(10),
            roles: row.get(11),
            is_alive: row.get(12),
            is_in_danger: row.get(16),
            reproductive_role,
            adult_height_meters: row.get(14),
            adult_width_meters: row.get(15),
        },
        plant_identity: PlantIdentity {
            common_name: row.get(19),
            botanical_taxon,
        },
        plant_cultivar: row
            .get::<_, Option<String>>(21)
            .map(|cultivar| PlantCultivar {
                cultivar,
                trade_name: row.get(22),
            }),
        harvest_windows,
    })
}

fn watering_run_from_row(
    client: &mut Client,
    row: &Row,
) -> Result<WateringRun, OrchardStorageError> {
    let stored_run_id = row.get::<_, i64>(0);
    let entries = client
        .query(
            "SELECT tree_id, watered_at IS NOT NULL
             FROM watering_run_trees
             WHERE watering_run_id = $1
             ORDER BY row_rank",
            &[&stored_run_id],
        )
        .map_err(|_| OrchardStorageError::WateringRunCouldNotBeRead)?;
    let ordered_tree_ids = entries
        .iter()
        .map(|entry| {
            u64::try_from(entry.get::<_, i64>(0))
                .map(TreeId)
                .map_err(|_| OrchardStorageError::WateringRunCouldNotBeRead)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let watered_tree_ids = entries
        .iter()
        .filter(|entry| entry.get::<_, bool>(1))
        .map(|entry| {
            u64::try_from(entry.get::<_, i64>(0))
                .map(TreeId)
                .map_err(|_| OrchardStorageError::WateringRunCouldNotBeRead)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(WateringRun {
        id: WateringRunId(
            u64::try_from(stored_run_id)
                .map_err(|_| OrchardStorageError::WateringRunCouldNotBeRead)?,
        ),
        orchard_id: OrchardId(
            u64::try_from(row.get::<_, i64>(1))
                .map_err(|_| OrchardStorageError::WateringRunCouldNotBeRead)?,
        ),
        target: match row.get::<_, &str>(2) {
            "row" => WateringRunTarget::Row(
                row.get::<_, Option<String>>(3)
                    .ok_or(OrchardStorageError::WateringRunCouldNotBeRead)?,
            ),
            "danger" => WateringRunTarget::DangerTrees,
            _ => return Err(OrchardStorageError::WateringRunCouldNotBeRead),
        },
        water_source: match (row.get::<_, Option<f64>>(5), row.get::<_, Option<f64>>(6)) {
            (Some(longitude), Some(latitude)) => Some(GeoPoint {
                longitude,
                latitude,
            }),
            (None, None) => None,
            _ => return Err(OrchardStorageError::WateringRunCouldNotBeRead),
        },
        ordered_tree_ids,
        watered_tree_ids,
        completed: row.get(4),
    })
}

fn harvest_windows_from_row(
    row: &postgres::Row,
) -> Result<Vec<AnnualHarvestWindow>, OrchardStorageError> {
    #[derive(serde::Deserialize)]
    struct StoredHarvestWindow {
        start_month: i16,
        start_day: i16,
        end_month: i16,
        end_day: i16,
        reference_region: Option<String>,
        harvested_part: HarvestedPart,
        data_origin: HarvestDataOrigin,
        source_url: Option<String>,
    }

    let values = serde_json::from_str::<Vec<StoredHarvestWindow>>(&row.get::<_, String>(24))
        .map_err(|_| OrchardStorageError::TreesCouldNotBeRead)?;
    values
        .into_iter()
        .map(|window| {
            Ok(AnnualHarvestWindow {
                start: annual_date(window.start_month, window.start_day)?,
                end: annual_date(window.end_month, window.end_day)?,
                reference_region: window.reference_region,
                harvested_part: window.harvested_part,
                data_origin: window.data_origin,
                source_url: window.source_url,
            })
        })
        .collect()
}

fn annual_date(month: i16, day: i16) -> Result<AnnualDate, OrchardStorageError> {
    let month = u8::try_from(month).map_err(|_| OrchardStorageError::TreesCouldNotBeRead)?;
    let day = u8::try_from(day).map_err(|_| OrchardStorageError::TreesCouldNotBeRead)?;
    AnnualDate::new(month, day).ok_or(OrchardStorageError::TreesCouldNotBeRead)
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

fn resolve_plant_identification(
    client: &mut Client,
    plant_identification: PlantIdentification,
) -> Result<PlantIdentityReference, OrchardStorageError> {
    let PlantIdentification {
        plant_identity,
        plant_cultivar,
        ..
    } = plant_identification;
    let botanical_taxon = serde_json::to_string(&plant_identity.botanical_taxon)
        .map_err(|_| OrchardStorageError::PlantIdentityCouldNotBeResolved)?;
    let returned_id = client
        .query_opt(
            "INSERT INTO plant_identities (
                common_name, botanical_taxon
            ) VALUES ($1, $2::TEXT::jsonb)
            ON CONFLICT (botanical_taxon) DO NOTHING
            RETURNING id",
            &[&plant_identity.common_name, &botanical_taxon],
        )
        .map_err(|_| OrchardStorageError::PlantIdentityCouldNotBeResolved)?
        .map(|row| row.get::<_, i64>(0));
    let id = match returned_id {
        Some(id) => id,
        None => client
            .query_one(
                "SELECT id FROM plant_identities WHERE botanical_taxon = $1::TEXT::jsonb",
                &[&botanical_taxon],
            )
            .map_err(|_| OrchardStorageError::PlantIdentityCouldNotBeResolved)?
            .get::<_, i64>(0),
    };
    let cultivar_id = match plant_cultivar {
        None => None,
        Some(PlantCultivar {
            cultivar,
            trade_name,
        }) => Some(
            client
                .query_one(
                    "INSERT INTO plant_cultivars (plant_identity_id, cultivar, trade_name)
                     VALUES ($1, $2, $3)
                     ON CONFLICT (plant_identity_id, cultivar) DO UPDATE
                     SET trade_name = COALESCE(plant_cultivars.trade_name, EXCLUDED.trade_name)
                     RETURNING id",
                    &[&id, &cultivar, &trade_name],
                )
                .map_err(|_| OrchardStorageError::PlantIdentityCouldNotBeResolved)?
                .get::<_, i64>(0),
        ),
    };
    let plant_identity_id = u64::try_from(id)
        .map(PlantIdentityId)
        .map_err(|_| OrchardStorageError::PlantIdentityCouldNotBeResolved)?;
    let cultivar_id = cultivar_id
        .map(|id| {
            u64::try_from(id)
                .map(PlantCultivarId)
                .map_err(|_| OrchardStorageError::PlantIdentityCouldNotBeResolved)
        })
        .transpose()?;
    Ok(PlantIdentityReference {
        plant_identity_id,
        cultivar_id,
    })
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
    let cultivar_id = tree.cultivar_id.map(|cultivar_id| cultivar_id.0 as i64);
    let identification_status = match tree.identification_status {
        IdentificationStatus::Confirmed => "confirmed",
        IdentificationStatus::Uncertain => "uncertain",
    };
    let reproductive_role = tree.reproductive_role.map(|role| match role {
        ReproductiveRole::Female => "female",
        ReproductiveRole::Male => "male",
        ReproductiveRole::SelfFertile => "self_fertile",
        ReproductiveRole::Parthenocarpic => "parthenocarpic",
    });
    client
        .execute(
            "INSERT INTO trees (
                legacy_feature_id, plant_identity_id, cultivar_id, identification_status, location,
                legacy_name, legacy_latin_name,
                legacy_source_url,
                legacy_identification_name, legacy_identification_latin_name,
                planted_on, row_name, roles, is_alive, is_in_danger, reproductive_role,
                adult_height_meters, adult_width_meters
            ) VALUES (
                $1, $2, $3, $4, ST_SetSRID(ST_MakePoint($5, $6), 4326),
                $7, $8, $9, $10, $11, $12::TEXT::DATE, $13,
                $14, $15, $16, $17, $18, $19
            )",
            &[
                &legacy_feature_id,
                &plant_identity_id,
                &cultivar_id,
                &identification_status,
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
                &tree.is_in_danger,
                &reproductive_role,
                &tree.adult_height_meters,
                &tree.adult_width_meters,
            ],
        )
        .map(|_| ())
}
