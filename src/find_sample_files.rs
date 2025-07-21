use anyhow::Result;
use rusqlite::Connection;

pub fn find_sample_files(catalog_path: &str) -> Result<()> {
    let conn = Connection::open(catalog_path)?;
    
    println!("=== Finding Sample Files for Testing ===\n");
    
    // Find some files with ratings
    println!("📸 Files with 4+ star ratings:");
    let mut stmt = conn.prepare("
        SELECT f.idx_filename, i.rating, folder.pathFromRoot
        FROM Adobe_images i
        JOIN AgLibraryFile f ON i.rootFile = f.id_local
        JOIN AgLibraryFolder folder ON f.folder = folder.id_local
        WHERE i.rating >= 4
        LIMIT 5
    ")?;
    
    let rated_files = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, f64>(1)?,
            row.get::<_, String>(2)?
        ))
    })?;
    
    for file in rated_files {
        let (filename, rating, path) = file?;
        println!("  - {} (★{}) in {}", filename, rating as i32, path);
    }
    
    // Find files with keywords
    println!("\n📸 Files with keywords:");
    let mut stmt = conn.prepare("
        SELECT DISTINCT f.idx_filename, COUNT(k.name) as keyword_count
        FROM Adobe_images i
        JOIN AgLibraryFile f ON i.rootFile = f.id_local
        JOIN AgLibraryKeywordImage ki ON ki.image = i.id_local
        JOIN AgLibraryKeyword k ON ki.tag = k.id_local
        GROUP BY f.idx_filename
        ORDER BY keyword_count DESC
        LIMIT 5
    ")?;
    
    let keyword_files = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i32>(1)?
        ))
    })?;
    
    for file in keyword_files {
        let (filename, count) = file?;
        println!("  - {} ({} keywords)", filename, count);
    }
    
    // Find files by type
    println!("\n📸 Sample files by type:");
    let types = ["%.CR2", "%.NEF", "%.DNG", "%.PEF", "%.JPG", "%.RAF"];
    
    for pattern in &types {
        let mut stmt = conn.prepare("
            SELECT f.idx_filename
            FROM Adobe_images i
            JOIN AgLibraryFile f ON i.rootFile = f.id_local
            WHERE f.idx_filename LIKE ?1
            LIMIT 2
        ")?;
        
        let files: Vec<String> = stmt.query_map([pattern], |row| {
            row.get::<_, String>(0)
        })?.filter_map(Result::ok).collect();
        
        if !files.is_empty() {
            println!("  {} files: {:?}", &pattern[2..], files);
        }
    }
    
    Ok(())
}