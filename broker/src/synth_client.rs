//! OpenAI-compatible synthesis client for `generate_wiki`.
//!
//! This is the one place the broker calls an external LLM directly — an
//! explicit, scoped exception to the system-wide vendor-neutral, no-synthesis
//! principle (approved plan decision (b)/(c), spec Fact#11). The endpoint is
//! entirely user-configured (`[synthesis]` in `broker.toml`); nothing here is
//! hardcoded to a specific vendor, and a local Ollama endpoint works the same
//! as a hosted one.
//!
//! Non-streaming only: a single `POST {base_url}/chat/completions`, parsed as
//! a complete JSON response. This guarantees no partial/truncated content can
//! ever reach the validation gate in [`crate::wiki`] (pre-mortem scenario 3:
//! a streamed partial response could otherwise leave an orphaned document).

use crate::config::SynthesisConfig;
use crate::wiki::WikiError;
use serde::Deserialize;
use std::time::Duration;

/// Stateless (per-call) client — constructed fresh for each `generate_wiki`
/// invocation, matching the broker's no-resident-daemon posture.
pub struct SynthClient {
    base_url: String,
    api_key: Option<String>,
    model: String,
    http: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    content: String,
}

impl SynthClient {
    /// Build a client from `[synthesis]` config. The API key file is
    /// optional (unauthenticated local endpoints like Ollama have none); a
    /// missing/unreadable key file is treated as "no key", not an error.
    pub fn new(config: &SynthesisConfig) -> Self {
        let api_key = std::fs::read_to_string(&config.api_key_file)
            .ok()
            .map(|raw| raw.trim().to_string())
            .filter(|k| !k.is_empty());

        let timeout = Duration::from_secs(config.timeout_secs);
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .connect_timeout(timeout)
            .build()
            .unwrap_or_default();

        Self {
            base_url: config.base_url.trim_end_matches('/').to_string(),
            api_key,
            model: config.model.clone(),
            http,
        }
    }

    /// Synthesize `prompt` into a single markdown-ready string via a
    /// non-streaming chat-completion call.
    pub async fn synthesize(&self, prompt: &str) -> Result<String, WikiError> {
        let url = format!("{}/chat/completions", self.base_url);
        let body = serde_json::json!({
            "model": self.model,
            "stream": false,
            "messages": [
                { "role": "user", "content": prompt }
            ],
        });

        let mut req = self.http.post(&url).json(&body);
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }

        // Any transport-level failure (connect refused, DNS, timeout, or a
        // body-stream error mid-request) means "no usable response was
        // obtained" — always hard-refuse rather than risk a partial write.
        let resp = req
            .send()
            .await
            .map_err(|e| WikiError::SynthOffline(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(WikiError::SynthHttp {
                status: status.as_u16(),
                body,
            });
        }

        let parsed: ChatCompletionResponse = resp
            .json()
            .await
            .map_err(|e| WikiError::SynthOffline(format!("malformed response body: {e}")))?;

        parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| {
                WikiError::SynthOffline("synthesis response contained no choices".to_string())
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SynthesisConfig;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn config_for(base_url: String) -> SynthesisConfig {
        SynthesisConfig {
            enable: true,
            base_url,
            api_key_file: std::path::PathBuf::from("/nonexistent/.synth-key"),
            model: "gpt-4o-mini".to_string(),
            timeout_secs: 2,
        }
    }

    #[tokio::test]
    async fn parses_successful_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [
                    { "message": { "content": "synthesized markdown body" } }
                ]
            })))
            .mount(&server)
            .await;

        let client = SynthClient::new(&config_for(server.uri()));
        let result = client.synthesize("summarize rust ownership").await.unwrap();
        assert_eq!(result, "synthesized markdown body");
    }

    #[tokio::test]
    async fn http_500_maps_to_synth_http() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
            .mount(&server)
            .await;

        let client = SynthClient::new(&config_for(server.uri()));
        let err = client.synthesize("query").await.unwrap_err();
        match err {
            WikiError::SynthHttp { status, body } => {
                assert_eq!(status, 500);
                assert_eq!(body, "internal error");
            }
            other => panic!("expected SynthHttp, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn timeout_maps_to_synth_offline() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
            .mount(&server)
            .await;

        let mut cfg = config_for(server.uri());
        cfg.timeout_secs = 1;
        let client = SynthClient::new(&cfg);
        let err = client.synthesize("query").await.unwrap_err();
        assert!(matches!(err, WikiError::SynthOffline(_)), "got {err:?}");
    }

    #[tokio::test]
    async fn connect_failure_maps_to_synth_offline() {
        // Port not bound: connection refused.
        let client = SynthClient::new(&config_for("http://127.0.0.1:1".to_string()));
        let err = client.synthesize("query").await.unwrap_err();
        assert!(matches!(err, WikiError::SynthOffline(_)), "got {err:?}");
    }
}
