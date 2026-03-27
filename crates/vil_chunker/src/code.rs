use crate::strategy::{ChunkMeta, ChunkStrategy, ChunkType, TextChunk, estimate_tokens};
use regex::Regex;

/// Code-aware chunker that splits on function / class boundaries.
///
/// Recognises `fn `, `def `, `class `, `function `, `pub fn `, `pub(crate) fn `,
/// `async fn `, etc. Falls back to line-count splitting when no boundaries are found.
pub struct CodeChunker {
    /// Maximum lines per chunk when no boundaries are found.
    pub max_lines: usize,
    /// Optional language hint (e.g. "rust", "python").
    pub language: Option<String>,
}

impl CodeChunker {
    pub fn new(max_lines: usize) -> Self {
        Self {
            max_lines,
            language: None,
        }
    }

    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.language = Some(lang.into());
        self
    }
}

impl ChunkStrategy for CodeChunker {
    fn chunk(&self, text: &str) -> Vec<TextChunk> {
        if text.is_empty() {
            return Vec::new();
        }

        let boundary_re = Regex::new(
            r"(?m)^[ \t]*(pub(\([^)]*\))?\s+)?(async\s+)?(fn |def |class |function )"
        ).unwrap();

        let boundary_positions: Vec<usize> = boundary_re
            .find_iter(text)
            .map(|m| m.start())
            .collect();

        if boundary_positions.is_empty() {
            // Fall back to line-based splitting.
            return self.split_by_lines(text);
        }

        let mut chunks = Vec::new();

        for (i, &start) in boundary_positions.iter().enumerate() {
            let end = if i + 1 < boundary_positions.len() {
                boundary_positions[i + 1]
            } else {
                text.len()
            };

            let content = text[start..end].trim_end();
            if content.is_empty() {
                continue;
            }

            let token_count = estimate_tokens(content);
            chunks.push(TextChunk {
                content: content.to_string(),
                start,
                end,
                token_count,
                metadata: ChunkMeta {
                    chunk_type: ChunkType::Code,
                    language: self.language.clone(),
                },
            });
        }

        // Include any preamble before the first boundary (imports, module docs).
        if let Some(&first) = boundary_positions.first() {
            if first > 0 {
                let preamble = text[..first].trim();
                if !preamble.is_empty() {
                    let token_count = estimate_tokens(preamble);
                    chunks.insert(
                        0,
                        TextChunk {
                            content: preamble.to_string(),
                            start: 0,
                            end: first,
                            token_count,
                            metadata: ChunkMeta {
                                chunk_type: ChunkType::Code,
                                language: self.language.clone(),
                            },
                        },
                    );
                }
            }
        }

        chunks
    }
}

impl CodeChunker {
    fn split_by_lines(&self, text: &str) -> Vec<TextChunk> {
        let lines: Vec<&str> = text.lines().collect();
        let mut chunks = Vec::new();
        let mut offset = 0;

        for chunk_lines in lines.chunks(self.max_lines) {
            let content = chunk_lines.join("\n");
            let trimmed = content.trim();
            if trimmed.is_empty() {
                offset += content.len() + 1; // +1 for the newline
                continue;
            }
            let token_count = estimate_tokens(trimmed);
            chunks.push(TextChunk {
                content: trimmed.to_string(),
                start: offset,
                end: offset + content.len(),
                token_count,
                metadata: ChunkMeta {
                    chunk_type: ChunkType::Code,
                    language: self.language.clone(),
                },
            });
            offset += content.len() + 1;
        }

        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_on_fn_boundaries() {
        let code = "\
fn hello() {
    println!(\"hello\");
}

fn world() {
    println!(\"world\");
}";
        let chunker = CodeChunker::new(50);
        let chunks = chunker.chunk(code);
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].content.contains("hello"));
        assert!(chunks[1].content.contains("world"));
    }

    #[test]
    fn splits_python_functions() {
        let code = "\
def hello():
    print('hello')

def world():
    print('world')
";
        let chunker = CodeChunker::new(50).with_language("python");
        let chunks = chunker.chunk(code);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].metadata.language.as_deref(), Some("python"));
    }

    #[test]
    fn splits_class_boundaries() {
        let code = "\
class Foo:
    pass

class Bar:
    pass
";
        let chunker = CodeChunker::new(50);
        let chunks = chunker.chunk(code);
        assert_eq!(chunks.len(), 2);
    }

    #[test]
    fn falls_back_to_line_splitting() {
        let code = "line1\nline2\nline3\nline4\nline5\nline6";
        let chunker = CodeChunker::new(3);
        let chunks = chunker.chunk(code);
        assert!(chunks.len() >= 2, "expected >= 2 line-based chunks");
    }

    #[test]
    fn empty_input() {
        let chunker = CodeChunker::new(50);
        assert!(chunker.chunk("").is_empty());
    }

    #[test]
    fn includes_preamble() {
        let code = "\
use std::io;
use std::fs;

fn main() {
    println!(\"hi\");
}";
        let chunker = CodeChunker::new(50);
        let chunks = chunker.chunk(code);
        assert!(chunks.len() >= 2);
        assert!(chunks[0].content.contains("use std::io"));
    }

    #[test]
    fn chunk_type_is_code() {
        let chunker = CodeChunker::new(50);
        let chunks = chunker.chunk("fn foo() {}");
        assert_eq!(chunks[0].metadata.chunk_type, ChunkType::Code);
    }
}
