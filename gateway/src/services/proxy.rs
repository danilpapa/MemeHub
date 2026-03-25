use axum::body::Body;
use axum::http::{HeaderMap, Request, Response, StatusCode};
use axum::response::IntoResponse;
use reqwest::Client;

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
        self,
        path: String,
        req: Request<Body>,
    ) -> impl IntoResponse {
        let (parts, body) = req.into_parts();

        let method = parts
            .method;
        let headers = parts
            .headers;
        let query_string = parts.uri.query()
            .map(|query| format!("?{query}"))
            .unwrap_or_else(String::new);

        let url = format!(
            "{}/{}{}",
            self.base_url,
            path,
            query_string
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

        let resp = match builder.body(bytes).send().await {
            Ok(response) => response,
            Err(_) => {
                return (StatusCode::BAD_GATEWAY, "upstream error").into_response()
            }
        };

        let status = resp.status();
        let mut out_headers = HeaderMap::new();
        for (k, v) in resp.headers() {
            out_headers.insert(k, v.clone());
        }
        let body = resp.bytes().await.unwrap_or_default();

        (status, out_headers, body).into_response()
    }
}