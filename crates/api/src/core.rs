#![cfg(not(target_arch = "wasm32"))]
use {
    crate::*,
    anyhow::Result,
    hmac::{Hmac, Mac},
    serde::de::DeserializeOwned,
    sha1::Sha1,
};

use serde::{Deserialize, Serialize};

use code_generator::SwaggerClient;
use derive_more::Display;

pub const API_URL: &str = "https://timetableapi.ptv.vic.gov.au";

type PtvHmac = Hmac<Sha1>;

use anyhow::Error;

#[derive(SwaggerClient)]
#[swagger(
    path = "v3",
    strip_prefix = "V3.",
    extra_names = [("RouteType", "ty::RouteType")],
    skip = ["signature"]
)]
pub struct Client {
    #[swagger(static)]
    devid: String,
    #[swagger(static)]
    token: String,
}
pub fn to_query<T: Serialize>(s: T) -> String {
    serde_json::to_value(s)
        .unwrap()
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| {
            // If v is an array, define k={v[0]}&k={v[1]}&...
            if v.is_array() {
                v.as_array()
                    .unwrap()
                    .iter()
                    .map(|v| {
                        format!(
                            "{}={}",
                            k,
                            url_escape::encode_query(&clean(v.to_string())).into_owned()
                        )
                    })
                    .collect::<Vec<String>>()
                    .join("&")
            } else {
                format!("{}={}", k, clean(v.to_string()))
            }
        })
        .collect::<Vec<String>>()
        .join("&")
}

impl Client {
    pub fn new(devid: String, token: String) -> Self {
        Self { devid, token }
    }
    pub async fn rq<T: DeserializeOwned>(&self, path: String) -> Result<T> {
        println!("Request path: {}", path);
        let path = format!(
            "{path}{}devid={}",
            {
                if !path.contains('?') {
                    "?"
                } else if path.ends_with('?') {
                    ""
                } else {
                    "&"
                }
            },
            self.devid
        );

        let mut hasher: PtvHmac = Hmac::new_from_slice(self.token.as_bytes()).unwrap();
        hasher.update(path.as_bytes());

        let hash = hex::encode(hasher.finalize().into_bytes());
        let url = format!("{API_URL}{}&signature={}", path, hash);

        if std::env::var("DEBUG").is_ok() {
            println!("Requesting: {}", url);
        }

        let res = reqwest::get(&url).await?;
        if !res.status().is_success() {
            let status = res.status();
            if let Ok(ApiError { message, .. }) = res.json().await {
                return Err(anyhow::anyhow!("Request failed: {} - {}", status, message));
            }
            return Err(anyhow::anyhow!("Request failed: {}", status));
        }

        Ok(res.json().await?)
    }
}
