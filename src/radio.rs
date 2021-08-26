use anyhow::{Context, Result};
use std::io::BufRead;

use uom::fmt::DisplayStyle::Abbreviation;
use uom::si::{angle, u16::Angle};
use uom::si::{energy, f32::Energy};
use uom::si::{f32::Length, length};
use uom::si::{f32::ThermodynamicTemperature, thermodynamic_temperature};
use uom::si::{time, u32::Time};
use uom::si::{u16::Velocity, velocity};

pub(crate) struct RTL433;

pub(crate) struct Sensor<R> {
    _child: std::process::Child,
    stdout: Option<std::io::BufReader<std::process::ChildStdout>>,
    _stderr: Option<std::io::BufReader<std::process::ChildStderr>>,
    channel_type: std::marker::PhantomData<R>,
}

impl Sensor<RTL433> {
    pub(crate) fn new(conf: &crate::config::Config) -> Result<Self> {
        let binpath = conf
            .rtl_433
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Path to rtl_433 binary not set."))?;
        let mut proc = std::process::Command::new(binpath.as_os_str());
        proc.arg("-Mutc")
            .arg("-Fjson")
            .arg("-f915M")
            .arg("-R113")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped());

        // Swallow all of rtl_433's stderr output, unless we're logging at debug or higher
        if conf.get_log_level() < log::LevelFilter::Debug {
            proc.stderr(std::process::Stdio::piped());
        }

        // When logging at trace level, add signal level and protocol information to the
        // captured information
        if conf.get_log_level() >= log::LevelFilter::Trace {
            proc.arg("-Mlevel").arg("-Mprotocol");
        }
        let mut child = proc.spawn().with_context(|| {
            format!(
                "Unable to launch rtl_433 binary at the configured location ({})",
                binpath.display()
            )
        })?;

        let stdout = child.stdout.take().map(std::io::BufReader::new);
        let stderr = child.stderr.take().map(std::io::BufReader::new);
        Ok(Sensor {
            _child: child,
            stdout,
            _stderr: stderr,
            channel_type: std::marker::PhantomData,
        })
    }

    pub(crate) fn get_line(&mut self) -> Option<String> {
        if let Some(stdout) = &mut self.stdout {
            let mut line = String::new();
            while line.is_empty() {
                let result = stdout.read_line(&mut line);
                log::trace!("Reading from rtl_433: {:?} => '{}'", result, line);
                match result {
                    Ok(0) => return None,
                    Ok(_) => return Some(line),
                    Err(_) => (),
                }
                log::error!("Error reading from rtl_433: {:?}", result);
            }
            unreachable!();
        } else {
            log::error!("No output pipe for rtl_433 process!");
            None
        }
    }
}

impl Iterator for Sensor<RTL433> {
    type Item = Record;

    fn next(&mut self) -> Option<Self::Item> {
        // retry getting lines and parsing them as json until we get one that
        // parses correctly, or until we reach the end of child process
        loop {
            let line = match self.get_line() {
                None => return None,
                Some(l) => l,
            };
            let json_result: std::result::Result<serde_json::Value, serde_json::Error> =
                serde_json::from_str(&line);
            let json = match json_result {
                Ok(json) => json,
                Err(e) => {
                    log::error!("Error parsing rtl_433 output: {:?}", e);
                    return None;
                }
            };
            if let Ok(record) = crate::ambientweather::try_parse(&json) {
                return Some(record);
            }
            if let Ok(record) = crate::idm::try_parse(&json) {
                return Some(record);
            }
        }
        /*
        if let Ok(Some(status)) = self.child.try_wait() {
            return None;
        }
        */
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Measurement {
    TotalEnergyConsumption(Energy),
    DifferentialEnergyConsumption(Energy, Time),
    BatteryOk(bool),
    Temperature(ThermodynamicTemperature),
    RelativeHumidity(u8),
    BatteryLevelRaw(u8),
    Clock(chrono::Utc),
    Rainfall(Length),
    Lux(u16),
    WindSpeed(Velocity),
    WindGust(Velocity),
    WindDirection(Angle),
    None,
}

impl std::fmt::Display for Measurement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name(), self.value())
    }
}

impl Measurement {
    pub(crate) fn name(&self) -> String {
        let text = match self {
            Self::TotalEnergyConsumption(_) => "TotalEnergy",
            Self::DifferentialEnergyConsumption(_, _) => "EnergyOverTime",
            Self::BatteryOk(_) => "BatteryOk",
            Self::Temperature(_) => "TemperatureF",
            Self::RelativeHumidity(_) => "Humidity",
            Self::BatteryLevelRaw(_) => "BatteryLevel",
            Self::Clock(_) => "Clock",
            Self::Rainfall(_) => "Rainfall",
            Self::Lux(_) => "Lux",
            Self::WindSpeed(_) => "WindSpeed",
            Self::WindGust(_) => "WindGust",
            Self::WindDirection(_) => "WindDirection",
            Self::None => "None",
        };

        text.to_owned()
    }

    pub(crate) fn value(&self) -> String {
        match self {
            Self::TotalEnergyConsumption(e) => e
                .into_format_args(energy::kilowatt_hour, Abbreviation)
                .to_string(),
            Self::DifferentialEnergyConsumption(e, t) => format!(
                "{} over the last {:.1}",
                e.into_format_args(energy::kilowatt_hour, Abbreviation)
                    .to_string(),
                t.into_format_args(time::hour, Abbreviation)
            ),
            Self::BatteryOk(b) => b.to_string(),
            Self::Temperature(t) => format!(
                "{:.1}",
                t.into_format_args(thermodynamic_temperature::degree_fahrenheit, Abbreviation)
            ),
            Self::RelativeHumidity(h) => format!("{}%", h),
            Self::BatteryLevelRaw(b) => b.to_string(),
            Self::Clock(t) => t.to_string(),
            Self::Rainfall(m) => m
                .into_format_args(length::millimeter, Abbreviation)
                .to_string(),
            Self::Lux(l) => l.to_string(),
            Self::WindSpeed(w) => w
                .into_format_args(velocity::kilometer_per_hour, Abbreviation)
                .to_string(),
            Self::WindGust(w) => w
                .into_format_args(velocity::kilometer_per_hour, Abbreviation)
                .to_string(),
            Self::WindDirection(w) => w.into_format_args(angle::degree, Abbreviation).to_string(),
            Self::None => String::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Record {
    pub(crate) timestamp: chrono::DateTime<chrono::Local>,
    pub(crate) sensor_id: String,
    pub(crate) measurements: Vec<Measurement>,
}

impl std::fmt::Display for Record {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for measurement in &self.measurements {
            write!(
                f,
                "[{}][{}] {}",
                self.timestamp, self.sensor_id, measurement
            )?;
        }
        write!(f, "")
    }
}
