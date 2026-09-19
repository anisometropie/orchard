use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OrchardId(pub u64);

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Orchard {
    pub id: OrchardId,
    pub name: String,
    pub longitude: f64,
    pub latitude: f64,
    pub reference_region: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrchardSharePermission {
    View,
    Watering,
    HarvestAndWatering,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct OrchardSharePermissions {
    pub harvest: bool,
    pub water: bool,
    pub add_photos: bool,
}

impl From<OrchardSharePermission> for OrchardSharePermissions {
    fn from(permission: OrchardSharePermission) -> Self {
        match permission {
            OrchardSharePermission::View => Self::default(),
            OrchardSharePermission::Watering => Self {
                water: true,
                ..Self::default()
            },
            OrchardSharePermission::HarvestAndWatering => Self {
                harvest: true,
                water: true,
                add_photos: false,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct OrchardShareTokenId(pub u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OrchardShareAccess {
    pub orchard_id: OrchardId,
    pub permissions: OrchardSharePermissions,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatedOrchardShareToken {
    pub id: OrchardShareTokenId,
    pub token: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct IssuedOrchardShareToken {
    pub id: OrchardShareTokenId,
    pub permissions: OrchardSharePermissions,
    pub created_at_unix_seconds: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct UserId(pub u64);

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct User {
    pub id: UserId,
    pub username: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AuthenticatedSession {
    pub token: String,
    pub user: User,
    pub orchards: Vec<Orchard>,
}
