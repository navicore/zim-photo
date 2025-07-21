use anyhow::Result;
use walkdir::WalkDir;
use std::path::{Path, PathBuf};
use std::collections::HashSet;

/// Supported image extensions
const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "JPG", "JPEG",
    "cr2", "CR2",  // Canon RAW
    "nef", "NEF",  // Nikon RAW
    "dng", "DNG",  // Adobe Digital Negative
    "pef", "PEF",  // Pentax RAW
    "raf", "RAF",  // Fuji RAW
    "tif", "tiff", "TIF", "TIFF",
    "png", "PNG",
];

#[derive(Debug, Clone)]
pub struct PhotoFile {
    pub path: PathBuf,
    pub filename: String,
    pub extension: String,
    pub sidecar_path: PathBuf,
    pub has_sidecar: bool,
}

impl PhotoFile {
    pub fn new(path: PathBuf) -> Option<Self> {
        let filename = path.file_name()?.to_str()?.to_string();
        let extension = path.extension()?.to_str()?.to_string();
        
        // Check if this is a supported image type
        if !IMAGE_EXTENSIONS.contains(&extension.as_str()) {
            return None;
        }
        
        // Generate sidecar path: append .md to the full filename
        let sidecar_path = path.with_extension(format!("{}.md", extension));
        
        let has_sidecar = sidecar_path.exists();
        
        Some(PhotoFile {
            path,
            filename,
            extension,
            sidecar_path,
            has_sidecar,
        })
    }
}

pub struct PhotoWalker {
    root_path: PathBuf,
    skip_existing: bool,
}

impl PhotoWalker {
    pub fn new<P: AsRef<Path>>(root_path: P, skip_existing: bool) -> Self {
        PhotoWalker {
            root_path: root_path.as_ref().to_path_buf(),
            skip_existing,
        }
    }
    
    /// Walk directory tree and find all image files
    pub fn find_photos(&self) -> Result<Vec<PhotoFile>> {
        let mut photos = Vec::new();
        let mut seen_files = HashSet::new();
        
        for entry in WalkDir::new(&self.root_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            
            // Skip hidden files and directories
            if path.to_str().map_or(false, |s| s.contains("/.")) {
                continue;
            }
            
            // Skip @eaDir (Synology metadata directories)
            if path.to_str().map_or(false, |s| s.contains("@eaDir")) {
                continue;
            }
            
            if let Some(photo) = PhotoFile::new(path.to_path_buf()) {
                // Skip if we already have a sidecar and skip_existing is true
                if self.skip_existing && photo.has_sidecar {
                    println!("  ⏭️  Skipping {} (sidecar exists)", photo.filename);
                    continue;
                }
                
                // Track filename to identify duplicates
                if !seen_files.insert(photo.filename.clone()) {
                    println!("  ⚠️  Duplicate filename: {}", photo.filename);
                }
                
                photos.push(photo);
            }
        }
        
        photos.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(photos)
    }
    
    /// Get statistics about the photo collection
    pub fn get_stats(&self) -> Result<PhotoStats> {
        let photos = self.find_photos()?;
        let mut stats = PhotoStats::default();
        
        for photo in &photos {
            stats.total_files += 1;
            
            if photo.has_sidecar {
                stats.with_sidecars += 1;
            }
            
            *stats.by_extension.entry(photo.extension.clone()).or_insert(0) += 1;
        }
        
        stats.without_sidecars = stats.total_files - stats.with_sidecars;
        Ok(stats)
    }
}

#[derive(Debug, Default)]
pub struct PhotoStats {
    pub total_files: usize,
    pub with_sidecars: usize,
    pub without_sidecars: usize,
    pub by_extension: std::collections::HashMap<String, usize>,
}

impl PhotoStats {
    pub fn print_summary(&self) {
        println!("\n📊 Photo Collection Statistics:");
        println!("  Total files: {}", self.total_files);
        println!("  With sidecars: {} ✅", self.with_sidecars);
        println!("  Need processing: {} 🔄", self.without_sidecars);
        
        println!("\n📸 By format:");
        let mut extensions: Vec<_> = self.by_extension.iter().collect();
        extensions.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
        
        for (ext, count) in extensions {
            println!("  {}: {} files", ext.to_uppercase(), count);
        }
    }
}