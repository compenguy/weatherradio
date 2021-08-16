use anyhow::{Context, Result};
use clap::{app_from_crate, crate_name, crate_version};
use flexi_logger::{colored_default_format, detailed_format, Logger};

use thiserror::Error;

mod measurement;
mod radio;

#[derive(Error, Debug)]
pub(crate) enum ApplicationError {
    #[error("Missing argument")]
    MissingArgument,
    #[error("File not found: {0}")]
    FileNotFound(std::path::PathBuf),
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
        .arg(
            clap::Arg::new("rtl_433_bin")
                .short('r')
                .long("rtl-433")
                .takes_value(true)
                .value_name("PROGRAM")
                .required(true)
                .about("Path to the rtl_433 binary"),
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

    let rtl_433_bin = matches.value_of("rtl_433_bin").map(|s| std::path::PathBuf::from(&s)).ok_or(ApplicationError::MissingArgument)?;
    if !rtl_433_bin.is_file() {
        return Err(ApplicationError::FileNotFound(rtl_433_bin).into());
    }

    let weather = radio::Weather::<radio::RTL433>::new(rtl_433_bin)?;
    for record in weather {
        for measurement in record.measurements {
            println!("{}", measurement);
        }
    }
    Ok(())
}
