use crate::hexagon::models::{OrchardId, TreeId};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

pub struct TreeConditionChanged {
    pub tree_id: TreeId,
    pub is_alive: Option<bool>,
    pub is_in_danger: Option<bool>,
}

pub struct OrchardTreeConditionChanged {
    pub orchard_id: OrchardId,
    pub tree_id: TreeId,
    pub is_alive: Option<bool>,
    pub is_in_danger: Option<bool>,
}

#[derive(Debug, PartialEq)]
pub enum TreeConditionChangeError {
    NoChangesRequested,
    TreeNotFound,
    DeadTreeCannotBeInDanger,
    TreeCouldNotBeChanged,
}

pub fn change_tree_condition(
    event: TreeConditionChanged,
    orchard_storage: &mut impl OrchardStorage,
) -> Result<(), TreeConditionChangeError> {
    if event.is_alive.is_none() && event.is_in_danger.is_none() {
        return Err(TreeConditionChangeError::NoChangesRequested);
    }

    orchard_storage.transaction(|orchard| {
        let Some(currently_alive) = orchard
            .tree_is_alive(event.tree_id)
            .map_err(|_| TreeConditionChangeError::TreeCouldNotBeChanged)?
        else {
            return Err(TreeConditionChangeError::TreeNotFound);
        };
        let resulting_alive = event.is_alive.unwrap_or(currently_alive);

        if !resulting_alive && event.is_in_danger == Some(true) {
            return Err(TreeConditionChangeError::DeadTreeCannotBeInDanger);
        }

        if event.is_alive == Some(true) {
            orchard
                .change_tree_life_status(event.tree_id, true)
                .map_err(|_| TreeConditionChangeError::TreeCouldNotBeChanged)?;
        }

        if resulting_alive {
            if let Some(is_in_danger) = event.is_in_danger {
                orchard
                    .change_tree_danger(event.tree_id, is_in_danger)
                    .map_err(|_| TreeConditionChangeError::TreeCouldNotBeChanged)?;
            }
        } else {
            orchard
                .change_tree_danger(event.tree_id, false)
                .map_err(|_| TreeConditionChangeError::TreeCouldNotBeChanged)?;
        }

        if event.is_alive == Some(false) {
            orchard
                .change_tree_life_status(event.tree_id, false)
                .map_err(|_| TreeConditionChangeError::TreeCouldNotBeChanged)?;
        }

        Ok(())
    })
}

pub fn change_orchard_tree_condition(
    event: OrchardTreeConditionChanged,
    orchard_storage: &mut impl OrchardStorage,
) -> Result<(), TreeConditionChangeError> {
    let belongs_to_orchard = orchard_storage
        .tree_belongs_to_orchard(event.tree_id, event.orchard_id)
        .map_err(|_| TreeConditionChangeError::TreeCouldNotBeChanged)?;
    if !belongs_to_orchard {
        return Err(TreeConditionChangeError::TreeNotFound);
    }
    change_tree_condition(
        TreeConditionChanged {
            tree_id: event.tree_id,
            is_alive: event.is_alive,
            is_in_danger: event.is_in_danger,
        },
        orchard_storage,
    )
}

impl From<OrchardStorageError> for TreeConditionChangeError {
    fn from(_: OrchardStorageError) -> Self {
        Self::TreeCouldNotBeChanged
    }
}
