use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    Orchard, OrchardId, OrchardSharePermissions, OrchardShareTokenId,
};
use orchard_api::hexagon::ports::AccessControl;
use orchard_api::hexagon::use_cases::revoke_orchard_share::{
    OrchardShareRevoked, revoke_orchard_share,
};

#[test]
fn revoke_one_issued_token_without_affecting_the_others() {
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
    let revoked = storage
        .create_share_token(owner.id, OrchardId(7), OrchardSharePermissions::default())
        .unwrap();
    let retained = storage
        .create_share_token(owner.id, OrchardId(7), OrchardSharePermissions::default())
        .unwrap();

    revoke_orchard_share(
        OrchardShareRevoked {
            orchard_id: OrchardId(7),
            share_id: OrchardShareTokenId(1),
            session_token,
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(
        storage.orchard_share_for_token(&revoked.token).unwrap(),
        None
    );
    assert!(
        storage
            .orchard_share_for_token(&retained.token)
            .unwrap()
            .is_some()
    );
}
