use crate::hexagon::models::OrchardId;
use crate::hexagon::ports::AccessControl;
use crate::hexagon::use_cases::authorize_orchard_owner::{
    OrchardOwnerAccessError, OrchardOwnerAccessRequested, authorize_orchard_owner,
};

pub enum OrchardReadCredential {
    OwnerSession(String),
    ShareToken(String),
}

pub struct OrchardReadAccessRequested {
    pub orchard_id: OrchardId,
    pub credential: OrchardReadCredential,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrchardReadAccess {
    Editable,
    ReadOnly,
}

#[derive(Debug, PartialEq)]
pub enum OrchardReadAccessError {
    AccessNotFound,
    AccessCouldNotBeChecked,
}

pub fn authorize_orchard_reader(
    event: OrchardReadAccessRequested,
    access_control: &mut impl AccessControl,
) -> Result<OrchardReadAccess, OrchardReadAccessError> {
    match event.credential {
        OrchardReadCredential::OwnerSession(session_token) => authorize_orchard_owner(
            OrchardOwnerAccessRequested {
                orchard_id: event.orchard_id,
                session_token,
            },
            access_control,
        )
        .map(|_| OrchardReadAccess::Editable)
        .map_err(|error| match error {
            OrchardOwnerAccessError::SessionNotFound | OrchardOwnerAccessError::OrchardNotOwned => {
                OrchardReadAccessError::AccessNotFound
            }
            OrchardOwnerAccessError::AccessCouldNotBeChecked => {
                OrchardReadAccessError::AccessCouldNotBeChecked
            }
        }),
        OrchardReadCredential::ShareToken(share_token) => access_control
            .orchard_share_for_token(&share_token)
            .map_err(|_| OrchardReadAccessError::AccessCouldNotBeChecked)?
            .filter(|access| access.orchard_id == event.orchard_id)
            .map(|_| OrchardReadAccess::ReadOnly)
            .ok_or(OrchardReadAccessError::AccessNotFound),
    }
}
