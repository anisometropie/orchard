use crate::hexagon::models::AuthenticatedSession;
use crate::hexagon::ports::AccessControl;

pub struct UserLoginRequested {
    pub username: String,
    pub password: String,
}

#[derive(Debug, PartialEq)]
pub enum UserLoginError {
    InvalidCredentials,
    AuthenticationUnavailable,
}

pub fn log_in_user(
    event: UserLoginRequested,
    access_control: &mut impl AccessControl,
) -> Result<AuthenticatedSession, UserLoginError> {
    let user = access_control
        .verify_credentials(&event.username, &event.password)
        .map_err(|_| UserLoginError::AuthenticationUnavailable)?
        .ok_or(UserLoginError::InvalidCredentials)?;
    let orchards = access_control
        .orchards_owned_by(user.id)
        .map_err(|_| UserLoginError::AuthenticationUnavailable)?;
    let token = access_control
        .create_session(user.id)
        .map_err(|_| UserLoginError::AuthenticationUnavailable)?;
    Ok(AuthenticatedSession {
        token,
        user,
        orchards,
    })
}
