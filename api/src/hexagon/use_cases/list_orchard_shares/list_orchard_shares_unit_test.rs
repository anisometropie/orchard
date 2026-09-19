use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{Orchard, OrchardId, OrchardSharePermissions};
use orchard_api::hexagon::ports::AccessControl;
use orchard_api::hexagon::use_cases::list_orchard_shares::{
    OrchardSharesRequested, list_orchard_shares,
};

#[test]
fn list_only_the_owned_orchards_issued_tokens_without_their_secrets() {
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
    storage
        .create_share_token(
            owner.id,
            OrchardId(7),
            OrchardSharePermissions {
                harvest: true,
                water: false,
                add_photos: true,
            },
        )
        .unwrap();

    let shares = list_orchard_shares(
        OrchardSharesRequested {
            orchard_id: OrchardId(7),
            session_token,
        },
        &mut storage,
    )
    .unwrap();

    assert_eq!(shares.len(), 1);
    assert_eq!(shares[0].id.0, 1);
    assert_eq!(
        shares[0].permissions,
        OrchardSharePermissions {
            harvest: true,
            water: false,
            add_photos: true,
        }
    );
}
