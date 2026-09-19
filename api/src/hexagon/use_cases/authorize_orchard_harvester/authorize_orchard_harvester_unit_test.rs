use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{Orchard, OrchardId, OrchardSharePermission};
use orchard_api::hexagon::ports::AccessControl;
use orchard_api::hexagon::use_cases::authorize_orchard_harvester::{
    OrchardHarvestingAccessError, OrchardHarvestingAccessRequested, OrchardHarvestingCredential,
    authorize_orchard_harvester,
};

#[test]
fn allow_only_the_owner_or_combined_share_token_to_harvest_its_orchard() {
    let (mut storage, _) = InMemoryOrchardStorage::with_user_owned_orchard(
        "owner",
        "password",
        Orchard {
            id: OrchardId(7),
            name: "My orchard".into(),
            longitude: -73.5,
            latitude: 12.25,
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
        .create_share_token(owner.id, OrchardId(7), OrchardSharePermission::View.into())
        .unwrap()
        .token;
    let watering_token = storage
        .create_share_token(
            owner.id,
            OrchardId(7),
            OrchardSharePermission::Watering.into(),
        )
        .unwrap()
        .token;
    let harvest_watering_token = storage
        .create_share_token(
            owner.id,
            OrchardId(7),
            OrchardSharePermission::HarvestAndWatering.into(),
        )
        .unwrap()
        .token;

    assert_eq!(
        authorize_orchard_harvester(
            OrchardHarvestingAccessRequested {
                orchard_id: OrchardId(7),
                credential: OrchardHarvestingCredential::OwnerSession(session_token),
            },
            &mut storage,
        ),
        Ok(())
    );
    for token in [view_token, watering_token] {
        assert_eq!(
            authorize_orchard_harvester(
                OrchardHarvestingAccessRequested {
                    orchard_id: OrchardId(7),
                    credential: OrchardHarvestingCredential::ShareToken(token),
                },
                &mut storage,
            ),
            Err(OrchardHarvestingAccessError::PermissionDenied)
        );
    }
    assert_eq!(
        authorize_orchard_harvester(
            OrchardHarvestingAccessRequested {
                orchard_id: OrchardId(7),
                credential: OrchardHarvestingCredential::ShareToken(harvest_watering_token.clone(),),
            },
            &mut storage,
        ),
        Ok(())
    );
    assert_eq!(
        authorize_orchard_harvester(
            OrchardHarvestingAccessRequested {
                orchard_id: OrchardId(8),
                credential: OrchardHarvestingCredential::ShareToken(harvest_watering_token),
            },
            &mut storage,
        ),
        Err(OrchardHarvestingAccessError::AccessNotFound)
    );
}
