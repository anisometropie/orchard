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
        legacy_feature_id: None,
        longitude: event.longitude,
        latitude: event.latitude,
        name: event.name,
        latin_name: event.latin_name,
        planted_on: None,
        row_name: None,
        roles: event.roles,
        is_alive: true,
        harvest_start_day: event.harvest_start_day,
        harvest_end_day: event.harvest_end_day,
        adult_height_meters: None,
        adult_width_meters: None,
    };
    trees
        .save(tree.clone())
        .expect("tree repository could not save a newly created tree");
    tree
}
