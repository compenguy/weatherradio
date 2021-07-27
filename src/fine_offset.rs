// Fine Offset (original manufacturer)
// Ambient Weather (rebadge), Froggit (rebadge), EcoWitt (rebadge)
// https://www.wxforum.net/index.php?topic=40730.0
use std::convert::TryFrom;

use thiserror::Error;

const PREAMBLE: [u8; 3] = [0xaa, 0x2d, 0xd4];

fn bitstream_to_payload(bitstream: &[u8]) -> Option<&[u8]> {
    let (offset, _) = bitstream
        .windows(PREAMBLE.len())
        .enumerate()
        .find(|(_, seq)| PREAMBLE == *seq)?;
    let payload = &bitstream[offset..];
    if payload.is_empty() {
        None
    } else {
        Some(payload)
    }
}

fn payload_to_message(
    payload: &[u8],
    expected_id: u8,
    expected_len: usize,
) -> std::result::Result<&[u8], SensorError> {
    let _ = payload
        .get(0)
        .ok_or(SensorError::EmptyMessage)
        .and_then(|id| {
            if id == &expected_id {
                Ok(())
            } else {
                Err(SensorError::IncorrectMessageType(*id))
            }
        })?;
    if payload.len() < expected_len {
        return Err(SensorError::TruncatedMessage);
    }
    let crc_idx = expected_len - 2;
    let mut crc_8 = crc_any::CRCu8::crc8maxim();
    //payload[..=crc_idx].iter().map(|x| crc_8.digest(x));
    crc_8.digest(&payload[..=crc_idx]);
    if crc_8.get_crc() != 0 {
        return Err(SensorError::InvalidCrc);
    }
    let chksum_idx = expected_len - 1;
    if payload[..=crc_idx]
        .iter()
        .fold(0, |a: u8, &b| a.wrapping_add(b))
        - payload[chksum_idx]
        != 0
    {
        return Err(SensorError::InvalidChecksum);
    }
    Ok(&payload[..crc_idx])
}

#[derive(Error, Debug)]
pub(crate) enum SensorError {
    #[error("sensor message was empty")]
    EmptyMessage,
    #[error("sensor message was truncated")]
    TruncatedMessage,
    #[error("incorrect message type `{0}`")]
    IncorrectMessageType(u8),
    #[error("message integrity check failed crc")]
    InvalidCrc,
    #[error("message integrity check failed checksum")]
    InvalidChecksum,
}

#[derive(Clone, Debug)]
pub(crate) struct Measured {
    pub(crate) timestamp: chrono::DateTime<chrono::Utc>,
    pub(crate) device_id: Option<u16>,
    pub(crate) channel: Option<u8>,
    pub(crate) measurement: Measurement,
}

#[derive(Clone, Debug)]
pub(crate) enum Measurement {
    BatteryOk(bool),
    BatteryLevelRaw(u8),
    TemperatureC(f32),
    RelativeHumidity(u8),
    Clock(chrono::NaiveDateTime),
    RainfallMm(f32),
    Lux(u16),
    WindSpeed(u8),
    WindGust(u8),
    WindDirection(u8),
}

impl std::fmt::Display for Measurement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BatteryOk(b) => write!(f, "Battery Ok: {}", b),
            Self::BatteryLevelRaw(b) => write!(f, "Battery Level: {}", b),
            Self::TemperatureC(c) => write!(f, "Temperature: {} C", c),
            Self::RelativeHumidity(h) => write!(f, "Relative Humidity :{}%", h),
            Self::Clock(t) => write!(f, "Clock: {}", t),
            Self::RainfallMm(m) => write!(f, "Rainfall: {}mm", m),
            Self::Lux(l) => write!(f, "Lux: {}??", l),
            Self::WindSpeed(w) => write!(f, "Wind speed: {}??", w),
            Self::WindGust(w) => write!(f, "Wind gust: {}??", w),
            Self::WindDirection(w) => write!(f, "Wind direction: {}??", w),
        }
    }
}

pub(crate) trait ToMeasurements {
    fn to_measurements(&self) -> Result<Vec<Measured>, SensorError>;
}

impl ToMeasurements for &[u8] {
    fn to_measurements(&self) -> Result<Vec<Measured>, SensorError> {
        if let Some(payload) = bitstream_to_payload(self) {
            match payload[0] {
                Wh31::ID_E => Wh31::try_from(payload).and_then(|s| s.to_measurements()),
                Wh31::ID_B => Wh31::try_from(payload).and_then(|s| s.to_measurements()),
                WhRcc::ID => WhRcc::try_from(payload).and_then(|s| s.to_measurements()),
                Wh40::ID => Wh40::try_from(payload).and_then(|s| s.to_measurements()),
                Ws68::ID => Ws68::try_from(payload).and_then(|s| s.to_measurements()),
                x => Err(SensorError::IncorrectMessageType(x)),
            }
        } else {
            Ok(Vec::new())
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Wh31 {
    type_id: u8,
    dev_id: u8,
    battery_ok: bool,
    channel: u8,
    temp_c: f32,
    rel_hum: u8,
}

impl Wh31 {
    const ID_E: u8 = 0x30;
    const ID_B: u8 = 0x37;
    const MSGLEN: usize = 7; // 5 bytes data, 1 byte crc, 1 byte checksum
}

impl TryFrom<&[u8]> for Wh31 {
    type Error = SensorError;
    fn try_from(bytes: &[u8]) -> std::result::Result<Self, Self::Error> {
        let message = payload_to_message(bytes, Self::ID_E, Self::MSGLEN)
            .or_else(|_| payload_to_message(bytes, Self::ID_B, Self::MSGLEN))?;
        let type_id = message[0];
        let dev_id = message[1];
        let battery_ok = ((message[2] & 0x80) >> 7) != 0;
        let channel = ((message[2] & 0x70) >> 4) + 1;
        let temp_raw = ((((message[2] & 0x0F) as u16) << 8) | message[3] as u16) as f32;
        let temp_c = (temp_raw * 0.1) - 40.0;
        let rel_hum = message[4];
        Ok(Self {
            type_id,
            dev_id,
            battery_ok,
            channel,
            temp_c,
            rel_hum,
        })
    }
}

impl ToMeasurements for Wh31 {
    fn to_measurements(&self) -> Result<Vec<Measured>, SensorError> {
        let mut measurements: Vec<Measured> = Vec::with_capacity(3);
        let mut tmp: Measured = Measured {
            timestamp: chrono::Utc::now(),
            device_id: Some(self.dev_id as u16),
            channel: Some(self.channel),
            measurement: Measurement::BatteryOk(self.battery_ok),
        };
        measurements.push(tmp.clone());
        tmp.measurement = Measurement::TemperatureC(self.temp_c);
        measurements.push(tmp.clone());
        tmp.measurement = Measurement::RelativeHumidity(self.rel_hum);
        measurements.push(tmp);
        Ok(measurements)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct WhRcc {
    type_id: u8,
    dev_id: u8,
    timestamp: chrono::NaiveDateTime,
}

impl WhRcc {
    const ID: u8 = 0x52;
    const MSGLEN: usize = 11; // 9 bytes data, 1 byte crc, 1 byte checksum
}

impl TryFrom<&[u8]> for WhRcc {
    type Error = SensorError;
    fn try_from(bytes: &[u8]) -> std::result::Result<Self, Self::Error> {
        let message = payload_to_message(bytes, Self::ID, Self::MSGLEN)?;
        let type_id = message[0];
        let dev_id = message[1];
        let yy: i32 = ((message[3] & 0xF0) >> 4) as i32 * 10 + (message[3] & 0x0F) as i32 + 2000;
        let mo: u32 = ((message[4] & 0x10) >> 4) as u32 * 10 + (message[4] & 0x0F) as u32;
        let dd: u32 = ((message[5] & 0x30) >> 4) as u32 * 10 + (message[5] & 0x0F) as u32;
        let hh: u32 = ((message[6] & 0x30) >> 4) as u32 * 10 + (message[6] & 0x0F) as u32;
        let mm: u32 = ((message[7] & 0x70) >> 4) as u32 * 10 + (message[7] & 0x0F) as u32;
        let ss: u32 = ((message[8] & 0x70) >> 4) as u32 * 10 + (message[8] & 0x0F) as u32;
        let timestamp = chrono::NaiveDate::from_ymd(yy, mo, dd).and_hms(hh, mm, ss);

        Ok(Self {
            type_id,
            dev_id,
            timestamp,
        })
    }
}

impl ToMeasurements for WhRcc {
    fn to_measurements(&self) -> Result<Vec<Measured>, SensorError> {
        let measurements: Vec<Measured> = vec![Measured {
            timestamp: chrono::Utc::now(),
            device_id: Some(self.dev_id as u16),
            channel: None,
            measurement: Measurement::Clock(self.timestamp),
        }];
        Ok(measurements)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Wh40 {
    type_id: u8,
    dev_id: u16,
    channel: u8,
    battery_ok: bool,
    rain_mm: f32,
}

impl Wh40 {
    const ID: u8 = 0x40;
    const MSGLEN: usize = 9; // 7 bytes data, 1 byte crc, 1 byte checksum
}

impl TryFrom<&[u8]> for Wh40 {
    type Error = SensorError;
    fn try_from(bytes: &[u8]) -> std::result::Result<Self, Self::Error> {
        let message = payload_to_message(bytes, Self::ID, Self::MSGLEN)?;
        let type_id = message[0];
        let dev_id = ((message[2] as u16) << 8) | message[3] as u16;
        let battery_ok = ((message[4] & 0x80) >> 7) != 0;
        let channel = ((message[4] & 0x70) >> 4) + 1;
        let rain_raw: f32 = (((message[5] as u16) << 8) | message[6] as u16) as f32;
        let rain_mm = rain_raw * 0.1;

        Ok(Self {
            type_id,
            dev_id,
            channel,
            battery_ok,
            rain_mm,
        })
    }
}

impl ToMeasurements for Wh40 {
    fn to_measurements(&self) -> Result<Vec<Measured>, SensorError> {
        let mut measurements: Vec<Measured> = Vec::with_capacity(2);
        let mut tmp: Measured = Measured {
            timestamp: chrono::Utc::now(),
            device_id: Some(self.dev_id),
            channel: Some(self.channel),
            measurement: Measurement::BatteryOk(self.battery_ok),
        };
        measurements.push(tmp.clone());
        tmp.measurement = Measurement::RainfallMm(self.rain_mm);
        measurements.push(tmp);
        Ok(measurements)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Ws68 {
    type_id: u8,
    dev_id: u16,
    lux: u16,
    battery: u8,
    w_speed: u8,
    w_gust: u8,
    w_dir: u8,
}

impl Ws68 {
    const ID: u8 = 0x68;
    const MSGLEN: usize = 16; // 14 bytes data, 1 byte crc, 1 byte checksum
}

impl TryFrom<&[u8]> for Ws68 {
    type Error = SensorError;
    fn try_from(bytes: &[u8]) -> std::result::Result<Self, Self::Error> {
        let message = payload_to_message(bytes, Self::ID, Self::MSGLEN)?;
        let type_id = message[0];
        let dev_id = ((message[2] as u16) << 8) | message[3] as u16;
        let lux = ((message[4] as u16) << 8) | message[5] as u16;
        let battery = message[6];
        let w_speed = message[10];
        let w_gust = message[12];
        let w_dir = ((message[7] & 0x20) >> 5) | message[11];

        Ok(Self {
            type_id,
            dev_id,
            lux,
            battery,
            w_speed,
            w_gust,
            w_dir,
        })
    }
}

impl ToMeasurements for Ws68 {
    fn to_measurements(&self) -> Result<Vec<Measured>, SensorError> {
        let mut measurements: Vec<Measured> = Vec::with_capacity(6);
        let mut tmp: Measured = Measured {
            timestamp: chrono::Utc::now(),
            device_id: Some(self.dev_id),
            channel: None,
            measurement: Measurement::BatteryLevelRaw(self.battery),
        };
        measurements.push(tmp.clone());
        tmp.measurement = Measurement::BatteryOk(self.battery > 0x30); // Just a guess
        measurements.push(tmp.clone());
        tmp.measurement = Measurement::Lux(self.lux);
        measurements.push(tmp.clone());
        tmp.measurement = Measurement::WindSpeed(self.w_speed);
        measurements.push(tmp.clone());
        tmp.measurement = Measurement::WindGust(self.w_gust);
        measurements.push(tmp.clone());
        tmp.measurement = Measurement::WindDirection(self.w_dir);
        measurements.push(tmp);
        Ok(measurements)
    }
}
