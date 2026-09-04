use crate::hexagon::models::{Orchard, OrchardId, User, UserId};

#[derive(Debug, PartialEq)]
pub enum AccessControlError {
    CredentialsCouldNotBeChecked,
    SessionCouldNotBeCreated,
    OrchardsCouldNotBeRead,
    SessionCouldNotBeRead,
    OrchardOwnershipCouldNotBeRead,
    ShareTokenCouldNotBeCreated,
    ShareTokenCouldNotBeRead,
    SessionCouldNotBeDeleted,
    PasswordCouldNotBeChanged,
}

pub trait AccessControl {
    fn verify_credentials(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<Option<User>, AccessControlError>;

    fn create_session(&mut self, user_id: UserId) -> Result<String, AccessControlError>;

    fn orchards_owned_by(&mut self, user_id: UserId) -> Result<Vec<Orchard>, AccessControlError>;

    fn user_for_session(&mut self, token: &str) -> Result<Option<User>, AccessControlError>;

    fn user_owns_orchard(
        &mut self,
        user_id: UserId,
        orchard_id: OrchardId,
    ) -> Result<bool, AccessControlError>;

    fn replace_share_token(
        &mut self,
        user_id: UserId,
        orchard_id: OrchardId,
    ) -> Result<String, AccessControlError>;

    fn orchard_for_share_token(
        &mut self,
        token: &str,
    ) -> Result<Option<OrchardId>, AccessControlError>;

    fn delete_session(&mut self, token: &str) -> Result<(), AccessControlError>;

    fn set_user_password(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<bool, AccessControlError>;
}
