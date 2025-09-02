use std::{
    env::args,
    io::{Read, Seek},
};

use nom::{
    IResult, Parser,
    bytes::{complete::tag, take},
    combinator::eof,
    multi::{count, many_till},
    number::{le_f64, le_i32, le_u8, le_u16, le_u32, le_u64},
};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

const HEADER_SIZE: i64 = 78;
const SIGNATURE_SIZE: i64 = 32;

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
    let (rest, id) = le_u16().parse(data)?;
    let (rest, size) = le_u32().parse(rest)?;

    Ok((rest, TrailerMetadata { id, size }))
}

fn header_parser(header: &[u8]) -> IResult<&[u8], Trailer> {
    let (rest, metadata) = count(parse_trailer_metadata, 7).parse(header)?;
    let (rest, version_num) = le_i32().parse(rest)?;
    let (rest, signature) = tag(SIGNATURE).parse(rest)?;
    assert_eq!(0, rest.len());

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
#[derive(FromPrimitive, Debug, PartialEq)]
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

const INDEX_FRAME_HEADER_SIZE: i32 = 10;

#[derive(Debug)]
enum Frame {
    Index(IndexFrame),
}

fn frame_trailer(frame: &[u8]) -> IResult<&[u8], FrameTrailer> {
    let (rest, frame_ver) = take(1 as usize).parse(frame)?;
    let (rest, frame_type_code) = take(1 as usize).parse(rest)?;
    let (rest, frame_size) = le_i32().parse(rest)?;

    assert_eq!(0, rest.len());

    let raw_frame_type = frame_type_code[0];
    if raw_frame_type != 0 {
        println!("Frame type code: {}", raw_frame_type);
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
    let (rest, frame_type) = take(1 as usize).parse(input)?;
    let (rest, version) = take(1 as usize).parse(rest)?;
    let (rest, size) = le_u32().parse(rest)?;
    let (rest, offset) = le_u32().parse(rest)?;
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

#[derive(Debug)]
struct GpsFrame {
    timestamp: u64,
    latitude: f64,
    northsouth: char,
    longitude: f64,
    eastwest: char,
    speed: f64,
    track: f64,
    altitude: f64,
}

fn parse_gps_frame(frame: &[u8]) -> IResult<&[u8], GpsFrame> {
    let (rest, timestamp) = le_u64().parse(frame)?;
    let (rest, _) = le_u16().parse(rest)?;
    let (rest, _) = take(1usize).parse(rest)?;
    let (rest, latitude) = le_f64().parse(rest)?;
    let (rest, latitude_ns) = le_u8().parse(rest)?;
    let (rest, longitude) = le_f64().parse(rest)?;
    let (rest, longitude_ew) = le_u8().parse(rest)?;
    let (rest, speed) = le_f64().parse(rest)?;
    let (rest, track) = le_f64().parse(rest)?;
    let (rest, altitude) = le_f64().parse(rest)?;
    Ok((
        rest,
        GpsFrame {
            timestamp,
            latitude,
            northsouth: latitude_ns as char,
            longitude,
            eastwest: longitude_ew as char,
            speed,
            track,
            altitude,
        },
    ))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_name = args().nth(1).expect("No file name given");
    println!("File name: {}", file_name);

    let mut file = std::fs::File::open(file_name).expect("Failed to open file");
    file.seek(std::io::SeekFrom::End(HEADER_SIZE * -1))
        .expect("Failed to seek");

    let mut buffer = [0; HEADER_SIZE as usize];
    let res = file.read(&mut buffer).expect("Failed to read");
    assert_eq!(res as i64, HEADER_SIZE);

    assert_eq!(file.metadata()?.len(), file.stream_position()?);

    println!("header: {:#x?}", buffer);

    let (_, header) = header_parser(&buffer).expect("Failed to parse header");
    println!("{:?}", header);

    let metadata_pos = file.metadata()?.len() as u64 - header.metadata_size as u64;
    println!("Metadata at: {}", metadata_pos);

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
    println!("{:?}", frame_trailer);

    // POSITION: just before the frame's data.
    let frame_buf = &mut vec![0; frame_trailer.frame_size as usize];
    file.read_exact(frame_buf)?;

    let (_, index_frame) = parse_index_frame(frame_buf).expect("Failed to parse index frame");

    for frame in index_frame.frames {
        // println!("Frame in index: {:#?}", frame);
        match frame.frame_type {
            FrameType::Gps => {
                let file_offset = metadata_pos + frame.frame_offset as u64;
                println!("GPS frame at: {}", file_offset);
                file.seek(std::io::SeekFrom::Start(file_offset))?;

                let gps_frame_buf = &mut vec![0; frame.frame_size as usize];
                file.read_exact(gps_frame_buf)?;

                let (_, gps_frame) =
                    parse_gps_frame(gps_frame_buf).expect("Failed to parse GPS frame");
                println!("GPS: {:?}", gps_frame);
            }
            _ => println!("Other frame"),
        }
    }

    Ok(())
}
