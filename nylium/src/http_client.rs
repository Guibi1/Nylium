use std::any::type_name;
use std::mem;
use std::time::Duration;

use async_compat::CompatExt;
use gpui::http_client;
use http_client::{Url, http};
use reqwest::header::{HeaderMap, HeaderValue};
use smol::future::FutureExt;

/// This is a gutted-out struct that implements gpui::HttpClient.
///
/// Original code from <https://github.com/zed-industries/zed/blob/main/crates/reqwest_client/src/reqwest_client.rs>
pub struct HttpClient {
    client: reqwest::Client,
    user_agent: Option<HeaderValue>,
}

impl HttpClient {
    pub fn new() -> Self {
        let user_agent = HeaderValue::from_str("nylium-gpui").unwrap();
        let mut map = HeaderMap::new();
        map.insert(http::header::USER_AGENT, user_agent.clone());

        Self {
            client: reqwest::Client::builder()
                .use_rustls_tls()
                .connect_timeout(Duration::from_secs(10))
                .default_headers(map)
                .build()
                .expect("Failed to initialize HTTP client"),
            user_agent: Some(user_agent),
        }
    }
}

impl http_client::HttpClient for HttpClient {
    fn proxy(&self) -> Option<&Url> {
        None
    }

    fn type_name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn user_agent(&self) -> Option<&HeaderValue> {
        self.user_agent.as_ref()
    }

    fn send(
        &self,
        req: http::Request<http_client::AsyncBody>,
    ) -> smol::future::Boxed<gpui::Result<http_client::Response<http_client::AsyncBody>>> {
        let (parts, _body) = req.into_parts();

        let mut request: reqwest::RequestBuilder =
            self.client.request(parts.method, parts.uri.to_string());
        request = request.headers(parts.headers);

        async move {
            let mut response = request.send().compat().await?;

            let mut builder = http::Response::builder();
            *builder.headers_mut().unwrap() = mem::take(response.headers_mut());

            gpui::Result::Ok(
                builder
                    .status(response.status().as_u16())
                    .version(response.version())
                    .body(http_client::AsyncBody::from_bytes(response.bytes().await?))
                    .unwrap(),
            )
        }
        .boxed()
    }
}
