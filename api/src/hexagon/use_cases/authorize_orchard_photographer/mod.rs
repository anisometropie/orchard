use crate::hexagon::models::OrchardId;
use crate::hexagon::ports::AccessControl;
use crate::hexagon::use_cases::authorize_orchard_owner::{
    OrchardOwnerAccessError, OrchardOwnerAccessRequested, authorize_orchard_owner,
};

pub enum OrchardPhotographyCredential {
    OwnerSession(String),
    ShareToken(String),
}

pub struct OrchardPhotographyAccessRequested {
    pub orchard_id: OrchardId,
    pub credential: OrchardPhotographyCredential,
}

#[derive(Debug, PartialEq)]
pub enum OrchardPhotographyAccessError {
    AccessNotFound,
    PermissionDenied,
    AccessCouldNotBeChecked,
}

pub fn authorize_orchard_photographer(
    event: OrchardPhotographyAccessRequested,
    access_control: &mut impl AccessControl,
) -> Result<(), OrchardPhotographyAccessError> {
    match event.credential {
        OrchardPhotographyCredential::OwnerSession(session_token) => authorize_orchard_owner(
            OrchardOwnerAccessRequested {
                orchard_id: event.orchard_id,
                session_token,
            },
            access_control,
        )
        .map(|_| ())
        .map_err(|error| match error {
            OrchardOwnerAccessError::SessionNotFound | OrchardOwnerAccessError::OrchardNotOwned => {
                OrchardPhotographyAccessError::AccessNotFound
            }
            OrchardOwnerAccessError::AccessCouldNotBeChecked => {
                OrchardPhotographyAccessError::AccessCouldNotBeChecked
            }
        }),
        OrchardPhotographyCredential::ShareToken(share_token) => {
            let access = access_control
                .orchard_share_for_token(&share_token)
                .map_err(|_| OrchardPhotographyAccessError::AccessCouldNotBeChecked)?
                .filter(|access| access.orchard_id == event.orchard_id)
                .ok_or(OrchardPhotographyAccessError::AccessNotFound)?;
            if access.permissions.add_photos {
                Ok(())
            } else {
                Err(OrchardPhotographyAccessError::PermissionDenied)
            }
        }
    }
}
