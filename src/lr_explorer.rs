use anyhow::Result;
use rusqlite::{Connection, params};
use std::collections::HashMap;

pub fn find_image_by_path(conn: &Connection, relative_path: &str) -> Result<Option<HashMap<String, String>>> {
    // Split path like "2019-08-04/EJS20123.CR2" into parts
    let parts: Vec<&str> = relative_path.split('/').collect();
    if parts.len() < 2 {
        return Ok(None);
    }
    
    let folder_name = parts[parts.len() - 2];
    let filename = parts[parts.len() - 1];
    
    println!("  Debug: Looking for filename='{}' in folder='{}'", filename, folder_name);
    
    // First, let's see if this file exists at all
    let mut check_stmt = conn.prepare("
        SELECT COUNT(*) FROM AgLibraryFile 
        WHERE idx_filename = ?1
    ")?;
    let count: i32 = check_stmt.query_row([filename], |row| row.get(0))?;
    println!("  Debug: Found {} files with name '{}'", count, filename);
    
    // Show all paths for this file
    if count > 0 {
        let mut path_stmt = conn.prepare("
            SELECT folder.pathFromRoot
            FROM AgLibraryFile f
            JOIN AgLibraryFolder folder ON f.folder = folder.id_local
            WHERE f.idx_filename = ?1
        ")?;
        let paths: Vec<String> = path_stmt.query_map([filename], |row| {
            row.get::<_, String>(0)
        })?.filter_map(Result::ok).collect();
        
        println!("  Debug: File found in paths:");
        for p in &paths {
            println!("    - '{}'", p);
        }
        
        // Check if it has an Adobe_images record
        let mut img_stmt = conn.prepare("
            SELECT COUNT(*)
            FROM Adobe_images i
            JOIN AgLibraryFile f ON i.rootFile = f.id_local
            WHERE f.idx_filename = ?1
        ")?;
        let img_count: i32 = img_stmt.query_row([filename], |row| row.get(0))?;
        println!("  Debug: Found {} Adobe_images records for this file", img_count);
    }
    
    // Find the image in Lightroom catalog
    let mut stmt = conn.prepare("
        SELECT 
            i.id_local,
            i.rating,
            i.colorLabels,
            i.pick,
            folder.pathFromRoot,
            f.idx_filename
        FROM Adobe_images i
        JOIN AgLibraryFile f ON i.rootFile = f.id_local
        JOIN AgLibraryFolder folder ON f.folder = folder.id_local
        WHERE f.idx_filename = ?1 
        AND folder.pathFromRoot = ?2
    ")?;
    
    // The pathFromRoot in DB already ends with '/', so we match exactly
    let pattern = format!("{}/", folder_name);
    println!("  Debug: Using pattern='{}'", pattern);
    let mut metadata = HashMap::new();
    
    println!("  Debug: Executing query with filename='{}' and pattern='{}'", filename, pattern);
    let result = stmt.query_row(params![filename, pattern], |row| {
        let id = row.get::<_, i32>(0)?;
        let rating = row.get::<_, Option<f64>>(1)?;
        let color_label = row.get::<_, Option<String>>(2)?;
        let pick = row.get::<_, Option<i32>>(3)?;
        let path_root = row.get::<_, String>(4)?;
        let filename = row.get::<_, String>(5)?;
        
        Ok((id, rating, color_label, pick, path_root, filename))
    });
    
    let (image_id, rating, color_label, pick, path_root, filename) = match result {
        Ok(data) => data,
        Err(_) => return Ok(None),
    };
    
    metadata.insert("lr_id".to_string(), image_id.to_string());
    metadata.insert("lr_rating".to_string(), rating.unwrap_or(0.0).to_string());
    metadata.insert("lr_color_label".to_string(), color_label.unwrap_or_default());
    metadata.insert("lr_pick".to_string(), pick.unwrap_or(0).to_string());
    metadata.insert("lr_full_path".to_string(), format!("{}{}", path_root, filename));
    
    // Get keywords
    let mut stmt = conn.prepare("
        SELECT k.name
        FROM AgLibraryKeywordImage ki
        JOIN AgLibraryKeyword k ON ki.tag = k.id_local
        WHERE ki.image = ?1
    ")?;
    
    let keywords: Vec<String> = stmt.query_map([image_id], |row| {
        row.get::<_, String>(0)
    })?.filter_map(Result::ok).collect();
    
    if !keywords.is_empty() {
        metadata.insert("lr_keywords".to_string(), keywords.join(", "));
    }
    
    // Get IPTC data
    let mut stmt = conn.prepare("
        SELECT caption, headline, title
        FROM AgLibraryIPTC
        WHERE image = ?1
    ")?;
    
    let _ = stmt.query_row([image_id], |row| {
        if let Ok(Some(caption)) = row.get::<_, Option<String>>(0) {
            metadata.insert("lr_caption".to_string(), caption);
        }
        if let Ok(Some(headline)) = row.get::<_, Option<String>>(1) {
            metadata.insert("lr_headline".to_string(), headline);
        }
        if let Ok(Some(title)) = row.get::<_, Option<String>>(2) {
            metadata.insert("lr_title".to_string(), title);
        }
        Ok(())
    });
    
    // Get GPS data from exif
    let mut stmt = conn.prepare("
        SELECT gpsLatitude, gpsLongitude, gpsAltitude
        FROM AgHarvestedExifMetadata
        WHERE image = ?1
    ")?;
    
    let _ = stmt.query_row([image_id], |row| {
        if let Ok(Some(lat)) = row.get::<_, Option<f64>>(0) {
            metadata.insert("gps_latitude".to_string(), lat.to_string());
        }
        if let Ok(Some(lon)) = row.get::<_, Option<f64>>(1) {
            metadata.insert("gps_longitude".to_string(), lon.to_string());
        }
        if let Ok(Some(alt)) = row.get::<_, Option<f64>>(2) {
            metadata.insert("gps_altitude".to_string(), alt.to_string());
        }
        Ok(())
    });
    
    Ok(Some(metadata))
}

pub fn explore_catalog(catalog_path: &str) -> Result<()> {
    let conn = Connection::open(catalog_path)?;
    
    // Test with a sample path pattern
    println!("Testing path lookup functionality...\n");
    
    // Let's find some actual paths in the catalog first
    let mut stmt = conn.prepare("
        SELECT DISTINCT
            folder.pathFromRoot,
            f.idx_filename,
            folder.pathFromRoot || '/' || f.idx_filename as full_path
        FROM Adobe_images i
        JOIN AgLibraryFile f ON i.rootFile = f.id_local
        JOIN AgLibraryFolder folder ON f.folder = folder.id_local
        WHERE f.idx_filename = 'imgp1555.pef'
        LIMIT 5
    ")?;
    
    let paths = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?, // pathFromRoot
            row.get::<_, String>(1)?, // filename
            row.get::<_, String>(2)?  // full path
        ))
    })?;
    
    println!("Debug: imgp1555.pef location:");
    for path in paths {
        let (root, filename, full) = path?;
        println!("  Root: '{}'", root);
        println!("  Filename: '{}'", filename);
        println!("  Full: '{}'", full);
    }
    
    // Check what formats are in the catalog
    println!("\n=== File format distribution ===");
    let mut stmt = conn.prepare("
        SELECT fileFormat, COUNT(*) as count
        FROM Adobe_images
        GROUP BY fileFormat
        ORDER BY count DESC
    ")?;
    
    let formats = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i32>(1)?
        ))
    })?;
    
    for fmt in formats {
        let (format, count) = fmt?;
        println!("  {}: {} files", format, count);
    }
    
    Ok(())
}