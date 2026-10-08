use axum::{body::Body, extract::Request, middleware::Next, response::Response};

/// Normalizes the Content-Type header for multipart/form-data requests.
///
/// Safari/WebKit sends a quoted boundary parameter:
///   Content-Type: multipart/form-data; boundary="----WebKitFormBoundaryXXX"
///
/// The multer library requires an unquoted boundary token, so we strip the
/// quotes before the Multipart extractor runs.
pub async fn fix_multipart_boundary(mut request: Request<Body>, next: Next) -> Response {
    if let Some(ct) = request.headers().get(axum::http::header::CONTENT_TYPE).cloned() {
        if let Ok(ct_str) = ct.to_str() {
            if ct_str.contains("multipart/form-data") {
                if let Some(fixed) = unquote_boundary(ct_str) {
                    if let Ok(new_ct) = fixed.parse() {
                        request
                            .headers_mut()
                            .insert(axum::http::header::CONTENT_TYPE, new_ct);
                    }
                }
            }
        }
    }
    next.run(request).await
}

fn unquote_boundary(content_type: &str) -> Option<String> {
    let boundary_key = "boundary=\"";
    let idx = content_type.find(boundary_key)?;
    let after_open = &content_type[idx + boundary_key.len()..];
    let close = after_open.find('"')?;
    let value = &after_open[..close];
    let prefix = &content_type[..idx + "boundary=".len()];
    let suffix = &after_open[close + 1..];
    Some(format!("{}{}{}", prefix, value, suffix))
}
