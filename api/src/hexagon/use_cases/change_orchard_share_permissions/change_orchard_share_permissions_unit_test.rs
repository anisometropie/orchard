use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    Orchard, OrchardId, OrchardSharePermissions, OrchardShareTokenId,
};
use orchard_api::hexagon::ports::AccessControl;
use orchard_api::hexagon::use_cases::change_orchard_share_permissions::{
    OrchardSharePermissionsChanged, change_orchard_share_permissions,
};

#[test]
fn change_permissions_without_changing_the_issued_token() {
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
    let created = storage
        .create_share_token(owner.id, OrchardId(7), OrchardSharePermissions::default())
        .unwrap();
    let changed_permissions = OrchardSharePermissions {
        harvest: true,
        water: true,
        add_photos: true,
    };

    change_orchard_share_permissions(
        OrchardSharePermissionsChanged {
            orchard_id: OrchardId(7),
            share_id: OrchardShareTokenId(1),
            permissions: changed_permissions,
            session_token,
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(
        storage
            .orchard_share_for_token(&created.token)
            .unwrap()
            .unwrap()
            .permissions,
        changed_permissions
    );
}
