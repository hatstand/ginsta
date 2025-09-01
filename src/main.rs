mod reverse_file_reader;
use std::{
    env::args,
    io::{Read, Seek},
};

use nom::{IResult, Parser, bytes::take, character::char, number::le_i32};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

const HEADER_SIZE: i64 = 72;
const SIGNATURE_SIZE: i64 = 32;

#[derive(Debug)]
struct Header {
    version_num: i32,
    metadata_size: i32,
    signature: Vec<u8>,
    unknown: Vec<u8>,
}

fn header_parser(header: &[u8]) -> IResult<&[u8], Header> {
    let (rest, unknown) = take(32 as usize).parse(header)?;
    let (rest, metadata_size) = le_i32().parse(rest)?;
    let (rest, version_num) = le_i32().parse(rest)?;
    let (rest, signature) = take(SIGNATURE_SIZE as usize).parse(rest)?;

    assert_eq!(0, rest.len());

    Ok((
        rest,
        Header {
            unknown: unknown.to_vec(),
            metadata_size,
            version_num,
            signature: signature.to_vec(),
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
struct FrameHeader {
    frame_version: u8,
    frame_type: FrameType,
    frame_size: i32,
}

const INDEX_FRAME_HEADER_SIZE: i32 = 10;

#[derive(Debug)]
enum Frame {
    Index(IndexFrame),
}

fn frame_header(frame: &[u8]) -> IResult<&[u8], FrameHeader> {
    let (rest, frame_ver) = take(1 as usize).parse(frame)?;
    let (rest, frame_type_code) = take(1 as usize).parse(rest)?;
    let (rest, frame_size) = le_i32().parse(rest)?;

    assert_eq!(0, rest.len());

    Ok((
        rest,
        FrameHeader {
            frame_version: frame_ver[0],
            frame_type: FrameType::from_u8(frame_type_code[0]).unwrap(),
            frame_size,
        },
    ))
}

#[derive(Debug)]
struct IndexFrame {
    header: FrameHeader,
    frames: Vec<IndexFrameHeader>,
}

#[derive(Debug)]
struct IndexFrameHeader {
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

    let (_, header) = header_parser(&buffer).expect("Failed to parse header");
    println!("{:?}", header);

    let metadata_pos = file.metadata()?.len() as u64 - header.metadata_size as u64;
    println!("Metadata at: {}", metadata_pos);

    // Read frames one at a time backwards from just before the header/trailer.
    file.seek(std::io::SeekFrom::End(HEADER_SIZE * -1))
        .expect("Failed to seek");

    file.seek_relative(FRAME_HEADER_SIZE * -1)?;

    // Parse one frame?
    let mut frame_header_buf = [0; FRAME_HEADER_SIZE as usize];
    file.read_exact(&mut frame_header_buf)
        .expect("Failed to read frame header");
    let (_, frame_header) = frame_header(&frame_header_buf).expect("Failed to parse frame header");
    println!("{:?}", frame_header);

    file.seek_relative(((frame_header.frame_size + FRAME_HEADER_SIZE as i32) * -1).into())?;
    let index_frame_buf = &mut vec![0; frame_header.frame_size as usize];
    file.read_exact(index_frame_buf)?;

    assert_eq!(index_frame_buf.len() % INDEX_FRAME_HEADER_SIZE as usize, 0);
    let index_frames = index_frame_buf
        .chunks_exact(INDEX_FRAME_HEADER_SIZE as usize)
        .map(|chunk| {
            let (rest, frame_version) = take::<usize, &[u8], ()>(1 as usize).parse(chunk).unwrap();
            let (rest, frame_type_code) = take::<usize, &[u8], ()>(1 as usize).parse(rest).unwrap();
            let (rest, frame_size) = le_i32::<&[u8], ()>().parse(rest).unwrap();
            let (rest, frame_offset) = le_i32::<&[u8], ()>().parse(rest).unwrap();

            assert_eq!(0, rest.len());

            IndexFrameHeader {
                frame_version: frame_version[0],
                frame_type: FrameType::from_u8(frame_type_code[0]).unwrap(),
                frame_size,
                frame_offset: frame_offset as i64 + metadata_pos as i64,
            }
        })
        .collect::<Vec<_>>();
    println!("Index frames: {:#?}", index_frames);

    Ok(())
}
