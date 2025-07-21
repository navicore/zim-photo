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
mod ollama_vision;

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
    },
    
    /// Test the metadata pipeline
    Test {
        /// Directory to test
        #[arg(default_value = "tmp")]
        directory: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let catalog_path = "data/lr/lightroom_main.lrcat";
    
    match cli.command {
        Commands::Update { directory, catalog, progress, force, ai } => {
            if ai {
                // Check if Ollama is available
                if !ollama_vision::check_ollama_available()? {
                    println!("⚠️  Warning: Ollama is not running or llama3.2-vision is not available");
                    println!("   Make sure Ollama is running: ollama serve");
                    println!("   And the model is pulled: ollama pull llama3.2-vision");
                    return Ok(());
                }
            }
            sidecar_writer::process_directory(&directory, &catalog, !force, progress, ai)?;
        }
        Commands::Test { directory } => {
            if std::path::Path::new(&directory).exists() {
                test_pipeline::test_pipeline(&directory, catalog_path)?;
            } else {
                println!("⚠️  Test directory '{}' not found", directory);
                test_lookup::test_multiple_lookups(catalog_path)?;
            }
        }
    }
    
    Ok(())
}