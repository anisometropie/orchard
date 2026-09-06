use crate::hexagon::models::{OrchardId, TreeId, TreePhoto, TreePhotoVariant};

#[derive(Debug, PartialEq)]
pub enum TreePhotoStorageError {
    PhotoCouldNotBeSaved,
    PhotoCouldNotBeRead,
}

pub trait TreePhotoStorage {
    fn save_tree_photo(
        &mut self,
        orchard_id: OrchardId,
        tree_id: TreeId,
        photo: TreePhoto,
    ) -> Result<bool, TreePhotoStorageError>;

    fn latest_tree_photo(
        &mut self,
        orchard_id: OrchardId,
        tree_id: TreeId,
        variant: TreePhotoVariant,
    ) -> Result<Option<Vec<u8>>, TreePhotoStorageError>;
}
