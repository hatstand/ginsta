use nom::{
    Parser,
    number::{be_f64, be_i64, be_u64, le_f64, le_i64, le_u64},
};
use std::env::args;

fn try_various_parsers_64(
    bytes: &[u8],
) -> (
    Option<f64>,
    Option<f64>,
    Option<u64>,
    Option<u64>,
    Option<i64>,
    Option<i64>,
) {
    let float64_le = le_f64::<&[u8], nom::error::Error<&[u8]>>()
        .parse(bytes)
        .ok()
        .map(|(_, v)| v);
    let float64_be = be_f64::<&[u8], nom::error::Error<&[u8]>>()
        .parse(bytes)
        .ok()
        .map(|(_, v)| v);
    let uint64_le = le_u64::<&[u8], nom::error::Error<&[u8]>>()
        .parse(bytes)
        .ok()
        .map(|(_, v)| v);
    let uint64_be = be_u64::<&[u8], nom::error::Error<&[u8]>>()
        .parse(bytes)
        .ok()
        .map(|(_, v)| v);
    let int64_le = le_i64::<&[u8], nom::error::Error<&[u8]>>()
        .parse(bytes)
        .ok()
        .map(|(_, v)| v);
    let int64_be = be_i64::<&[u8], nom::error::Error<&[u8]>>()
        .parse(bytes)
        .ok()
        .map(|(_, v)| v);

    (
        float64_le, float64_be, uint64_le, uint64_be, int64_le, int64_be,
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let hex_data = args().nth(1).expect("No hex data given");
    let data = hex::decode(hex_data).expect("Failed to decode hex data");

    if data.len() == 8 {
        let (lef64, bef64, leu64, beu64, lei64, bei64) = try_various_parsers_64(data.as_slice());
        println!("64-bit float: LE: {:?}, BE: {:?}", lef64, bef64);
        println!("64-bit uint:  LE: {:?}, BE: {:?}", leu64, beu64);
        println!("64-bit int:   LE: {:?}, BE: {:?}", lei64, bei64);
    }

    Ok(())
}
