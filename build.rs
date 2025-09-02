use std::io::Result;
fn main() -> Result<()> {
    prost_build::compile_protos(&["src/proto/extra_metadata.proto"], &["src/proto/"])?;
    Ok(())
}
