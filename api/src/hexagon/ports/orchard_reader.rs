use crate::hexagon::models::OrchardTree;

#[derive(Debug, PartialEq)]
pub enum OrchardReadError {
    TreesCouldNotBeRead,
}

pub trait OrchardReader {
    fn trees(&mut self) -> Result<Vec<OrchardTree>, OrchardReadError>;
}
