use crate::hexagon::models::AuthenticatedSession;
use crate::hexagon::ports::AccessControl;

#[derive(Debug, PartialEq)]
pub enum UserSessionRestorationError {
    SessionNotFound,
    AuthenticationUnavailable,
}

pub fn restore_user_session(
    session_token: String,
    access_control: &mut impl AccessControl,
) -> Result<AuthenticatedSession, UserSessionRestorationError> {
    let user = access_control
        .user_for_session(&session_token)
        .map_err(|_| UserSessionRestorationError::AuthenticationUnavailable)?
        .ok_or(UserSessionRestorationError::SessionNotFound)?;
    let orchards = access_control
        .orchards_owned_by(user.id)
        .map_err(|_| UserSessionRestorationError::AuthenticationUnavailable)?;
    Ok(AuthenticatedSession {
        token: session_token,
        user,
        orchards,
    })
}
