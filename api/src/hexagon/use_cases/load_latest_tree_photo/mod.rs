use crate::hexagon::models::{OrchardId, TreeId, TreePhotoVariant};
use crate::hexagon::ports::TreePhotoStorage;

pub struct LatestTreePhotoRequested {
    pub orchard_id: OrchardId,
    pub tree_id: TreeId,
    pub variant: TreePhotoVariant,
}

#[derive(Debug, PartialEq)]
pub enum LatestTreePhotoLoadError {
    PhotoNotFound,
    PhotoCouldNotBeRead,
}

pub fn load_latest_tree_photo(
    request: LatestTreePhotoRequested,
    storage: &mut impl TreePhotoStorage,
) -> Result<Vec<u8>, LatestTreePhotoLoadError> {
    storage
        .latest_tree_photo(request.orchard_id, request.tree_id, request.variant)
        .map_err(|_| LatestTreePhotoLoadError::PhotoCouldNotBeRead)?
        .ok_or(LatestTreePhotoLoadError::PhotoNotFound)
}
