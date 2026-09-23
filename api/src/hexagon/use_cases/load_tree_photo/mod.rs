use crate::hexagon::models::{OrchardId, TreeId, TreePhotoId, TreePhotoVariant};
use crate::hexagon::ports::TreePhotoStorage;

pub struct TreePhotoRequested {
    pub orchard_id: OrchardId,
    pub tree_id: TreeId,
    pub photo_id: TreePhotoId,
    pub variant: TreePhotoVariant,
}

#[derive(Debug, PartialEq)]
pub enum TreePhotoLoadError {
    PhotoNotFound,
    PhotoCouldNotBeRead,
}

pub fn load_tree_photo(
    request: TreePhotoRequested,
    storage: &mut impl TreePhotoStorage,
) -> Result<Vec<u8>, TreePhotoLoadError> {
    storage
        .tree_photo(
            request.orchard_id,
            request.tree_id,
            request.photo_id,
            request.variant,
        )
        .map_err(|_| TreePhotoLoadError::PhotoCouldNotBeRead)?
        .ok_or(TreePhotoLoadError::PhotoNotFound)
}
