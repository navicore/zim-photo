use anyhow::{Result, Context, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use image::{DynamicImage, ImageFormat};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::Path;

const OLLAMA_API_URL: &str = "http://localhost:11434/api/generate";
const MODEL: &str = "llama3.2-vision";

// Standard tags to consider
const STANDARD_TAGS: &str = "landscape, portrait, street photography, nature, sunset, sunrise, \
animal, bird, mountains, ocean, beach, boat, car, tree, flower, people, crowd, pet, city, \
building, macro, insect, computer, electronics, tools, motorcycle, sign, street sign";

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    images: Vec<String>,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
    done: bool,
}

#[derive(Debug, Clone)]
pub struct VisionAnalysis {
    pub description: String,
    pub tags: Vec<String>,
}

/// Analyze an image using Ollama's vision model
pub fn analyze_image<P: AsRef<Path>>(image_path: P) -> Result<VisionAnalysis> {
    let image_path = image_path.as_ref();
    
    // Load and convert image to JPEG
    let jpeg_data = convert_to_jpeg(image_path)?;
    let base64_image = BASE64.encode(&jpeg_data);
    
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
    
    // Create request
    let request = OllamaRequest {
        model: MODEL.to_string(),
        prompt,
        images: vec![base64_image],
        stream: false,
    };
    
    // Send request
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(OLLAMA_API_URL)
        .json(&request)
        .send()
        .context("Failed to send request to Ollama")?;
    
    if !response.status().is_success() {
        return Err(anyhow!("Ollama API error: {}", response.status()));
    }
    
    let ollama_response: OllamaResponse = response
        .json()
        .context("Failed to parse Ollama response")?;
    
    // Parse the response
    parse_vision_response(&ollama_response.response)
}

/// Convert an image to JPEG format in memory
fn convert_to_jpeg<P: AsRef<Path>>(image_path: P) -> Result<Vec<u8>> {
    let image = image::open(image_path.as_ref())
        .with_context(|| format!("Failed to open image: {}", image_path.as_ref().display()))?;
    
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
    
    Ok(buffer.into_inner())
}

/// Parse the model's response into structured data
fn parse_vision_response(response: &str) -> Result<VisionAnalysis> {
    let mut description = String::new();
    let mut tags = Vec::new();
    
    for line in response.lines() {
        let line = line.trim();
        
        if let Some(desc) = line.strip_prefix("DESCRIPTION:") {
            description = desc.trim().to_string();
        } else if let Some(tag_str) = line.strip_prefix("TAGS:") {
            tags = tag_str
                .split(',')
                .map(|t| t.trim().to_lowercase())
                .filter(|t| !t.is_empty())
                .collect();
        }
    }
    
    if description.is_empty() {
        // Fallback: use the entire response as description if parsing fails
        description = response.trim().to_string();
    }
    
    Ok(VisionAnalysis { description, tags })
}

/// Check if Ollama is running and the model is available
pub fn check_ollama_available() -> Result<bool> {
    let client = reqwest::blocking::Client::new();
    
    // Try to connect to Ollama
    match client.get("http://localhost:11434/api/tags").send() {
        Ok(response) if response.status().is_success() => {
            // TODO: Could parse response to check if llama3.2-vision is actually available
            Ok(true)
        }
        _ => Ok(false)
    }
}