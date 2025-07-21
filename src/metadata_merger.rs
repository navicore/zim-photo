use anyhow::Result;
use std::collections::HashMap;
use rusqlite::Connection;
use crate::lr_explorer_simple::find_image_by_filename;
use crate::exif_reader::read_exif;
use crate::photo_walker::PhotoFile;
use crate::ollama_vision::{analyze_image, VisionAnalysis};

/// Combined metadata from all sources
#[derive(Debug)]
pub struct PhotoMetadata {
    pub filename: String,
    pub exif_data: HashMap<String, String>,
    pub lightroom_data: HashMap<String, String>,
    pub ai_analysis: Option<VisionAnalysis>,
    pub merged_data: HashMap<String, String>,
}

impl PhotoMetadata {
    pub fn new(filename: String) -> Self {
        PhotoMetadata {
            filename,
            exif_data: HashMap::new(),
            lightroom_data: HashMap::new(),
            ai_analysis: None,
            merged_data: HashMap::new(),
        }
    }
    
    /// Merge data with priority: Lightroom > EXIF > defaults
    pub fn merge(&mut self) {
        // Start with EXIF data
        self.merged_data.extend(self.exif_data.clone());
        
        // Override with Lightroom data (higher priority)
        for (key, value) in &self.lightroom_data {
            // Map Lightroom keys to our standard keys
            match key.as_str() {
                "lr_rating" => {
                    if let Ok(rating) = value.parse::<f64>() {
                        if rating > 0.0 {
                            self.merged_data.insert("rating".to_string(), rating.to_string());
                        }
                    }
                },
                "lr_keywords" => {
                    self.merged_data.insert("keywords".to_string(), value.clone());
                },
                "lr_caption" => {
                    self.merged_data.insert("caption".to_string(), value.clone());
                },
                "lr_title" => {
                    self.merged_data.insert("title".to_string(), value.clone());
                },
                "lr_color_label" => {
                    if !value.is_empty() {
                        self.merged_data.insert("color_label".to_string(), value.clone());
                    }
                },
                "gps_latitude" | "gps_longitude" => {
                    // Lightroom GPS overrides EXIF GPS
                    self.merged_data.insert(key.clone(), value.clone());
                },
                _ => {}
            }
        }
        
        // Always include filename
        self.merged_data.insert("filename".to_string(), self.filename.clone());
        
        // Add AI analysis if available (kept separate from human content)
        if let Some(ref ai) = self.ai_analysis {
            // AI description is always separate from human caption/title
            if !ai.description.is_empty() {
                self.merged_data.insert("ai_description".to_string(), ai.description.clone());
            }
            
            // AI tags are stored separately too
            if !ai.tags.is_empty() {
                self.merged_data.insert("ai_tags".to_string(), ai.tags.join(", "));
            }
        }
        
        // Add source info
        let mut sources = Vec::new();
        if !self.exif_data.is_empty() {
            sources.push("exif");
        }
        if !self.lightroom_data.is_empty() {
            sources.push("lightroom");
        }
        if self.ai_analysis.is_some() {
            sources.push("ai");
        }
        if !sources.is_empty() {
            self.merged_data.insert("metadata_sources".to_string(), sources.join(", "));
        }
    }
    
    /// Generate YAML frontmatter
    pub fn to_yaml_frontmatter(&self) -> Result<String> {
        // Order matters for readability
        let mut ordered_data = serde_yaml::Mapping::new();
        
        // Core identification
        if let Some(v) = self.merged_data.get("filename") {
            ordered_data.insert("filename".into(), v.clone().into());
        }
        
        // Capture info
        if let Some(v) = self.merged_data.get("captured") {
            ordered_data.insert("captured".into(), v.clone().into());
        }
        
        // Camera info
        if let Some(make) = self.merged_data.get("camera_make") {
            if let Some(model) = self.merged_data.get("camera_model") {
                ordered_data.insert("camera".into(), format!("{} {}", make, model).into());
            }
        }
        
        if let Some(v) = self.merged_data.get("lens") {
            ordered_data.insert("lens".into(), v.clone().into());
        }
        
        // Settings
        let mut settings = serde_yaml::Mapping::new();
        if let Some(v) = self.merged_data.get("iso") {
            settings.insert("iso".into(), v.clone().into());
        }
        if let Some(v) = self.merged_data.get("aperture") {
            settings.insert("aperture".into(), v.clone().into());
        }
        if let Some(v) = self.merged_data.get("shutter_speed") {
            settings.insert("shutter".into(), v.clone().into());
        }
        if let Some(v) = self.merged_data.get("focal_length") {
            settings.insert("focal_length".into(), v.clone().into());
        }
        
        if !settings.is_empty() {
            ordered_data.insert("settings".into(), settings.into());
        }
        
        // Lightroom metadata
        if let Some(v) = self.merged_data.get("rating") {
            if let Ok(rating) = v.parse::<f64>() {
                ordered_data.insert("rating".into(), (rating as i64).into());
            }
        }
        
        if let Some(v) = self.merged_data.get("color_label") {
            ordered_data.insert("color_label".into(), v.clone().into());
        }
        
        if let Some(v) = self.merged_data.get("keywords") {
            let keywords: Vec<_> = v.split(", ").collect();
            ordered_data.insert("keywords".into(), keywords.into());
        }
        
        if let Some(v) = self.merged_data.get("title") {
            ordered_data.insert("title".into(), v.clone().into());
        }
        
        if let Some(v) = self.merged_data.get("caption") {
            ordered_data.insert("caption".into(), v.clone().into());
        }
        
        // AI generated content (kept separate)
        if let Some(v) = self.merged_data.get("ai_description") {
            ordered_data.insert("ai_description".into(), v.clone().into());
        }
        
        if let Some(v) = self.merged_data.get("ai_tags") {
            let ai_tags: Vec<_> = v.split(", ").collect();
            ordered_data.insert("ai_tags".into(), ai_tags.into());
        }
        
        // GPS
        if let Some(lat) = self.merged_data.get("gps_latitude") {
            if let Some(lon) = self.merged_data.get("gps_longitude") {
                let mut gps = serde_yaml::Mapping::new();
                gps.insert("latitude".into(), lat.parse::<f64>().unwrap_or(0.0).into());
                gps.insert("longitude".into(), lon.parse::<f64>().unwrap_or(0.0).into());
                
                if let Some(alt) = self.merged_data.get("gps_altitude") {
                    gps.insert("altitude".into(), alt.clone().into());
                }
                
                ordered_data.insert("gps".into(), gps.into());
            }
        }
        
        // Metadata source tracking
        if let Some(v) = self.merged_data.get("metadata_sources") {
            ordered_data.insert("_metadata_sources".into(), v.clone().into());
        }
        
        let yaml = serde_yaml::to_string(&ordered_data)?;
        Ok(format!("---\n{}---\n", yaml))
    }
}

/// Extract and merge metadata for a photo
pub fn extract_metadata(photo: &PhotoFile, lr_conn: Option<&Connection>) -> Result<PhotoMetadata> {
    extract_metadata_verbose(photo, lr_conn, false, false)
}

/// Extract and merge metadata for a photo with optional verbose output and AI analysis
pub fn extract_metadata_verbose(photo: &PhotoFile, lr_conn: Option<&Connection>, verbose: bool, use_ai: bool) -> Result<PhotoMetadata> {
    let mut metadata = PhotoMetadata::new(photo.filename.clone());
    
    // Try to read EXIF data
    match read_exif(&photo.path) {
        Ok(exif_data) => {
            metadata.exif_data = exif_data.to_hashmap();
            if verbose {
                println!("  ✅ Read EXIF data");
            }
        },
        Err(e) => {
            if verbose {
                println!("  ⚠️  Could not read EXIF: {}", e);
            }
        }
    }
    
    // Try to get Lightroom data
    if let Some(conn) = lr_conn {
        match find_image_by_filename(conn, &photo.filename) {
            Ok(Some(lr_data)) => {
                metadata.lightroom_data = lr_data;
                if verbose {
                    println!("  ✅ Found in Lightroom catalog");
                }
            },
            Ok(None) => {
                if verbose {
                    println!("  ℹ️  Not found in Lightroom catalog");
                }
            },
            Err(e) => {
                if verbose {
                    println!("  ⚠️  Lightroom lookup error: {}", e);
                }
            }
        }
    }
    
    // Try AI analysis if requested
    if use_ai {
        match analyze_image(&photo.path) {
            Ok(analysis) => {
                metadata.ai_analysis = Some(analysis);
                if verbose {
                    println!("  ✅ AI vision analysis complete");
                }
            },
            Err(e) => {
                if verbose {
                    println!("  ⚠️  AI analysis failed: {}", e);
                }
            }
        }
    }
    
    // Merge the data
    metadata.merge();
    
    Ok(metadata)
}