use crate::hexagon::ports::AccessControl;

pub struct UserPasswordChangeRequested {
    pub username: String,
    pub new_password: String,
}

#[derive(Debug, PartialEq)]
pub enum UserPasswordChangeError {
    PasswordTooShort,
    UserNotFound,
    PasswordCouldNotBeChanged,
}

pub fn set_user_password(
    event: UserPasswordChangeRequested,
    access_control: &mut impl AccessControl,
) -> Result<(), UserPasswordChangeError> {
    if event.new_password.chars().count() < 12 {
        return Err(UserPasswordChangeError::PasswordTooShort);
    }
    access_control
        .set_user_password(&event.username, &event.new_password)
        .map_err(|_| UserPasswordChangeError::PasswordCouldNotBeChanged)?
        .then_some(())
        .ok_or(UserPasswordChangeError::UserNotFound)
}

#[cfg(test)]
mod tests {
    use crate::adapters::secondary::InMemoryOrchardStorage;
    use crate::hexagon::models::UserId;
    use crate::hexagon::ports::AccessControl;

    use super::{UserPasswordChangeError, UserPasswordChangeRequested, set_user_password};

    #[test]
    fn change_a_users_password_and_revoke_existing_sessions() {
        let (mut access_control, _) =
            InMemoryOrchardStorage::with_user_credentials("alice", "old password");
        let token = access_control.create_session(UserId(1)).unwrap();

        let changed = set_user_password(
            UserPasswordChangeRequested {
                username: "alice".into(),
                new_password: "correct horse battery staple".into(),
            },
            &mut access_control,
        );

        assert_eq!(changed, Ok(()));
        assert!(
            access_control
                .verify_credentials("alice", "correct horse battery staple")
                .unwrap()
                .is_some()
        );
        assert_eq!(access_control.user_for_session(&token), Ok(None));
    }

    #[test]
    fn reject_a_short_password_without_changing_it() {
        let (mut access_control, _) =
            InMemoryOrchardStorage::with_user_credentials("alice", "old password");

        let changed = set_user_password(
            UserPasswordChangeRequested {
                username: "alice".into(),
                new_password: "too short".into(),
            },
            &mut access_control,
        );

        assert_eq!(changed, Err(UserPasswordChangeError::PasswordTooShort));
    }
}
