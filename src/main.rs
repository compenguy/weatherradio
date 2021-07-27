use anyhow::Result;

mod fine_offset;
use crate::fine_offset::ToMeasurements;

fn main() -> Result<()> {
    let bitstream: Vec<u8> = Vec::new();
    for measured in bitstream.as_slice().to_measurements()? {
        print!("{}: ", measured.timestamp);
        if let Some(channel) = measured.channel {
            println!(
                "[{:20}] Channel {} {}",
                measured.timestamp, channel, measured.measurement
            );
        } else {
            println!("[{:20}] {}", measured.timestamp, measured.measurement);
        }
    }

    Ok(())
}
