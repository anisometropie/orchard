use crate::hexagon::models::MapConfiguration;
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
