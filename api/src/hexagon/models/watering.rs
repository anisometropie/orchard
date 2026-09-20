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
    pub carry_capacity: Option<u32>,
    pub ordered_tree_ids: Vec<TreeId>,
    pub watered_tree_ids: Vec<TreeId>,
    pub completed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletedWateringRunTree {
    pub tree_id: TreeId,
    pub watered_at_unix_seconds: Option<i64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompletedWateringRun {
    pub id: WateringRunId,
    pub target: WateringRunTarget,
    pub carry_capacity: Option<u32>,
    pub started_at_unix_seconds: i64,
    pub completed_at_unix_seconds: i64,
    pub trees: Vec<CompletedWateringRunTree>,
}
