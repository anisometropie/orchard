use super::{OrchardId, TreeId};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WateringRunId(pub u64);

#[derive(Clone, Debug, PartialEq)]
pub struct WateringRun {
    pub id: WateringRunId,
    pub orchard_id: OrchardId,
    pub row_name: String,
    pub ordered_tree_ids: Vec<TreeId>,
    pub watered_tree_ids: Vec<TreeId>,
    pub completed: bool,
}
