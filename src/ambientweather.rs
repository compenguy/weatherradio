use chrono::{Local, TimeZone};

use anyhow::Result;
use thiserror::Error;

use uom::si::{f32::ThermodynamicTemperature, thermodynamic_temperature};

#[derive(Error, Debug)]
pub(crate) enum MeasurementError {
    #[error("Record root not dictionary")]
    NotDictionary,
    #[error("Record missing timestamp")]
    MissingTimestamp,
    #[error("Failed while parsing record timestamp from record data")]
    TimestampFormat(#[from] chrono::format::ParseError),
    #[error("Record missing sensor id")]
    MissingSensorId,
}

// {"time" : "2021-08-15 16:13:12", "model" : "AmbientWeather-WH31E", "id" : 248, "channel" : 5, "battery_ok" : 1, "temperature_F" : 74.480, "humidity" : 54, "data" : "2200000000", "mic" : "CRC"}
pub(crate) fn try_parse(json: &serde_json::Value) -> Result<crate::radio::Record> {
    if let serde_json::Value::Object(m) = json {
        let timestamp: chrono::DateTime<chrono::Local> =
            if let Some(serde_json::Value::String(time)) = m.get("time") {
                let from = chrono::NaiveDateTime::parse_from_str(time, "%Y-%m-%d %H:%M:%S")?;
                Local
                    .from_local_datetime(&from)
                    .earliest()
                    .ok_or(anyhow::anyhow!("Invalid datetime string conversion"))?
            } else {
                return Err(MeasurementError::MissingTimestamp.into());
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
        let model = if let Some(serde_json::Value::String(model)) = m.get("model") {
            Some(model)
        } else {
            None
        };
        let sensor_id = match (model, device_id, channel) {
            (Some(model), _, Some(channel)) => format!("{}/{}", model, channel),
            (None, Some(id), Some(channel)) => format!("{}/{}", id, channel),
            (Some(model), Some(id), None) => format!("{}/{}", model, id),
            (None, None, Some(channel)) => format!("{}", channel),
            (None, Some(id), None) => format!("{}", id),
            (_, None, None) => return Err(MeasurementError::MissingSensorId.into()),
        };
        let mut measurements = Vec::new();
        if let Some(serde_json::Value::Number(b)) = m.get("battery_ok") {
            if let Some(ok) = b.as_u64().map(|b| b != 0) {
                measurements.push(crate::radio::Measurement::BatteryOk(ok));
            }
        }
        if let Some(serde_json::Value::Number(f)) = m.get("temperature_F") {
            if let Some(temp_f) = f.as_f64().map(|f| f as f32) {
                measurements.push(crate::radio::Measurement::Temperature(
                    ThermodynamicTemperature::new::<thermodynamic_temperature::degree_fahrenheit>(
                        temp_f,
                    ),
                ));
            }
        }
        if let Some(serde_json::Value::Number(c)) = m.get("temperature_C") {
            if let Some(temp_c) = c.as_f64().map(|c| c as f32) {
                measurements.push(crate::radio::Measurement::Temperature(
                    ThermodynamicTemperature::new::<thermodynamic_temperature::degree_celsius>(
                        temp_c,
                    ),
                ));
            }
        }
        if let Some(serde_json::Value::Number(h)) = m.get("humidity") {
            if let Some(hum) = h.as_u64().map(|h| h as u8) {
                measurements.push(crate::radio::Measurement::RelativeHumidity(hum));
            }
        }
        Ok(crate::radio::Record {
            timestamp,
            sensor_id,
            record_json: json.clone(),
            measurements,
        })
    } else {
        Err(MeasurementError::NotDictionary.into())
    }
}
