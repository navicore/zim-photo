# zim-photo Project Context

## Goal
Create a Rust CLI tool that generates markdown sidecar files for photos, similar to zim-studio but for images. The tool should:
- Walk directories looking for image files (DNG, PEF, JPG, TIFF, CR2)
- Generate `.md` sidecar files with YAML frontmatter containing metadata
- Extract metadata from both EXIF and Lightroom catalogs
- Skip files that already have sidecar files
- Handle path format: `YYYY-MM-DD/filename.ext` (e.g., `2019-08-04/EJS20123.CR2`)

## Sidecar Format
```yaml
---
filename: DSC_1234.dng
captured: 2020-03-15T14:30:00Z
camera: "Nikon D750"
lens: "24-70mm f/2.8"
settings:
  iso: 400
  aperture: 5.6
  shutter: "1/250"
  focal_length: 35
lr_rating: 4
lr_color_label: "red"
lr_keywords: [landscape, mountains, sunset]
lr_title: "Sunset over Mt. Rainier"
lr_caption: "Golden hour at Paradise visitor center"
gps:
  latitude: 46.7865
  longitude: -121.7353
  altitude: 1645
location: "Mt. Rainier National Park, WA"
---

# Title

Personal notes...
```

## Lightroom Catalog Location
- Path: `data/lr/`
- Catalog file: `data/lr/lightroom_main.lrcat` (renamed from "Lightroom Catalog.lrcat")
- Format: SQLite database
- Contains: 29.5k RAW, 18.9k DNG, 10.5k TIFF, 878 JPG, 451 PSD files
- Path structure in DB: `YYYY-MM-DD//filename.ext` (note double slash)

## Key Dependencies
- rusqlite: For reading Lightroom catalogs
- rexiv2 or little_exif: For reading image metadata
- walkdir: For directory traversal
- serde/serde_yaml: For YAML generation

## Key Findings
- Camera-generated filenames are mostly unique (can search by filename alone)
- Some duplicates exist (DSCF*.RAF Fuji files)
- Lightroom schema quirks:
  - pick column is REAL not INTEGER
  - IPTC table doesn't have headline/title columns
  - GPS altitude not in AgHarvestedExifMetadata
- Simplified lookup by filename works well