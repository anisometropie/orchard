use crate::hexagon::models::{
    HarvestDate, OrchardId, PlantIdentityId, TreeId, eligible_harvest_trees,
};
use crate::hexagon::ports::OrchardStorage;

pub struct HarvestCandidatesRequested {
    pub orchard_id: OrchardId,
    pub action_date: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HarvestCandidate {
    pub tree_id: TreeId,
    pub plant_identity_id: PlantIdentityId,
}

#[derive(Debug, PartialEq)]
pub enum HarvestCandidatesError {
    InvalidActionDate,
    HarvestCandidatesCouldNotBeListed,
}

pub fn list_harvest_candidates(
    event: HarvestCandidatesRequested,
    storage: &mut impl OrchardStorage,
) -> Result<Vec<HarvestCandidate>, HarvestCandidatesError> {
    let action_date = HarvestDate::parse_iso(&event.action_date)
        .ok_or(HarvestCandidatesError::InvalidActionDate)?;
    let orchard_trees = storage
        .trees_in_orchard(event.orchard_id)
        .map_err(|_| HarvestCandidatesError::HarvestCandidatesCouldNotBeListed)?;
    let previous_outcomes = storage
        .harvest_tree_outcomes(event.orchard_id)
        .map_err(|_| HarvestCandidatesError::HarvestCandidatesCouldNotBeListed)?;
    let mut candidates = eligible_harvest_trees(&orchard_trees, &previous_outcomes, action_date)
        .into_iter()
        .map(|candidate| HarvestCandidate {
            tree_id: candidate.tree.id,
            plant_identity_id: candidate.tree.tree.plant_identity_id,
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|candidate| candidate.tree_id.0);
    Ok(candidates)
}
