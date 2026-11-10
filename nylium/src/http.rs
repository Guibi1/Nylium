use gpui::http_client::{self, AsyncBody, HttpClient, Request, Response, Url};
use smol::io::AsyncReadExt;
use std::{future::Future, pin::Pin, time::Duration};

pub(crate) struct ImageHttpClient;

impl HttpClient for ImageHttpClient {
    fn user_agent(&self) -> Option<&http_client::http::HeaderValue> {
        None
    }
    fn proxy(&self) -> Option<&Url> {
        None
    }
    fn send(
        &self,
        request: Request<AsyncBody>,
    ) -> Pin<Box<dyn Future<Output = http_client::Result<Response<AsyncBody>>> + Send>> {
        Box::pin(async move {
            let (parts, mut body) = request.into_parts();
            let mut bytes = Vec::new();
            body.read_to_end(&mut bytes).await?;
            smol::unblock(move || {
                let client = reqwest::blocking::Client::builder()
                    .timeout(Duration::from_secs(10))
                    .user_agent("Nylium")
                    .build()?;
                let response = client
                    .request(parts.method, parts.uri.to_string())
                    .headers(parts.headers)
                    .body(bytes)
                    .send()?;
                let mut builder = Response::builder()
                    .status(response.status())
                    .version(response.version());
                *builder.headers_mut().expect("valid HTTP status") = response.headers().clone();
                Ok(builder.body(AsyncBody::from_bytes(response.bytes()?))?)
            })
            .await
        })
    }
}
