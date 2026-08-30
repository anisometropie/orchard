use crate::hexagon::models::{PlantIdentification, Tree};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

pub struct TreeCreationRequested {
    pub longitude: f64,
    pub latitude: f64,
    pub plant_identification: PlantIdentification,
    pub roles: Vec<String>,
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
        let identification_status = event.plant_identification.identification_status;
        let plant_identity = orchard
            .resolve_plant_identification(event.plant_identification)
            .map_err(|_| TreeCreationError::PlantIdentityCouldNotBeResolved)?;
        let tree = Tree {
            legacy_source: None,
            plant_identity_id: plant_identity.plant_identity_id,
            cultivar_id: plant_identity.cultivar_id,
            identification_status,
            longitude: event.longitude,
            latitude: event.latitude,
            planted_on: None,
            row_name: None,
            roles: event.roles,
            is_alive: true,
            is_in_danger: false,
            reproductive_role: None,
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
