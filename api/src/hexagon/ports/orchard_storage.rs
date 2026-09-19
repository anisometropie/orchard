use crate::hexagon::models::{
    AnnualHarvestWindow, GeoPoint, HarvestDate, HarvestRun, HarvestRunId, HarvestRunTarget,
    HarvestRunTree, HarvestScheduleOwner, HarvestTreeActionUndo, HarvestTreeOutcome,
    HarvestTreeOutcomeRecord, HarvestWindowExtension, OrchardId, OrchardTree, PlantIdentification,
    PlantIdentityReference, Tree, TreeId, WateringRun, WateringRunId, WateringRunTarget,
};

#[derive(Debug, PartialEq)]
pub enum OrchardStorageError {
    AtomicOperationCouldNotBegin,
    ExistingLegacyTreeCouldNotBeChecked,
    PlantIdentityCouldNotBeResolved,
    HarvestWindowsCouldNotBeReplaced,
    TreeCouldNotBeSaved,
    TreeCouldNotBeRead,
    TreeDangerCouldNotBeChanged,
    TreeLifeStatusCouldNotBeChanged,
    RowOrderCouldNotBeSaved,
    WateringRunCouldNotBeRead,
    WateringRunCouldNotBeCreated,
    WateringRunCouldNotBeChanged,
    WateringRunCouldNotBeDeleted,
    HarvestRunCouldNotBeRead,
    HarvestRunCouldNotBeCreated,
    HarvestRunCouldNotBeChanged,
    HarvestRunCouldNotBeDeleted,
    HarvestHistoryCouldNotBeRead,
    HarvestWindowCouldNotBeExtended,
    AtomicOperationCouldNotCommit,
    TreesCouldNotBeRead,
}

pub trait OrchardStorage {
    fn transaction<T, E>(
        &mut self,
        operation: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E>
    where
        E: From<OrchardStorageError>;

    fn is_legacy_tree_already_imported(
        &mut self,
        legacy_feature_id: u32,
    ) -> Result<bool, OrchardStorageError>;
    fn resolve_plant_identification(
        &mut self,
        plant_identification: PlantIdentification,
    ) -> Result<PlantIdentityReference, OrchardStorageError>;
    fn replace_harvest_windows(
        &mut self,
        owner: HarvestScheduleOwner,
        harvest_windows: Vec<AnnualHarvestWindow>,
    ) -> Result<bool, OrchardStorageError>;
    fn replace_orchard_harvest_windows(
        &mut self,
        orchard_id: OrchardId,
        owner: HarvestScheduleOwner,
        harvest_windows: Vec<AnnualHarvestWindow>,
    ) -> Result<bool, OrchardStorageError>;
    fn save_tree(&mut self, tree: Tree) -> Result<(), OrchardStorageError>;
    fn tree_is_alive(&mut self, tree_id: TreeId) -> Result<Option<bool>, OrchardStorageError>;
    fn change_tree_danger(
        &mut self,
        tree_id: TreeId,
        is_in_danger: bool,
    ) -> Result<(), OrchardStorageError>;
    fn change_tree_life_status(
        &mut self,
        tree_id: TreeId,
        is_alive: bool,
    ) -> Result<(), OrchardStorageError>;
    fn trees(&mut self) -> Result<Vec<OrchardTree>, OrchardStorageError>;
    fn trees_in_orchard(
        &mut self,
        orchard_id: OrchardId,
    ) -> Result<Vec<OrchardTree>, OrchardStorageError>;
    fn tree_belongs_to_orchard(
        &mut self,
        tree_id: TreeId,
        orchard_id: OrchardId,
    ) -> Result<bool, OrchardStorageError>;
    fn replace_row_order(
        &mut self,
        orchard_id: OrchardId,
        row_name: &str,
        ordered_tree_ids: &[TreeId],
    ) -> Result<(), OrchardStorageError>;
    fn active_watering_run(
        &mut self,
        orchard_id: OrchardId,
    ) -> Result<Option<WateringRun>, OrchardStorageError>;
    fn watering_run(
        &mut self,
        watering_run_id: WateringRunId,
    ) -> Result<Option<WateringRun>, OrchardStorageError>;
    fn create_watering_run(
        &mut self,
        orchard_id: OrchardId,
        target: &WateringRunTarget,
        water_source: Option<GeoPoint>,
        carry_capacity: Option<u32>,
        ordered_tree_ids: &[TreeId],
    ) -> Result<WateringRunId, OrchardStorageError>;
    fn mark_watering_tree_watered(
        &mut self,
        watering_run_id: WateringRunId,
        tree_id: TreeId,
    ) -> Result<(), OrchardStorageError>;
    fn complete_watering_run(
        &mut self,
        watering_run_id: WateringRunId,
    ) -> Result<(), OrchardStorageError>;
    fn delete_watering_run(
        &mut self,
        watering_run_id: WateringRunId,
    ) -> Result<(), OrchardStorageError>;
    fn harvest_tree_outcomes(
        &mut self,
        orchard_id: OrchardId,
    ) -> Result<Vec<HarvestTreeOutcomeRecord>, OrchardStorageError>;
    fn active_harvest_run(
        &mut self,
        orchard_id: OrchardId,
    ) -> Result<Option<HarvestRun>, OrchardStorageError>;
    fn harvest_run(
        &mut self,
        harvest_run_id: HarvestRunId,
    ) -> Result<Option<HarvestRun>, OrchardStorageError>;
    fn create_harvest_run(
        &mut self,
        orchard_id: OrchardId,
        target: HarvestRunTarget,
        harvested_parts: &[crate::hexagon::models::HarvestedPart],
        started_on: HarvestDate,
        ordered_trees: &[HarvestRunTree],
    ) -> Result<HarvestRunId, OrchardStorageError>;
    fn record_harvest_tree_outcome(
        &mut self,
        harvest_run_id: HarvestRunId,
        tree_id: TreeId,
        outcome: HarvestTreeOutcome,
    ) -> Result<(), OrchardStorageError>;
    fn extend_harvest_run_tree_period(
        &mut self,
        harvest_run_id: HarvestRunId,
        tree_id: TreeId,
        new_end: HarvestDate,
        action_date: HarvestDate,
    ) -> Result<(), OrchardStorageError>;
    fn extend_orchard_harvest_window(
        &mut self,
        orchard_id: OrchardId,
        extension: &HarvestWindowExtension,
    ) -> Result<bool, OrchardStorageError>;
    fn restore_orchard_harvest_window(
        &mut self,
        orchard_id: OrchardId,
        extension: &HarvestWindowExtension,
    ) -> Result<bool, OrchardStorageError>;
    fn save_harvest_tree_action_undo(
        &mut self,
        harvest_run_id: HarvestRunId,
        undo: &HarvestTreeActionUndo,
    ) -> Result<(), OrchardStorageError>;
    fn harvest_tree_action_undo(
        &mut self,
        harvest_run_id: HarvestRunId,
    ) -> Result<Option<HarvestTreeActionUndo>, OrchardStorageError>;
    fn restore_harvest_tree_action(
        &mut self,
        harvest_run_id: HarvestRunId,
        undo: &HarvestTreeActionUndo,
    ) -> Result<(), OrchardStorageError>;
    fn complete_harvest_run(
        &mut self,
        harvest_run_id: HarvestRunId,
    ) -> Result<(), OrchardStorageError>;
    fn delete_harvest_run(
        &mut self,
        harvest_run_id: HarvestRunId,
    ) -> Result<(), OrchardStorageError>;
}
