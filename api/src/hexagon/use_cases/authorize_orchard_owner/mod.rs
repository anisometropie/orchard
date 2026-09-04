use crate::hexagon::models::{OrchardId, User};
use crate::hexagon::ports::AccessControl;

pub struct OrchardOwnerAccessRequested {
    pub orchard_id: OrchardId,
    pub session_token: String,
}

#[derive(Debug, PartialEq)]
pub enum OrchardOwnerAccessError {
    SessionNotFound,
    OrchardNotOwned,
    AccessCouldNotBeChecked,
}

pub fn authorize_orchard_owner(
    event: OrchardOwnerAccessRequested,
    access_control: &mut impl AccessControl,
) -> Result<User, OrchardOwnerAccessError> {
    let user = access_control
        .user_for_session(&event.session_token)
        .map_err(|_| OrchardOwnerAccessError::AccessCouldNotBeChecked)?
        .ok_or(OrchardOwnerAccessError::SessionNotFound)?;
    let owns_orchard = access_control
        .user_owns_orchard(user.id, event.orchard_id)
        .map_err(|_| OrchardOwnerAccessError::AccessCouldNotBeChecked)?;
    if !owns_orchard {
        return Err(OrchardOwnerAccessError::OrchardNotOwned);
    }
    Ok(user)
}
