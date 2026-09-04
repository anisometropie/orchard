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
