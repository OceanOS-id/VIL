use crate::strategy::{ChunkMeta, ChunkStrategy, ChunkType, TextChunk, estimate_tokens};

/// Table / CSV row chunker.
///
/// Splits CSV or table-like text into chunks of `rows_per_chunk` rows.
/// The header row (first line) is prepended to every chunk so downstream
/// consumers always know the column names.
pub struct TableChunker {
    /// Maximum number of data rows per chunk (excluding the header).
    pub rows_per_chunk: usize,
}

impl TableChunker {
    pub fn new(rows_per_chunk: usize) -> Self {
        assert!(rows_per_chunk > 0, "rows_per_chunk must be > 0");
        Self { rows_per_chunk }
    }
}

impl ChunkStrategy for TableChunker {
    fn chunk(&self, text: &str) -> Vec<TextChunk> {
        if text.is_empty() {
            return Vec::new();
        }

        let lines: Vec<&str> = text.lines().collect();
        if lines.is_empty() {
            return Vec::new();
        }

        let header = lines[0];
        let data_rows = &lines[1..];

        if data_rows.is_empty() {
            // Only a header — return as a single chunk.
            let token_count = estimate_tokens(header);
            return vec![TextChunk {
                content: header.to_string(),
                start: 0,
                end: header.len(),
                token_count,
                metadata: ChunkMeta {
                    chunk_type: ChunkType::Table,
                    language: None,
                },
            }];
        }

        let mut chunks = Vec::new();
        let mut offset = header.len() + 1; // +1 for the newline after header

        for batch in data_rows.chunks(self.rows_per_chunk) {
            let mut content = String::from(header);
            content.push('\n');
            for (i, row) in batch.iter().enumerate() {
                content.push_str(row);
                if i < batch.len() - 1 {
                    content.push('\n');
                }
            }

            let token_count = estimate_tokens(&content);
            let batch_byte_len: usize = batch.iter().map(|r| r.len() + 1).sum();
            chunks.push(TextChunk {
                content,
                start: offset,
                end: offset + batch_byte_len,
                token_count,
                metadata: ChunkMeta {
                    chunk_type: ChunkType::Table,
                    language: None,
                },
            });
            offset += batch_byte_len;
        }

        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_csv_chunking() {
        let csv = "name,age,city\nAlice,30,NYC\nBob,25,LA\nCharlie,35,SF\nDave,28,CHI";
        let chunker = TableChunker::new(2);
        let chunks = chunker.chunk(csv);
        assert_eq!(chunks.len(), 2);
        // Every chunk should start with the header.
        for c in &chunks {
            assert!(c.content.starts_with("name,age,city"));
        }
    }

    #[test]
    fn header_only() {
        let csv = "col1,col2,col3";
        let chunker = TableChunker::new(5);
        let chunks = chunker.chunk(csv);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, "col1,col2,col3");
    }

    #[test]
    fn empty_input() {
        let chunker = TableChunker::new(5);
        assert!(chunker.chunk("").is_empty());
    }

    #[test]
    fn single_data_row() {
        let csv = "h1,h2\nv1,v2";
        let chunker = TableChunker::new(10);
        let chunks = chunker.chunk(csv);
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].content.contains("h1,h2"));
        assert!(chunks[0].content.contains("v1,v2"));
    }

    #[test]
    fn chunk_type_is_table() {
        let csv = "a,b\n1,2\n3,4";
        let chunker = TableChunker::new(1);
        for c in chunker.chunk(csv) {
            assert_eq!(c.metadata.chunk_type, ChunkType::Table);
        }
    }

    #[test]
    #[should_panic]
    fn zero_rows_per_chunk() {
        TableChunker::new(0);
    }
}
