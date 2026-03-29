use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode};
use axum::response::{IntoResponse, Response as AxumResponse};
use reqwest::Client;
use tower_http::request_id::RequestId;
use tracing::{Instrument, error, info, info_span};

#[derive(Clone)]
pub struct ProxyService {
    pub client: Client,
    pub base_url: String,
}

impl ProxyService {
    pub fn new(client: Client, base_url: String) -> Self {
        Self { client, base_url }
    }

    pub async fn forward(
        &self,
        path: String,
        req: Request<Body>,
    ) -> AxumResponse {
        let (parts, body) = req.into_parts();

        let method = parts
            .method;
        let headers = parts
            .headers;
        let query_string = parts.uri.query()
            .map(|query| format!("?{query}"))
            .unwrap_or_else(String::new);
        let request_id = parts
            .extensions
            .get::<RequestId>()
            .and_then(|request_id| request_id.header_value().to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        let url = format!(
            "{}/{}{}",
            self.base_url,
            path,
            query_string
        );
        let upstream_span = info_span!(
            "proxy_upstream",
            request_id = %request_id,
            upstream_url = %url,
            upstream_method = %method,
        );

        let mut builder = self.client.request(
            method.clone(),
            &url
        );

        for (name, val) in headers.iter() {
            if name.as_str().eq_ignore_ascii_case("host") {
                continue;
            }
            builder = builder.header(name, val);
        }

        let bytes = axum::body::to_bytes(body, usize::MAX)
            .await
            .unwrap_or_default();
        async move {
            info!("forwarding request to upstream");

            let resp = match builder.body(bytes).send().await {
                Ok(response) => response,
                Err(error) => {
                    error!(error = %error, "upstream request failed");
                    return (StatusCode::BAD_GATEWAY, "upstream error").into_response()
                }
            };

            let status = resp.status();
            let mut out_headers = HeaderMap::new();
            for (k, v) in resp.headers() {
                out_headers.insert(k, v.clone());
            }
            let body = resp.bytes().await.unwrap_or_default();

            info!(upstream_status = status.as_u16(), response_size = body.len(), "upstream response received");

            (status, out_headers, body).into_response()
        }
        .instrument(upstream_span)
        .await
    }
}
