// Fine Offset (original manufacturer)
// Ambient Weather (rebadge), Froggit (rebadge), EcoWitt (rebadge)
// https://www.wxforum.net/index.php?topic=40730.0
use std::convert::TryFrom;

use thiserror::Error;
use uom::fmt::DisplayStyle::Abbreviation;
use uom::si::{angle, length, thermodynamic_temperature, velocity};
use uom::si::{f32::Length, f32::ThermodynamicTemperature};
use uom::si::{u16::Angle, u16::Velocity};

pub(crate) const PREAMBLE: [u8; 3] = [0xaa, 0x2d, 0xd4];

pub(crate) const WH31_E_SAMPLES: [[u8; 11]; 11] = [
    [
        0x30, 0xc3, 0x82, 0x0a, 0x5e, 0xdf, 0xbc, 0x07, 0x56, 0xa7, 0xae,
    ],
    [
        0x30, 0x44, 0x92, 0x1a, 0x39, 0x5a, 0xb3, 0x07, 0x45, 0x04, 0x5f,
    ],
    [
        0x30, 0xc3, 0x81, 0xd5, 0x5c, 0x2a, 0xcf, 0x08, 0x35, 0x44, 0x2c,
    ],
    [
        0x30, 0x35, 0xc2, 0x2f, 0x3c, 0x0f, 0xa1, 0x07, 0x52, 0x29, 0x9f,
    ],
    [
        0x30, 0x35, 0xc2, 0x2e, 0x3c, 0xfb, 0x8c, 0x07, 0x52, 0x29, 0x9f,
    ],
    [
        0x30, 0xc9, 0xa2, 0x1e, 0x40, 0x0c, 0x05, 0x07, 0x34, 0xc6, 0xb1,
    ],
    [
        0x30, 0x2b, 0xb2, 0x14, 0x3d, 0x94, 0xf2, 0x08, 0x53, 0x78, 0xe6,
    ],
    [
        0x30, 0xc9, 0xa2, 0x1f, 0x40, 0xf8, 0xf2, 0x07, 0x34, 0xc6, 0xb1,
    ],
    [
        0x30, 0x44, 0x92, 0x13, 0x3e, 0x0e, 0x65, 0x07, 0x45, 0x04, 0x5f,
    ],
    [
        0x30, 0x44, 0x92, 0x15, 0x3d, 0x07, 0x5f, 0x07, 0x45, 0x04, 0x5f,
    ],
    [
        0x30, 0xc3, 0x81, 0xd6, 0x5b, 0x90, 0x35, 0x08, 0x35, 0x44, 0x2c,
    ],
];

pub(crate) const WH31_E_RCC_SAMPLES: [[u8; 11]; 14] = [
    [
        0x52, 0x27, 0x4a, 0x20, 0x10, 0x20, 0x02, 0x06, 0x55, 0x05, 0x75,
    ],
    [
        0x52, 0x27, 0x4a, 0x20, 0x10, 0x20, 0x02, 0x08, 0x02, 0x81, 0xa0,
    ],
    [
        0x52, 0x75, 0x4a, 0x20, 0x10, 0x20, 0x07, 0x35, 0x03, 0x8a, 0x2a,
    ],
    [
        0x52, 0x58, 0x4a, 0x20, 0x10, 0x20, 0x07, 0x35, 0x51, 0x48, 0x19,
    ],
    [
        0x52, 0x75, 0x4a, 0x20, 0x10, 0x20, 0x07, 0x36, 0x05, 0x01, 0xa4,
    ],
    [
        0x52, 0x58, 0x4a, 0x20, 0x10, 0x20, 0x07, 0x36, 0x54, 0x90, 0x65,
    ],
    [
        0x52, 0x75, 0x4a, 0x20, 0x10, 0x20, 0x07, 0x37, 0x07, 0x97, 0x3d,
    ],
    [
        0x52, 0x58, 0x4a, 0x20, 0x10, 0x20, 0x07, 0x37, 0x57, 0x37, 0x10,
    ],
    [
        0x52, 0x75, 0x4a, 0x20, 0x10, 0x20, 0x07, 0x38, 0x09, 0x11, 0xba,
    ],
    [
        0x52, 0x58, 0x4a, 0x20, 0x10, 0x20, 0x07, 0x39, 0x00, 0xb3, 0x37,
    ],
    [
        0x52, 0xa0, 0x4a, 0x20, 0x10, 0x20, 0x08, 0x05, 0x50, 0x0f, 0xf8,
    ],
    [
        0x52, 0xa0, 0x4a, 0x20, 0x10, 0x20, 0x08, 0x06, 0x58, 0x9b, 0x8d,
    ],
    [
        0x52, 0xa0, 0x4a, 0x20, 0x10, 0x20, 0x08, 0x08, 0x06, 0x97, 0x39,
    ],
    [
        0x52, 0xa0, 0x4a, 0x20, 0x10, 0x20, 0x08, 0x09, 0x14, 0x42, 0xf3,
    ],
];

pub(crate) const WH40_SAMPLES: [[u8; 7]; 11] = [
    [0x40, 0x00, 0xcd, 0x6f, 0x10, 0x00, 0x00],
    [0x40, 0x00, 0xcd, 0x6f, 0x10, 0x00, 0x01],
    [0x40, 0x00, 0xcd, 0x6f, 0x10, 0x00, 0x02],
    [0x40, 0x00, 0xcd, 0x6f, 0x10, 0x00, 0x03],
    [0x40, 0x00, 0xcd, 0x6f, 0x10, 0x00, 0x04],
    [0x40, 0x00, 0xcd, 0x6f, 0x10, 0x00, 0x05],
    [0x40, 0x00, 0xcd, 0x6f, 0x10, 0x00, 0x06],
    [0x40, 0x00, 0xcd, 0x6f, 0x10, 0x00, 0x07],
    [0x40, 0x00, 0xcd, 0x6f, 0x10, 0x00, 0x08],
    [0x40, 0x00, 0xcd, 0x6f, 0x10, 0x00, 0x09],
    [0x40, 0x00, 0xcd, 0x6f, 0x10, 0x00, 0x0a],
];

pub(crate) const WS68_SAMPLES: [[u8; 13]; 6] = [
    [
        0x68, 0x00, 0x00, 0xc5, 0x00, 0x00, 0x4b, 0x0f, 0xff, 0xff, 0x00, 0x5a, 0x00,
    ],
    [
        0x68, 0x00, 0x00, 0xc5, 0x00, 0x00, 0x4b, 0x0f, 0xff, 0xff, 0x00, 0xb4, 0x00,
    ],
    [
        0x68, 0x00, 0x00, 0xc5, 0x00, 0x00, 0x4b, 0x0f, 0xff, 0xff, 0x7e, 0xe0, 0x94,
    ],
    [
        0x68, 0x00, 0x00, 0xc5, 0x00, 0x00, 0x4b, 0x2f, 0xff, 0xff, 0x00, 0x0e, 0x00,
    ],
    [
        0x68, 0x00, 0x00, 0xc5, 0x00, 0x0f, 0x4b, 0x0f, 0xff, 0xff, 0x00, 0x2e, 0x00,
    ],
    [
        0x68, 0x00, 0x00, 0xc5, 0x01, 0x07, 0x4b, 0x0f, 0xff, 0xff, 0x00, 0x2e, 0x00,
    ],
];

fn bitstream_to_payload(bitstream: &[u8]) -> Option<&[u8]> {
    let (offset, _) = bitstream
        .windows(PREAMBLE.len())
        .enumerate()
        .find(|(_, seq)| PREAMBLE == *seq)?;
    //log::debug!("Offset of payload into bitstream: {}", offset);
    if let Some(p) = bitstream.get(offset + PREAMBLE.len()..) {
        if p.is_empty() {
            None
        } else {
            //log::debug!("Payload: {:02X?}", p);
            Some(p)
        }
    } else {
        None
    }
}

fn payload_to_message(
    payload: &[u8],
    expected_id: u8,
    expected_len: usize,
    strict: bool,
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
    if strict {
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

impl std::fmt::Display for Measured {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}][{:04x}/{:02x}] {}",
            self.timestamp,
            self.device_id.unwrap_or(0x0000),
            self.channel.unwrap_or(0x00),
            self.measurement
        )
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Measurement {
    BatteryOk(bool),
    BatteryLevelRaw(u8),
    Temperature(ThermodynamicTemperature),
    RelativeHumidity(u8),
    Clock(chrono::NaiveDateTime),
    Rainfall(Length),
    Lux(u16),
    WindSpeed(Velocity),
    WindGust(Velocity),
    WindDirection(Angle),
}

impl Measurement {
    pub(crate) fn name_token(&self) -> &'static str {
        match self {
            Self::BatteryOk(_) => "batteryok",
            Self::BatteryLevelRaw(_) => "batterylevel",
            Self::Temperature(_) => "temperature",
            Self::RelativeHumidity(_) => "humidity",
            Self::Clock(_) => "time",
            Self::Rainfall(_) => "rainfall",
            Self::Lux(_) => "lux",
            Self::WindSpeed(_) => "windspeed",
            Self::WindGust(_) => "windgust",
            Self::WindDirection(_) => "winddirection",
        }
    }

    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::BatteryOk(_) => "Battery Ok",
            Self::BatteryLevelRaw(_) => "Battery Level",
            Self::Temperature(_) => "Temperature",
            Self::RelativeHumidity(_) => "Relative Humidity",
            Self::Clock(_) => "Clock",
            Self::Rainfall(_) => "Rainfall",
            Self::Lux(_) => "Lux",
            Self::WindSpeed(_) => "Wind Speed",
            Self::WindGust(_) => "Wind Gust",
            Self::WindDirection(_) => "Wind Direction",
        }
    }

    pub(crate) fn value(&self) -> String {
        match self {
            Self::BatteryOk(b) => b.to_string(),
            Self::BatteryLevelRaw(b) => b.to_string(),
            Self::Temperature(c) => format!(
                "{:.1}",
                c.into_format_args(thermodynamic_temperature::degree_celsius, Abbreviation)
            ),
            Self::RelativeHumidity(h) => format!("{}%", h),
            Self::Clock(t) => t.to_string(),
            Self::Rainfall(m) => format!(
                "{:.1}",
                m.into_format_args(length::millimeter, Abbreviation)
            ),
            Self::Lux(l) => l.to_string(),
            Self::WindSpeed(w) => w
                .into_format_args(velocity::kilometer_per_hour, Abbreviation)
                .to_string(),
            Self::WindGust(w) => w
                .into_format_args(velocity::kilometer_per_hour, Abbreviation)
                .to_string(),
            Self::WindDirection(w) => w.into_format_args(angle::degree, Abbreviation).to_string(),
        }
    }
}

impl std::fmt::Display for Measurement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name(), self.value())
    }
}

pub(crate) trait ToMeasurements {
    fn to_measurements(&self) -> Result<Vec<Measured>, SensorError>;
}

impl ToMeasurements for &[u8] {
    fn to_measurements(&self) -> Result<Vec<Measured>, SensorError> {
        if let Some(payload) = bitstream_to_payload(self) {
            //log::debug!("Converting payload {:02X?} into measurements...", payload);
            //log::debug!("Measurement device type: {:02X?}", payload[0]);
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
        let message = if bytes[0] == Self::ID_E {
            payload_to_message(bytes, Self::ID_E, Self::MSGLEN, false)?
        } else {
            payload_to_message(bytes, Self::ID_B, Self::MSGLEN, false)?
        };
        let type_id = message[0];
        let dev_id = message[1];
        let battery_ok = ((message[2] & 0x80) >> 7) != 0;
        let channel = ((message[2] & 0x70) >> 4) + 1;
        let temp_raw = (((message[2] & 0x0F) as u16) << 8) | message[3] as u16;
        //log::debug!("Decoded raw temperature: {}", temp_raw);
        let temp_c = (temp_raw as f32 * 0.1) - 40.0;
        //log::debug!("Decoded celsius temperature: {}", temp_c);
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
        tmp.measurement = Measurement::Temperature(ThermodynamicTemperature::new::<
            thermodynamic_temperature::degree_celsius,
        >(self.temp_c));
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
        let message = payload_to_message(bytes, Self::ID, Self::MSGLEN, false)?;
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
        let message = payload_to_message(bytes, Self::ID, Self::MSGLEN, false)?;
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
        tmp.measurement = Measurement::Rainfall(Length::new::<length::millimeter>(self.rain_mm));
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
    w_dir: u16,
}

impl Ws68 {
    const ID: u8 = 0x68;
    const MSGLEN: usize = 16; // 14 bytes data, 1 byte crc, 1 byte checksum
}

impl TryFrom<&[u8]> for Ws68 {
    type Error = SensorError;
    fn try_from(bytes: &[u8]) -> std::result::Result<Self, Self::Error> {
        let message = payload_to_message(bytes, Self::ID, Self::MSGLEN, false)?;
        let type_id = message[0];
        let dev_id = ((message[2] as u16) << 8) | message[3] as u16;
        let lux = ((message[4] as u16) << 8) | message[5] as u16;
        let battery = message[6];
        let w_speed = message[10];
        let w_gust = message[12];
        let w_dir = (((message[7] & 0x20) as u16) << 3) | (message[11] as u16);

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
        tmp.measurement = Measurement::WindSpeed(Velocity::new::<velocity::kilometer_per_hour>(
            self.w_speed as u16,
        ));
        measurements.push(tmp.clone());
        tmp.measurement = Measurement::WindGust(Velocity::new::<velocity::kilometer_per_hour>(
            self.w_gust as u16,
        ));
        measurements.push(tmp.clone());
        tmp.measurement = Measurement::WindDirection(Angle::new::<angle::degree>(self.w_dir));
        measurements.push(tmp);
        Ok(measurements)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_wh31_e() {
        // Test increasing amounts of sync values at the beginning of the payload
        for (i, row) in WH31_E_SAMPLES.iter().enumerate() {
            let mut row_vec: Vec<u8> = vec![0xaa; i];
            row_vec.extend_from_slice(&PREAMBLE);
            row_vec.extend_from_slice(row);
            row_vec.extend_from_slice(&[0x00; 4]);

            let payload =
                bitstream_to_payload(&row_vec).expect("Found no payload in the bitstream");
            assert_eq!(&payload[0..payload.len() - 4], row);

            let measurements = row_vec
                .as_slice()
                .to_measurements()
                .expect("Failed to convert payload data to content");
            assert!(measurements.len() > 0);
            for measurement in measurements {
                println!("Measurement: {}", measurement);
            }
        }
    }

    #[test]
    fn test_wh31_e_rcc() {
        // Test increasing amounts of sync values at the beginning of the payload
        for (i, row) in WH31_E_RCC_SAMPLES.iter().enumerate() {
            let mut row_vec: Vec<u8> = vec![0xaa; i];
            row_vec.extend_from_slice(&PREAMBLE);
            row_vec.extend_from_slice(row);
            row_vec.extend_from_slice(&[0x00; 4]);

            let payload =
                bitstream_to_payload(&row_vec).expect("Found no payload in the bitstream");
            assert_eq!(&payload[0..payload.len() - 4], row);

            let measurements = row_vec
                .as_slice()
                .to_measurements()
                .expect("Failed to convert payload data to content");
            assert!(measurements.len() > 0);
            for measurement in measurements {
                println!("Measurement: {}", measurement);
            }
        }
    }

    #[test]
    fn test_wh40() {
        // Test increasing amounts of sync values at the beginning of the payload
        for (i, row) in WH40_SAMPLES.iter().enumerate() {
            let mut row_vec: Vec<u8> = vec![0xaa; i];
            row_vec.extend_from_slice(&PREAMBLE);
            row_vec.extend_from_slice(row);
            row_vec.extend_from_slice(&[0x00; 4]);

            let payload =
                bitstream_to_payload(&row_vec).expect("Found no payload in the bitstream");
            assert_eq!(&payload[0..payload.len() - 4], row);

            let measurements = row_vec
                .as_slice()
                .to_measurements()
                .expect("Failed to convert payload data to content");
            assert!(measurements.len() > 0);
            for measurement in measurements {
                println!("Measurement: {}", measurement);
            }
        }
    }

    #[test]
    fn test_ws68() {
        // Test increasing amounts of sync values at the beginning of the payload
        for (i, row) in WS68_SAMPLES.iter().enumerate() {
            let mut row_vec: Vec<u8> = vec![0xaa; i];
            row_vec.extend_from_slice(&PREAMBLE);
            row_vec.extend_from_slice(row);
            row_vec.extend_from_slice(&[0x00; 4]);

            let payload =
                bitstream_to_payload(&row_vec).expect("Found no payload in the bitstream");
            assert_eq!(&payload[0..payload.len() - 4], row);

            let measurements = row_vec
                .as_slice()
                .to_measurements()
                .expect("Failed to convert payload data to content");
            assert!(measurements.len() > 0);
            for measurement in measurements {
                println!("Measurement: {}", measurement);
            }
        }
    }
}
