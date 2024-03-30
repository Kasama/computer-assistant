use std::process::Command;
use std::str::{from_utf8, FromStr};

use serde::{Deserialize, Serialize};

use crate::homeassistant::State;
use crate::{HomeAssistantConfig, _default_off_state, _default_on_state};

use super::{ComputerAssistantConfig, Name, Publishable, Updateable};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Switch {
    pub name: Name,
    pub state_script: String,
    pub on_script: String,
    pub off_script: String,
    pub toggle_script: Option<String>,
}

impl Switch {
    pub fn ha_config(&self, config: &ComputerAssistantConfig) -> HomeAssistantConfig {
        let base_cmd_switch_topic = format!(
            "{}/entities/cmd/switch/{}",
            config.base_topic,
            self.name.as_id()
        );
        let base_stat_switch_topic = format!(
            "{}/entities/stat/switch/{}",
            config.base_topic,
            self.name.as_id()
        );

        HomeAssistantConfig::Switch {
            base_topic: "".to_string(),
            command_topic: base_cmd_switch_topic,
            state_topic: base_stat_switch_topic,
            availability_topic: format!("{}/{}", config.base_topic, config.availability_topic),
            device: config.device.clone(),
            name: self.name.to_string(),
            unique_id: self.name.as_id(),
            value_template: Some("{{value}}".to_string()),
            state_on: _default_on_state(),
            state_off: _default_off_state(),
            // entity_category: "diagnostic".to_string(),
        }
    }
}

impl Updateable for Switch {
    fn update(&self, topic: &[&str], state: &str) -> anyhow::Result<()> {
        match topic {
            ["switch", id] if id == &self.name.as_id() => {
                let script = match State::from_str(state)? {
                    State::On => &self.on_script,
                    State::Off => &self.off_script,
                };
                Command::new("bash").arg("-c").arg(script).spawn()?.wait()?;
            }
            _ => {}
        };
        Ok(())
    }
}

impl Publishable for Switch {
    fn state_script(&self) -> &str {
        &self.state_script
    }

    fn state_topic(&self, config: &ComputerAssistantConfig) -> String {
        format!(
            "{}/entities/stat/switch/{}",
            config.base_topic,
            self.name.as_id()
        )
    }
}
