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
    #[error("Application missing configuration option for rtl_433 path")]
    MissingArgumentRtl433,
}

fn main() -> Result<()> {
    let json_config_path = dirs::config_dir()
        .ok_or(AppError::AppDirNotFound)
        .with_context(|| "User configuration directory not found")?
        .join(crate_name!())
        .join("config.json");

    let matches = app_from_crate!("")
        .setting(clap::AppSettings::ColorAuto)
        .setting(clap::AppSettings::ColoredHelp)
        .arg(
            clap::Arg::new("quiet")
                .short('q')
                .long("quiet")
                .global(true)
                .about("Suppress all application output"),
        )
        .arg(
            clap::Arg::new("debug")
                .short('g')
                .long("debug")
                .multiple_occurrences(true)
                .hidden(true)
                .global(true)
                .about("Enable debug-level output"),
        )
        .arg(
            clap::Arg::new("rtl_433_bin")
                .short('r')
                .long("rtl-433")
                .takes_value(true)
                .value_name("PROGRAM")
                .about("Path to the rtl_433 binary"),
        )
        .arg(
            clap::Arg::new("mqtt_broker")
                .short('b')
                .long("mqtt-broker")
                .takes_value(true)
                .value_name("BROKER")
                .about(
                    "Network identifier of the mqtt broker to publish to, e.g. 'localhost:1883'",
                ),
        )
        .arg(
            clap::Arg::new("mqtt_user")
                .short('u')
                .long("mqtt-user")
                .takes_value(true)
                .value_name("USER")
                .about("Account user for connecting to the mqtt broker"),
        )
        // TODO: this should be change to not accept an arg and securely query password on command
        // line and store in secret storage
        .arg(
            clap::Arg::new("mqtt_password")
                .short('p')
                .long("mqtt-password")
                .takes_value(true)
                .value_name("PASSWORD")
                .about("Account password for connecting to the mqtt broker"),
        )
        .arg(
            clap::Arg::new("ignore")
                .short('i')
                .long("ignore")
                .multiple_occurrences(true)
                .takes_value(true)
                .value_name("SENSOR_ID")
                .about("Ignore the specified sensor topic; can be repeated"),
        )
        .arg(
            clap::Arg::new("generate_config")
                .short('G')
                .long("generate-config")
                .about(&format!("Generates a json-formatted configuration file at {}, populated by the current invocation arguments, and defaults where arguments were omitted, and then exits the program", json_config_path.display())),
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

    let crate_log_level = match conf.output_level.unwrap_or(1) {
        0 => log::LevelFilter::Off,
        1 => log::LevelFilter::Error,
        2 => log::LevelFilter::Warn,
        3 => log::LevelFilter::Info,
        4 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
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

    if matches.is_present("generate_config") {
        // TODO: create config dir
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
        let mqtt_session = paho_mqtt::Client::new(format!("tcp://{}", mqtt.broker))?;
        let mut mqtt_opts = paho_mqtt::ConnectOptionsBuilder::new();
        mqtt_opts
            .keep_alive_interval(std::time::Duration::from_secs(20))
            .clean_session(true);
        if let Some(username) = &mqtt.user {
            mqtt_opts.user_name(username);
        }
        if let Some(password) = &mqtt.password {
            mqtt_opts.password(password);
        }
        mqtt_session.connect(mqtt_opts.finalize())?;
        log::info!("Connected to mqtt broker {}", mqtt.broker);
        Some(mqtt_session)
    } else {
        None
    };

    log::debug!("Opening rtl_433...");
    let rtl_433_bin = conf
        .rtl_433
        .as_ref()
        .ok_or(AppError::MissingArgumentRtl433)?;
    let weather = radio::Sensor::<radio::RTL433>::new(rtl_433_bin)?;
    for record in weather.filter(|r| !conf.sensor_ignores.contains(&r.sensor_id)) {
        let recordmeta = format!("weatherradio/{}", record.sensor_id);
        log::trace!("[RECORD] {} {}", record.timestamp, recordmeta);
        for measurement in record.measurements {
            log::info!("[{}]:{} {}", record.timestamp, recordmeta, measurement);
            if let Some(ref session) = session_opt {
                let topic = format!("{}/{}", recordmeta, measurement.name());
                let msg = paho_mqtt::Message::new(&topic, measurement.value(), 0);
                session.publish(msg)?;
                log::info!("mqtt <== {}({})", topic, measurement.value());
            }
        }
    }
    Ok(())
}
