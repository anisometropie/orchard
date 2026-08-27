use crate::hexagon::models::{PlantIdentity, Tree};
use crate::hexagon::ports::{OrchardTransaction, OrchardUnitOfWork};

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
    orchard_unit_of_work: &mut U,
) -> Result<Tree, TreeCreationError>
where
    U: OrchardUnitOfWork,
{
    let mut transaction = orchard_unit_of_work
        .begin()
        .map_err(|_| TreeCreationError::TransactionCouldNotBegin)?;
    let plant_identity_id = match transaction.find_or_create_plant_identity(event.plant_identity) {
        Ok(plant_identity_id) => plant_identity_id,
        Err(_) => {
            transaction.rollback();
            return Err(TreeCreationError::PlantIdentityCouldNotBeResolved);
        }
    };
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
    if transaction.save_tree(tree.clone()).is_err() {
        transaction.rollback();
        return Err(TreeCreationError::TreeCouldNotBeSaved);
    }
    transaction
        .commit()
        .map_err(|_| TreeCreationError::TransactionCouldNotCommit)?;
    Ok(tree)
}
