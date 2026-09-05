use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{Orchard, OrchardId, OrchardSharePermission};
use orchard_api::hexagon::ports::AccessControl;
use orchard_api::hexagon::use_cases::authorize_orchard_waterer::{
    OrchardWateringAccessError, OrchardWateringAccessRequested, OrchardWateringCredential,
    authorize_orchard_waterer,
};

#[test]
fn allow_the_owner_or_current_share_token_to_water_only_its_orchard() {
    let (mut storage, _) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        Orchard {
            id: OrchardId(7),
            name: "My orchard".into(),
            longitude: 5.0,
            latitude: 45.0,
            reference_region: "Drôme".into(),
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
        .replace_share_token(owner.id, OrchardId(7), OrchardSharePermission::View)
        .unwrap();
    let watering_token = storage
        .replace_share_token(owner.id, OrchardId(7), OrchardSharePermission::Watering)
        .unwrap();

    assert_eq!(
        authorize_orchard_waterer(
            OrchardWateringAccessRequested {
                orchard_id: OrchardId(7),
                credential: OrchardWateringCredential::OwnerSession(session_token),
            },
            &mut storage,
        ),
        Ok(())
    );
    assert_eq!(
        authorize_orchard_waterer(
            OrchardWateringAccessRequested {
                orchard_id: OrchardId(7),
                credential: OrchardWateringCredential::ShareToken(view_token),
            },
            &mut storage,
        ),
        Err(OrchardWateringAccessError::PermissionDenied)
    );
    assert_eq!(
        authorize_orchard_waterer(
            OrchardWateringAccessRequested {
                orchard_id: OrchardId(7),
                credential: OrchardWateringCredential::ShareToken(watering_token.clone()),
            },
            &mut storage,
        ),
        Ok(())
    );
    assert_eq!(
        authorize_orchard_waterer(
            OrchardWateringAccessRequested {
                orchard_id: OrchardId(8),
                credential: OrchardWateringCredential::ShareToken(watering_token),
            },
            &mut storage,
        ),
        Err(OrchardWateringAccessError::AccessNotFound)
    );
}
