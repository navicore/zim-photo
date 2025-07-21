use anyhow::Result;
use rusqlite::{Connection, params};
use std::collections::HashMap;

/// Find image metadata by filename only (no path needed)
pub fn find_image_by_filename(conn: &Connection, filename: &str) -> Result<Option<HashMap<String, String>>> {
    let mut metadata = HashMap::new();
    
    // Find the image by filename alone
    let mut stmt = conn.prepare("
        SELECT 
            i.id_local,
            i.rating,
            i.colorLabels,
            i.pick,
            folder.pathFromRoot || f.idx_filename as full_path
        FROM Adobe_images i
        JOIN AgLibraryFile f ON i.rootFile = f.id_local
        JOIN AgLibraryFolder folder ON f.folder = folder.id_local
        WHERE f.idx_filename = ?1
    ")?;
    
    let result = stmt.query_row(params![filename], |row| {
        Ok((
            row.get::<_, i32>(0)?,
            row.get::<_, Option<f64>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<f64>>(3)?,
            row.get::<_, String>(4)?
        ))
    });
    
    let (image_id, rating, color_label, pick, full_path) = match result {
        Ok(data) => data,
        Err(_) => return Ok(None),
    };
    
    metadata.insert("lr_id".to_string(), image_id.to_string());
    metadata.insert("lr_rating".to_string(), rating.unwrap_or(0.0).to_string());
    metadata.insert("lr_color_label".to_string(), color_label.unwrap_or_default());
    metadata.insert("lr_pick".to_string(), pick.unwrap_or(0.0).to_string());
    metadata.insert("lr_full_path".to_string(), full_path);
    
    // Get keywords
    let mut stmt = conn.prepare("
        SELECT k.name
        FROM AgLibraryKeywordImage ki
        JOIN AgLibraryKeyword k ON ki.tag = k.id_local
        WHERE ki.image = ?1
    ")?;
    
    let keywords: Vec<String> = stmt.query_map(params![image_id], |row| {
        row.get::<_, String>(0)
    })?.filter_map(Result::ok).collect();
    
    if !keywords.is_empty() {
        metadata.insert("lr_keywords".to_string(), keywords.join(", "));
    }
    
    // Get IPTC data
    let mut stmt = conn.prepare("
        SELECT caption
        FROM AgLibraryIPTC
        WHERE image = ?1
    ")?;
    
    let _ = stmt.query_row(params![image_id], |row| {
        if let Ok(Some(caption)) = row.get::<_, Option<String>>(0) {
            metadata.insert("lr_caption".to_string(), caption);
        }
        Ok(())
    });
    
    // Get GPS data
    let mut stmt = conn.prepare("
        SELECT gpsLatitude, gpsLongitude
        FROM AgHarvestedExifMetadata
        WHERE image = ?1
    ")?;
    
    let _ = stmt.query_row(params![image_id], |row| {
        if let Ok(Some(lat)) = row.get::<_, Option<f64>>(0) {
            metadata.insert("gps_latitude".to_string(), lat.to_string());
        }
        if let Ok(Some(lon)) = row.get::<_, Option<f64>>(1) {
            metadata.insert("gps_longitude".to_string(), lon.to_string());
        }
        Ok(())
    });
    
    Ok(Some(metadata))
}