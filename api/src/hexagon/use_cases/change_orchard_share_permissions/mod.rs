use crate::hexagon::models::{OrchardId, OrchardSharePermissions, OrchardShareTokenId};
use crate::hexagon::ports::AccessControl;
use crate::hexagon::use_cases::authorize_orchard_owner::{
    OrchardOwnerAccessError, OrchardOwnerAccessRequested, authorize_orchard_owner,
};

pub struct OrchardSharePermissionsChanged {
    pub orchard_id: OrchardId,
    pub share_id: OrchardShareTokenId,
    pub permissions: OrchardSharePermissions,
    pub session_token: String,
}

#[derive(Debug, PartialEq)]
pub enum OrchardSharePermissionsChangeError {
    SessionNotFound,
    OrchardNotOwned,
    ShareNotFound,
    PermissionsCouldNotBeChanged,
}

pub fn change_orchard_share_permissions(
    event: OrchardSharePermissionsChanged,
    access_control: &mut impl AccessControl,
) -> Result<(), OrchardSharePermissionsChangeError> {
    let user = authorize_orchard_owner(
        OrchardOwnerAccessRequested {
            orchard_id: event.orchard_id,
            session_token: event.session_token,
        },
        access_control,
    )
    .map_err(|error| match error {
        OrchardOwnerAccessError::SessionNotFound => {
            OrchardSharePermissionsChangeError::SessionNotFound
        }
        OrchardOwnerAccessError::OrchardNotOwned => {
            OrchardSharePermissionsChangeError::OrchardNotOwned
        }
        OrchardOwnerAccessError::AccessCouldNotBeChecked => {
            OrchardSharePermissionsChangeError::PermissionsCouldNotBeChanged
        }
    })?;
    access_control
        .change_orchard_share_permissions(
            user.id,
            event.orchard_id,
            event.share_id,
            event.permissions,
        )
        .map_err(|_| OrchardSharePermissionsChangeError::PermissionsCouldNotBeChanged)?
        .then_some(())
        .ok_or(OrchardSharePermissionsChangeError::ShareNotFound)
}
