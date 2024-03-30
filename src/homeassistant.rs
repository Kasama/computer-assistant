use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum State {
    On,
    Off,
}

impl ToString for State {
    fn to_string(&self) -> String {
        match self {
            State::On => "ON".to_string(),
            State::Off => "OFF".to_string(),
        }
    }
}

impl FromStr for State {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "ON" => Ok(State::On),
            "OFF" => Ok(State::Off),
            _ => Err(anyhow::anyhow!("Unknown state: {}", s)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct HomeAssistantDevice {
    ids: Vec<String>,
    #[serde(alias = "sw")]
    version: String,
    #[serde(alias = "mf")]
    manufacturer: String,
    #[serde(alias = "cu")]
    ip: String,
    #[serde(alias = "mdl")]
    model: String,
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged, rename_all = "snake_case")]
pub enum HomeAssistantConfig {
    Switch {
        #[serde(rename = "~")]
        base_topic: String,
        #[serde(rename = "cmd_t")]
        command_topic: String,
        #[serde(rename = "stat_t")]
        state_topic: String,
        #[serde(rename = "dev")]
        device: HomeAssistantDevice,
        name: String,
        #[serde(rename = "uniq_id")]
        unique_id: String,
        value_template: String,
        entity_category: String,
    },
}

pub trait Switch {
    /// Get the current state of the switch
    fn get_state(&self) -> State;

    /// Set the state of the switch based on an arbitrary payload sent by homeassistant
    ///
    /// The payload is usually "ON" or "OFF" but can be configured differently in homeassistant
    fn handle_set_state(&self, payload: &str);

    /// Check if this switch should handle_set_state for a specific topic.
    ///
    /// topic is a list of subtopics, split by "/". The base topic is already stripped out.
    fn should_handle_topic(&self, topic: &[&str]) -> bool;

    /// Get the homeassistant configuration for this switch
    fn get_homeassistant_config(&self) -> HomeAssistantConfig;
}
