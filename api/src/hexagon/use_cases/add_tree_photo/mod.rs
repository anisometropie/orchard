use crate::hexagon::models::{OrchardId, TreeId, TreePhoto};
use crate::hexagon::ports::TreePhotoStorage;

pub const MAX_FULL_PHOTO_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_THUMBNAIL_BYTES: usize = 512 * 1024;

pub struct TreePhotoAdded {
    pub orchard_id: OrchardId,
    pub tree_id: TreeId,
    pub full_webp: Vec<u8>,
    pub thumbnail_webp: Vec<u8>,
}

#[derive(Debug, PartialEq)]
pub enum TreePhotoAddError {
    InvalidWebp,
    PhotoTooLarge,
    TreeNotFound,
    PhotoCouldNotBeSaved,
}

pub fn add_tree_photo(
    event: TreePhotoAdded,
    storage: &mut impl TreePhotoStorage,
) -> Result<(), TreePhotoAddError> {
    if event.full_webp.len() > MAX_FULL_PHOTO_BYTES
        || event.thumbnail_webp.len() > MAX_THUMBNAIL_BYTES
    {
        return Err(TreePhotoAddError::PhotoTooLarge);
    }
    if !is_webp(&event.full_webp) || !is_webp(&event.thumbnail_webp) {
        return Err(TreePhotoAddError::InvalidWebp);
    }

    storage
        .save_tree_photo(
            event.orchard_id,
            event.tree_id,
            TreePhoto {
                full_webp: event.full_webp,
                thumbnail_webp: event.thumbnail_webp,
            },
        )
        .map_err(|_| TreePhotoAddError::PhotoCouldNotBeSaved)?
        .then_some(())
        .ok_or(TreePhotoAddError::TreeNotFound)
}

fn is_webp(bytes: &[u8]) -> bool {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return false;
    }
    let declared_size = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    declared_size.checked_add(8) == Some(bytes.len())
}
