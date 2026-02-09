
use encoding_rs::Encoding;
use crate::settings::AppSettings;
use std::path::Path;
use glob::Pattern;

/// Resolves the encoding for a given file path based on settings.
/// Returns the encoding name string if found, or None.
pub fn resolve_file_encoding(path: &Path, settings: &AppSettings) -> Option<String> {
    // 1. Check exact match (if we were support exact path mapping, but for now we use globs)
    // The settings structure we planned uses a map of Glob -> Encoding Name
    
    // Normalize path just in case
    let path_str = path.to_string_lossy().replace('\\', "/");

    for (pattern_str, encoding_name) in &settings.file_encodings {
        if let Ok(pattern) = Pattern::new(pattern_str) {
            if pattern.matches(&path_str) {
                return Some(encoding_name.clone());
            }
        }
    }
    
    None
}

/// Decodes bytes using the specified encoding.
/// Falls back to UTF-8 lossy if encoding is unknown or decoding fails (though encoding_rs handles validity).
/// If encoding_name is None, it defaults to UTF-8 lossy.
pub fn decode_bytes(data: &[u8], path: &Path, settings: &AppSettings, override_encoding: Option<String>) -> String {
    let encoding_name = override_encoding.or_else(|| resolve_file_encoding(path, settings));

    if let Some(enc_name) = encoding_name {
        if let Some(encoding) = Encoding::for_label(enc_name.as_bytes()) {
            let (cow, _, _malformed) = encoding.decode(data);
            return cow.into_owned();
        }
    }

    // Default behavior: UTF-8 lossy
    String::from_utf8_lossy(data).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_resolve_encoding() {
        let mut settings = AppSettings::default();
        settings.file_encodings.insert("*.txt".to_string(), "windows-1252".to_string());
        settings.file_encodings.insert("src/**/*.rs".to_string(), "utf-8".to_string());

        assert_eq!(
            resolve_file_encoding(Path::new("test.txt"), &settings).as_deref(),
            Some("windows-1252")
        );
        assert_eq!(
            resolve_file_encoding(Path::new("src/main.rs"), &settings).as_deref(),
            Some("utf-8")
        );
        assert_eq!(
            resolve_file_encoding(Path::new("image.png"), &settings),
            None
        );
    }

    #[test]
    fn test_decode_bytes() {
        let mut settings = AppSettings::default();
        settings.file_encodings.insert("*.txt".to_string(), "windows-1252".to_string());

        // Windows-1252 encoded "café" (E9 is é)
        let data = vec![0x63, 0x61, 0x66, 0xE9]; 
        
        let decoded = decode_bytes(&data, Path::new("test.txt"), &settings);
        assert_eq!(decoded, "café");

        // UTF-8 (default)
        let data_utf8 = "café".as_bytes();
        let decoded_utf8 = decode_bytes(data_utf8, Path::new("other.rs"), &settings);
        assert_eq!(decoded_utf8, "café");
    }
}
