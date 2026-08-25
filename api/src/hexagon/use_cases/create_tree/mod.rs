use crate::hexagon::models::Tree;
use crate::hexagon::ports::TreeRepository;

pub struct TreeCreationRequested {
    pub longitude: f64,
    pub latitude: f64,
    pub name: String,
    pub latin_name: Option<String>,
    pub roles: Vec<String>,
    pub harvest_start_day: Option<u16>,
    pub harvest_end_day: Option<u16>,
}

pub fn create_tree<R: TreeRepository>(event: TreeCreationRequested, trees: &mut R) -> Tree {
    let tree = Tree {
        longitude: event.longitude,
        latitude: event.latitude,
        name: event.name,
        latin_name: event.latin_name,
        roles: event.roles,
        harvest_start_day: event.harvest_start_day,
        harvest_end_day: event.harvest_end_day,
    };
    trees.save(tree.clone());
    tree
}
