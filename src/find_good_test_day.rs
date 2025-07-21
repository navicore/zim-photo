use anyhow::Result;
use rusqlite::Connection;

pub fn find_test_days(catalog_path: &str) -> Result<()> {
    let conn = Connection::open(catalog_path)?;
    
    // Find days with multiple 4+ star photos
    let query = "
    SELECT 
        substr(folder.pathFromRoot, 1, 10) as day,
        COUNT(*) as photo_count
    FROM AgLibraryFile f
    JOIN AgLibraryFolder folder ON f.folder = folder.id_local
    JOIN Adobe_images i ON f.id_local = i.rootFile
    WHERE i.rating >= 4
        AND folder.pathFromRoot LIKE '____-__-__%'
    GROUP BY day
    HAVING photo_count >= 2
    ORDER BY photo_count DESC
    LIMIT 30";
    
    let mut stmt = conn.prepare(query)?;
    
    println!("Days with MULTIPLE (2+) photos rated 4+ stars:\n");
    println!("{:<12} {:<10}", "Date", "4+ Stars");
    println!("{}", "-".repeat(25));
    
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,     // day
            row.get::<_, i32>(1)?,         // count
        ))
    })?;
    
    for row in rows {
        let (day, count) = row?;
        println!("{:<12} {:<10}", day, count);
    }
    
    println!("\nThese directories would be good for testing AI processing with rating filters.");
    
    Ok(())
}