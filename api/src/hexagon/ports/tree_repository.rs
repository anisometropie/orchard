use crate::hexagon::models::Tree;

#[derive(Debug, PartialEq)]
pub enum TreeRepositoryError {
    TreeCouldNotBeSaved,
}

pub trait TreeRepository {
    fn save(&mut self, tree: Tree) -> Result<(), TreeRepositoryError>;
}
