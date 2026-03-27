use std::time::Instant;

/// Result of a microbenchmark run.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BenchResult {
    /// Number of search iterations performed.
    pub iterations: usize,
    /// Total elapsed time in microseconds.
    pub total_us: u64,
    /// Average time per search in microseconds.
    pub avg_us: f64,
    /// Number of documents in the index.
    pub docs: usize,
}

/// Run a microbenchmark of the realtime RAG index search.
///
/// Searches the provided `index` with a synthetic query vector
/// for `iterations` rounds and reports timing statistics.
pub fn bench_search(
    index: &super::index::RealtimeIndex,
    dimension: usize,
    iterations: usize,
) -> BenchResult {
    let query = vec![0.5f32; dimension];

    let start = Instant::now();
    for _ in 0..iterations {
        let _ = index.search(&query, 5);
    }
    let total = start.elapsed();

    BenchResult {
        iterations,
        total_us: total.as_micros() as u64,
        avg_us: total.as_micros() as f64 / iterations as f64,
        docs: index.count(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{DocEntry, RealtimeIndex};

    #[test]
    fn bench_runs_without_panic() {
        let idx = RealtimeIndex::new(384);
        // Add a few docs so there's something to search.
        for i in 0..10u64 {
            let v: Vec<f32> = (0..384)
                .map(|d| ((i * 384 + d) as f32).sin())
                .collect();
            idx.add(
                &v,
                DocEntry {
                    id: format!("doc_{i}"),
                    text: format!("document {i}"),
                    metadata: serde_json::json!({}),
                },
            );
        }

        let result = bench_search(&idx, 384, 100);
        assert_eq!(result.iterations, 100);
        assert!(result.avg_us > 0.0);
        assert_eq!(result.docs, 10);
    }

    #[test]
    fn bench_empty_index() {
        let idx = RealtimeIndex::new(64);
        let result = bench_search(&idx, 64, 50);
        assert_eq!(result.iterations, 50);
        assert_eq!(result.docs, 0);
    }
}
