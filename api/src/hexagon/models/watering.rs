use super::{GeoPoint, OrchardId, TreeId};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WateringRunId(pub u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WateringRunTarget {
    Row(String),
    DangerTrees,
}

impl WateringRunTarget {
    pub fn label(&self) -> &str {
        match self {
            Self::Row(row_name) => row_name,
            Self::DangerTrees => "Danger trees",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WateringRun {
    pub id: WateringRunId,
    pub orchard_id: OrchardId,
    pub target: WateringRunTarget,
    pub water_source: Option<GeoPoint>,
    pub ordered_tree_ids: Vec<TreeId>,
    pub watered_tree_ids: Vec<TreeId>,
    pub completed: bool,
}
