use crate::hexagon::models::{AerialOverlayId, AerialOverlayImage};
use crate::hexagon::ports::MapConfigurationStorage;

#[derive(Debug, PartialEq)]
pub enum AerialOverlayImageLoadError {
    ImageNotFound,
    ImageCouldNotBeRead,
}

pub fn load_aerial_overlay_image(
    overlay_id: AerialOverlayId,
    storage: &mut impl MapConfigurationStorage,
) -> Result<AerialOverlayImage, AerialOverlayImageLoadError> {
    storage
        .aerial_overlay_image(overlay_id)
        .map_err(|_| AerialOverlayImageLoadError::ImageCouldNotBeRead)?
        .ok_or(AerialOverlayImageLoadError::ImageNotFound)
}
