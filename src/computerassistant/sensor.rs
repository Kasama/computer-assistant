use serde::{Deserialize, Serialize};

use super::{DisplayPrecision, Name};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Sensor {
    pub name: Name,
    pub state_script: String,
    pub unit_of_measurement: String,
    #[serde(default)]
    pub suggested_display_precision: DisplayPrecision,
}
