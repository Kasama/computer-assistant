use serde::{Deserialize, Serialize};

use crate::{HomeAssistantConfig, _default_off_state, _default_on_state};

use super::{ComputerAssistantConfig, Name, Publishable};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BinarySensor {
    pub name: Name,
    pub state_script: String,
}

impl BinarySensor {
    pub fn ha_config(&self, config: &ComputerAssistantConfig) -> HomeAssistantConfig {
        let base_stat_switch_topic = format!(
            "{}/entities/stat/binary_sensor/{}",
            config.base_topic,
            self.name.as_id()
        );
        HomeAssistantConfig::BinarySensor {
            base_topic: "".to_string(),
            state_topic: base_stat_switch_topic,
            availability_topic: format!("{}/{}", config.base_topic, config.availability_topic),
            device: config.device.clone(),
            name: self.name.to_string(),
            unique_id: self.name.as_id(),
            value_template: Some("{{value}}".to_string()),
            state_on: _default_on_state(),
            state_off: _default_off_state(),
        }
    }
}

impl Publishable for BinarySensor {
    fn state_script(&self) -> &str {
        &self.state_script
    }

    fn state_topic(&self, config: &ComputerAssistantConfig) -> String {
        format!(
            "{}/entities/stat/binary_sensor/{}",
            config.base_topic,
            self.name.as_id()
        )
    }
}
