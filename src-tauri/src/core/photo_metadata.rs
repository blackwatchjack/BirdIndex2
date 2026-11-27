use chrono::{Datelike, NaiveDateTime};
use exif::{In, Reader, Tag, Value};
use std::{fs::File, io::BufReader, path::Path};

pub fn capture_time(path: &Path) -> anyhow::Result<Option<String>> {
    let mut file = BufReader::new(File::open(path)?);
    let exif = match Reader::new().read_from_container(&mut file) {
        Ok(exif) => exif,
        Err(exif::Error::NotFound(_)) => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    let Some(Value::Ascii(values)) = exif
        .get_field(Tag::DateTimeOriginal, In::PRIMARY)
        .map(|field| &field.value)
    else {
        return Ok(None);
    };
    Ok(values.first().and_then(|bytes| format_capture_time(bytes)))
}

fn format_capture_time(bytes: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(bytes).ok()?;
    if text.len() != 19 {
        return None;
    }
    let date = NaiveDateTime::parse_from_str(text, "%Y:%m:%d %H:%M:%S").ok()?;
    (date.year() > 0).then(|| date.format("%Y-%m-%d %H:%M:%S").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_original_camera_time_and_distinguishes_missing_files() {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        // The JPEG also has a later ModifyDate and a +09:00 camera offset.
        assert_eq!(
            capture_time(&fixtures.join("capture-time.jpg"))
                .unwrap()
                .as_deref(),
            Some("2024-02-29 06:15:37")
        );
        assert_eq!(capture_time(&fixtures.join("no-exif.png")).unwrap(), None);
        assert!(capture_time(&fixtures.join("missing.jpg")).is_err());
    }

    #[test]
    fn invalid_camera_dates_are_absent() {
        for value in ["2025:02:29 06:15:37", "0000:00:00 00:00:00", ""] {
            assert_eq!(format_capture_time(value.as_bytes()), None);
        }
    }
}
