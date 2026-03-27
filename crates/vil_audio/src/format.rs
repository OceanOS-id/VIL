use serde::{Deserialize, Serialize};

/// Supported audio formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioFormat {
    Wav,
    Mp3,
    Flac,
    Ogg,
    Webm,
    Unknown,
}

impl AudioFormat {
    /// Human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            AudioFormat::Wav => "WAV",
            AudioFormat::Mp3 => "MP3",
            AudioFormat::Flac => "FLAC",
            AudioFormat::Ogg => "Ogg Vorbis",
            AudioFormat::Webm => "WebM",
            AudioFormat::Unknown => "Unknown",
        }
    }

    /// Common file extension.
    pub fn extension(&self) -> &'static str {
        match self {
            AudioFormat::Wav => "wav",
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Flac => "flac",
            AudioFormat::Ogg => "ogg",
            AudioFormat::Webm => "webm",
            AudioFormat::Unknown => "bin",
        }
    }
}

/// Detect audio format from the first bytes of a file (magic bytes).
pub fn detect_format(header: &[u8]) -> AudioFormat {
    if header.len() < 4 {
        return AudioFormat::Unknown;
    }

    // WAV: starts with "RIFF" ... "WAVE"
    if header.len() >= 12 && &header[0..4] == b"RIFF" && &header[8..12] == b"WAVE" {
        return AudioFormat::Wav;
    }

    // MP3: starts with 0xFF 0xFB, 0xFF 0xF3, 0xFF 0xF2 (frame sync) or ID3 tag
    if (header[0] == 0xFF && (header[1] & 0xE0) == 0xE0)
        || &header[0..3] == b"ID3"
    {
        return AudioFormat::Mp3;
    }

    // FLAC: starts with "fLaC"
    if &header[0..4] == b"fLaC" {
        return AudioFormat::Flac;
    }

    // OGG: starts with "OggS"
    if &header[0..4] == b"OggS" {
        return AudioFormat::Ogg;
    }

    // WebM: starts with 0x1A 0x45 0xDF 0xA3 (EBML header, shared with Matroska)
    if header.len() >= 4 && header[0] == 0x1A && header[1] == 0x45 && header[2] == 0xDF && header[3] == 0xA3 {
        return AudioFormat::Webm;
    }

    AudioFormat::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_wav() {
        let mut header = vec![0u8; 12];
        header[0..4].copy_from_slice(b"RIFF");
        header[8..12].copy_from_slice(b"WAVE");
        assert_eq!(detect_format(&header), AudioFormat::Wav);
    }

    #[test]
    fn test_detect_mp3_frame_sync() {
        let header = vec![0xFF, 0xFB, 0x90, 0x00];
        assert_eq!(detect_format(&header), AudioFormat::Mp3);
    }

    #[test]
    fn test_detect_mp3_id3() {
        let header = b"ID3\x04\x00\x00\x00\x00\x00\x00";
        assert_eq!(detect_format(header), AudioFormat::Mp3);
    }

    #[test]
    fn test_detect_flac() {
        let header = b"fLaC\x00\x00\x00\x22";
        assert_eq!(detect_format(header), AudioFormat::Flac);
    }

    #[test]
    fn test_detect_ogg() {
        let header = b"OggS\x00\x02\x00\x00";
        assert_eq!(detect_format(header), AudioFormat::Ogg);
    }

    #[test]
    fn test_detect_webm() {
        let header = vec![0x1A, 0x45, 0xDF, 0xA3, 0x01, 0x00];
        assert_eq!(detect_format(&header), AudioFormat::Webm);
    }

    #[test]
    fn test_detect_unknown() {
        let header = vec![0x00, 0x00, 0x00, 0x00];
        assert_eq!(detect_format(&header), AudioFormat::Unknown);
    }

    #[test]
    fn test_detect_too_short() {
        let header = vec![0xFF];
        assert_eq!(detect_format(&header), AudioFormat::Unknown);
    }
}
