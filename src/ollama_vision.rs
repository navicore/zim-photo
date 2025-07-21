use anyhow::{Result, Context, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use image::ImageFormat;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::Path;

const OLLAMA_API_URL: &str = "http://localhost:11434/api/chat";
const MODEL: &str = "qwen2.5vl";

// Standard tags to consider
const STANDARD_TAGS: &str = "landscape, portrait, street photography, nature, sunset, sunrise, \
animal, bird, mountains, ocean, beach, boat, car, tree, flower, people, crowd, pet, city, \
building, macro, insect, computer, electronics, tools, motorcycle, sign, street sign";

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
    images: Vec<String>,
}

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    message: MessageResponse,
    done: bool,
}

#[derive(Debug, Deserialize)]
struct MessageResponse {
    content: String,
}

#[derive(Debug, Clone)]
pub struct VisionAnalysis {
    pub description: String,
    pub tags: Vec<String>,
}

/// Analyze an image using Ollama's vision model
pub fn analyze_image<P: AsRef<Path>>(image_path: P) -> Result<VisionAnalysis> {
    let image_path = image_path.as_ref();
    println!("  🔍 Analyzing image with AI: {}", image_path.display());
    
    // Load and convert image to JPEG
    let jpeg_data = convert_to_jpeg(image_path)?;
    println!("  📊 JPEG data size: {} bytes", jpeg_data.len());
    let base64_image = BASE64.encode(&jpeg_data);
    println!("  📊 Base64 size: {} chars", base64_image.len());
    
    // Craft the prompt
    let prompt = format!(
        "Analyze this photograph and provide:

1. A brief, descriptive caption (1-2 sentences) that captures what's shown in the image
2. A list of relevant tags for searching and categorization

When tagging, consider these standard categories if applicable: {}

Also add any other specific, relevant tags that would help someone find this image later. 
Keep tags concise (1-2 words each) and focus on observable content, style, and mood.

Format your response as:
DESCRIPTION: [your caption here]
TAGS: [comma-separated list of tags]

Be specific and accurate. Focus on what's actually visible in the image.",
        STANDARD_TAGS
    );
    
    // Create request using chat format
    let message = Message {
        role: "user".to_string(),
        content: prompt,
        images: vec![base64_image],
    };
    
    let request = OllamaRequest {
        model: MODEL.to_string(),
        messages: vec![message],
        stream: false,
    };
    
    // Send request with longer timeout for vision models
    println!("  📤 Sending request to Ollama (this may take a while)...");
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(600))  // 10 minutes - vision models can be slow
        .build()?;
    let response = client
        .post(OLLAMA_API_URL)
        .json(&request)
        .send()
        .context("Failed to send request to Ollama")?;
    
    if !response.status().is_success() {
        return Err(anyhow!("Ollama API error: {}", response.status()));
    }
    
    let response_text = response.text()?;
    println!("  📥 Raw Ollama response: {}", &response_text[..200.min(response_text.len())]);
    
    let ollama_response: OllamaResponse = serde_json::from_str(&response_text)
        .context("Failed to parse Ollama response")?;
    
    println!("  📝 Parsed response: {}", &ollama_response.message.content[..200.min(ollama_response.message.content.len())]);
    
    // Parse the response
    parse_vision_response(&ollama_response.message.content)
}

/// Convert an image to JPEG format in memory
fn convert_to_jpeg<P: AsRef<Path>>(image_path: P) -> Result<Vec<u8>> {
    let path = image_path.as_ref();
    println!("  🖼️  Converting image to JPEG: {}", path.display());
    
    // Check if this is a RAW format that the image crate can't handle
    let extension = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    
    let jpeg_bytes = match extension.as_str() {
        "cr2" | "dng" | "nef" | "arw" | "orf" | "raf" | "pef" => {
            // Try to extract embedded JPEG from RAW file
            extract_jpeg_from_raw(path)?
        }
        _ => {
            // Use image crate for supported formats
            let image = image::open(path)
                .with_context(|| format!("Failed to open image: {}", path.display()))?;
            
            // Resize if too large (Ollama handles better with reasonable sizes)
            let resized = if image.width() > 2048 || image.height() > 2048 {
                image.resize(2048, 2048, image::imageops::FilterType::Lanczos3)
            } else {
                image
            };
            
            // Convert to JPEG
            let mut buffer = Cursor::new(Vec::new());
            resized.write_to(&mut buffer, ImageFormat::Jpeg)
                .context("Failed to encode image as JPEG")?;
            
            buffer.into_inner()
        }
    };
    
    Ok(jpeg_bytes)
}

/// Extract embedded JPEG from RAW file using dcraw or exiftool
fn extract_jpeg_from_raw<P: AsRef<Path>>(raw_path: P) -> Result<Vec<u8>> {
    let path = raw_path.as_ref();
    
    // First try with dcraw (if available)
    if let Ok(output) = std::process::Command::new("dcraw")
        .args(&["-e", "-c", path.to_str().unwrap()])
        .output()
    {
        if output.status.success() && !output.stdout.is_empty() {
            println!("  ✅ Extracted embedded JPEG using dcraw ({} bytes)", output.stdout.len());
            return resize_jpeg_if_needed(output.stdout);
        }
    }
    
    // Try with exiftool (more commonly available)
    if let Ok(output) = std::process::Command::new("exiftool")
        .args(&["-b", "-PreviewImage", path.to_str().unwrap()])
        .output()
    {
        if output.status.success() && !output.stdout.is_empty() {
            println!("  ✅ Extracted preview image using exiftool ({} bytes)", output.stdout.len());
            return resize_jpeg_if_needed(output.stdout);
        }
    }
    
    // Try to get JPEG thumbnail as last resort
    if let Ok(output) = std::process::Command::new("exiftool")
        .args(&["-b", "-ThumbnailImage", path.to_str().unwrap()])
        .output()
    {
        if output.status.success() && !output.stdout.is_empty() {
            println!("  ⚠️  Using thumbnail image (lower quality, {} bytes)", output.stdout.len());
            return resize_jpeg_if_needed(output.stdout);
        }
    }
    
    Err(anyhow!(
        "Unable to extract JPEG from RAW file. Please install dcraw or ensure exiftool is available."
    ))
}

/// Resize JPEG data if it's too large for Ollama
fn resize_jpeg_if_needed(jpeg_data: Vec<u8>) -> Result<Vec<u8>> {
    // Load the JPEG
    let img = image::load_from_memory(&jpeg_data)
        .context("Failed to load extracted JPEG")?;
    
    // Resize if too large (be less aggressive - Ollama can handle larger images)
    let resized = if img.width() > 2048 || img.height() > 2048 {
        println!("  📏 Resizing from {}x{} to fit 2048x2048", img.width(), img.height());
        img.resize(2048, 2048, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };
    
    // Convert back to JPEG
    let mut buffer = Cursor::new(Vec::new());
    resized.write_to(&mut buffer, ImageFormat::Jpeg)
        .context("Failed to encode resized image as JPEG")?;
    
    let result = buffer.into_inner();
    println!("  📦 Final JPEG size: {} bytes", result.len());
    Ok(result)
}

/// Parse the model's response into structured data
fn parse_vision_response(response: &str) -> Result<VisionAnalysis> {
    let mut description = String::new();
    let mut tags = Vec::new();
    
    for line in response.lines() {
        let line = line.trim();
        
        if let Some(desc) = line.strip_prefix("DESCRIPTION:") {
            description = desc.trim().to_string();
            println!("  ✅ Found description: {}", description);
        } else if let Some(tag_str) = line.strip_prefix("TAGS:") {
            tags = tag_str
                .split(',')
                .map(|t| t.trim().to_lowercase())
                .filter(|t| !t.is_empty())
                .collect();
            println!("  ✅ Found {} tags", tags.len());
        }
    }
    
    if description.is_empty() {
        // Fallback: use the entire response as description if parsing fails
        description = response.trim().to_string();
        println!("  ⚠️  Using fallback description (parsing failed)");
    }
    
    println!("  📋 Final: desc={} chars, {} tags", description.len(), tags.len());
    Ok(VisionAnalysis { description, tags })
}

/// Check if Ollama is running and the model is available
pub fn check_ollama_available() -> Result<bool> {
    let client = reqwest::blocking::Client::new();
    
    // Try to connect to Ollama
    match client.get("http://localhost:11434/api/tags").send() {
        Ok(response) if response.status().is_success() => {
            // TODO: Could parse response to check if qwen2.5vl is actually available
            Ok(true)
        }
        _ => Ok(false)
    }
}