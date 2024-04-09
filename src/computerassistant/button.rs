use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::{HomeAssistantConfig, _default_payload_press};

use super::{ComputerAssistantConfig, Name, Updateable};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Button {
    pub name: Name,
    pub command_script: String,
}

impl Button {
    pub fn ha_config(&self, config: &ComputerAssistantConfig) -> HomeAssistantConfig {
        let base_cmd_switch_topic = format!(
            "{}/entities/cmd/button/{}",
            config.base_topic,
            self.name.as_id()
        );

        HomeAssistantConfig::Button {
            base_topic: Default::default(),
            command_topic: base_cmd_switch_topic,
            availability_topic: format!("{}/{}", config.base_topic, config.availability_topic),
            device: config.device.clone(),
            name: self.name.to_string(),
            unique_id: self.name.as_id(),
            value_template: Some("{{value}}".to_string()),
            payload_press: _default_payload_press(),
        }
    }
}

impl Updateable for Button {
    fn update(&self, topic: &[&str], state: &str) -> anyhow::Result<()> {
        match topic {
            ["button", id] if id == &self.name.as_id() => {
                if state != "PRESS" {
                    return Ok(());
                }
                Command::new("bash")
                    .arg("-c")
                    .arg(&self.command_script)
                    .spawn()?
                    .wait()?;
            }
            _ => {}
        };
        Ok(())
    }
}
