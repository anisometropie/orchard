use crate::hexagon::models::{IssuedOrchardShareToken, OrchardId};
use crate::hexagon::ports::AccessControl;
use crate::hexagon::use_cases::authorize_orchard_owner::{
    OrchardOwnerAccessError, OrchardOwnerAccessRequested, authorize_orchard_owner,
};

pub struct OrchardSharesRequested {
    pub orchard_id: OrchardId,
    pub session_token: String,
}

#[derive(Debug, PartialEq)]
pub enum OrchardSharesListError {
    SessionNotFound,
    OrchardNotOwned,
    SharesCouldNotBeListed,
}

pub fn list_orchard_shares(
    event: OrchardSharesRequested,
    access_control: &mut impl AccessControl,
) -> Result<Vec<IssuedOrchardShareToken>, OrchardSharesListError> {
    let user = authorize_orchard_owner(
        OrchardOwnerAccessRequested {
            orchard_id: event.orchard_id,
            session_token: event.session_token,
        },
        access_control,
    )
    .map_err(|error| match error {
        OrchardOwnerAccessError::SessionNotFound => OrchardSharesListError::SessionNotFound,
        OrchardOwnerAccessError::OrchardNotOwned => OrchardSharesListError::OrchardNotOwned,
        OrchardOwnerAccessError::AccessCouldNotBeChecked => {
            OrchardSharesListError::SharesCouldNotBeListed
        }
    })?;
    access_control
        .issued_orchard_share_tokens(user.id, event.orchard_id)
        .map_err(|_| OrchardSharesListError::SharesCouldNotBeListed)
}
