# zim-photo

A Rust CLI tool for managing photo metadata through markdown sidecar files. Extract metadata from EXIF, Adobe Lightroom catalogs, and AI vision analysis to create portable, human-readable `.md` files for each photo.

## Features

- **EXIF Extraction**: Read camera settings, GPS data, and timestamps directly from image files
- **Lightroom Integration**: Import ratings, keywords, titles, and captions from `.lrcat` files
- **AI Vision Analysis**: Generate descriptions and tags using Ollama vision models
- **Smart Rating Filters**: Process only your best photos with AI based on star ratings
- **Portable Metadata**: Store all metadata in markdown files with YAML frontmatter
- **Lightroom Independence**: After initial extraction, no longer need Lightroom catalogs

## Supported Formats

- RAW: CR2, DNG, NEF, ARW, ORF, RAF, PEF
- Processed: JPG, JPEG, TIFF, TIF, PSD

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/zim-photo.git
cd zim-photo

# Build with Cargo
cargo build --release

# Optional: Install to PATH
cargo install --path .
```

## Prerequisites

### For AI Features (Optional)
1. Install [Ollama](https://ollama.ai)
2. Pull a vision model:
   ```bash
   ollama pull qwen2.5vl  # Recommended: faster
   # or
   ollama pull llama3.2-vision  # Alternative: larger
   ```
3. Start Ollama:
   ```bash
   ollama serve
   ```

### Required Tools
- `exiftool` - For extracting embedded previews from RAW files

## Usage

### Phase 1: Extract All Metadata (Fast)

Process your entire photo collection to extract EXIF and Lightroom metadata:

```bash
cargo run -- update ~/Photos --catalog ~/lightroom/catalog.lrcat --progress
```

This creates `.md` sidecar files for all photos in seconds. After this completes, you can delete your Lightroom catalog!

### Phase 2: AI Enhancement (Selective)

Add AI-generated descriptions and tags to your best photos:

```bash
# Process only 4+ star photos
cargo run -- update ~/Photos --ai --ai-min-rating 4 --progress --force

# Process only 5-star photos  
cargo run -- update ~/Photos --ai --ai-min-rating 5 --progress --force
```

**Note**: AI processing takes ~50 seconds per image. For 4+ star photos only, this is much more manageable than processing your entire collection.

## Command Reference

### update

Update or create sidecar files for photos in a directory.

```bash
cargo run -- update [OPTIONS] <DIRECTORY>
```

**Options:**
- `-c, --catalog <PATH>` - Path to Lightroom catalog (default: `data/lr/lightroom_main.lrcat`)
- `-p, --progress` - Show progress while processing
- `-f, --force` - Overwrite existing sidecar files
- `-a, --ai` - Enable AI vision analysis
- `--ai-min-rating <N>` - Only use AI for photos rated N stars or higher (1-5)

### find-test-days

Find directories with multiple high-rated photos for testing:

```bash
cargo run -- find-test-days --catalog ~/lightroom/catalog.lrcat
```

## Sidecar File Format

Each photo gets a companion `.md` file (e.g., `IMG_1234.CR2` → `IMG_1234.CR2.md`):

```yaml
---
filename: IMG_1234.CR2
captured: 2020-03-15T14:30:00Z
camera: "Canon EOS 5D Mark III"
lens: "24-70mm f/2.8"
settings:
  iso: 400
  aperture: f/5.6
  shutter: "1/250"
  focal_length: 35mm
rating: 4
keywords: 
  - landscape
  - mountains
  - sunset
title: "Sunset at Mt. Rainier"
caption: "Golden hour at Paradise visitor center"
gps:
  latitude: 46.7865
  longitude: -121.7353
ai_description: "A stunning sunset illuminates Mt. Rainier with golden light, viewed from Paradise visitor center."
ai_tags:
  - mountain
  - sunset
  - landscape
  - nature
  - golden hour
_metadata_sources: exif, lightroom, ai
---

# IMG_1234

<!-- Add your personal notes about this photo here -->
```

## Workflow Example

For a collection of 60,000 photos with ~20,000 rated 4+ stars:

1. **Initial extraction** (all photos, no AI):
   ```bash
   cargo run -- update ~/Photos --catalog ~/Lightroom.lrcat --progress
   # Time: ~10 minutes
   ```

2. **AI enhancement** (4+ stars only):
   ```bash
   cargo run -- update ~/Photos --ai --ai-min-rating 4 --progress --force
   # Time: ~2 weeks (runs continuously)
   ```

3. **Delete Lightroom catalog** - You're now free!

## Tips

- The first AI pass uses ratings from existing sidecar files, not the Lightroom catalog
- Run AI processing on a machine that can stay on continuously
- Consider starting with 5-star photos only for faster initial results
- Use `find-test-days` to test on smaller directories first

## Performance

- Metadata extraction: ~150 files/second
- AI analysis: ~50 seconds/file (varies by model and hardware)
- Supported batch sizes: Tested up to 100,000+ files

## Building from Source

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test
```

## License

[Your chosen license]

## Acknowledgments

- Uses Ollama for local AI vision analysis
- Inspired by the need to escape proprietary photo management software