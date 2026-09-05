use std::collections::HashSet;

use crate::hexagon::models::{OrchardId, OrchardTree, TreeId};
use crate::hexagon::ports::{OrchardStorage, OrchardStorageError};

#[derive(Debug, PartialEq)]
pub enum RowOrder {
    Manual(Vec<TreeId>),
    EastToWest,
    WestToEast,
    NorthToSouth,
    SouthToNorth,
}

pub struct OrchardRowOrderRequested {
    pub orchard_id: OrchardId,
    pub row_name: String,
    pub order: RowOrder,
}

#[derive(Debug, PartialEq)]
pub enum OrchardRowOrderError {
    RowNotFound,
    InvalidManualOrder,
    OrderCouldNotBeSaved,
}

pub fn order_orchard_row(
    event: OrchardRowOrderRequested,
    storage: &mut impl OrchardStorage,
) -> Result<Vec<TreeId>, OrchardRowOrderError> {
    storage.transaction(|orchard| {
        let mut row_trees = orchard
            .trees_in_orchard(event.orchard_id)
            .map_err(|_| OrchardRowOrderError::OrderCouldNotBeSaved)?
            .into_iter()
            .filter(|tree| tree.tree.row_name.as_deref() == Some(event.row_name.as_str()))
            .collect::<Vec<_>>();
        if row_trees.is_empty() {
            return Err(OrchardRowOrderError::RowNotFound);
        }

        let ordered_tree_ids = match event.order {
            RowOrder::Manual(tree_ids) => validate_manual_order(&row_trees, tree_ids)?,
            direction => {
                sort_trees(&mut row_trees, direction);
                row_trees.into_iter().map(|tree| tree.id).collect()
            }
        };
        orchard
            .replace_row_order(event.orchard_id, &event.row_name, &ordered_tree_ids)
            .map_err(|_| OrchardRowOrderError::OrderCouldNotBeSaved)?;
        Ok(ordered_tree_ids)
    })
}

fn validate_manual_order(
    row_trees: &[OrchardTree],
    tree_ids: Vec<TreeId>,
) -> Result<Vec<TreeId>, OrchardRowOrderError> {
    let expected = row_trees.iter().map(|tree| tree.id).collect::<HashSet<_>>();
    let requested = tree_ids.iter().copied().collect::<HashSet<_>>();
    if tree_ids.len() != expected.len() || requested != expected {
        return Err(OrchardRowOrderError::InvalidManualOrder);
    }
    Ok(tree_ids)
}

fn sort_trees(trees: &mut [OrchardTree], order: RowOrder) {
    trees.sort_by(|left, right| {
        let coordinate_order = match order {
            RowOrder::EastToWest => right.tree.longitude.total_cmp(&left.tree.longitude),
            RowOrder::WestToEast => left.tree.longitude.total_cmp(&right.tree.longitude),
            RowOrder::NorthToSouth => right.tree.latitude.total_cmp(&left.tree.latitude),
            RowOrder::SouthToNorth => left.tree.latitude.total_cmp(&right.tree.latitude),
            RowOrder::Manual(_) => unreachable!("manual order is validated without sorting"),
        };
        coordinate_order.then_with(|| left.id.0.cmp(&right.id.0))
    });
}

impl From<OrchardStorageError> for OrchardRowOrderError {
    fn from(_: OrchardStorageError) -> Self {
        Self::OrderCouldNotBeSaved
    }
}
