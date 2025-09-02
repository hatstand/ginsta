use std::{env::args, io::Read};

use nom::{
    IResult, Parser,
    bytes::take,
    character::one_of,
    combinator::eof,
    multi::many_till,
    number::{le_f64, le_u32},
};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct GpsRecord {
    timestamp: u64, // Seconds.
    latitude: f64,
    longitude: f64,
    speed: f64, // Probably metres / second.
    track: f64,
    altitude: f64, // Probably metres.
}

const NS: &[u8] = &[b'N', b'S'];
const EW: &[u8] = &[b'E', b'W'];

fn parse_gps_record(frame: &[u8]) -> IResult<&[u8], GpsRecord> {
    let timestamp = le_u32();
    let latitude = le_f64();
    let northsouth = one_of(NS);
    let longitude = le_f64();
    let eastwest = one_of(EW);

    let speed = le_f64();
    let track = le_f64();
    let altitude = le_f64();

    let mut parser = (
        timestamp,
        take(7usize), // Slope maybe?
        latitude,
        northsouth,
        longitude,
        eastwest,
        speed,
        track,
        altitude,
    );

    let (rest, (timestamp, _, latitude, northsouth, longitude, eastwest, speed, track, altitude)) =
        parser.parse(frame)?;

    Ok((
        rest,
        GpsRecord {
            timestamp: timestamp as u64,
            latitude: if northsouth == 'S' {
                -latitude
            } else {
                latitude
            },
            longitude: if eastwest == 'W' {
                -longitude
            } else {
                longitude
            },
            speed,
            track,
            altitude,
        },
    ))
}

fn parse_gps_records(frame: &[u8]) -> IResult<&[u8], Vec<GpsRecord>> {
    let (rest, records) = many_till(parse_gps_record, eof).parse(frame)?;
    Ok((rest, records.0))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let file_name = args().nth(1).expect("No file name given");

    let mut file = std::fs::File::open(file_name).expect("Failed to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");

    let (_, gps_records) = parse_gps_records(&buffer).expect("Failed to parse GPS record");

    let mut csv_writer = csv::Writer::from_writer(std::io::stdout());
    gps_records.iter().for_each(|record| {
        csv_writer.serialize(record).expect("Failed to write CSV");
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gps_record() {
        let data = include_bytes!("../testdata/Gps_1752824363158.insgps");
        let (_, records) = parse_gps_records(data).expect("Failed to parse GPS records");
        assert_eq!(records.len(), 14915);

        let record = records.first().unwrap();
        assert_eq!(record.timestamp, 1752824362);
        assert_eq!(record.latitude, 49.25853492931603);
        assert_eq!(record.longitude, 4.03079459928793);
        assert_eq!(record.speed, 0.0);
        assert_eq!(record.track, 335.23572083279436);
        assert_eq!(record.altitude, 86.40542984008789);
    }
}
