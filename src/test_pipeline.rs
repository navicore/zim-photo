use anyhow::Result;
use rusqlite::Connection;
use crate::photo_walker::{PhotoWalker, PhotoFile};
use crate::metadata_merger::extract_metadata_verbose;

pub fn test_pipeline(photo_dir: &str, catalog_path: &str) -> Result<()> {
    println!("🚀 Testing Photo Metadata Pipeline\n");
    
    // Open Lightroom catalog
    let lr_conn = Connection::open(catalog_path)?;
    println!("✅ Connected to Lightroom catalog\n");
    
    // Create walker
    let walker = PhotoWalker::new(photo_dir, true);
    
    // Get stats first
    let stats = walker.get_stats()?;
    stats.print_summary();
    
    // Find a few photos to test
    println!("\n\n🔍 Testing metadata extraction on sample files:");
    println!("{}", "=".repeat(60));
    
    let photos = walker.find_photos()?;
    // Get a mix of file types to test
    let samples: Vec<&PhotoFile> = photos.iter()
        .filter(|p| !p.has_sidecar)
        .take(8)
        .collect();
    
    for photo in samples {
        println!("\n📸 Processing: {}", photo.filename);
        println!("   Path: {}", photo.path.display());
        
        let metadata = extract_metadata_verbose(photo, Some(&lr_conn), true, false)?;
        
        // Show what we found with clear source indicators
        if !metadata.exif_data.is_empty() {
            println!("\n   📷 EXIF data (from actual file):");
            for (key, value) in &metadata.exif_data {
                println!("     {}: {}", key, value);
            }
        } else {
            println!("\n   ❌ No EXIF data found in file");
        }
        
        if !metadata.lightroom_data.is_empty() {
            println!("\n   💾 Lightroom data (from catalog):");
            for (key, value) in &metadata.lightroom_data {
                if key.starts_with("lr_") {
                    println!("     {}: {}", &key[3..], value);
                } else {
                    println!("     {}: {}", key, value);
                }
            }
        } else {
            println!("\n   ❌ Not found in Lightroom catalog");
        }
        
        // Show the generated YAML
        println!("\n   Generated sidecar content:");
        println!("   {}", "-".repeat(40));
        let yaml = metadata.to_yaml_frontmatter()?;
        for line in yaml.lines() {
            println!("   {}", line);
        }
        println!("   {}", "-".repeat(40));
    }
    
    Ok(())
}