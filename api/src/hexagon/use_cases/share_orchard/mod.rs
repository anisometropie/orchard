use crate::hexagon::models::{CreatedOrchardShareToken, OrchardId, OrchardSharePermissions};
use crate::hexagon::ports::AccessControl;
use crate::hexagon::use_cases::authorize_orchard_owner::{
    OrchardOwnerAccessError, OrchardOwnerAccessRequested, authorize_orchard_owner,
};

pub struct OrchardShareLinkRequested {
    pub orchard_id: OrchardId,
    pub session_token: String,
    pub permissions: OrchardSharePermissions,
}

#[derive(Debug, PartialEq)]
pub enum OrchardShareError {
    SessionNotFound,
    OrchardNotOwned,
    ShareLinkCouldNotBeCreated,
}

pub fn share_orchard(
    event: OrchardShareLinkRequested,
    access_control: &mut impl AccessControl,
) -> Result<CreatedOrchardShareToken, OrchardShareError> {
    let user = authorize_orchard_owner(
        OrchardOwnerAccessRequested {
            orchard_id: event.orchard_id,
            session_token: event.session_token,
        },
        access_control,
    )
    .map_err(|error| match error {
        OrchardOwnerAccessError::SessionNotFound => OrchardShareError::SessionNotFound,
        OrchardOwnerAccessError::OrchardNotOwned => OrchardShareError::OrchardNotOwned,
        OrchardOwnerAccessError::AccessCouldNotBeChecked => {
            OrchardShareError::ShareLinkCouldNotBeCreated
        }
    })?;
    access_control
        .create_share_token(user.id, event.orchard_id, event.permissions)
        .map_err(|_| OrchardShareError::ShareLinkCouldNotBeCreated)
}
