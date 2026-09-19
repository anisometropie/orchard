use crate::hexagon::models::{
    CreatedOrchardShareToken, IssuedOrchardShareToken, Orchard, OrchardId, OrchardShareAccess,
    OrchardSharePermissions, OrchardShareTokenId, User, UserId,
};

#[derive(Debug, PartialEq)]
pub enum AccessControlError {
    CredentialsCouldNotBeChecked,
    SessionCouldNotBeCreated,
    OrchardsCouldNotBeRead,
    SessionCouldNotBeRead,
    OrchardOwnershipCouldNotBeRead,
    ShareTokenCouldNotBeCreated,
    ShareTokenCouldNotBeRead,
    ShareTokensCouldNotBeListed,
    ShareTokenCouldNotBeChanged,
    ShareTokenCouldNotBeRevoked,
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

    fn create_share_token(
        &mut self,
        user_id: UserId,
        orchard_id: OrchardId,
        permissions: OrchardSharePermissions,
    ) -> Result<CreatedOrchardShareToken, AccessControlError>;

    fn orchard_share_for_token(
        &mut self,
        token: &str,
    ) -> Result<Option<OrchardShareAccess>, AccessControlError>;

    fn issued_orchard_share_tokens(
        &mut self,
        user_id: UserId,
        orchard_id: OrchardId,
    ) -> Result<Vec<IssuedOrchardShareToken>, AccessControlError>;

    fn change_orchard_share_permissions(
        &mut self,
        user_id: UserId,
        orchard_id: OrchardId,
        share_id: OrchardShareTokenId,
        permissions: OrchardSharePermissions,
    ) -> Result<bool, AccessControlError>;

    fn revoke_orchard_share_token(
        &mut self,
        user_id: UserId,
        orchard_id: OrchardId,
        share_id: OrchardShareTokenId,
    ) -> Result<bool, AccessControlError>;

    fn delete_session(&mut self, token: &str) -> Result<(), AccessControlError>;

    fn set_user_password(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<bool, AccessControlError>;
}
