use crate::hexagon::models::{PlantIdentity, Tree};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

pub struct TreeCreationRequested {
    pub longitude: f64,
    pub latitude: f64,
    pub plant_identity: PlantIdentity,
    pub roles: Vec<String>,
    pub harvest_start_day: Option<u16>,
    pub harvest_end_day: Option<u16>,
}

#[derive(Debug, PartialEq)]
pub enum TreeCreationError {
    PlantIdentityCouldNotBeResolved,
    TreeCouldNotBeSaved,
    TransactionCouldNotBegin,
    TransactionCouldNotCommit,
}

pub fn create_tree<U>(
    event: TreeCreationRequested,
    orchard_storage: &mut U,
) -> Result<Tree, TreeCreationError>
where
    U: OrchardStorage,
{
    orchard_storage.transaction(|orchard| {
        let plant_identity_id = orchard
            .find_or_create_plant_identity(event.plant_identity)
            .map_err(|_| TreeCreationError::PlantIdentityCouldNotBeResolved)?;
        let tree = Tree {
            legacy_source: None,
            plant_identity_id,
            longitude: event.longitude,
            latitude: event.latitude,
            planted_on: None,
            row_name: None,
            roles: event.roles,
            is_alive: true,
            reproductive_role: None,
            harvest_start_day: event.harvest_start_day,
            harvest_end_day: event.harvest_end_day,
            adult_height_meters: None,
            adult_width_meters: None,
        };
        orchard
            .save_tree(tree.clone())
            .map_err(|_| TreeCreationError::TreeCouldNotBeSaved)?;
        Ok(tree)
    })
}

impl From<OrchardStorageError> for TreeCreationError {
    fn from(error: OrchardStorageError) -> Self {
        match error {
            OrchardStorageError::AtomicOperationCouldNotBegin => Self::TransactionCouldNotBegin,
            OrchardStorageError::AtomicOperationCouldNotCommit => Self::TransactionCouldNotCommit,
            OrchardStorageError::PlantIdentityCouldNotBeResolved => {
                Self::PlantIdentityCouldNotBeResolved
            }
            OrchardStorageError::TreeCouldNotBeSaved => Self::TreeCouldNotBeSaved,
            _ => Self::TreeCouldNotBeSaved,
        }
    }
}
