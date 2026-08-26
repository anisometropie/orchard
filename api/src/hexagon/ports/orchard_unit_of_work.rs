use crate::hexagon::ports::TreeRepository;

#[derive(Debug, PartialEq)]
pub enum OrchardTransactionError {
    CouldNotBegin,
    CouldNotCommit,
}

pub trait OrchardTransaction: TreeRepository {
    fn commit(self) -> Result<(), OrchardTransactionError>;
    fn rollback(self);
}

pub trait OrchardUnitOfWork {
    type Transaction: OrchardTransaction;

    fn begin(&mut self) -> Result<Self::Transaction, OrchardTransactionError>;
}
