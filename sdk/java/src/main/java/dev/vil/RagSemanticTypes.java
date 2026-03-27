package dev.vil;

/**
 * RAG plugin semantic types for VIL Phase 6.
 *
 * <p>Mirror of Rust vil_rag::semantic types for Java SDK interop.
 */
public class RagSemanticTypes {

    /** RAG query completion event. */
    public static class RagQueryEvent {
        public String query;
        public int resultCount;
        public String[] sources;

        public RagQueryEvent() {}

        public RagQueryEvent(String query, int resultCount, String[] sources) {
            this.query = query;
            this.resultCount = resultCount;
            this.sources = sources;
        }
    }

    /** RAG document ingest event. */
    public static class RagIngestEvent {
        public String docId;
        public int chunkCount;
        public String indexName;

        public RagIngestEvent() {}

        public RagIngestEvent(String docId, int chunkCount, String indexName) {
            this.docId = docId;
            this.chunkCount = chunkCount;
            this.indexName = indexName;
        }
    }

    /** RAG fault (index error, retrieval failure, etc.). */
    public static class RagFault {
        public String code;
        public String message;
        public String indexName;

        public RagFault() {}

        public RagFault(String code, String message, String indexName) {
            this.code = code;
            this.message = message;
            this.indexName = indexName;
        }
    }

    /** RAG index state. */
    public static class RagIndexState {
        public long docCount;
        public long chunkCount;
        public String lastUpdated;

        public RagIndexState() {}

        public RagIndexState(long docCount, long chunkCount, String lastUpdated) {
            this.docCount = docCount;
            this.chunkCount = chunkCount;
            this.lastUpdated = lastUpdated;
        }
    }
}
