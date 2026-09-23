use crate::hexagon::models::{OrchardId, TreeId, TreePhotoSummary};
use crate::hexagon::ports::TreePhotoStorage;

pub struct TreePhotosRequested {
    pub orchard_id: OrchardId,
    pub tree_id: TreeId,
}

#[derive(Debug, PartialEq)]
pub enum TreePhotosListError {
    TreeNotFound,
    PhotosCouldNotBeListed,
}

pub fn list_tree_photos(
    request: TreePhotosRequested,
    storage: &mut impl TreePhotoStorage,
) -> Result<Vec<TreePhotoSummary>, TreePhotosListError> {
    storage
        .tree_photos(request.orchard_id, request.tree_id)
        .map_err(|_| TreePhotosListError::PhotosCouldNotBeListed)?
        .ok_or(TreePhotosListError::TreeNotFound)
}
