use crate::hexagon::ports::AccessControl;

#[derive(Debug, PartialEq)]
pub enum UserLogoutError {
    SessionCouldNotBeRevoked,
}

pub fn log_out_user(
    session_token: &str,
    access_control: &mut impl AccessControl,
) -> Result<(), UserLogoutError> {
    access_control
        .delete_session(session_token)
        .map_err(|_| UserLogoutError::SessionCouldNotBeRevoked)
}
