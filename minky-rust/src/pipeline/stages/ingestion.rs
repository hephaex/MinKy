//! Ingestion stage - handles document input from various sources
//!
//! This is the first stage in the pipeline, responsible for reading
//! raw document content from files, URLs, or direct text input.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::pipeline::{PipelineContext, PipelineError, PipelineResult, PipelineStage};

/// Source of the document being ingested
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentSource {
    /// Direct text content
    Text {
        title: String,
        content: String,
        /// Optional explicit source_path (e.g. upload's S1 source_path-based
        /// upsert). `None` for plain-text ingests with no path identity.
        source_path: Option<String>,
    },

    /// File path (local filesystem)
    File {
        path: String,
    },

    /// URL to fetch
    Url {
        url: String,
    },

    /// Raw bytes with MIME type
    Bytes {
        data: Vec<u8>,
        mime_type: String,
        filename: Option<String>,
    },
}

/// Options for document ingestion
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IngestionOptions {
    /// Override the detected MIME type
    pub mime_type: Option<String>,

    /// Character encoding (default: UTF-8)
    pub encoding: Option<String>,

    /// Maximum content size in bytes (default: 10MB)
    pub max_size: Option<usize>,

    /// Whether to strip HTML tags from content
    pub strip_html: bool,
}

/// Input to the ingestion stage
#[derive(Debug, Clone)]
pub struct IngestionInput {
    /// Source of the document
    pub source: DocumentSource,

    /// Ingestion options
    pub options: IngestionOptions,
}

impl IngestionInput {
    /// Create input from raw text
    pub fn from_text(title: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            source: DocumentSource::Text {
                title: title.into(),
                content: content.into(),
                source_path: None,
            },
            options: IngestionOptions::default(),
        }
    }

    /// Create input from raw text with an explicit `source_path`.
    ///
    /// Used by the upload endpoint's S1 source_path-based upsert: the file
    /// content already lives in memory (not on disk, so `from_file` does not
    /// apply), but the resulting document must still carry a `source_path` so
    /// the storage stage's `(user_id, source_path)` upsert lookup treats
    /// repeated uploads to the same path as versions of the same document.
    pub fn from_text_with_source_path(
        title: impl Into<String>,
        content: impl Into<String>,
        source_path: impl Into<String>,
    ) -> Self {
        Self {
            source: DocumentSource::Text {
                title: title.into(),
                content: content.into(),
                source_path: Some(source_path.into()),
            },
            options: IngestionOptions::default(),
        }
    }

    /// Create input from a file path
    pub fn from_file(path: impl Into<String>) -> Self {
        Self {
            source: DocumentSource::File { path: path.into() },
            options: IngestionOptions::default(),
        }
    }

    /// Create input from a URL
    pub fn from_url(url: impl Into<String>) -> Self {
        Self {
            source: DocumentSource::Url { url: url.into() },
            options: IngestionOptions::default(),
        }
    }

    /// Create input from bytes
    pub fn from_bytes(data: Vec<u8>, mime_type: impl Into<String>) -> Self {
        Self {
            source: DocumentSource::Bytes {
                data,
                mime_type: mime_type.into(),
                filename: None,
            },
            options: IngestionOptions::default(),
        }
    }

    /// Set options
    pub fn with_options(mut self, options: IngestionOptions) -> Self {
        self.options = options;
        self
    }
}

/// Raw document output from ingestion
#[derive(Debug, Clone)]
pub struct RawDocument {
    /// Original filename or title
    pub title: String,

    /// Raw content as string
    pub content: String,

    /// Detected or specified MIME type
    pub mime_type: String,

    /// Source type for tracking
    pub source_type: String,

    /// Original source path/URL if applicable
    pub source_path: Option<String>,
}

/// Maximum redirects followed by [`fetch_public`]; each hop is re-validated.
const MAX_REDIRECTS: usize = 5;

/// `true` only for globally routable unicast addresses. Rejects loopback, private
/// (RFC1918, ULA), link-local (incl. cloud metadata 169.254.169.254), CGNAT,
/// unspecified, multicast, broadcast, documentation and IPv4-mapped equivalents.
fn is_public_ip(ip: std::net::IpAddr) -> bool {
    use std::net::IpAddr;
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            !(v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_multicast()
                || v4.is_documentation()
                || o[0] == 0
                || (o[0] == 100 && (64..=127).contains(&o[1])) // CGNAT 100.64/10
                || (o[0] == 192 && o[1] == 0 && o[2] == 0) // IETF protocol 192.0.0/24
                || (o[0] == 198 && (o[1] == 18 || o[1] == 19)) // benchmarking 198.18/15
                || o[0] >= 240) // reserved 240/4
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_public_ip(IpAddr::V4(v4));
            }
            let seg = v6.segments();
            !(v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || (seg[0] & 0xfe00) == 0xfc00 // ULA fc00::/7
                || (seg[0] & 0xffc0) == 0xfe80 // link-local fe80::/10
                || (seg[0] == 0x2001 && seg[1] == 0x0db8) // documentation
                || (seg[0] == 0x0064 && seg[1] == 0xff9b)) // NAT64 64:ff9b::/96
        }
    }
}

/// GET `url` with SSRF protection: http(s) only, no userinfo, the host must resolve
/// exclusively to public IPs, the connection is pinned to the validated address
/// (no DNS-rebinding window) and every redirect hop is re-validated.
async fn fetch_public(url: &str) -> PipelineResult<reqwest::Response> {
    let mut current = url::Url::parse(url)
        .map_err(|e| PipelineError::InvalidInput(format!("Invalid URL: {}", e)))?;

    for _ in 0..=MAX_REDIRECTS {
        if !matches!(current.scheme(), "http" | "https") {
            return Err(PipelineError::InvalidInput("Only http(s) URLs are allowed".into()));
        }
        if !current.username().is_empty() || current.password().is_some() {
            return Err(PipelineError::InvalidInput("URL credentials are not allowed".into()));
        }
        let port = current.port_or_known_default().unwrap_or(80);
        // Parse via `Host` so IP literals (incl. bracketed IPv6) skip DNS entirely.
        let (host, addrs): (String, Vec<std::net::SocketAddr>) = match current.host() {
            Some(url::Host::Ipv4(ip)) => (ip.to_string(), vec![(ip, port).into()]),
            Some(url::Host::Ipv6(ip)) => (ip.to_string(), vec![(ip, port).into()]),
            Some(url::Host::Domain(d)) => {
                let found = tokio::net::lookup_host((d, port))
                    .await
                    .map_err(|e| PipelineError::ExternalService(format!("Failed to resolve host: {}", e)))?
                    .collect();
                (d.to_string(), found)
            }
            None => return Err(PipelineError::InvalidInput("URL has no host".into())),
        };
        if addrs.is_empty() || addrs.iter().any(|a| !is_public_ip(a.ip())) {
            return Err(PipelineError::InvalidInput(
                "URL host resolves to a non-public address".into(),
            ));
        }

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(30))
            .resolve(&host, addrs[0])
            .build()
            .map_err(|e| PipelineError::Configuration(format!("HTTP client: {}", e)))?;
        let response = client
            .get(current.clone())
            .send()
            .await
            .map_err(|e| PipelineError::ExternalService(format!("Failed to fetch URL: {}", e)))?;

        if response.status().is_redirection() {
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| PipelineError::ExternalService("Redirect without Location".into()))?;
            current = current
                .join(location)
                .map_err(|e| PipelineError::InvalidInput(format!("Invalid redirect URL: {}", e)))?;
            continue;
        }
        return Ok(response);
    }
    Err(PipelineError::ExternalService("Too many redirects".into()))
}

/// Ingestion stage - reads documents from various sources
#[derive(Debug, Clone, Default)]
pub struct IngestionStage {
    http_client: Option<reqwest::Client>,
}

impl IngestionStage {
    /// Create a new ingestion stage
    pub fn new() -> Self {
        Self {
            http_client: Some(reqwest::Client::new()),
        }
    }

    /// Create without HTTP client (for testing)
    pub fn without_http() -> Self {
        Self { http_client: None }
    }

    async fn ingest_text(
        &self,
        title: String,
        content: String,
        source_path: Option<String>,
    ) -> PipelineResult<RawDocument> {
        Ok(RawDocument {
            title,
            content,
            mime_type: "text/plain".to_string(),
            source_type: "text".to_string(),
            source_path,
        })
    }

    async fn ingest_file(&self, path: &str) -> PipelineResult<RawDocument> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| PipelineError::InvalidInput(format!("Failed to read file: {}", e)))?;

        let filename = std::path::Path::new(path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mime_type = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();

        Ok(RawDocument {
            title: filename,
            content,
            mime_type,
            source_type: "file".to_string(),
            source_path: Some(path.to_string()),
        })
    }

    async fn ingest_url(&self, url: &str) -> PipelineResult<RawDocument> {
        if self.http_client.is_none() {
            return Err(PipelineError::Configuration("HTTP client not configured".into()));
        }

        let response = fetch_public(url).await?;

        if !response.status().is_success() {
            return Err(PipelineError::ExternalService(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        let mime_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/html")
            .to_string();

        let content = response
            .text()
            .await
            .map_err(|e| PipelineError::ExternalService(format!("Failed to read response: {}", e)))?;

        // Extract title from URL
        let title = url::Url::parse(url)
            .ok()
            .and_then(|u| {
                u.path_segments()
                    .and_then(|mut s| s.next_back().map(|s| s.to_string()))
            })
            .unwrap_or_else(|| "untitled".to_string());

        Ok(RawDocument {
            title,
            content,
            mime_type,
            source_type: "url".to_string(),
            source_path: Some(url.to_string()),
        })
    }

    async fn ingest_bytes(
        &self,
        data: Vec<u8>,
        mime_type: String,
        filename: Option<String>,
    ) -> PipelineResult<RawDocument> {
        let content = String::from_utf8(data)
            .map_err(|e| PipelineError::InvalidInput(format!("Invalid UTF-8: {}", e)))?;

        let title = filename.unwrap_or_else(|| "untitled".to_string());

        Ok(RawDocument {
            title,
            content,
            mime_type,
            source_type: "bytes".to_string(),
            source_path: None,
        })
    }
}

#[async_trait]
impl PipelineStage<IngestionInput, RawDocument> for IngestionStage {
    fn name(&self) -> &'static str {
        "ingestion"
    }

    async fn process(
        &self,
        input: IngestionInput,
        context: &mut PipelineContext,
    ) -> PipelineResult<RawDocument> {
        let mut raw_doc = match input.source {
            DocumentSource::Text { title, content, source_path } => {
                self.ingest_text(title, content, source_path).await?
            }
            DocumentSource::File { path } => self.ingest_file(&path).await?,
            DocumentSource::Url { url } => self.ingest_url(&url).await?,
            DocumentSource::Bytes {
                data,
                mime_type,
                filename,
            } => self.ingest_bytes(data, mime_type, filename).await?,
        };

        // Apply options
        if let Some(mime_override) = input.options.mime_type {
            raw_doc.mime_type = mime_override;
        }

        // Check size limits
        if let Some(max_size) = input.options.max_size {
            if raw_doc.content.len() > max_size {
                return Err(PipelineError::InvalidInput(format!(
                    "Content exceeds maximum size of {} bytes",
                    max_size
                )));
            }
        }

        // Record metrics
        context.metrics.add_characters(raw_doc.content.len() as u64);
        context.set_metadata("source_type", &raw_doc.source_type);
        context.set_metadata("mime_type", &raw_doc.mime_type);

        Ok(raw_doc)
    }

    fn validate(&self, input: &IngestionInput) -> PipelineResult<()> {
        match &input.source {
            DocumentSource::Text { content, .. } => {
                if content.trim().is_empty() {
                    return Err(PipelineError::InvalidInput(
                        "Content cannot be empty".to_string(),
                    ));
                }
            }
            DocumentSource::File { path } => {
                if path.trim().is_empty() {
                    return Err(PipelineError::InvalidInput(
                        "File path cannot be empty".to_string(),
                    ));
                }
            }
            DocumentSource::Url { url } => {
                if url.trim().is_empty() {
                    return Err(PipelineError::InvalidInput(
                        "URL cannot be empty".to_string(),
                    ));
                }
                // Basic URL validation
                url::Url::parse(url).map_err(|e| {
                    PipelineError::InvalidInput(format!("Invalid URL: {}", e))
                })?;
            }
            DocumentSource::Bytes { data, .. } => {
                if data.is_empty() {
                    return Err(PipelineError::InvalidInput(
                        "Bytes cannot be empty".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingestion_input_from_text() {
        let input = IngestionInput::from_text("Test", "Hello world");
        match input.source {
            DocumentSource::Text { title, content, source_path } => {
                assert_eq!(title, "Test");
                assert_eq!(content, "Hello world");
                assert_eq!(source_path, None);
            }
            _ => panic!("Expected Text source"),
        }
    }

    #[test]
    fn test_ingestion_input_from_text_with_source_path() {
        let input = IngestionInput::from_text_with_source_path(
            "Test",
            "Hello world",
            "/uploads/test.md",
        );
        match input.source {
            DocumentSource::Text { title, content, source_path } => {
                assert_eq!(title, "Test");
                assert_eq!(content, "Hello world");
                assert_eq!(source_path.as_deref(), Some("/uploads/test.md"));
            }
            _ => panic!("Expected Text source"),
        }
    }

    #[test]
    fn test_ingestion_input_from_file() {
        let input = IngestionInput::from_file("/path/to/file.md");
        match input.source {
            DocumentSource::File { path } => {
                assert_eq!(path, "/path/to/file.md");
            }
            _ => panic!("Expected File source"),
        }
    }

    #[test]
    fn test_ingestion_input_from_url() {
        let input = IngestionInput::from_url("https://example.com/doc");
        match input.source {
            DocumentSource::Url { url } => {
                assert_eq!(url, "https://example.com/doc");
            }
            _ => panic!("Expected URL source"),
        }
    }

    #[tokio::test]
    async fn test_ingest_text() {
        let stage = IngestionStage::without_http();
        let mut context = PipelineContext::new();

        let input = IngestionInput::from_text("My Doc", "Some content here");
        let result = stage.process(input, &mut context).await;

        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.title, "My Doc");
        assert_eq!(doc.content, "Some content here");
        assert_eq!(doc.source_type, "text");
    }

    #[test]
    fn test_validate_empty_content() {
        let stage = IngestionStage::without_http();
        let input = IngestionInput::from_text("Title", "   ");
        let result = stage.validate(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_public_ip_rejects_internal_ranges() {
        for s in [
            "127.0.0.1", "10.1.2.3", "172.16.0.1", "172.31.255.254", "192.168.29.22", "169.254.169.254",
            "100.64.0.1", "0.0.0.0", "224.0.0.1", "255.255.255.255", "198.18.0.1", "240.0.0.1",
            "::1", "::", "fe80::1", "fc00::1", "fd12::1", "::ffff:127.0.0.1", "::ffff:10.0.0.1", "ff02::1",
        ] {
            assert!(!is_public_ip(s.parse().unwrap()), "{} must be rejected", s);
        }
        for s in ["8.8.8.8", "1.1.1.1", "93.184.216.34", "172.32.0.1", "2606:4700:4700::1111", "::ffff:8.8.8.8"] {
            assert!(is_public_ip(s.parse().unwrap()), "{} must be allowed", s);
        }
    }

    #[tokio::test]
    async fn test_fetch_public_blocks_internal_and_bad_schemes() {
        for u in [
            "http://127.0.0.1:1/", "http://localhost/", "http://169.254.169.254/latest/meta-data/",
            "http://[::1]/", "http://192.168.29.22/", "http://10.0.0.1/",
            "file:///etc/passwd", "ftp://example.com/", "http://user:pw@example.com/",
        ] {
            let err = fetch_public(u).await.expect_err(u);
            assert!(matches!(err, PipelineError::InvalidInput(_)), "{} -> {:?}", u, err);
        }
    }

    #[test]
    fn test_validate_invalid_url() {
        let stage = IngestionStage::without_http();
        let input = IngestionInput::from_url("not a url");
        let result = stage.validate(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_valid_url() {
        let stage = IngestionStage::without_http();
        let input = IngestionInput::from_url("https://example.com/doc");
        let result = stage.validate(&input);
        assert!(result.is_ok());
    }
}
