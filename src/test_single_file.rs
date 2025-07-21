use anyhow::Result;
use crate::exif_reader::read_exif;
use std::path::Path;

pub fn test_single_file(file_path: &str) -> Result<()> {
    println!("Testing EXIF extraction for: {}", file_path);
    
    match read_exif(Path::new(file_path)) {
        Ok(exif_data) => {
            println!("✅ Successfully read EXIF data!");
            let data = exif_data.to_hashmap();
            for (key, value) in &data {
                println!("  {}: {}", key, value);
            }
        },
        Err(e) => {
            println!("❌ Failed to read EXIF: {}", e);
        }
    }
    
    Ok(())
}