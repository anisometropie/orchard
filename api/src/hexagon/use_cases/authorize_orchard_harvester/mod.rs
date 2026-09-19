use crate::hexagon::models::{OrchardId, OrchardSharePermission};
use crate::hexagon::ports::AccessControl;
use crate::hexagon::use_cases::authorize_orchard_owner::{
    OrchardOwnerAccessError, OrchardOwnerAccessRequested, authorize_orchard_owner,
};

pub enum OrchardHarvestingCredential {
    OwnerSession(String),
    ShareToken(String),
}

pub struct OrchardHarvestingAccessRequested {
    pub orchard_id: OrchardId,
    pub credential: OrchardHarvestingCredential,
}

#[derive(Debug, PartialEq)]
pub enum OrchardHarvestingAccessError {
    AccessNotFound,
    PermissionDenied,
    AccessCouldNotBeChecked,
}

pub fn authorize_orchard_harvester(
    event: OrchardHarvestingAccessRequested,
    access_control: &mut impl AccessControl,
) -> Result<(), OrchardHarvestingAccessError> {
    match event.credential {
        OrchardHarvestingCredential::OwnerSession(session_token) => authorize_orchard_owner(
            OrchardOwnerAccessRequested {
                orchard_id: event.orchard_id,
                session_token,
            },
            access_control,
        )
        .map(|_| ())
        .map_err(|error| match error {
            OrchardOwnerAccessError::SessionNotFound | OrchardOwnerAccessError::OrchardNotOwned => {
                OrchardHarvestingAccessError::AccessNotFound
            }
            OrchardOwnerAccessError::AccessCouldNotBeChecked => {
                OrchardHarvestingAccessError::AccessCouldNotBeChecked
            }
        }),
        OrchardHarvestingCredential::ShareToken(share_token) => {
            let access = access_control
                .orchard_share_for_token(&share_token)
                .map_err(|_| OrchardHarvestingAccessError::AccessCouldNotBeChecked)?
                .filter(|access| access.orchard_id == event.orchard_id)
                .ok_or(OrchardHarvestingAccessError::AccessNotFound)?;
            match access.permission {
                OrchardSharePermission::HarvestAndWatering => Ok(()),
                OrchardSharePermission::View | OrchardSharePermission::Watering => {
                    Err(OrchardHarvestingAccessError::PermissionDenied)
                }
            }
        }
    }
}
