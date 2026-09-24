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
    pub skipped_tree_ids: Vec<TreeId>,
    pub completed: bool,
    pub paused: bool,
}

impl WateringRun {
    pub fn tree_is_pending(&self, tree_id: TreeId) -> bool {
        self.ordered_tree_ids.contains(&tree_id)
            && !self.watered_tree_ids.contains(&tree_id)
            && !self.skipped_tree_ids.contains(&tree_id)
    }

    pub fn next_tree_id(&self) -> Option<TreeId> {
        self.ordered_tree_ids
            .iter()
            .copied()
            .find(|id| self.tree_is_pending(*id))
    }

    pub fn is_finished(&self) -> bool {
        self.next_tree_id().is_none()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletedWateringRunTree {
    pub tree_id: TreeId,
    pub watered_at_unix_seconds: Option<i64>,
    pub skipped_at_unix_seconds: Option<i64>,
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
