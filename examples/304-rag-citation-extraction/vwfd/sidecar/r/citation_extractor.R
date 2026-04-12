#!/usr/bin/env Rscript
# Citation Extractor — R Sidecar
# Extracts [DocN] references from LLM output for legal audit trails.
library(jsonlite)
input <- fromJSON(readLines(stdin(), warn=FALSE))
text <- input$text %||% ""
# Extract [Doc1], [Doc2], etc.
refs <- regmatches(text, gregexpr("\\[Doc\\d+\\]", text))[[1]]
unique_refs <- unique(refs)
citations <- lapply(seq_along(unique_refs), function(i) {
  list(ref = unique_refs[i], count = sum(refs == unique_refs[i]), position = i)
})
result <- list(citations = citations, total = length(unique_refs), text_length = nchar(text))
cat(toJSON(result, auto_unbox = TRUE))
