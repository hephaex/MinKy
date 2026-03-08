//! Chunking stage - splits documents into chunks for embedding
//!
//! Supports multiple chunking strategies:
//! - Fixed size: Split by word count with overlap
//! - Semantic: Split at natural boundaries (paragraphs, headings)
//! - Paragraph: Split at paragraph boundaries

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::pipeline::{PipelineContext, PipelineResult, PipelineStage};

use super::metadata::DocumentWithMetadata;

/// Strategy for chunking documents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkingStrategy {
    /// Fixed size chunks with overlap
    FixedSize {
        /// Number of words per chunk
        size: usize,
        /// Number of words to overlap between chunks
        overlap: usize,
    },

    /// Semantic chunking based on document structure
    Semantic {
        /// Maximum tokens per chunk
        max_tokens: usize,
        /// Prefer breaking at paragraph/heading boundaries
        respect_boundaries: bool,
    },

    /// Split at paragraph boundaries
    Paragraph {
        /// Minimum paragraph length (chars) to be its own chunk
        min_length: usize,
        /// Maximum chunk length before forced split
        max_length: usize,
    },
}

impl Default for ChunkingStrategy {
    fn default() -> Self {
        Self::Semantic {
            max_tokens: 512,
            respect_boundaries: true,
        }
    }
}

/// A single chunk of text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// Chunk index (0-based)
    pub index: usize,

    /// Chunk text content
    pub text: String,

    /// Start offset in original plain text
    pub start_offset: usize,

    /// End offset in original plain text
    pub end_offset: usize,

    /// Approximate token count
    pub token_count: usize,

    /// Optional metadata
    pub metadata: Option<serde_json::Value>,
}

/// Document with chunks ready for embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkedDocument {
    /// Document title
    pub title: String,

    /// Original plain text
    pub plain_text: String,

    /// Original content (for storage)
    pub original_content: String,

    /// MIME type
    pub mime_type: String,

    /// Extracted metadata
    pub metadata: super::metadata::ExtractedMetadata,

    /// Generated chunks
    pub chunks: Vec<Chunk>,

    /// Headings from parsing
    pub headings: Vec<super::parsing::Heading>,

    /// Links from parsing
    pub links: Vec<super::parsing::Link>,

    /// Code blocks from parsing
    pub code_blocks: Vec<super::parsing::CodeBlock>,

    /// Source information
    pub source_type: String,
    pub source_path: Option<String>,
}

/// Chunking stage - splits documents into chunks
#[derive(Debug, Clone)]
pub struct ChunkingStage {
    strategy: ChunkingStrategy,
}

impl ChunkingStage {
    /// Create a new chunking stage with the given strategy
    pub fn new(strategy: ChunkingStrategy) -> Self {
        Self { strategy }
    }

    /// Create with default semantic chunking
    pub fn semantic(max_tokens: usize) -> Self {
        Self::new(ChunkingStrategy::Semantic {
            max_tokens,
            respect_boundaries: true,
        })
    }

    /// Create with fixed size chunking
    pub fn fixed_size(size: usize, overlap: usize) -> Self {
        Self::new(ChunkingStrategy::FixedSize { size, overlap })
    }

    /// Create with paragraph chunking
    pub fn paragraph(min_length: usize, max_length: usize) -> Self {
        Self::new(ChunkingStrategy::Paragraph {
            min_length,
            max_length,
        })
    }

    /// Chunk using fixed size strategy
    fn chunk_fixed_size(&self, text: &str, size: usize, overlap: usize) -> Vec<Chunk> {
        if text.is_empty() || size == 0 {
            return Vec::new();
        }

        let words: Vec<&str> = text.split_whitespace().collect();
        if words.is_empty() {
            return Vec::new();
        }

        let mut chunks = Vec::new();
        let step = if overlap < size { size - overlap } else { 1 };
        let mut current_offset: usize = 0;
        let mut i = 0;

        while i < words.len() {
            let end = (i + size).min(words.len());
            let chunk_words = &words[i..end];
            let chunk_text = chunk_words.join(" ");

            let start_offset = current_offset;
            let end_offset = start_offset + chunk_text.len();

            chunks.push(Chunk {
                index: chunks.len(),
                text: chunk_text.clone(),
                start_offset,
                end_offset,
                token_count: Self::estimate_tokens(&chunk_text),
                metadata: None,
            });

            // Update offset for next chunk (approximate)
            let overlap_chars = chunk_words
                .iter()
                .rev()
                .take(overlap)
                .map(|w| w.len() + 1)
                .sum::<usize>();
            current_offset = end_offset.saturating_sub(overlap_chars);

            i += step;
        }

        chunks
    }

    /// Chunk using semantic strategy
    fn chunk_semantic(&self, text: &str, max_tokens: usize, respect_boundaries: bool) -> Vec<Chunk> {
        if text.is_empty() {
            return Vec::new();
        }

        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut current_start = 0;
        let mut current_pos = 0;

        // Split by paragraphs first
        let paragraphs: Vec<&str> = text.split("\n\n").collect();

        for para in paragraphs {
            let para = para.trim();
            if para.is_empty() {
                current_pos += 2; // account for \n\n
                continue;
            }

            let para_tokens = Self::estimate_tokens(para);

            // If adding this paragraph would exceed max, save current chunk
            let combined_tokens = Self::estimate_tokens(&current_chunk) + para_tokens;
            if !current_chunk.is_empty() && combined_tokens > max_tokens {
                // Save current chunk
                chunks.push(Chunk {
                    index: chunks.len(),
                    text: current_chunk.trim().to_string(),
                    start_offset: current_start,
                    end_offset: current_pos,
                    token_count: Self::estimate_tokens(&current_chunk),
                    metadata: None,
                });

                current_chunk = String::new();
                current_start = current_pos;
            }

            // If paragraph itself is too long, split it
            if para_tokens > max_tokens {
                if !current_chunk.is_empty() {
                    chunks.push(Chunk {
                        index: chunks.len(),
                        text: current_chunk.trim().to_string(),
                        start_offset: current_start,
                        end_offset: current_pos,
                        token_count: Self::estimate_tokens(&current_chunk),
                        metadata: None,
                    });
                    current_chunk = String::new();
                }

                // Split long paragraph by sentences or fixed size
                let sub_chunks = if respect_boundaries {
                    self.split_by_sentences(para, max_tokens, current_pos)
                } else {
                    self.chunk_fixed_size(para, max_tokens * 3 / 4, max_tokens / 8) // Approx word count
                };

                for mut sub_chunk in sub_chunks {
                    sub_chunk.index = chunks.len();
                    current_pos = sub_chunk.end_offset;
                    chunks.push(sub_chunk);
                }
                current_start = current_pos;
            } else {
                if !current_chunk.is_empty() {
                    current_chunk.push_str("\n\n");
                }
                current_chunk.push_str(para);
            }

            current_pos += para.len() + 2;
        }

        // Don't forget the last chunk
        if !current_chunk.is_empty() {
            chunks.push(Chunk {
                index: chunks.len(),
                text: current_chunk.trim().to_string(),
                start_offset: current_start,
                end_offset: current_pos,
                token_count: Self::estimate_tokens(&current_chunk),
                metadata: None,
            });
        }

        chunks
    }

    /// Split text by sentences
    fn split_by_sentences(&self, text: &str, max_tokens: usize, base_offset: usize) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut current_start = base_offset;
        let mut current_pos = base_offset;

        // Simple sentence splitting by period followed by space or newline
        let sentence_re = regex::Regex::new(r"[.!?]+\s+").unwrap();
        let sentences: Vec<&str> = sentence_re.split(text).collect();

        for sentence in sentences {
            let sentence = sentence.trim();
            if sentence.is_empty() {
                continue;
            }

            let sentence_tokens = Self::estimate_tokens(sentence);
            let combined_tokens = Self::estimate_tokens(&current_chunk) + sentence_tokens;

            if !current_chunk.is_empty() && combined_tokens > max_tokens {
                chunks.push(Chunk {
                    index: 0, // Will be fixed later
                    text: current_chunk.trim().to_string(),
                    start_offset: current_start,
                    end_offset: current_pos,
                    token_count: Self::estimate_tokens(&current_chunk),
                    metadata: None,
                });

                current_chunk = String::new();
                current_start = current_pos;
            }

            if !current_chunk.is_empty() {
                current_chunk.push(' ');
            }
            current_chunk.push_str(sentence);
            current_chunk.push('.');
            current_pos += sentence.len() + 2;
        }

        if !current_chunk.is_empty() {
            chunks.push(Chunk {
                index: 0,
                text: current_chunk.trim().to_string(),
                start_offset: current_start,
                end_offset: current_pos,
                token_count: Self::estimate_tokens(&current_chunk),
                metadata: None,
            });
        }

        chunks
    }

    /// Chunk using paragraph strategy
    fn chunk_paragraph(&self, text: &str, min_length: usize, max_length: usize) -> Vec<Chunk> {
        if text.is_empty() {
            return Vec::new();
        }

        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut current_start = 0;
        let mut current_pos = 0;

        for para in text.split("\n\n") {
            let para = para.trim();
            if para.is_empty() {
                current_pos += 2;
                continue;
            }

            // If paragraph is long enough on its own
            if para.len() >= min_length {
                // Save any accumulated small paragraphs first
                if !current_chunk.is_empty() {
                    chunks.push(Chunk {
                        index: chunks.len(),
                        text: current_chunk.trim().to_string(),
                        start_offset: current_start,
                        end_offset: current_pos,
                        token_count: Self::estimate_tokens(&current_chunk),
                        metadata: None,
                    });
                    current_chunk = String::new();
                }

                // If paragraph exceeds max, split it
                if para.len() > max_length {
                    let sub_chunks = self.chunk_fixed_size(para, max_length / 5, max_length / 20);
                    for mut sub_chunk in sub_chunks {
                        sub_chunk.index = chunks.len();
                        sub_chunk.start_offset += current_pos;
                        sub_chunk.end_offset += current_pos;
                        chunks.push(sub_chunk);
                    }
                } else {
                    chunks.push(Chunk {
                        index: chunks.len(),
                        text: para.to_string(),
                        start_offset: current_pos,
                        end_offset: current_pos + para.len(),
                        token_count: Self::estimate_tokens(para),
                        metadata: None,
                    });
                }
                current_start = current_pos + para.len() + 2;
            } else {
                // Accumulate small paragraphs
                if !current_chunk.is_empty() {
                    current_chunk.push_str("\n\n");
                }
                current_chunk.push_str(para);

                // If accumulated text exceeds min_length, save it
                if current_chunk.len() >= min_length {
                    chunks.push(Chunk {
                        index: chunks.len(),
                        text: current_chunk.trim().to_string(),
                        start_offset: current_start,
                        end_offset: current_pos + para.len(),
                        token_count: Self::estimate_tokens(&current_chunk),
                        metadata: None,
                    });
                    current_chunk = String::new();
                    current_start = current_pos + para.len() + 2;
                }
            }

            current_pos += para.len() + 2;
        }

        // Don't forget remaining content
        if !current_chunk.is_empty() {
            chunks.push(Chunk {
                index: chunks.len(),
                text: current_chunk.trim().to_string(),
                start_offset: current_start,
                end_offset: current_pos,
                token_count: Self::estimate_tokens(&current_chunk),
                metadata: None,
            });
        }

        chunks
    }

    /// Estimate token count (rough approximation: 1 token ≈ 4 characters)
    fn estimate_tokens(text: &str) -> usize {
        text.len() / 4 + 1
    }
}

impl Default for ChunkingStage {
    fn default() -> Self {
        Self::semantic(512)
    }
}

#[async_trait]
impl PipelineStage<DocumentWithMetadata, ChunkedDocument> for ChunkingStage {
    fn name(&self) -> &'static str {
        "chunking"
    }

    async fn process(
        &self,
        input: DocumentWithMetadata,
        context: &mut PipelineContext,
    ) -> PipelineResult<ChunkedDocument> {
        let chunks = match &self.strategy {
            ChunkingStrategy::FixedSize { size, overlap } => {
                self.chunk_fixed_size(&input.plain_text, *size, *overlap)
            }
            ChunkingStrategy::Semantic {
                max_tokens,
                respect_boundaries,
            } => self.chunk_semantic(&input.plain_text, *max_tokens, *respect_boundaries),
            ChunkingStrategy::Paragraph {
                min_length,
                max_length,
            } => self.chunk_paragraph(&input.plain_text, *min_length, *max_length),
        };

        // Update metrics
        context.metrics.add_chunks(chunks.len() as u32);
        let total_tokens: usize = chunks.iter().map(|c| c.token_count).sum();
        context.metrics.add_tokens(total_tokens as u32);

        context.set_metadata("chunk_count", chunks.len());
        context.set_metadata("total_tokens", total_tokens);

        Ok(ChunkedDocument {
            title: input.title,
            plain_text: input.plain_text,
            original_content: input.original_content,
            mime_type: input.mime_type,
            metadata: input.metadata,
            chunks,
            headings: input.headings,
            links: input.links,
            code_blocks: input.code_blocks,
            source_type: input.source_type,
            source_path: input.source_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_doc_with_metadata(content: &str) -> DocumentWithMetadata {
        DocumentWithMetadata {
            title: "Test".to_string(),
            plain_text: content.to_string(),
            original_content: content.to_string(),
            mime_type: "text/plain".to_string(),
            metadata: super::super::metadata::ExtractedMetadata::default(),
            headings: Vec::new(),
            links: Vec::new(),
            code_blocks: Vec::new(),
            source_type: "test".to_string(),
            source_path: None,
        }
    }

    #[tokio::test]
    async fn test_fixed_size_chunking() {
        let stage = ChunkingStage::fixed_size(5, 1);
        let mut context = PipelineContext::new();

        let doc = make_doc_with_metadata("one two three four five six seven eight nine ten");
        let result = stage.process(doc, &mut context).await.unwrap();

        assert!(result.chunks.len() >= 2);
        assert_eq!(result.chunks[0].text.split_whitespace().count(), 5);
    }

    #[tokio::test]
    async fn test_semantic_chunking() {
        let stage = ChunkingStage::semantic(50); // Small token limit for testing
        let mut context = PipelineContext::new();

        let doc = make_doc_with_metadata(
            "First paragraph with some content.\n\nSecond paragraph with different content.\n\nThird paragraph.",
        );
        let result = stage.process(doc, &mut context).await.unwrap();

        assert!(!result.chunks.is_empty());
    }

    #[tokio::test]
    async fn test_paragraph_chunking() {
        let stage = ChunkingStage::paragraph(20, 200);
        let mut context = PipelineContext::new();

        let doc = make_doc_with_metadata(
            "Short para.\n\nThis is a longer paragraph that should be its own chunk because it exceeds the minimum length requirement for standalone chunks.",
        );
        let result = stage.process(doc, &mut context).await.unwrap();

        assert!(!result.chunks.is_empty());
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(ChunkingStage::estimate_tokens("hello"), 2);
        assert_eq!(ChunkingStage::estimate_tokens("hello world this is a test"), 7);
    }

    #[tokio::test]
    async fn test_empty_content() {
        let stage = ChunkingStage::default();
        let mut context = PipelineContext::new();

        let doc = make_doc_with_metadata("");
        let result = stage.process(doc, &mut context).await.unwrap();

        assert!(result.chunks.is_empty());
    }

    #[tokio::test]
    async fn test_chunk_indices() {
        let stage = ChunkingStage::fixed_size(3, 0);
        let mut context = PipelineContext::new();

        let doc = make_doc_with_metadata("one two three four five six");
        let result = stage.process(doc, &mut context).await.unwrap();

        for (i, chunk) in result.chunks.iter().enumerate() {
            assert_eq!(chunk.index, i);
        }
    }
}
