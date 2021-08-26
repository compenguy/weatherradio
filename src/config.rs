use std::collections::HashSet;
use std::convert::TryFrom;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum ConfigError {
    #[error("File read error")]
    ReadError(#[from] std::io::Error),
    #[error("Json parse error")]
    JsonError(#[from] serde_json::Error),
    #[error("Argument error: missing mqtt broker for mqtt credentials")]
    BrokerArgError,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct MqttConfig {
    pub(crate) broker: String,
    pub(crate) user: Option<String>,
    // TODO: store in keyring
    pub(crate) password: Option<String>,
}

impl MqttConfig {
    pub(crate) fn new<S: Into<String>>(broker: S) -> Self {
        MqttConfig {
            broker: broker.into(),
            user: None,
            password: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Config {
    pub(crate) output_level: Option<u8>,
    pub(crate) rtl_433: Option<std::path::PathBuf>,
    pub(crate) mqtt: Option<MqttConfig>,
    pub(crate) sensor_ignores: HashSet<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            output_level: None,
            rtl_433: None,
            mqtt: None,
            sensor_ignores: HashSet::new(),
        }
    }
}

impl TryFrom<&std::path::Path> for Config {
    type Error = ConfigError;

    fn try_from(path: &std::path::Path) -> std::result::Result<Self, Self::Error> {
        Self::try_from(&path.to_path_buf())
    }
}

impl TryFrom<&std::path::PathBuf> for Config {
    type Error = ConfigError;

    fn try_from(path: &std::path::PathBuf) -> std::result::Result<Self, Self::Error> {
        let reader = std::io::BufReader::new(std::fs::File::open(path)?);
        let config = serde_json::from_reader(reader)?;
        Ok(config)
    }
}

impl Config {
    pub(crate) fn update_from_args(&mut self, arg_matches: &clap::ArgMatches) -> Result<()> {
        // We want to be a little bit careful that the absence of configuration
        // args isn't taken as a request to overwrite the configured values with
        // the default
        if arg_matches.is_present("quiet") || arg_matches.is_present("debug") {
            self.output_level = if arg_matches.is_present("quiet") {
                Some(0)
            } else {
                Some(arg_matches.occurrences_of("debug") as u8 + 1)
            };
        }

        if let Some(rtl_433_path) = arg_matches
            .value_of("rtl_433_bin")
            .map(|s| std::path::PathBuf::from(&s))
        {
            self.rtl_433 = Some(rtl_433_path);
        }

        if let Some(broker) = arg_matches.value_of("mqtt-broker") {
            if let Some(ref mut mqtt) = &mut self.mqtt {
                mqtt.broker = broker.to_owned();
            } else {
                self.mqtt = Some(MqttConfig::new(broker));
            }
        }

        if let Some(ref mut mqtt) = &mut self.mqtt {
            if let Some(user) = arg_matches.value_of("mqtt-user") {
                mqtt.user = Some(user.to_owned());
            }
            if let Some(password) = arg_matches.value_of("mqtt-password") {
                mqtt.password = Some(password.to_owned());
            }
        } else if arg_matches.is_present("mqtt-user") || arg_matches.is_present("mqtt-password") {
            return Err(ConfigError::BrokerArgError.into());
        }

        self.sensor_ignores.extend(
            arg_matches
                .values_of("ignore")
                .iter_mut()
                .flatten()
                .map(|s| s.to_owned()),
        );

        Ok(())
    }

    pub(crate) fn get_log_level(&self) -> log::LevelFilter {
        match self.output_level.unwrap_or(1) {
            0 => log::LevelFilter::Off,
            1 => log::LevelFilter::Error,
            2 => log::LevelFilter::Warn,
            3 => log::LevelFilter::Info,
            4 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        }
    }
}
