use anyhow::{Result, Context};
use std::path::Path;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use exif::{In, Tag, Value};
use chrono::{DateTime, NaiveDateTime, Local};

pub struct ExifData {
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub lens_info: Option<String>,
    pub date_taken: Option<DateTime<Local>>,
    pub iso: Option<u32>,
    pub aperture: Option<f64>,
    pub shutter_speed: Option<String>,
    pub focal_length: Option<f64>,
    pub gps_latitude: Option<f64>,
    pub gps_longitude: Option<f64>,
    pub gps_altitude: Option<f64>,
}

impl ExifData {
    pub fn to_hashmap(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        
        if let Some(make) = &self.camera_make {
            map.insert("camera_make".to_string(), make.clone());
        }
        if let Some(model) = &self.camera_model {
            map.insert("camera_model".to_string(), model.clone());
        }
        if let Some(lens) = &self.lens_info {
            map.insert("lens".to_string(), lens.clone());
        }
        if let Some(date) = &self.date_taken {
            map.insert("captured".to_string(), date.format("%Y-%m-%dT%H:%M:%S").to_string());
        }
        if let Some(iso) = self.iso {
            map.insert("iso".to_string(), iso.to_string());
        }
        if let Some(aperture) = self.aperture {
            map.insert("aperture".to_string(), format!("f/{:.1}", aperture));
        }
        if let Some(shutter) = &self.shutter_speed {
            map.insert("shutter_speed".to_string(), shutter.clone());
        }
        if let Some(focal) = self.focal_length {
            map.insert("focal_length".to_string(), format!("{}mm", focal));
        }
        if let Some(lat) = self.gps_latitude {
            map.insert("gps_latitude".to_string(), lat.to_string());
        }
        if let Some(lon) = self.gps_longitude {
            map.insert("gps_longitude".to_string(), lon.to_string());
        }
        if let Some(alt) = self.gps_altitude {
            map.insert("gps_altitude".to_string(), format!("{}m", alt));
        }
        
        map
    }
}

pub fn read_exif<P: AsRef<Path>>(path: P) -> Result<ExifData> {
    let file = File::open(&path)
        .with_context(|| format!("Failed to open file: {}", path.as_ref().display()))?;
    let mut reader = BufReader::new(file);
    
    let exif_reader = exif::Reader::new();
    let exif = exif_reader.read_from_container(&mut reader)
        .with_context(|| format!("Failed to read EXIF from: {}", path.as_ref().display()))?;
    
    let mut data = ExifData {
        camera_make: None,
        camera_model: None,
        lens_info: None,
        date_taken: None,
        iso: None,
        aperture: None,
        shutter_speed: None,
        focal_length: None,
        gps_latitude: None,
        gps_longitude: None,
        gps_altitude: None,
    };
    
    for field in exif.fields() {
        match field.tag {
            Tag::Make => data.camera_make = field.display_value().to_string().into(),
            Tag::Model => data.camera_model = field.display_value().to_string().into(),
            Tag::LensModel => data.lens_info = field.display_value().to_string().into(),
            Tag::DateTime | Tag::DateTimeOriginal | Tag::DateTimeDigitized => {
                if data.date_taken.is_none() {
                    data.date_taken = parse_exif_datetime(&field.display_value().to_string());
                }
            },
            Tag::ISOSpeed => {
                if let Value::Short(ref vals) = field.value {
                    if let Some(&iso) = vals.first() {
                        data.iso = Some(iso as u32);
                    }
                }
            },
            Tag::FNumber | Tag::ApertureValue => {
                if let Value::Rational(ref vals) = field.value {
                    if let Some(ratio) = vals.first() {
                        data.aperture = Some(ratio.num as f64 / ratio.denom as f64);
                    }
                }
            },
            Tag::ExposureTime => {
                if let Value::Rational(ref vals) = field.value {
                    if let Some(ratio) = vals.first() {
                        if ratio.denom == 1 {
                            data.shutter_speed = Some(format!("{}", ratio.num));
                        } else {
                            data.shutter_speed = Some(format!("{}/{}", ratio.num, ratio.denom));
                        }
                    }
                }
            },
            Tag::FocalLength => {
                if let Value::Rational(ref vals) = field.value {
                    if let Some(ratio) = vals.first() {
                        data.focal_length = Some(ratio.num as f64 / ratio.denom as f64);
                    }
                }
            },
            _ => {}
        }
    }
    
    // GPS data extraction
    if let Some(lat) = get_gps_coordinate(&exif, Tag::GPSLatitude, Tag::GPSLatitudeRef) {
        data.gps_latitude = Some(lat);
    }
    if let Some(lon) = get_gps_coordinate(&exif, Tag::GPSLongitude, Tag::GPSLongitudeRef) {
        data.gps_longitude = Some(lon);
    }
    if let Some(alt_field) = exif.get_field(Tag::GPSAltitude, In::PRIMARY) {
        if let Value::Rational(ref vals) = alt_field.value {
            if let Some(ratio) = vals.first() {
                data.gps_altitude = Some(ratio.num as f64 / ratio.denom as f64);
            }
        }
    }
    
    Ok(data)
}

fn parse_exif_datetime(datetime_str: &str) -> Option<DateTime<Local>> {
    // EXIF datetime format: "2023:08:15 14:30:45"
    let cleaned = datetime_str.trim_matches('"');
    NaiveDateTime::parse_from_str(cleaned, "%Y:%m:%d %H:%M:%S")
        .ok()
        .map(|naive| DateTime::from_naive_utc_and_offset(naive, *Local::now().offset()))
}

fn get_gps_coordinate(exif: &exif::Exif, coord_tag: Tag, ref_tag: Tag) -> Option<f64> {
    let coord_field = exif.get_field(coord_tag, In::PRIMARY)?;
    let ref_field = exif.get_field(ref_tag, In::PRIMARY)?;
    
    if let Value::Rational(ref vals) = coord_field.value {
        if vals.len() >= 3 {
            let degrees = vals[0].num as f64 / vals[0].denom as f64;
            let minutes = vals[1].num as f64 / vals[1].denom as f64;
            let seconds = vals[2].num as f64 / vals[2].denom as f64;
            
            let mut coord = degrees + minutes / 60.0 + seconds / 3600.0;
            
            let ref_val = ref_field.display_value().to_string();
            if ref_val.contains('S') || ref_val.contains('W') {
                coord = -coord;
            }
            
            return Some(coord);
        }
    }
    
    None
}