# insgps Format

The `insgps` format consists of a sequence of GPS records, each with the following binary structure:

All fields are Little Endian.

| Field         | Type    | Description                        |
|---------------|---------|------------------------------------|
| timestamp     | u32     | Seconds since epoch                |
| slope?        | [u8; 7] | 7 bytes, purpose unknown/ignored   |
| latitude      | f64     | Latitude in degrees                |
| north_south   | u8      | ASCII 'N' or 'S'                   |
| longitude     | f64     | Longitude in degrees               |
| east_west     | u8      | ASCII 'E' or 'W'                   |
| speed         | f64     | Meters per second                  |
| track         | f64     | Track bearing                      |
| altitude      | f64     | Altitude in meters                 |

- If `north_south` is 'S', latitude should be negated.
- If `east_west` is 'W', longitude should be negated.
- Records are packed sequentially with no delimiter.
- The file ends when all records are read.
