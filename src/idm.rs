use chrono::{Local, TimeZone};

use anyhow::Result;
use thiserror::Error;

use uom::si::{energy, f32::Energy};

#[derive(Error, Debug)]
pub(crate) enum MeasurementError {
    #[error("Record root not dictionary")]
    NotDictionary,
    #[error("Record missing timestamp")]
    MissingTimestamp,
    #[error("Failed while parsing record timestamp from json record data")]
    TimestampFormat(#[from] chrono::format::ParseError),
    #[error("Record missing sensor id")]
    MissingSensorId,
}

// {
//      "time" : "2021-08-24 19:56:51",
//      "protocol" : 161,
//      "model" : "NETIDM",
//      "PacketTypeID" : "0x1C",
//      "PacketLength" : 92,
//      "ApplicationVersion" : 2,
//      "ERTType" : 23,
//      "ERTSerialNumber" : 45027331,
//      "ConsumptionIntervalCount" : 46,
//      "ModuleProgrammingState" : 156,
//      "TamperCounters" : "0x0204030D0600",
//      "Unknown_field_1" : "0xD2630000000000",
//      "LastGenerationCount" : 84,
//      "Unknown_field_2" : "0x8B1F03",
//      "LastConsumptionCount" : 29425873,
//      "DifferentialConsumptionIntervals" : [533, 1122, 6224, 2693, 2216, 769, 12336, 1794, 8304, 3079, 224, 7180, 448, 12312, 897, 8232, 1537, 32, 514, 64, 3074, 128, 4104, 256, 8208, 769, 8224],
//      "TransmitTimeOffset" : 90,
//      "MeterIdCRC" : 13546,
//      "PacketCRC" : 20443,
//      "MeterType" : "Electric",
//      "mic" : "CRC"
// }
// {
//      "time" : "2021-08-24 19:56:52",
//      "protocol" : 160,
//      "model" : "IDM",
//      "PacketTypeID" : "0x1C",
//      "PacketLength" : 92,
//      "ApplicationVersion" : 2,
//      "ERTType" : 23,
//      "ERTSerialNumber" : 44991025,
//      "ConsumptionIntervalCount" : 116,
//      "ModuleProgrammingState" : 156,
//      "TamperCounters" :
//      "0x050803120100",
//      "AsynchronousCounters" : 43357,
//      "PowerOutageFlags" : "0x000000000000",
//      "LastConsumptionCount" : 4298559,
//      "DifferentialConsumptionIntervals" : [4, 3, 3, 7, 4, 4, 3, 4, 4, 7, 4, 7, 3, 4, 3, 5, 3, 4, 3, 4, 3, 6, 5, 4, 9, 17, 17, 22, 28, 24, 23, 34, 37, 40, 37, 6, 9, 15, 20, 18, 30, 34, 34, 34, 33, 37, 38],
//      "TransmitTimeOffset" : 2592,
//      "MeterIdCRC" : 27458,
//      "PacketCRC" : 42556,
//      "MeterType" : "Electric",
//      "mic" : "CRC"
// }
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
        let meter_type = if let Some(serde_json::Value::Number(meter_type)) = m.get("ERTType") {
            meter_type.as_u64().map(|meter_type| meter_type as u8)
        } else {
            None
        };
        let meter_id = if let Some(serde_json::Value::Number(meter_id)) = m.get("ERTSerialNumber") {
            meter_id.as_u64().map(|meter_id| meter_id as u32)
        } else {
            None
        };
        let sensor_id = match (meter_type, meter_id) {
            (Some(id), Some(channel)) => format!("{}/{}", id, channel),
            (None, Some(channel)) => format!("{}", channel),
            (Some(id), None) => format!("{}", id),
            (None, None) => return Err(MeasurementError::MissingSensorId.into()),
        };
        let mut measurements = Vec::new();
        if let Some(serde_json::Value::Number(b)) = m.get("LastConsumptionCount") {
            if let Some(cwh) = b.as_u64().map(|cwh| cwh as f32) {
                measurements.push(crate::radio::Measurement::TotalEnergyConsumption(
                    Energy::new::<energy::watt_hour>(cwh / 100.0),
                ));
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
