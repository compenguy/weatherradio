use chrono::TimeZone;
use std::convert::TryFrom;

use thiserror::Error;
use uom::fmt::DisplayStyle::Abbreviation;
use uom::si::{f32::ThermodynamicTemperature, thermodynamic_temperature};
//use uom::si::{f32::Length, length};
//use uom::si::{u16::Angle, angle};
//use uom::si::{u16::Velocity, velocity};

#[derive(Error, Debug)]
pub(crate) enum MeasurementError {
    #[error("Json record root not dictionary")]
    JsonNotDictionary,
    #[error("Json record missing timestamp")]
    JsonMissingTimestamp,
    #[error("Failed while parsing record timestamp from json record data")]
    JsonTimestampFormat(#[from] chrono::format::ParseError),
}

#[derive(Clone, Debug)]
pub(crate) struct Record {
    pub(crate) timestamp: chrono::DateTime<chrono::Local>,
    pub(crate) device_id: Option<u16>,
    pub(crate) channel: Option<u8>,
    pub(crate) measurements: Vec<Measurement>,
}

impl std::fmt::Display for Record {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for measurement in &self.measurements {
            write!(
                f,
                "[{}][{:04x}/{:02x}] {}",
                self.timestamp,
                self.device_id.unwrap_or(0x0000),
                self.channel.unwrap_or(0x00),
                measurement
            )?;
        }
        write!(f, "")
    }
}

// {"time" : "2021-08-15 16:13:12", "model" : "AmbientWeather-WH31E", "id" : 248, "channel" : 5, "battery_ok" : 1, "temperature_F" : 74.480, "humidity" : 54, "data" : "2200000000", "mic" : "CRC"}
impl TryFrom<serde_json::Value> for Record {
    type Error = MeasurementError;
    fn try_from(json: serde_json::Value) -> std::result::Result<Self, Self::Error> {
        if let serde_json::Value::Object(m) = json {
            let timestamp: chrono::DateTime<chrono::Local> =
                if let Some(serde_json::Value::String(time)) = m.get("time") {
                    //let naive_time = chrono::NaiveDateTime::parse_from_str(time, "%Y-%m-%d %H:%M:%S").map_err(|e| e.into())?;
                    //naive_time.into()
                    chrono::Local
                        .datetime_from_str(time, "%Y-%m-%d %H:%M:%S")
                        .map_err(MeasurementError::from)?
                } else {
                    return Err(MeasurementError::JsonMissingTimestamp);
                };
            let device_id = if let Some(serde_json::Value::Number(id)) = m.get("id") {
                id.as_u64().map(|id| id as u16)
            } else {
                None
            };
            let channel = if let Some(serde_json::Value::Number(channel)) = m.get("channel") {
                channel.as_u64().map(|ch| ch as u8)
            } else {
                None
            };
            let mut measurements = Vec::new();
            if let Some(serde_json::Value::Number(b)) = m.get("battery_ok") {
                if let Some(ok) = b.as_u64().map(|b| b != 0) {
                    measurements.push(Measurement::BatteryOk(ok));
                }
            }
            if let Some(serde_json::Value::Number(f)) = m.get("temperature_F") {
                if let Some(temp_f) = f.as_f64().map(|f| f as f32) {
                    measurements.push(Measurement::Temperature(ThermodynamicTemperature::new::<
                        thermodynamic_temperature::degree_fahrenheit,
                    >(temp_f)));
                }
            }
            if let Some(serde_json::Value::Number(c)) = m.get("temperature_C") {
                if let Some(temp_c) = c.as_f64().map(|c| c as f32) {
                    measurements.push(Measurement::Temperature(ThermodynamicTemperature::new::<
                        thermodynamic_temperature::degree_celsius,
                    >(temp_c)));
                }
            }
            if let Some(serde_json::Value::Number(h)) = m.get("humidity") {
                if let Some(hum) = h.as_u64().map(|h| h as u8) {
                    measurements.push(Measurement::RelativeHumidity(hum));
                }
            }
            Ok(Record {
                timestamp,
                device_id,
                channel,
                measurements,
            })
        } else {
            Err(MeasurementError::JsonNotDictionary)
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Measurement {
    BatteryOk(bool),
    Temperature(ThermodynamicTemperature),
    RelativeHumidity(u8),
    /*
    BatteryLevelRaw(u8),
    Clock(chrono::Utc),
    Rainfall(Length),
    Lux(u16),
    WindSpeed(Velocity),
    WindGust(Velocity),
    WindDirection(Angle),
    None,
    */
}

impl Measurement {
    pub(crate) fn name(&self) -> String {
        let text = match self {
            Self::BatteryOk(_) => "BatteryOk",
            Self::Temperature(_) => "TemperatureF",
            Self::RelativeHumidity(_) => "Humidity",
            /*
            Self::BatteryLevelRaw(b) => "BatteryLevel",
            Self::Clock(t) => "Clock",
            Self::Rainfall(m) => "Rainfall",
            Self::Lux(l) => "Lux",
            Self::WindSpeed(w) => "WindSpeed",
            Self::WindGust(w) => "WindGust",
            Self::WindDirection(w) => "WindDirection",
            Self::None => "None",
            */
        };

        text.to_owned()
    }

    pub(crate) fn value(&self) -> String {
        match self {
            Self::BatteryOk(b) => b.to_string(),
            Self::Temperature(t) => format!(
                "{:.1}",
                t.into_format_args(thermodynamic_temperature::degree_fahrenheit, Abbreviation)
            ),
            Self::RelativeHumidity(h) => format!("{}%", h),
            /*
            Self::BatteryLevelRaw(b) => b.to_string(),
            Self::Clock(t) => t.to_string(),
            Self::Rainfall(m) => m.into_format_args(length::millimeter, Abbreviation).to_string(),
            Self::Lux(l) => l.to_string(),
            Self::WindSpeed(w) => w.into_format_args(velocity::kilometer_per_hour, Abbreviation).to_string(),
            Self::WindGust(w) => w.into_format_args(velocity::kilometer_per_hour, Abbreviation).to_string(),
            Self::WindDirection(w) => w.into_format_args(angle::degree, Abbreviation).to_string(),
            Self::None => String::new(),
            */
        }
    }
}

impl std::fmt::Display for Measurement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name(), self.value())
    }
}
