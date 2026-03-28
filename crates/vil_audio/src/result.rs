use serde::{Deserialize, Serialize};

/// Full transcript result from audio transcription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    /// The full transcribed text.
    pub text: String,
    /// Time-aligned segments.
    pub segments: Vec<Segment>,
    /// Detected language code.
    pub language: String,
    /// Total audio duration in milliseconds.
    pub duration_ms: u64,
}

/// A time-aligned segment of a transcript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    /// Start time in milliseconds.
    pub start_ms: u64,
    /// End time in milliseconds.
    pub end_ms: u64,
    /// Transcribed text for this segment.
    pub text: String,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f32,
}

impl Segment {
    /// Duration of this segment in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        self.end_ms.saturating_sub(self.start_ms)
    }
}

impl Transcript {
    /// Check if segments are in chronological order.
    pub fn is_ordered(&self) -> bool {
        self.segments
            .windows(2)
            .all(|w| w[0].start_ms <= w[1].start_ms)
    }

    /// Get average confidence across all segments.
    pub fn avg_confidence(&self) -> f32 {
        if self.segments.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.segments.iter().map(|s| s.confidence).sum();
        sum / self.segments.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_transcript() -> Transcript {
        Transcript {
            text: "Hello world. How are you?".into(),
            segments: vec![
                Segment {
                    start_ms: 0,
                    end_ms: 1000,
                    text: "Hello world.".into(),
                    confidence: 0.95,
                },
                Segment {
                    start_ms: 1200,
                    end_ms: 2500,
                    text: "How are you?".into(),
                    confidence: 0.88,
                },
            ],
            language: "en".into(),
            duration_ms: 2500,
        }
    }

    #[test]
    fn test_segment_duration() {
        let seg = Segment {
            start_ms: 100,
            end_ms: 500,
            text: "test".into(),
            confidence: 0.9,
        };
        assert_eq!(seg.duration_ms(), 400);
    }

    #[test]
    fn test_transcript_ordered() {
        let t = make_transcript();
        assert!(t.is_ordered());
    }

    #[test]
    fn test_transcript_avg_confidence() {
        let t = make_transcript();
        let avg = t.avg_confidence();
        assert!(avg > 0.9 && avg < 0.92);
    }

    #[test]
    fn test_transcript_unordered() {
        let t = Transcript {
            text: "".into(),
            segments: vec![
                Segment {
                    start_ms: 2000,
                    end_ms: 3000,
                    text: "b".into(),
                    confidence: 0.9,
                },
                Segment {
                    start_ms: 0,
                    end_ms: 1000,
                    text: "a".into(),
                    confidence: 0.9,
                },
            ],
            language: "en".into(),
            duration_ms: 3000,
        };
        assert!(!t.is_ordered());
    }
}
