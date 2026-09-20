use crate::hexagon::models::{GeoPoint, OrchardId, TreeId};
use crate::hexagon::ports::OrchardStorage;

pub struct OrchardTreeMoveConfirmed {
    pub orchard_id: OrchardId,
    pub tree_id: TreeId,
    pub position: GeoPoint,
}

#[derive(Debug, PartialEq)]
pub enum OrchardTreeMoveError {
    InvalidPosition,
    TreeNotFound,
    TreeCouldNotBeMoved,
}

pub fn move_orchard_tree(
    event: OrchardTreeMoveConfirmed,
    orchard_storage: &mut impl OrchardStorage,
) -> Result<(), OrchardTreeMoveError> {
    if !event.position.longitude.is_finite()
        || !event.position.latitude.is_finite()
        || !(-180.0..=180.0).contains(&event.position.longitude)
        || !(-90.0..=90.0).contains(&event.position.latitude)
    {
        return Err(OrchardTreeMoveError::InvalidPosition);
    }
    orchard_storage.transaction(|orchard| {
        let belongs_to_orchard = orchard
            .tree_belongs_to_orchard(event.tree_id, event.orchard_id)
            .map_err(|_| OrchardTreeMoveError::TreeCouldNotBeMoved)?;
        if !belongs_to_orchard {
            return Err(OrchardTreeMoveError::TreeNotFound);
        }
        orchard
            .change_tree_position(event.tree_id, event.position)
            .map_err(|_| OrchardTreeMoveError::TreeCouldNotBeMoved)
    })
}

impl From<crate::hexagon::ports::OrchardStorageError> for OrchardTreeMoveError {
    fn from(_: crate::hexagon::ports::OrchardStorageError) -> Self {
        Self::TreeCouldNotBeMoved
    }
}
