use anyhow::{Result, Context};
use std::fs;
use std::path::Path;
use std::time::Instant;
use crate::metadata_merger::PhotoMetadata;

pub struct SidecarWriter {
    pub files_written: usize,
    pub files_skipped: usize,
    pub errors: Vec<String>,
}

impl SidecarWriter {
    pub fn new() -> Self {
        SidecarWriter {
            files_written: 0,
            files_skipped: 0,
            errors: Vec::new(),
        }
    }
    
    /// Write a sidecar .md file for a photo
    pub fn write_sidecar(&mut self, sidecar_path: &Path, metadata: &PhotoMetadata) -> Result<()> {
        // Skip if sidecar already exists
        if sidecar_path.exists() {
            self.files_skipped += 1;
            return Ok(());
        }
        
        // Generate the content
        let yaml_frontmatter = metadata.to_yaml_frontmatter()
            .context("Failed to generate YAML frontmatter")?;
        
        // Add a default title based on filename (can be edited later)
        let title = Path::new(&metadata.filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .replace('_', " ")
            .replace('-', " ");
        
        let content = format!(
            "{}\n# {}\n\n<!-- Add your personal notes about this photo here -->\n",
            yaml_frontmatter,
            title
        );
        
        // Write the file
        fs::write(sidecar_path, content)
            .with_context(|| format!("Failed to write sidecar: {}", sidecar_path.display()))?;
        
        self.files_written += 1;
        Ok(())
    }
    
    pub fn print_summary(&self, elapsed: std::time::Duration) {
        println!("\n📊 Sidecar Generation Summary:");
        println!("  ✅ Files written: {}", self.files_written);
        println!("  ⏭️  Files skipped (already exist): {}", self.files_skipped);
        
        if !self.errors.is_empty() {
            println!("  ❌ Errors: {}", self.errors.len());
            for (i, error) in self.errors.iter().enumerate().take(5) {
                println!("     {}. {}", i + 1, error);
            }
            if self.errors.len() > 5 {
                println!("     ... and {} more", self.errors.len() - 5);
            }
        }
        
        println!("\n⏱️  Performance:");
        println!("  Total time: {:.2}s", elapsed.as_secs_f64());
        
        if self.files_written > 0 {
            let files_per_sec = self.files_written as f64 / elapsed.as_secs_f64();
            let ms_per_file = elapsed.as_millis() as f64 / self.files_written as f64;
            println!("  Files/second: {:.1}", files_per_sec);
            println!("  Time per file: {:.1}ms", ms_per_file);
        }
    }
}

/// Process a directory and generate all sidecar files
pub fn process_directory(
    photo_dir: &str,
    catalog_path: &str,
    skip_existing: bool,
    show_progress: bool,
    use_ai: bool,
) -> Result<()> {
    use crate::photo_walker::PhotoWalker;
    use crate::metadata_merger::extract_metadata_verbose;
    use rusqlite::Connection;
    
    println!("🚀 Starting sidecar generation for: {}\n", photo_dir);
    
    let start_time = Instant::now();
    let mut writer = SidecarWriter::new();
    
    // Open Lightroom catalog
    let lr_conn = Connection::open(catalog_path)?;
    println!("✅ Connected to Lightroom catalog");
    
    // Find all photos
    let walker = PhotoWalker::new(photo_dir, skip_existing);
    let photos = walker.find_photos()?;
    let total_photos = photos.len();
    
    println!("📸 Found {} photos to process", total_photos);
    if use_ai {
        println!("🤖 AI vision analysis enabled (this will be slower)");
    }
    println!();
    
    // Process each photo
    for (index, photo) in photos.iter().enumerate() {
        if show_progress && (index % 10 == 0 || index == total_photos - 1) {
            print!("\r  Processing: {}/{} ({:.1}%)", 
                index + 1, 
                total_photos, 
                (index + 1) as f64 / total_photos as f64 * 100.0
            );
            use std::io::{self, Write};
            io::stdout().flush()?;
        }
        
        // Extract metadata
        match extract_metadata_verbose(photo, Some(&lr_conn), false, use_ai) {
            Ok(metadata) => {
                // Write sidecar
                if let Err(e) = writer.write_sidecar(&photo.sidecar_path, &metadata) {
                    writer.errors.push(format!("{}: {}", photo.filename, e));
                }
            }
            Err(e) => {
                writer.errors.push(format!("{}: Failed to extract metadata: {}", photo.filename, e));
            }
        }
    }
    
    if show_progress {
        println!(); // New line after progress
    }
    
    let elapsed = start_time.elapsed();
    writer.print_summary(elapsed);
    
    Ok(())
}