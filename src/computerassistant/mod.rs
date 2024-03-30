mod switch;
mod binary_sensor;
mod sensor;
mod number;

use std::ops::{Deref, DerefMut};
use std::process::Command;

use mqtt::{AsyncClient, AsyncReceiver, Message, MessageBuilder};
use paho_mqtt as mqtt;

use serde::{Deserialize, Serialize};

use crate::homeassistant::State;
use crate::HomeAssistantDevice;

use self::binary_sensor::BinarySensor;
use self::sensor::Sensor;
use self::switch::Switch;
use self::number::Number;

pub trait Updateable {
    fn update(&self, topic: &[&str], state: &str) -> anyhow::Result<()>;
}

pub trait Publishable {
    fn state_script(&self) -> &str;
    fn state_topic(&self, config: &ComputerAssistantConfig) -> String;
    fn state_payload(&self, _config: &ComputerAssistantConfig) -> anyhow::Result<String> {
        let script = Command::new("bash")
            .arg("-c")
            .arg(self.state_script())
            .spawn()?
            .wait()?;

        Ok(if script.success() {
            State::On
        } else {
            State::Off
        }
        .to_string())
    }
    fn publish_state(&self, config: &ComputerAssistantConfig) -> anyhow::Result<mqtt::Message> {
        Ok(MessageBuilder::new()
            .topic(self.state_topic(config))
            .payload(self.state_payload(config)?)
            .qos(mqtt::QOS_1)
            .finalize())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DisplayPrecision(u8);

impl Default for DisplayPrecision {
    fn default() -> Self {
        Self(2)
    }
}

impl From<u8> for DisplayPrecision {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl From<DisplayPrecision> for u8 {
    fn from(value: DisplayPrecision) -> Self {
        value.0
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Name(String);

impl Deref for Name {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Name {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Name {
    pub fn as_id(&self) -> String {
        self.0.to_lowercase().replace(' ', "_")
    }
}

fn _default_homeassistant_topic() -> String {
    "homeassistant".to_string()
}

fn _default_availability_topic() -> String {
    "status".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComputerAssistantConfig {
    pub base_topic: String,
    pub device: HomeAssistantDevice,
    pub name: Name,
    pub unique_id: String,
    pub status_pub_interval: u64,
    #[serde(default = "_default_homeassistant_topic")]
    pub homeassistant_topic: String,
    #[serde(default = "_default_availability_topic")]
    pub availability_topic: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub computer_assistant: ComputerAssistantConfig,
    #[serde(default)]
    pub binary_sensor: Vec<BinarySensor>,
    #[serde(default)]
    pub sensor: Vec<Sensor>,
    #[serde(default)]
    pub switch: Vec<Switch>,
    #[serde(default)]
    pub number: Vec<Number>,
}

pub struct UpdateableHandlers(Vec<Box<dyn Updateable + Send + Sync>>);

impl UpdateableHandlers {
    pub async fn listen_mqtt(
        &self,
        config: &ComputerAssistantConfig,
        client: &AsyncClient,
        stream: &AsyncReceiver<Option<Message>>,
    ) -> anyhow::Result<()> {
        let base_command_topic = format!("{}/entities/cmd", config.base_topic);
        client
            .subscribe(format!("{}/#", base_command_topic), mqtt::QOS_1)
            .await?;

        while let Some(message) = stream.recv().await? {
            let topic = message.topic().to_string();
            if !topic.starts_with(&base_command_topic) {
                // ignore messages for other topics
                continue;
            }
            let subtopics = topic
                .trim_start_matches(base_command_topic.as_str())
                .trim_matches('/')
                .split('/')
                .collect::<Vec<_>>();
            let state = std::str::from_utf8(message.payload())?;

            for handler in &self.0 {
                handler.update(&subtopics, state)?;
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await
        }
        Ok(())
    }
}

pub struct PublishableHandlers(Vec<Box<dyn Publishable + Send + Sync>>);

impl PublishableHandlers {
    pub async fn publish_state_mqtt(
        &self,
        config: &ComputerAssistantConfig,
        client: &AsyncClient,
    ) -> anyhow::Result<()> {
        for handler in &self.0 {
            let state_msg = handler.publish_state(config)?;
            client.publish(state_msg).await?;
        }
        Ok(())
    }
}

impl Config {
    pub async fn connect_mqtt(
        &self,
        conn_opts: &mut mqtt::ConnectOptionsBuilder,
        client: &AsyncClient,
    ) -> anyhow::Result<()> {
        let status_topic = format!(
            "{}/{}",
            self.computer_assistant.base_topic, self.computer_assistant.availability_topic
        );
        client
            .connect(
                conn_opts
                    .will_message(
                        MessageBuilder::new()
                            .topic(&status_topic)
                            .payload("offline")
                            .finalize(),
                    )
                    .finalize(),
            )
            .await?;

        client
            .publish(
                MessageBuilder::new()
                    .topic(status_topic)
                    .payload("online")
                    .finalize(),
            )
            .await?;

        Ok(())
    }

    pub fn read_from<R: std::io::Read>(reader: R) -> anyhow::Result<Self> {
        let config: Self = serde_yaml::from_reader(reader)?;
        Ok(config)
    }

    pub async fn register_mqtt(
        &mut self,
        client: &AsyncClient,
    ) -> anyhow::Result<(UpdateableHandlers, PublishableHandlers)> {
        let mut updateable_handlers: UpdateableHandlers = UpdateableHandlers(vec![]);
        let mut publishable_handlers: PublishableHandlers = PublishableHandlers(vec![]);

        for switch in &self.switch {
            let msg = MessageBuilder::new()
                .topic(format!(
                    "{}/switch/{}/{}/config", // register in homeassistant's topic
                    self.computer_assistant.homeassistant_topic,
                    self.computer_assistant.base_topic,
                    switch.name.as_id()
                ))
                .payload(serde_json::to_vec(
                    &switch.ha_config(&self.computer_assistant),
                )?)
                .qos(mqtt::QOS_2)
                .retained(true)
                .finalize();
            client.publish(msg).await?;

            let new_switch_updateable = Box::new(switch.clone());
            let new_switch_publishable = Box::new(switch.clone());
            updateable_handlers.0.push(new_switch_updateable);
            publishable_handlers.0.push(new_switch_publishable);
        }

        for binary_sensor in &self.binary_sensor {
            let msg = MessageBuilder::new()
                .topic(format!(
                    "{}/binary_sensor/{}/{}/config", // register in homeassistant's topic
                    self.computer_assistant.homeassistant_topic,
                    self.computer_assistant.base_topic,
                    binary_sensor.name.as_id()
                ))
                .payload(serde_json::to_vec(
                    &binary_sensor.ha_config(&self.computer_assistant),
                )?)
                .qos(mqtt::QOS_2)
                .retained(true)
                .finalize();
            client.publish(msg).await?;

            let new_binary_sensor = Box::new(binary_sensor.clone());
            publishable_handlers.0.push(new_binary_sensor);
        }

        for number in &self.number {
            let msg = MessageBuilder::new()
                .topic(format!(
                    "{}/number/{}/{}/config", // register in homeassistant's topic
                    self.computer_assistant.homeassistant_topic,
                    self.computer_assistant.base_topic,
                    number.name.as_id()
                ))
                .payload(serde_json::to_vec(
                    &number.ha_config(&self.computer_assistant),
                )?)
                .qos(mqtt::QOS_2)
                .retained(true)
                .finalize();
            client.publish(msg).await?;

            let new_publishable_number = Box::new(number.clone());
            publishable_handlers.0.push(new_publishable_number);
            let new_updateable_number = Box::new(number.clone());
            updateable_handlers.0.push(new_updateable_number);
        }

        Ok((updateable_handlers, publishable_handlers))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_read_config() {
        let file = std::fs::File::open("config.yaml").expect("Failed to open config file");
        let _config = Config::read_from(file).expect("Failed to read config file");
    }
}
