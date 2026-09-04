use crate::hexagon::models::{MapConfiguration, OrchardId};
use crate::hexagon::ports::MapConfigurationStorage;

#[derive(Debug, PartialEq)]
pub enum MapConfigurationLoadError {
    ConfigurationNotFound,
    ConfigurationCouldNotBeRead,
}

pub fn load_map_configuration(
    storage: &mut impl MapConfigurationStorage,
) -> Result<MapConfiguration, MapConfigurationLoadError> {
    storage
        .map_configuration()
        .map_err(|_| MapConfigurationLoadError::ConfigurationCouldNotBeRead)?
        .ok_or(MapConfigurationLoadError::ConfigurationNotFound)
}

pub fn load_orchard_map_configuration(
    orchard_id: OrchardId,
    storage: &mut impl MapConfigurationStorage,
) -> Result<MapConfiguration, MapConfigurationLoadError> {
    storage
        .map_configuration_for_orchard(orchard_id)
        .map_err(|_| MapConfigurationLoadError::ConfigurationCouldNotBeRead)?
        .ok_or(MapConfigurationLoadError::ConfigurationNotFound)
}
