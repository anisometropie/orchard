use crate::hexagon::models::{
    OrchardId, TreeId, TreePhoto, TreePhotoId, TreePhotoSummary, TreePhotoVariant,
};

#[derive(Debug, PartialEq)]
pub enum TreePhotoStorageError {
    PhotoCouldNotBeSaved,
    PhotoCouldNotBeRead,
    PhotosCouldNotBeListed,
    PhotoCouldNotBeDeleted,
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

    /// Returns `None` when the tree does not belong to the orchard, and newest photos first.
    fn tree_photos(
        &mut self,
        orchard_id: OrchardId,
        tree_id: TreeId,
    ) -> Result<Option<Vec<TreePhotoSummary>>, TreePhotoStorageError>;

    fn tree_photo(
        &mut self,
        orchard_id: OrchardId,
        tree_id: TreeId,
        photo_id: TreePhotoId,
        variant: TreePhotoVariant,
    ) -> Result<Option<Vec<u8>>, TreePhotoStorageError>;

    fn delete_tree_photo(
        &mut self,
        orchard_id: OrchardId,
        tree_id: TreeId,
        photo_id: TreePhotoId,
    ) -> Result<bool, TreePhotoStorageError>;
}
