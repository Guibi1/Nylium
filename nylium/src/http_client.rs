use std::any::type_name;
use std::mem;
use std::sync::LazyLock;
use std::time::Duration;

use gpui::http_client;
use http_client::{Url, http};
use reqwest::header::{HeaderMap, HeaderValue};
use smol::future::FutureExt;

static RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("Failed to initialize HTTP client")
});

/// This is a gutted-out struct that implements gpui::HttpClient.
///
/// Original code from <https://github.com/zed-industries/zed/blob/main/crates/reqwest_client/src/reqwest_client.rs>
pub struct ReqwestClient {
    client: reqwest::Client,
    user_agent: Option<HeaderValue>,
}

impl ReqwestClient {
    pub fn new() -> Self {
        let user_agent = HeaderValue::from_str("nylium-gpui").unwrap();
        let mut map = HeaderMap::new();
        map.insert(http::header::USER_AGENT, user_agent.clone());

        Self {
            client: reqwest::Client::builder()
                // .use_rustls_tls()
                .connect_timeout(Duration::from_secs(10))
                .default_headers(map)
                .build()
                .expect("Failed to initialize HTTP client"),
            user_agent: Some(user_agent),
        }
    }
}

impl http_client::HttpClient for ReqwestClient {
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
        let (parts, body) = req.into_parts();

        let mut request: reqwest::RequestBuilder =
            self.client.request(parts.method, parts.uri.to_string());
        request = request.headers(parts.headers);

        let request = request.body(match body.0 {
            http_client::Inner::Empty => reqwest::Body::default(),
            http_client::Inner::Bytes(cursor) => cursor.into_inner().into(),
            http_client::Inner::AsyncReader(_) => {
                unimplemented!("AsyncReader is not needed, and therefore was removed.")
            }
        });

        async move {
            let handle =
                tokio::runtime::Handle::try_current().unwrap_or_else(|_| RUNTIME.handle().clone());

            let mut response = handle.spawn(async { request.send().await }).await??;

            let mut builder = http::Response::builder()
                .status(response.status().as_u16())
                .version(response.version());

            let headers = mem::take(response.headers_mut());
            *builder.headers_mut().unwrap() = headers;

            let bytes = response.bytes().await?;
            let body = http_client::AsyncBody::from_bytes(bytes);

            gpui::Result::Ok(builder.body(body).unwrap())
        }
        .boxed()
    }
}
