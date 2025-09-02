use core::time;
use std::{env::args, io::Read};

use nom::{
    IResult, Parser,
    bytes::{tag, take},
    character::one_of,
    combinator::eof,
    multi::many_till,
    number::{le_f64, le_i32, le_u16, le_u32, le_u64},
};

#[derive(Debug)]
struct GpsRecord {
    timestamp: u64,
    latitude: f64,
    longitude: f64,
    speed: f64,
    track: f64,
    altitude: f64,
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
        take(7usize),
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
    println!("Hello, world!");
    env_logger::init();
    let file_name = args().nth(1).expect("No file name given");

    let mut file = std::fs::File::open(file_name).expect("Failed to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");

    let (_, gps_record) = parse_gps_records(&buffer).expect("Failed to parse GPS record");

    println!("{:?}", gps_record);

    Ok(())
}
