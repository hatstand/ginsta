use std::{
    env::args,
    io::{Read, Seek},
};

use nom::{IResult, Parser, bytes::take, number::le_i32};

const HEADER_SIZE: i64 = 72;
const SIGNATURE_SIZE: i64 = 32;

#[derive(Debug)]
struct Header {
    unknown: Vec<u8>,
    metadata_size: i32,
    version_num: i32,
    signature: Vec<u8>,
}

fn header_parser(header: &[u8]) -> IResult<&[u8], Header> {
    let (rest, unknown) = take(32 as usize).parse(header)?;
    let (rest, metadata_size) = le_i32().parse(rest)?;
    let (rest, version_num) = le_i32().parse(rest)?;
    let (rest, signature) = take(SIGNATURE_SIZE as usize).parse(rest)?;

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

fn main() {
    println!("Hello, world!");

    let file_name = args().nth(1).expect("No file name given");
    println!("File name: {}", file_name);

    let mut file = std::fs::File::open(file_name).expect("Failed to open file");
    file.seek(std::io::SeekFrom::End(HEADER_SIZE * -1))
        .expect("Failed to seek");

    let mut buffer = [0; HEADER_SIZE as usize];
    let res = file.read(&mut buffer).expect("Failed to read");
    assert_eq!(res as i64, HEADER_SIZE);

    println!("Header: {:#x?}", buffer);

    let (_, header) = header_parser(&buffer).expect("Failed to parse header");
    print!("Header: {:?}", header)
}
