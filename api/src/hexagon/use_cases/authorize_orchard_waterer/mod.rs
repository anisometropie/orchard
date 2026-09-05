use crate::hexagon::models::{OrchardId, OrchardSharePermission};
use crate::hexagon::ports::AccessControl;
use crate::hexagon::use_cases::authorize_orchard_owner::{
    OrchardOwnerAccessError, OrchardOwnerAccessRequested, authorize_orchard_owner,
};

pub enum OrchardWateringCredential {
    OwnerSession(String),
    ShareToken(String),
}

pub struct OrchardWateringAccessRequested {
    pub orchard_id: OrchardId,
    pub credential: OrchardWateringCredential,
}

#[derive(Debug, PartialEq)]
pub enum OrchardWateringAccessError {
    AccessNotFound,
    PermissionDenied,
    AccessCouldNotBeChecked,
}

pub fn authorize_orchard_waterer(
    event: OrchardWateringAccessRequested,
    access_control: &mut impl AccessControl,
) -> Result<(), OrchardWateringAccessError> {
    match event.credential {
        OrchardWateringCredential::OwnerSession(session_token) => authorize_orchard_owner(
            OrchardOwnerAccessRequested {
                orchard_id: event.orchard_id,
                session_token,
            },
            access_control,
        )
        .map(|_| ())
        .map_err(|error| match error {
            OrchardOwnerAccessError::SessionNotFound | OrchardOwnerAccessError::OrchardNotOwned => {
                OrchardWateringAccessError::AccessNotFound
            }
            OrchardOwnerAccessError::AccessCouldNotBeChecked => {
                OrchardWateringAccessError::AccessCouldNotBeChecked
            }
        }),
        OrchardWateringCredential::ShareToken(share_token) => {
            let access = access_control
                .orchard_share_for_token(&share_token)
                .map_err(|_| OrchardWateringAccessError::AccessCouldNotBeChecked)?
                .filter(|access| access.orchard_id == event.orchard_id)
                .ok_or(OrchardWateringAccessError::AccessNotFound)?;
            match access.permission {
                OrchardSharePermission::Watering => Ok(()),
                OrchardSharePermission::View => Err(OrchardWateringAccessError::PermissionDenied),
            }
        }
    }
}
