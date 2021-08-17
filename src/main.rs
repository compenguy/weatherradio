use anyhow::{Context, Result};
use clap::{app_from_crate, crate_name, crate_version};
use flexi_logger::{colored_default_format, detailed_format, Logger};

mod measurement;
mod radio;

fn main() -> Result<()> {
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
                .required(true)
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
                .requires_all(&["mqtt_broker", "mqtt_password"])
                .about("Account user for connecting to the mqtt broker"),
        )
        .arg(
            clap::Arg::new("mqtt_password")
                .short('p')
                .long("mqtt-password")
                .takes_value(true)
                .value_name("PASSWORD")
                .requires_all(&["mqtt_broker", "mqtt_user"])
                .about("Account password for connecting to the mqtt broker"),
        )
        .get_matches();

    let crate_log_level = if matches.is_present("quiet") {
        log::LevelFilter::Off
    } else {
        match matches.occurrences_of("debug") + 1 {
            0 => log::LevelFilter::Off,
            1 => log::LevelFilter::Error,
            2 => log::LevelFilter::Warn,
            3 => log::LevelFilter::Info,
            4 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        }
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

    log::debug!("rtl-433: {:?}", matches.value_of("rtl_433_bin"));
    log::debug!("mqtt-broker: {:?}", matches.value_of("mqtt_broker"));
    log::debug!("mqtt-username: {:?}", matches.value_of("mqtt_user"));
    log::debug!("mqtt-password: {:?}", matches.value_of("mqtt_password"));
    let session_opt = if let Some(broker) = matches.value_of("mqtt_broker") {
        log::debug!("Establishing connection to mqtt broker {}", broker);
        let mqtt_session = paho_mqtt::Client::new(format!("tcp://{}", broker))?;
        let mut mqtt_opts = paho_mqtt::ConnectOptionsBuilder::new();
        mqtt_opts
            .keep_alive_interval(std::time::Duration::from_secs(20))
            .clean_session(true);
        if let Some(username) = matches.value_of("mqtt_user") {
            mqtt_opts.user_name(username);
        }
        if let Some(password) = matches.value_of("mqtt_password") {
            mqtt_opts.password(password);
        }
        mqtt_session.connect(mqtt_opts.finalize())?;
        log::info!("Connected to mqtt broker {}", broker);
        Some(mqtt_session)
    } else {
        None
    };

    log::debug!("Opening rtl_433...");
    let rtl_433_bin = matches
        .value_of("rtl_433_bin")
        .map(|s| std::path::PathBuf::from(&s))
        .expect("Missing requirement argument --rtl-433");
    let weather = radio::Weather::<radio::RTL433>::new(rtl_433_bin)?;
    for record in weather {
        let mut recordmeta = String::from("weatherradio");
        match (record.device_id, record.channel) {
            (Some(dev_id), Some(chan)) => recordmeta.push_str(&format!("/{}/{}", dev_id, chan)),
            (Some(dev_id), None) => recordmeta.push_str(&format!("/{}", dev_id)),
            (None, Some(chan)) => recordmeta.push_str(&format!("/{}", chan)),
            (None, None) => (),
        }
        log::trace!("[RECORD] {} {}", record.timestamp, recordmeta);
        for measurement in record.measurements {
            log::info!("[{}]:{} {}", record.timestamp, recordmeta, measurement);
            if let Some(ref session) = session_opt {
                let topic = format!("{}/{}", recordmeta, measurement.name());
                let msg = paho_mqtt::Message::new(
                    &topic,
                    measurement.value(),
                    0,
                );
                session.publish(msg)?;
                log::info!("mqtt <== {}({})", topic, measurement.value());
            }
        }
    }
    Ok(())
}
