use crate::hexagon::models::{OrchardId, TreeId, TreePhotoId};
use crate::hexagon::ports::TreePhotoStorage;

pub struct TreePhotoDeletionRequested {
    pub orchard_id: OrchardId,
    pub tree_id: TreeId,
    pub photo_id: TreePhotoId,
}

#[derive(Debug, PartialEq)]
pub enum TreePhotoDeleteError {
    PhotoNotFound,
    PhotoCouldNotBeDeleted,
}

pub fn delete_tree_photo(
    request: TreePhotoDeletionRequested,
    storage: &mut impl TreePhotoStorage,
) -> Result<(), TreePhotoDeleteError> {
    storage
        .delete_tree_photo(request.orchard_id, request.tree_id, request.photo_id)
        .map_err(|_| TreePhotoDeleteError::PhotoCouldNotBeDeleted)?
        .then_some(())
        .ok_or(TreePhotoDeleteError::PhotoNotFound)
}
