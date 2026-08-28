use crate::hexagon::models::OrchardTree;
use crate::hexagon::ports::{OrchardReadError, OrchardReader};

pub fn list_orchard_trees<R>(orchard_reader: &mut R) -> Result<Vec<OrchardTree>, OrchardReadError>
where
    R: OrchardReader,
{
    orchard_reader.trees()
}
