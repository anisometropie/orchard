use crate::hexagon::models::{OrchardId, OrchardShareTokenId};
use crate::hexagon::ports::AccessControl;
use crate::hexagon::use_cases::authorize_orchard_owner::{
    OrchardOwnerAccessError, OrchardOwnerAccessRequested, authorize_orchard_owner,
};

pub struct OrchardShareRevoked {
    pub orchard_id: OrchardId,
    pub share_id: OrchardShareTokenId,
    pub session_token: String,
}

#[derive(Debug, PartialEq)]
pub enum OrchardShareRevokeError {
    SessionNotFound,
    OrchardNotOwned,
    ShareNotFound,
    ShareCouldNotBeRevoked,
}

pub fn revoke_orchard_share(
    event: OrchardShareRevoked,
    access_control: &mut impl AccessControl,
) -> Result<(), OrchardShareRevokeError> {
    let user = authorize_orchard_owner(
        OrchardOwnerAccessRequested {
            orchard_id: event.orchard_id,
            session_token: event.session_token,
        },
        access_control,
    )
    .map_err(|error| match error {
        OrchardOwnerAccessError::SessionNotFound => OrchardShareRevokeError::SessionNotFound,
        OrchardOwnerAccessError::OrchardNotOwned => OrchardShareRevokeError::OrchardNotOwned,
        OrchardOwnerAccessError::AccessCouldNotBeChecked => {
            OrchardShareRevokeError::ShareCouldNotBeRevoked
        }
    })?;
    access_control
        .revoke_orchard_share_token(user.id, event.orchard_id, event.share_id)
        .map_err(|_| OrchardShareRevokeError::ShareCouldNotBeRevoked)?
        .then_some(())
        .ok_or(OrchardShareRevokeError::ShareNotFound)
}
