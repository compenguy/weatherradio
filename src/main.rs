use std::convert::TryFrom;
use std::io::Write;

use anyhow::{Context, Result};
use clap::{app_from_crate, crate_name, crate_version};
use flexi_logger::{colored_default_format, detailed_format, Logger};
use thiserror::Error;

mod ambientweather;
mod config;
mod idm;
mod radio;

#[derive(Error, Debug)]
pub(crate) enum AppError {
    #[error("Application configuration directory not found")]
    AppDirNotFound,
}

fn main() -> Result<()> {
    let json_config_path = dirs::config_dir()
        .ok_or(AppError::AppDirNotFound)
        .with_context(|| "User configuration directory not found")?
        .join(crate_name!())
        .join("config.json");

    let gen_cfg_help = format!("Generates a json-formatted configuration file at {}, populated by the current invocation arguments, and defaults where arguments were omitted, and then exits the program", json_config_path.display());

    let matches = app_from_crate!("")
        .arg(
            clap::Arg::new("quiet")
                .short('q')
                .long("quiet")
                .global(true)
                .help("Suppress all application output"),
        )
        .arg(
            clap::Arg::new("debug")
                .short('g')
                .long("debug")
                .multiple_occurrences(true)
                .hide(true)
                .global(true)
                .help("Enable debug-level output"),
        )
        .arg(
            clap::Arg::new("rtl_433_bin")
                .short('r')
                .long("rtl-433")
                .takes_value(true)
                .value_name("PROGRAM")
                .help("Path to the rtl_433 binary"),
        )
        .arg(
            clap::Arg::new("mqtt_broker")
                .short('b')
                .long("mqtt-broker")
                .takes_value(true)
                .value_name("BROKER")
                .help(
                    "Network identifier of the mqtt broker to publish to, e.g. 'localhost:1883'",
                ),
        )
        .arg(
            clap::Arg::new("mqtt_user")
                .short('u')
                .long("mqtt-user")
                .takes_value(true)
                .value_name("USER")
                .help("Account user for connecting to the mqtt broker"),
        )
        .arg(
            clap::Arg::new("mqtt_credentials_keyring")
                .short('k')
                .long("mqtt-credentials-keyring")
                .help("mqtt broker account password stored on session keyring, prompt on startup if no password set"),
        )
        .arg(
            clap::Arg::new("mqtt_credentials_config")
                .short('f')
                .long("mqtt-credentials-config")
                .help("mqtt broker account password stored in config file, prompt on startup if no password set"),
        )
        .arg(
            clap::Arg::new("ignore")
                .short('i')
                .long("ignore")
                .multiple_occurrences(true)
                .takes_value(true)
                .value_name("SENSOR_ID")
                .help("Ignore the specified sensor topic; can be repeated"),
        )
        .arg(
            clap::Arg::new("generate_config")
                .short('G')
                .long("generate-config")
                .help(gen_cfg_help.as_str())
        )
        .get_matches();

    let mut conf = if json_config_path.exists() {
        config::Config::try_from(&json_config_path).with_context(|| {
            format!(
                "Failed to read configuration settings from {}",
                json_config_path.display()
            )
        })?
    } else {
        config::Config::default()
    };
    conf.update_from_args(&matches)?;

    let crate_log_level = conf.get_log_level();
    let general_log_level = match crate_log_level {
        log::LevelFilter::Trace | log::LevelFilter::Debug => log::LevelFilter::Error,
        _ => log::LevelFilter::Off,
    };
    let spec = format!(
        "{}, {} = {}",
        general_log_level,
        crate_name!(),
        crate_log_level
    );
    Logger::try_with_str(&spec)?
        .format(detailed_format)
        .format_for_stderr(colored_default_format)
        .start()
        .with_context(|| "Failed to start FlexiLogger logging backend")?;

    log::info!("{} version {}", crate_name!(), crate_version!());

    log::debug!("rtl-433: {:?}", conf.rtl_433);
    log::debug!("mqtt: {:?}", conf.mqtt);
    log::debug!("sensors to ignore: {:?}", conf.sensor_ignores);

    if let Some(ref mut mqtt) = conf.mqtt {
        if let Some(cred) = &mqtt.credentials {
            if let Ok(None) = cred.password() {
                mqtt.credentials = Some(
                    cred.update_password(
                        rpassword::prompt_password_stdout(&format!(
                            "mqtt password for {}: ",
                            cred.username().unwrap_or_default()
                        ))?
                        .as_str(),
                    )?,
                )
            }
        }
    }

    if matches.is_present("generate_config") {
        std::fs::create_dir_all(json_config_path.parent().expect("Configuration file directory could not be determined from the provided configuration file path"))?;
        let mut config_file = std::io::BufWriter::new(
            std::fs::File::create(&json_config_path).with_context(|| {
                format!(
                    "Failed to create configuration file at {}",
                    json_config_path.display()
                )
            })?,
        );
        let json_out = serde_json::to_string(&conf)?;
        config_file.write_all(json_out.as_bytes())?;
        config_file.flush()?;
        return Ok(());
    }

    let session_opt = if let Some(mqtt) = &conf.mqtt {
        log::debug!("Establishing connection to mqtt broker {}", mqtt.broker);
        let broker_uri = format!("tcp://{}", mqtt.broker);
        let mqtt_session = paho_mqtt::Client::new(broker_uri.as_str())
            .with_context(|| format!("Failed to establish connection to broker {}", broker_uri))?;
        let mut mqtt_opts = paho_mqtt::ConnectOptionsBuilder::new();
        mqtt_opts
            .keep_alive_interval(std::time::Duration::from_secs(20))
            .clean_session(true);
        if let Some(cred) = &mqtt.credentials {
            if let Some((u, p)) = cred.get() {
                mqtt_opts.user_name(u);
                mqtt_opts.password(p);
            }
        }
        mqtt_session.connect(mqtt_opts.finalize())?;
        log::info!("Connected to mqtt broker {}", mqtt.broker);
        Some(mqtt_session)
    } else {
        None
    };

    log::debug!("Opening rtl_433...");
    let weather = radio::Sensor::<radio::RTL433>::new(&conf)?;
    // Dedup records
    let mut last: Option<crate::radio::Record> = None;
    for record in weather.filter(|r| !conf.sensor_ignores.contains(&r.sensor_id)) {
        if last.as_ref().map(|l| l == &record).unwrap_or(false) {
            log::trace!("Duplicate record.");
            continue;
        }
        log::trace!("[RECORD] {} {}", record.timestamp, record.sensor_id);
        if let Some(ref session) = session_opt {
            let msg = paho_mqtt::Message::new(
                &record.sensor_id,
                serde_json::to_vec(&record.record_json)?,
                2,
            );
            session.publish(msg)?;
            log::info!("mqtt <== {}({})", record.sensor_id, record.record_json);
        }
        /*
        for measurement in &record.measurements {
            log::info!("[{}]:{} {}", record.timestamp, record.sensor_id, measurement);
            if let Some(ref session) = session_opt {
                let topic = format!("{}/{}", record.sensor_id, measurement.name());
                let msg = paho_mqtt::Message::new(&topic, measurement.value(), 2);
                session.publish(msg)?;
                log::info!("mqtt <== {}({})", topic, measurement.value());
            }
        }
        */
        last = Some(record);
    }
    Ok(())
}
