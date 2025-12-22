use crate::gen;

/// HTTP client implementation for Starknet RPC calls
#[derive(Clone, Debug)]
pub struct Http(pub reqwest::Client);

impl Http {
    /// Create a new HTTP client
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(reqwest::Client::new())
    }
}

/// Generic HTTP POST function for JSON-RPC requests
async fn post<Q: serde::Serialize, R: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
    request: Q,
) -> std::result::Result<R, iamgroot::jsonrpc::Error> {
    let response = client
        .post(url)
        .json(&request)
        .send()
        .await
        .map_err(|e| {
            iamgroot::jsonrpc::Error::new(
                32101,
                format!("request failed: {e:?}"),
            )
        })?
        .json()
        .await
        .map_err(|e| {
            iamgroot::jsonrpc::Error::new(
                32102,
                format!("invalid response: {e:?}"),
            )
        })?;
    Ok(response)
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl gen::client::HttpClient for Http {
    async fn post(
        &self,
        url: &str,
        request: &iamgroot::jsonrpc::Request,
    ) -> std::result::Result<
        iamgroot::jsonrpc::Response,
        iamgroot::jsonrpc::Error,
    > {
        post(&self.0, url, request).await
    }
}

impl gen::client::blocking::HttpClient for Http {
    fn post(
        &self,
        url: &str,
        request: &iamgroot::jsonrpc::Request,
    ) -> std::result::Result<
        iamgroot::jsonrpc::Response,
        iamgroot::jsonrpc::Error,
    > {
        #[cfg(target_arch = "wasm32")]
        unreachable!("Blocking HTTP attempt: url={url} request={request:?}");

        #[cfg(not(target_arch = "wasm32"))]
        {
            ureq::post(url)
                .send_json(request)
                .map_err(|e| {
                    iamgroot::jsonrpc::Error::new(33101, e.to_string())
                })?
                .body_mut()
                .read_json()
                .map_err(|e| {
                    iamgroot::jsonrpc::Error::new(33102, e.to_string())
                })
        }
    }
}
