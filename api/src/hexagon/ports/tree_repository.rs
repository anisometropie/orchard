use crate::hexagon::models::Tree;

#[derive(Debug, PartialEq)]
pub enum TreeRepositoryError {
    CouldNotCheckExistingLegacyFeature,
    TreeCouldNotBeSaved,
}

pub trait TreeRepository {
    fn has_legacy_feature_id(
        &mut self,
        legacy_feature_id: u32,
    ) -> Result<bool, TreeRepositoryError>;
    fn save(&mut self, tree: Tree) -> Result<(), TreeRepositoryError>;
}
