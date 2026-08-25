use crate::hexagon::models::Tree;

pub trait TreeRepository {
    fn save(&mut self, tree: Tree);
}
