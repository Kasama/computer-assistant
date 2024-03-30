mod computerassistant;
mod homeassistant;

use std::time::Duration;

use clap::Parser;
use dotenvy::dotenv;
use paho_mqtt as mqtt;

use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct App {
    #[arg(
        long,
        default_value = "mqtt://homeassistant.local:1883",
        env = "MQTT_HOST"
    )]
    hostname: String,

    #[arg(long, env = "MQTT_USERNAME")]
    username: String,

    #[arg(long, env = "MQTT_PASSWORD")]
    password: Secret<String>,

    #[arg(long, env = "MQTT_KEEPALIVE", default_value = "30")]
    keepalive: u64,

    #[arg(long, env = "MQTT_TOPIC_PREFIX", default_value = "computer-assistant")]
    topic_prefix: String,
}

const CLIENT_ID: &str = "computer-assistant";

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let _ = dotenv(); // don't care if it fails

    let app = App::parse();

    let create_opts = mqtt::CreateOptionsBuilder::new_v3()
        .server_uri(&app.hostname)
        .client_id(CLIENT_ID)
        .finalize();

    let mut cli = mqtt::AsyncClient::new(create_opts)?;

    let config_file = std::fs::File::open("config.yaml")?;
    println!("Opened config file");
    let mut cfg = computerassistant::Config::read_from(&config_file)?;
    println!("Read config file: {:?}", cfg);

    let stream = cli.get_stream(25);

    cfg.connect_mqtt(
        mqtt::ConnectOptionsBuilder::new_v3()
            .keep_alive_interval(Duration::from_secs(app.keepalive))
            .clean_session(false)
            .user_name(&app.username)
            .password(app.password.expose_secret()),
        &cli,
    )
    .await?;

    println!("Connected to {}", app.hostname);

    let (updateable_handler, publishable_handler) = cfg.register_mqtt(&cli).await?;
    println!("Registered with Home Assistant");

    let update_interval =
        std::time::Duration::from_secs(cfg.computer_assistant.status_pub_interval);
    let new_cli = cli.clone();
    let publishing_computer_assistant_cfg = cfg.computer_assistant.clone();
    let update_states_handle: JoinHandle<Result<(), anyhow::Error>> = tokio::spawn(async move {
        loop {
            publishable_handler
                .publish_state_mqtt(&publishing_computer_assistant_cfg, &new_cli)
                .await?;
            tokio::time::sleep(update_interval).await;
        }
    });

    updateable_handler
        .listen_mqtt(&cfg.computer_assistant, &cli, &stream)
        .await?;

    update_states_handle.abort();
    let _ = update_states_handle.await;

    println!("Disconnected");

    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
struct HomeAssistantDevice {
    ids: Vec<String>,
    #[serde(rename = "sw", alias = "sw_version")]
    version: String,
    #[serde(rename = "mf", alias = "manufacturer")]
    manufacturer: String,
    #[serde(rename = "cu", alias = "configuration_url")]
    configuration_url: String,
    #[serde(rename = "mdl", alias = "model")]
    model: String,
    name: String,
}

fn _default_on_state() -> String {
    homeassistant::State::On.to_string()
}

fn _default_off_state() -> String {
    homeassistant::State::Off.to_string()
}

fn _default_min_number() -> f64 {
    0.0
}

fn _default_max_number() -> f64 {
    100.0
}

fn _default_step_number() -> f64 {
    1.0
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged, rename_all = "snake_case")]
enum HomeAssistantConfig {
    Switch {
        #[serde(rename = "~", default)]
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
        value_template: Option<String>,
        #[serde(default = "_default_on_state")]
        state_on: String,
        #[serde(default = "_default_off_state")]
        state_off: String,
        #[serde(rename = "avty_t")]
        availability_topic: String,
    },
    BinarySensor {
        #[serde(rename = "~", default)]
        base_topic: String,
        #[serde(rename = "stat_t")]
        state_topic: String,
        #[serde(rename = "dev")]
        device: HomeAssistantDevice,
        name: String,
        #[serde(rename = "uniq_id")]
        unique_id: String,
        value_template: Option<String>,
        #[serde(default = "_default_on_state")]
        state_on: String,
        #[serde(default = "_default_off_state")]
        state_off: String,
        #[serde(rename = "avty_t")]
        availability_topic: String,
    },
    Number {
        #[serde(rename = "~", default)]
        base_topic: String,
        #[serde(rename = "cmd_t")]
        command_topic: String,
        command_template: Option<String>,
        #[serde(rename = "stat_t")]
        state_topic: String,
        #[serde(rename = "dev")]
        device: HomeAssistantDevice,
        name: String,
        #[serde(rename = "uniq_id")]
        unique_id: String,
        value_template: Option<String>,
        #[serde(rename = "avty_t")]
        availability_topic: String,
        #[serde(default = "_default_min_number")]
        min: f64,
        #[serde(default = "_default_max_number")]
        max: f64,
        #[serde(default = "_default_step_number")]
        step: f64,
        #[serde(default)]
        unit_of_measurement: String,
    },
}
