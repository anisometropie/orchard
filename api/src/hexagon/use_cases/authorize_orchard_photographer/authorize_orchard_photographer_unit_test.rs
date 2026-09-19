use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{Orchard, OrchardId, OrchardSharePermissions};
use orchard_api::hexagon::ports::AccessControl;
use orchard_api::hexagon::use_cases::authorize_orchard_photographer::{
    OrchardPhotographyAccessError, OrchardPhotographyAccessRequested, OrchardPhotographyCredential,
    authorize_orchard_photographer,
};

#[test]
fn allow_only_the_owner_or_a_share_with_photo_permission_to_add_photos() {
    let (mut storage, _) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        Orchard {
            id: OrchardId(7),
            name: "My orchard".into(),
            longitude: 0.5,
            latitude: 0.5,
            reference_region: "Example Region".into(),
        },
        vec![],
        vec![],
    );
    let owner = storage
        .verify_credentials("owner", "password")
        .unwrap()
        .unwrap();
    let session_token = storage.create_session(owner.id).unwrap();
    let view_token = storage
        .create_share_token(owner.id, OrchardId(7), OrchardSharePermissions::default())
        .unwrap()
        .token;
    let photo_token = storage
        .create_share_token(
            owner.id,
            OrchardId(7),
            OrchardSharePermissions {
                add_photos: true,
                ..Default::default()
            },
        )
        .unwrap()
        .token;

    assert_eq!(
        authorize_orchard_photographer(
            OrchardPhotographyAccessRequested {
                orchard_id: OrchardId(7),
                credential: OrchardPhotographyCredential::OwnerSession(session_token),
            },
            &mut storage,
        ),
        Ok(())
    );
    assert_eq!(
        authorize_orchard_photographer(
            OrchardPhotographyAccessRequested {
                orchard_id: OrchardId(7),
                credential: OrchardPhotographyCredential::ShareToken(view_token),
            },
            &mut storage,
        ),
        Err(OrchardPhotographyAccessError::PermissionDenied)
    );
    assert_eq!(
        authorize_orchard_photographer(
            OrchardPhotographyAccessRequested {
                orchard_id: OrchardId(7),
                credential: OrchardPhotographyCredential::ShareToken(photo_token.clone()),
            },
            &mut storage,
        ),
        Ok(())
    );
    assert_eq!(
        authorize_orchard_photographer(
            OrchardPhotographyAccessRequested {
                orchard_id: OrchardId(8),
                credential: OrchardPhotographyCredential::ShareToken(photo_token),
            },
            &mut storage,
        ),
        Err(OrchardPhotographyAccessError::AccessNotFound)
    );
}
