use crate::lr_explorer_simple::find_image_by_filename;
use anyhow::Result;
use rusqlite::Connection;

pub fn test_multiple_lookups(catalog_path: &str) -> Result<()> {
    let conn = Connection::open(catalog_path)?;

    // Array of test filenames - modify these to test your own files
    let test_files = [
        // Files with ratings
        "_1EJ7478-Edit-2-Edit.tif",
        "merged_hawk_6_tonemapped-1.tif",
        // Files with many keywords
        "_2EJ4717.DNG",
        "HE1A5905.CR2",
        "HE1A8665.CR2",
        // Various formats
        "imgp1555.pef", // Pentax
        "HE1A0901.CR2", // Canon
        "_DSF0055.RAF", // Fuji
        "DSCF0010.RAF", // Fuji (has rating)
        "IMG_0001.JPG", // JPG with GPS
        // from nas
        "HE1A7921.CR2",
        "HE1A7922.CR2",
        "HE1A7920.CR2",
        "HE1A7924.CR2",
        "HE1A7774.CR2",
        "HE1A7610.CR2",
        "HE1A7611.CR2",
        "HE1A7561.CR2",
        "HE1A7876.CR2",
        // One that doesn't exist
        "NOTFOUND.CR2",
    ];

    println!("=== Testing Multiple File Lookups ===\n");

    for filename in &test_files {
        println!("📷 Looking up: {}", filename);
        println!("{}", "-".repeat(60));

        match find_image_by_filename(&conn, filename)? {
            Some(metadata) => {
                // Pretty print the metadata
                if let Some(path) = metadata.get("lr_full_path") {
                    println!("  📁 Path: {}", path);
                }

                // Rating and labels
                if let Some(rating) = metadata.get("lr_rating") {
                    let rating_num: f64 = rating.parse().unwrap_or(0.0);
                    if rating_num > 0.0 {
                        let stars = "⭐".repeat(rating_num as usize);
                        println!("  ⭐ Rating: {}", stars);
                    }
                }

                if let Some(color) = metadata.get("lr_color_label") {
                    if !color.is_empty() {
                        println!("  🏷️  Color Label: {}", color);
                    }
                }

                if let Some(pick) = metadata.get("lr_pick") {
                    let pick_val: f64 = pick.parse().unwrap_or(0.0);
                    if pick_val > 0.0 {
                        println!("  ✅ Pick Status: {}", pick);
                    }
                }

                // Keywords
                if let Some(keywords) = metadata.get("lr_keywords") {
                    println!("  🏷️  Keywords: {}", keywords);
                }

                // Caption
                if let Some(caption) = metadata.get("lr_caption") {
                    println!("  📝 Caption: {}", caption);
                }

                // GPS
                if metadata.contains_key("gps_latitude") && metadata.contains_key("gps_longitude") {
                    let lat = metadata.get("gps_latitude").unwrap();
                    let lon = metadata.get("gps_longitude").unwrap();
                    println!("  📍 GPS: {}, {}", lat, lon);
                }

                // Internal ID (useful for debugging)
                if let Some(id) = metadata.get("lr_id") {
                    println!("  🔑 Lightroom ID: {}", id);
                }
            }
            None => {
                println!("  ❌ Not found in catalog");
            }
        }
        println!();
    }

    // Also show some statistics
    println!("\n=== Catalog Statistics ===");

    // Count total images
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM Adobe_images")?;
    let total_images: i32 = stmt.query_row([], |row| row.get(0))?;
    println!("📊 Total images: {}", total_images);

    // Count rated images
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM Adobe_images WHERE rating > 0")?;
    let rated_images: i32 = stmt.query_row([], |row| row.get(0))?;
    println!("⭐ Rated images: {}", rated_images);

    // Count images with keywords
    let mut stmt = conn.prepare(
        "
        SELECT COUNT(DISTINCT image) 
        FROM AgLibraryKeywordImage
    ",
    )?;
    let keyworded_images: i32 = stmt.query_row([], |row| row.get(0))?;
    println!("🏷️  Images with keywords: {}", keyworded_images);

    // Count images with GPS
    let mut stmt = conn.prepare(
        "
        SELECT COUNT(*) 
        FROM AgHarvestedExifMetadata 
        WHERE gpsLatitude IS NOT NULL
    ",
    )?;
    let gps_images: i32 = stmt.query_row([], |row| row.get(0))?;
    println!("📍 Images with GPS: {}", gps_images);

    Ok(())
}
