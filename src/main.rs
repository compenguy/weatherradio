use anyhow::{Context, Result};
use clap::{app_from_crate, crate_name, crate_version};
use flexi_logger::{colored_default_format, detailed_format, Logger};

mod fine_offset;
use crate::fine_offset::Measured;
use crate::fine_offset::ToMeasurements;

const TOPIC_ROOT: &str = "ambientweather";

fn publish_measurement(session: Option<&paho_mqtt::Client>, measured: Measured) -> Result<()> {
    if let Some(channel) = measured.channel {
        let topic = format!(
            "{}/Channel {}/{}",
            TOPIC_ROOT,
            channel,
            measured.measurement.name_token()
        );
        log::debug!(
            "[{}] {} <= {}",
            measured.timestamp,
            topic,
            measured.measurement.value()
        );
        if let Some(client) = session {
            let msg = paho_mqtt::Message::new(topic, measured.measurement.value(), 0);
            client.publish(msg)?;
        }
    } else {
        let topic = format!("{}/{}", TOPIC_ROOT, measured.measurement.name_token());
        log::debug!(
            "[{}] {} <= {}",
            measured.timestamp,
            topic,
            measured.measurement.value()
        );
    }

    Ok(())
}

fn generate_dataset() -> Vec<Vec<u8>> {
    let mut bitstream: Vec<Vec<u8>> = Vec::new();

    for row in crate::fine_offset::WH31_E_SAMPLES {
        let mut row_vec: Vec<u8> = crate::fine_offset::PREAMBLE.to_vec();
        row_vec.extend_from_slice(&row);
        row_vec.extend_from_slice(&[0x00; 4]);
        bitstream.push(row_vec);
    }

    for row in crate::fine_offset::WH31_E_RCC_SAMPLES {
        let mut row_vec: Vec<u8> = crate::fine_offset::PREAMBLE.to_vec();
        row_vec.extend_from_slice(&row);
        row_vec.extend_from_slice(&[0x00; 4]);
        bitstream.push(row_vec);
    }

    for row in crate::fine_offset::WH40_SAMPLES {
        let mut row_vec: Vec<u8> = crate::fine_offset::PREAMBLE.to_vec();
        row_vec.extend_from_slice(&row);
        row_vec.extend_from_slice(&[0x00; 4]);
        bitstream.push(row_vec);
    }

    for row in crate::fine_offset::WS68_SAMPLES {
        let mut row_vec: Vec<u8> = crate::fine_offset::PREAMBLE.to_vec();
        row_vec.extend_from_slice(&row);
        row_vec.extend_from_slice(&[0x00; 4]);
        bitstream.push(row_vec);
    }

    bitstream
}

fn main() -> Result<()> {
    let matches = app_from_crate!("")
        .setting(clap::AppSettings::ColorAuto)
        .setting(clap::AppSettings::ColoredHelp)
        .arg(
            clap::Arg::new("debug")
                .short('g')
                .long("debug")
                .multiple_occurrences(true)
                .hidden(true)
                .global(true)
                .about("Enable debug-level output"),
        )
        .get_matches();

    let crate_log_level = match matches.occurrences_of("debug") {
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

    /*
    let mqtt_session = paho_mqtt::Client::new("tcp://localhost:1883")?;
    let mqtt_opts = paho_mqtt::ConnectOptionsBuilder::new()
        .keep_alive_interval(std::time::Duration::from_secs(20))
        .clean_session(true)
        .finalize();
    mqtt_session.connect(mqtt_opts)?;
    */

    for measurement in generate_dataset()
        .iter()
        .filter_map(|r| r.as_slice().to_measurements().ok())
        .flatten()
    {
        //publish_measurement(Some(&mqtt_session), measurement)?;
        publish_measurement(None, measurement)?;
    }
    Ok(())
}
