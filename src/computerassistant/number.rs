use std::process::Command;
use std::str::from_utf8;

use serde::{Deserialize, Serialize};

use crate::HomeAssistantConfig;

use super::{ComputerAssistantConfig, Name, Publishable, Updateable};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Number {
    pub name: Name,
    pub state_script: String,
    pub command_script: String,
    #[serde(default)]
    pub min: f64,
    #[serde(default)]
    pub max: f64,
    #[serde(default)]
    pub step: f64,
    #[serde(default)]
    pub unit_of_measurement: String,
}

impl Number {
    pub fn ha_config(&self, config: &ComputerAssistantConfig) -> HomeAssistantConfig {
        let base_cmd_switch_topic = format!(
            "{}/entities/cmd/number/{}",
            config.base_topic,
            self.name.as_id()
        );
        let base_stat_switch_topic = format!(
            "{}/entities/stat/number/{}",
            config.base_topic,
            self.name.as_id()
        );
        HomeAssistantConfig::Number {
            base_topic: "".to_string(),
            state_topic: base_stat_switch_topic,
            availability_topic: format!("{}/{}", config.base_topic, config.availability_topic),
            device: config.device.clone(),
            name: self.name.to_string(),
            unique_id: self.name.as_id(),
            value_template: Some("{{value}}".to_string()),
            command_topic: base_cmd_switch_topic,
            command_template: Some("{{value}}".to_string()),
            min: self.min,
            max: self.max,
            step: self.step,
            unit_of_measurement: self.unit_of_measurement.clone(),
        }
    }
}

impl Publishable for Number {
    fn state_script(&self) -> &str {
        &self.state_script
    }

    fn state_topic(&self, config: &ComputerAssistantConfig) -> String {
        format!(
            "{}/entities/stat/number/{}",
            config.base_topic,
            self.name.as_id()
        )
    }

    fn state_payload(&self, _config: &ComputerAssistantConfig) -> anyhow::Result<String> {
        // capture stdout and parse it as a float
        let state = Command::new("bash")
            .arg("-c")
            .arg(self.state_script())
            .output()?;

        let str_state = from_utf8(&state.stdout)?;

        Ok(str_state.trim().to_string())
    }
}

impl Updateable for Number {
    fn update(&self, topic: &[&str], state: &str) -> anyhow::Result<()> {
        match topic {
            ["number", id] if id == &self.name.as_id() => {
                let mut child = Command::new("bash")
                    .arg("-c")
                    .arg(&self.command_script)
                    .arg("computer-assistant")
                    .arg(state)
                    .spawn()?;

                child.wait()?;
            }
            _ => {}
        };
        Ok(())
    }
}
