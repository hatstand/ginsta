use std::{
    env::args,
    io::{Read, Seek},
};

use nom::{
    IResult, Parser,
    bytes::{complete::tag, take},
    multi::count,
    number::{le_i16, le_i32, le_u32},
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
    metadata_size: i32,
    signature: Vec<u8>,
    metadata: Vec<TrailerMetadata>,
}

#[derive(Debug)]
struct TrailerMetadata {
    id: i16,
    size: u32,
}

fn parse_trailer_metadata(data: &[u8]) -> IResult<&[u8], TrailerMetadata> {
    let (rest, id) = le_i16().parse(data)?;
    let (rest, size) = le_u32().parse(rest)?;

    Ok((rest, TrailerMetadata { id, size }))
}

fn header_parser(header: &[u8]) -> IResult<&[u8], Trailer> {
    let (rest, metadata) = count(parse_trailer_metadata, 7).parse(header)?;
    let (rest, version_num) = le_i32().parse(rest)?;
    let (rest, signature) = tag(SIGNATURE).parse(rest)?;
    assert_eq!(0, rest.len());

    Ok((
        rest,
        Trailer {
            metadata_size: 1,
            version_num,
            signature: signature.to_vec(),
            metadata,
        },
    ))
}

const FRAME_HEADER_SIZE: i64 = 6;

#[repr(i8)]
#[derive(FromPrimitive, Debug)]
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

fn frame_header(frame: &[u8]) -> IResult<&[u8], FrameTrailer> {
    let (rest, frame_ver) = take(1 as usize).parse(frame)?;
    let (rest, frame_type_code) = take(1 as usize).parse(rest)?;
    let (rest, frame_size) = le_i32().parse(rest)?;

    assert_eq!(0, rest.len());

    Ok((
        rest,
        FrameTrailer {
            frame_version: frame_ver[0],
            frame_type: FrameType::from_u8(frame_type_code[0]).unwrap(),
            frame_size,
        },
    ))
}

#[derive(Debug)]
struct IndexFrame {
    header: FrameTrailer,
    frames: Vec<IndexFrameTrailer>,
}

#[derive(Debug)]
struct IndexFrameTrailer {
    frame_version: u8,
    frame_type: FrameType,
    frame_size: i32,
    frame_offset: i64,
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
    file.seek(std::io::SeekFrom::End(HEADER_SIZE * -1))
        .expect("Failed to seek");

    // POSITION: just after the last frame.

    const MAX_FRAMES: i64 = 20;
    let mut frames_read = 0;
    while file.stream_position()? > metadata_pos && frames_read < MAX_FRAMES {
        file.seek_relative(FRAME_HEADER_SIZE * -1)?;
        // POSITION: just before the frame's header/trailer.

        // Read frame header.
        let mut frame_header_buf = [0; FRAME_HEADER_SIZE as usize];
        file.read_exact(&mut frame_header_buf)
            .expect("Failed to read frame header");
        // POSITION: just after the frame.
        let (_, frame_header) =
            frame_header(&frame_header_buf).expect("Failed to parse frame header");
        println!("{:?}", frame_header);

        file.seek_relative(((frame_header.frame_size + FRAME_HEADER_SIZE as i32) * -1).into())?;
        // POSITION: just before the frame's data.
        let frame_buf = &mut vec![0; frame_header.frame_size as usize];
        file.read_exact(frame_buf)?;
        // POSITION: just after the frame's data.
        file.seek_relative(frame_header.frame_size as i64 * -1)?;
        // POSITION: Just before the current frame.

        frames_read += 1;
    }

    Ok(())
}
