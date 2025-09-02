use std::{
    env::args,
    io::{Read, Seek},
};

use log::debug;
use nom::{
    bytes::{complete::tag, take},
    character::complete::one_of,
    combinator::eof,
    multi::{count, many_till},
    number::{le_f64, le_i32, le_u16, le_u32, le_u64},
    IResult, Parser,
};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;
use serde::Serialize;

const HEADER_SIZE: i64 = 78;

const SIGNATURE: &[u8] = &[
    0x38, 0x64, 0x62, 0x34, 0x32, 0x64, 0x36, 0x39, 0x34, 0x63, 0x63, 0x63, 0x34, 0x31, 0x38, 0x37,
    0x39, 0x30, 0x65, 0x64, 0x66, 0x66, 0x34, 0x33, 0x39, 0x66, 0x65, 0x30, 0x32, 0x36, 0x62, 0x66,
];

#[derive(Debug)]
struct Trailer {
    version_num: i32,
    signature: Vec<u8>,
    metadata: Vec<TrailerMetadata>,
    metadata_size: u32,
}

#[derive(Debug)]
struct TrailerMetadata {
    id: u16,
    size: u32,
}

fn parse_trailer_metadata(data: &[u8]) -> IResult<&[u8], TrailerMetadata> {
    let mut parser = (le_u16(), le_u32());
    let (rest, (id, size)) = parser.parse(data)?;

    Ok((rest, TrailerMetadata { id, size }))
}

fn header_parser(header: &[u8]) -> IResult<&[u8], Trailer> {
    let mut parser = (count(parse_trailer_metadata, 7), le_i32(), tag(SIGNATURE));
    let (rest, (metadata, version_num, signature)) = parser.parse(header)?;

    let metadata_size: u32 = metadata.last().unwrap().size;

    Ok((
        rest,
        Trailer {
            version_num,
            signature: signature.to_vec(),
            metadata,
            metadata_size,
        },
    ))
}

const FRAME_HEADER_SIZE: i64 = 6;

#[repr(i8)]
#[derive(FromPrimitive, ToPrimitive, Debug, PartialEq)]
enum FrameType {
    Raw = -1,
    Index = 0,
    Info = 1,
    Thumbnail = 2,
    Gyro = 3,
    Exposure = 4,
    ThumbnailExt = 5,
    Timelapse = 6,
    Gps = 7,
    StarNum = 8,
    ThreeAInTimestamp = 9,
    Anchors = 10,
    ThreeASimulation = 11,
    ExposureSecondary = 12,
    Magnetic = 13,
    Euler = 14,
    GyroSecondary = 15,
    Speed = 16,
    Tbox = 17,
    Editor = 18,
    Heartrate = 19,
    ForwardDirection = 20,
    Upview = 21,
    ShellRecognitionData = 22,
    Pos = 23,
    TimelapseQuat = 24,
}

#[derive(Debug)]
struct FrameTrailer {
    frame_version: u8,
    frame_type: FrameType,
    frame_size: i32,
}

fn frame_trailer(frame: &[u8]) -> IResult<&[u8], FrameTrailer> {
    let mut parser = (take(1usize), take(1usize), le_i32());
    let (rest, (frame_ver, frame_type_code, frame_size)) = parser.parse(frame)?;

    let raw_frame_type = frame_type_code[0];
    if raw_frame_type != 0 {
        debug!("Frame type code: {}", raw_frame_type);
    }

    Ok((
        rest,
        FrameTrailer {
            frame_version: frame_ver[0],
            frame_type: FrameType::from_u8(frame_type_code[0]).unwrap_or(FrameType::Raw),
            frame_size,
        },
    ))
}

#[derive(Debug)]
struct IndexFrame {
    frames: Vec<IndexFrameTrailer>,
}

#[derive(Debug)]
struct IndexFrameTrailer {
    frame_version: u8,
    frame_type: FrameType,
    frame_size: u32,
    frame_offset: u32, // Offset from metadata position.
}

fn parse_index(input: &[u8]) -> IResult<&[u8], IndexFrameTrailer> {
    let mut parser = (take(1usize), take(1usize), le_u32(), le_u32());
    let (rest, (frame_type, version, size, offset)) = parser.parse(input)?;

    Ok((
        rest,
        IndexFrameTrailer {
            frame_version: version[0],
            frame_type: FrameType::from_u8(frame_type[0]).unwrap_or(FrameType::Raw),
            frame_size: size,
            frame_offset: offset,
        },
    ))
}

fn parse_index_frame(frame: &[u8]) -> IResult<&[u8], IndexFrame> {
    let (rest, index_frames) = many_till(parse_index, eof).parse(frame)?;
    Ok((
        rest,
        IndexFrame {
            frames: index_frames.0,
        },
    ))
}

#[derive(Debug, Serialize)]
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
    let timestamp = le_u64();
    let latitude = le_f64();
    let northsouth = one_of(NS);
    let longitude = le_f64();
    let eastwest = one_of(EW);
    let speed = le_f64();
    let track = le_f64();
    let altitude = le_f64();

    let mut parser = (
        timestamp,
        take(3usize),
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
            timestamp,
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

#[derive(Debug)]
struct GpsFrame {
    records: Vec<GpsRecord>,
}

fn parse_gps_frame(frame: &[u8]) -> IResult<&[u8], GpsFrame> {
    let (rest, records) = many_till(parse_gps_record, eof).parse(frame)?;
    assert_eq!(0, rest.len());
    Ok((rest, GpsFrame { records: records.0 }))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let file_name = args().nth(1).expect("No file name given");

    let mut file = std::fs::File::open(file_name).expect("Failed to open file");
    file.seek(std::io::SeekFrom::End(HEADER_SIZE * -1))
        .expect("Failed to seek");

    let mut buffer = [0; HEADER_SIZE as usize];
    let res = file.read(&mut buffer).expect("Failed to read");
    assert_eq!(res as i64, HEADER_SIZE);

    assert_eq!(file.metadata()?.len(), file.stream_position()?);

    let (_, header) = header_parser(&buffer).expect("Failed to parse header");
    debug!("{:?}", header);

    let metadata_pos = file.metadata()?.len() as u64 - header.metadata_size as u64;

    // Read frames one at a time backwards from just before the header/trailer.
    file.seek(std::io::SeekFrom::End(HEADER_SIZE * -1 + FRAME_HEADER_SIZE))
        .expect("Failed to seek");

    // POSITION: just after the last frame.

    file.seek_relative(FRAME_HEADER_SIZE * -1)?;
    // POSITION: just before the frame's header/trailer.

    // Read frame header.
    let mut frame_header_buf = [0; FRAME_HEADER_SIZE as usize];
    file.read_exact(&mut frame_header_buf)
        .expect("Failed to read frame header");
    // POSITION: just after the frame.
    let (_, frame_trailer) =
        frame_trailer(&frame_header_buf).expect("Failed to parse frame header");
    assert_eq!(frame_trailer.frame_type, FrameType::Index);
    file.seek_relative(((frame_trailer.frame_size + FRAME_HEADER_SIZE as i32) * -1).into())?;
    debug!("{:?}", frame_trailer);

    // POSITION: just before the frame's data.
    let frame_buf = &mut vec![0; frame_trailer.frame_size as usize];
    file.read_exact(frame_buf)?;

    let (_, index_frame) = parse_index_frame(frame_buf).expect("Failed to parse index frame");

    for frame in index_frame.frames {
        match frame.frame_type {
            FrameType::Gps => {
                let file_offset = metadata_pos + frame.frame_offset as u64;
                file.seek(std::io::SeekFrom::Start(file_offset))?;

                let gps_frame_buf = &mut vec![0; frame.frame_size as usize];
                file.read_exact(gps_frame_buf)?;

                let (_, gps_frame) =
                    parse_gps_frame(gps_frame_buf).expect("Failed to parse GPS frame");

                let mut csv_writer = csv::Writer::from_writer(std::io::stdout());
                gps_frame.records.iter().for_each(|record| {
                    csv_writer.serialize(record).expect("Failed to write CSV");
                });
            }
            _ => debug!("Other frame"),
        }
    }

    Ok(())
}
