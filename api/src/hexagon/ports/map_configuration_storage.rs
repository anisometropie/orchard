use crate::hexagon::models::{AerialOverlayId, AerialOverlayImage, MapConfiguration};

#[derive(Debug, PartialEq)]
pub enum MapConfigurationStorageError {
    ConfigurationCouldNotBeRead,
    AerialOverlayImageCouldNotBeRead,
}

pub trait MapConfigurationStorage {
    fn map_configuration(
        &mut self,
    ) -> Result<Option<MapConfiguration>, MapConfigurationStorageError>;

    fn aerial_overlay_image(
        &mut self,
        overlay_id: AerialOverlayId,
    ) -> Result<Option<AerialOverlayImage>, MapConfigurationStorageError>;
}
