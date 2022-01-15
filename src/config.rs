use std::collections::HashSet;
use std::convert::TryFrom;

use anyhow::{Context, Result};
use clap::crate_name;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum ConfigError {
    #[error("File read error")]
    ReadError(#[from] std::io::Error),
    #[error("Json parse error")]
    JsonError(#[from] serde_json::Error),
    #[error("Argument error: missing mqtt broker for mqtt credentials")]
    MqttMissingBroker,
    #[error("Keyring access failure")]
    KeyringError(String),
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) enum Credentials {
    Keyring(String),
    ConfigFile(String, String),
}

impl Credentials {
    pub(crate) fn get(&self) -> Option<(String, String)> {
        match (self.username(), self.password().ok().flatten()) {
            (Some(u), Some(p)) if !u.is_empty() && !p.is_empty() => Some((u, p)),
            _ => None,
        }
    }

    pub(crate) fn username(&self) -> Option<String> {
        match self {
            Credentials::Keyring(u) if u.is_empty() => None,
            Credentials::Keyring(u) => Some(u.clone()),
            Credentials::ConfigFile(u, _) if u.is_empty() => None,
            Credentials::ConfigFile(u, _) => Some(u.clone()),
        }
    }

    pub(crate) fn password(&self) -> Result<Option<String>> {
        match self {
            Credentials::Keyring(u) => Credentials::get_from_keyring(u).with_context(|| {
                format!(
                    "Failed retrieving secrets for user {} from session keyring",
                    &u,
                )
            }),
            Credentials::ConfigFile(_, p) if p.is_empty() => Ok(None),
            Credentials::ConfigFile(_, p) => Ok(Some(p.clone())),
        }
    }

    #[must_use = "Credentials may not be mutated in-place. Calling \"update_<field>()\" creates a copy with the updated value."]
    pub(crate) fn update_username(&self, username: &str) -> Credentials {
        let mut dup = self.clone();
        let username = username.to_string();
        match dup {
            Credentials::Keyring(ref mut u) => {
                *u = username;
            }
            Credentials::ConfigFile(ref mut u, _) => {
                *u = username;
            }
        }
        dup
    }

    #[must_use = "Credentials may not be mutated in-place. Calling \"update_<field>()\" creates a copy with the updated value."]
    pub(crate) fn update_password(&self, password: &str) -> Result<Credentials> {
        let mut dup = self.clone();
        match &mut dup {
            Credentials::Keyring(u) => {
                Credentials::set_on_keyring(u, password).with_context(|| {
                    format!("Failed updating secret for user {} on session keyring", &u)
                })?
            }
            Credentials::ConfigFile(_, ref mut p) => {
                if *p != password {
                    *p = password.to_string();
                }
            }
        }
        Ok(dup)
    }

    #[must_use = "Credentials may not be converted between variants in-place. Calling \"as_<type>\" creates a copy as another variant."]
    pub(crate) fn as_keyring(&self) -> Result<Credentials> {
        match self {
            Self::Keyring(_) => Ok(self.clone()),
            c => {
                let username = c.username().unwrap_or_default();
                let password = c.password().ok().flatten().unwrap_or_default();
                if !username.is_empty() && !password.is_empty() {
                    Credentials::set_on_keyring(&username, &password)?;
                }
                Ok(Self::Keyring(username))
            }
        }
    }

    #[must_use = "Credentials may not be converted between variants in-place. Calling \"as_<type>\" creates a copy as another variant."]
    pub(crate) fn as_configfile(&self) -> Credentials {
        match self {
            Self::ConfigFile(_, _) => self.clone(),
            c => {
                let username = c.username().unwrap_or_default();
                let password = c.password().ok().flatten().unwrap_or_default();
                Self::ConfigFile(username, password)
            }
        }
    }

    fn get_from_keyring(username: &str) -> Result<Option<String>> {
        let service = String::from(crate_name!());
        let keyring = keyring::Entry::new(&service, username);
        match keyring.get_password() {
            Ok(p) => Ok(Some(p)),
            Err(keyring::error::Error::NoEntry) => Ok(None),
            Err(e) => Err(ConfigError::KeyringError(e.to_string())).with_context(|| {
                format!("Error contacting session keyring for user {}", &username)
            }),
        }
    }

    fn set_on_keyring(username: &str, password: &str) -> Result<()> {
        let service = String::from(crate_name!());
        let keyring = keyring::Entry::new(&service, username);
        keyring
            .set_password(password)
            .map_err(|e| ConfigError::KeyringError(e.to_string()))
            .with_context(|| {
                format!(
                    "Failed updating secret for user {} on session keyring",
                    &username
                )
            })
    }
}

impl Default for Credentials {
    fn default() -> Self {
        Credentials::ConfigFile(String::new(), String::new())
    }
}

// Custom implementation to avoid spilling secrets in log files, for example
impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Credentials::")?;
        match self {
            Self::Keyring(u) => write!(f, "Keyring({}, ******)", u),
            Self::ConfigFile(u, _) => write!(f, "ConfigFile({}, ******)", u),
        }
    }
}

impl std::cmp::PartialEq<Credentials> for Credentials {
    fn eq(&self, other: &Credentials) -> bool {
        if std::mem::discriminant(self) != std::mem::discriminant(other) {
            return false;
        }

        if self.username() != other.username() {
            return false;
        }

        if self.password().ok() != other.password().ok() {
            return false;
        }

        true
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct MqttConfig {
    pub(crate) broker: String,
    pub(crate) credentials: Option<Credentials>,
}

impl MqttConfig {
    pub(crate) fn new<S: Into<String>>(broker: S) -> Self {
        MqttConfig {
            broker: broker.into(),
            credentials: None,
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

        if let Some(broker) = arg_matches.value_of("mqtt_broker") {
            if let Some(ref mut mqtt) = &mut self.mqtt {
                mqtt.broker = broker.to_owned();
            } else {
                self.mqtt = Some(MqttConfig::new(broker));
            }
        }

        if let Some(ref mut mqtt) = &mut self.mqtt {
            let cred = mqtt.credentials.clone().unwrap_or_default();
            let mut new_cred = if arg_matches.is_present("mqtt_keyring_password") {
                cred.as_keyring()?
            } else if arg_matches.is_present("mqtt_config_password") {
                cred.as_configfile()
            } else {
                cred
            };
            if let Some(user) = arg_matches.value_of("mqtt_user") {
                new_cred = new_cred.update_username(user);
            }
            mqtt.credentials.replace(new_cred);
        } else if arg_matches.is_present("mqtt-user") || arg_matches.is_present("mqtt-password") {
            return Err(ConfigError::MqttMissingBroker.into());
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
