use serde::{Deserialize, Serialize};

/// Supported image formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Gif,
    Webp,
    Bmp,
    Unknown,
}

impl ImageFormat {
    /// Human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "JPEG",
            ImageFormat::Png => "PNG",
            ImageFormat::Gif => "GIF",
            ImageFormat::Webp => "WebP",
            ImageFormat::Bmp => "BMP",
            ImageFormat::Unknown => "Unknown",
        }
    }

    /// Common file extension.
    pub fn extension(&self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Png => "png",
            ImageFormat::Gif => "gif",
            ImageFormat::Webp => "webp",
            ImageFormat::Bmp => "bmp",
            ImageFormat::Unknown => "bin",
        }
    }
}

/// Detect image format from the first bytes of a file (magic bytes).
pub fn detect_format(header: &[u8]) -> ImageFormat {
    if header.len() < 4 {
        return ImageFormat::Unknown;
    }

    // JPEG: starts with 0xFF 0xD8 0xFF
    if header.len() >= 3 && header[0] == 0xFF && header[1] == 0xD8 && header[2] == 0xFF {
        return ImageFormat::Jpeg;
    }

    // PNG: starts with 0x89 0x50 0x4E 0x47 (‰PNG)
    if header[0] == 0x89 && header[1] == 0x50 && header[2] == 0x4E && header[3] == 0x47 {
        return ImageFormat::Png;
    }

    // GIF: starts with "GIF8"
    if &header[0..4] == b"GIF8" {
        return ImageFormat::Gif;
    }

    // WebP: starts with "RIFF" ... "WEBP"
    if header.len() >= 12 && &header[0..4] == b"RIFF" && &header[8..12] == b"WEBP" {
        return ImageFormat::Webp;
    }

    // BMP: starts with "BM"
    if header[0] == b'B' && header[1] == b'M' {
        return ImageFormat::Bmp;
    }

    ImageFormat::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_jpeg() {
        let header = vec![0xFF, 0xD8, 0xFF, 0xE0];
        assert_eq!(detect_format(&header), ImageFormat::Jpeg);
    }

    #[test]
    fn test_detect_png() {
        let header = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(detect_format(&header), ImageFormat::Png);
    }

    #[test]
    fn test_detect_gif() {
        let header = b"GIF89a\x00\x00";
        assert_eq!(detect_format(header), ImageFormat::Gif);
    }

    #[test]
    fn test_detect_webp() {
        let mut header = vec![0u8; 12];
        header[0..4].copy_from_slice(b"RIFF");
        header[8..12].copy_from_slice(b"WEBP");
        assert_eq!(detect_format(&header), ImageFormat::Webp);
    }

    #[test]
    fn test_detect_bmp() {
        let header = b"BM\x00\x00\x00\x00";
        assert_eq!(detect_format(header), ImageFormat::Bmp);
    }

    #[test]
    fn test_detect_unknown() {
        let header = vec![0x00, 0x00, 0x00, 0x00];
        assert_eq!(detect_format(&header), ImageFormat::Unknown);
    }

    #[test]
    fn test_detect_too_short() {
        assert_eq!(detect_format(&[0xFF]), ImageFormat::Unknown);
    }
}
