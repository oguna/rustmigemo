use std::error::Error;

use rustmigemo::migemo::compact_romaji_processor::build;
fn main() -> Result<(), Box<dyn Error>> {
    build();
    Ok(())
}