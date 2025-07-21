use anyhow::Result;
use clap::{Parser, Subcommand};

mod lr_explorer;
mod lr_explorer_simple;
mod test_lookup;
mod find_sample_files;
mod photo_walker;
mod exif_reader;
mod metadata_merger;
mod test_pipeline;
mod test_single_file;
mod sidecar_writer;
mod sidecar_reader;
mod ollama_vision;
mod find_good_test_day;

#[derive(Parser)]
#[command(name = "zim-photo")]
#[command(about = "Photo metadata management tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Update sidecar files for photos in a directory
    Update {
        /// Directory containing photos
        #[arg(default_value = ".")]
        directory: String,
        
        /// Path to Lightroom catalog
        #[arg(short, long, default_value = "data/lr/lightroom_main.lrcat")]
        catalog: String,
        
        /// Show progress while processing
        #[arg(short, long)]
        progress: bool,
        
        /// Process all files (don't skip existing sidecars)
        #[arg(short, long)]
        force: bool,
        
        /// Use AI (Ollama) to generate descriptions and tags
        #[arg(short, long)]
        ai: bool,
        
        /// Minimum rating for AI analysis (1-5)
        #[arg(long, help = "Only use AI for photos with this rating or higher")]
        ai_min_rating: Option<i32>,
    },
    
    /// Test the metadata pipeline
    Test {
        /// Directory to test
        #[arg(default_value = "tmp")]
        directory: String,
    },
    
    /// Find days with multiple high-rated photos for testing
    FindTestDays {
        /// Path to Lightroom catalog
        #[arg(short, long, default_value = "data/lr/lightroom_main.lrcat")]
        catalog: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let catalog_path = "data/lr/lightroom_main.lrcat";
    
    match cli.command {
        Commands::Update { directory, catalog, progress, force, ai, ai_min_rating } => {
            if ai {
                // Check if Ollama is available
                if !ollama_vision::check_ollama_available()? {
                    println!("⚠️  Warning: Ollama is not running or qwen2.5vl is not available");
                    println!("   Make sure Ollama is running: ollama serve");
                    println!("   And the model is pulled: ollama pull qwen2.5vl");
                    return Ok(());
                }
                if let Some(rating) = ai_min_rating {
                    println!("🎯 AI analysis enabled for photos with rating ≥ {}", rating);
                }
            }
            sidecar_writer::process_directory(&directory, &catalog, !force, progress, ai, ai_min_rating)?;
        }
        Commands::Test { directory } => {
            if std::path::Path::new(&directory).exists() {
                test_pipeline::test_pipeline(&directory, catalog_path)?;
            } else {
                println!("⚠️  Test directory '{}' not found", directory);
                test_lookup::test_multiple_lookups(catalog_path)?;
            }
        }
        Commands::FindTestDays { catalog } => {
            find_good_test_day::find_test_days(&catalog)?;
        }
    }
    
    Ok(())
}