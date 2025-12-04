#![cfg(not(target_arch = "wasm32"))]
use std::fmt::Debug;

use {
    crate::*,
    anyhow::Result,
    hmac::{Hmac, Mac},
    serde::de::DeserializeOwned,
    sha1::Sha1,
};

use serde::{Deserialize, Serialize};

use code_generator::SwaggerClient;

pub const API_URL: &str = "https://timetableapi.ptv.vic.gov.au";

type PtvHmac = Hmac<Sha1>;

use anyhow::Error;

#[derive(Serialize, Deserialize, Debug)]
pub struct Modes(#[serde(serialize_with = "ser_disruption_query")] pub DisruptionMode);

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum DateTime {
    Naive(chrono::NaiveDateTime),
    WithTz(chrono::DateTime<chrono::Utc>),
}

#[derive(SwaggerClient)]
#[swagger(
    path = "v3",
    strip_prefix = "V3.",
    extra_names = [("RouteType", "crate::ty::RouteType"), ("Status", "crate::ty::Status"), ("Expand", "Vec<crate::ty::ExpandOptions>"), ("ServiceOperator", "crate::ty::ServiceOperator"), ("DisruptionStatus", "crate::ty::DisruptionStatus"), ("Geopath", "Option<crate::ty::Geopath>"),("RouteId", "crate::ty::RouteId"),("StopId", "crate::ty::StopId"),("RunId", "crate::ty::RunId"),("DirectionId", "crate::ty::DirectionId"),("DisruptionId", "crate::ty::DisruptionId"), ("DisruptionMode", "crate::ty::DisruptionMode"), ("DisruptionModes", "crate::core::Modes"), ("DateTime", "crate::core::DateTime")],
    path_skip = ["/v3/disruptions/modes", "/v3/routes/types"],
    skip = ["signature"]
)]
pub struct Client {
    #[swagger(static)]
    devid: String,
    #[swagger(static)]
    token: String,
}

use helpers::to_query;

impl Client {
    pub fn new(devid: String, token: String) -> Self {
        Self { devid, token }
    }
    pub async fn rq<T: DeserializeOwned + Debug>(&self, path: String) -> Result<T> {
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

        let hash = hex::encode(hasher.finalize().into_bytes()).to_uppercase();
        let url = format!("{API_URL}{}&signature={}", path, hash);

        if std::env::var("DEBUG").is_ok() {
            println!("Requesting: |{}|", url);
        }

        let res = reqwest::get(&url).await?;
        if !res.status().is_success() {
            let status = res.status();
            if let Ok(ApiError { message, .. }) = res.json().await {
                return Err(anyhow::anyhow!("Request failed: {} - {}", status, message));
            }
            return Err(anyhow::anyhow!("Request failed: {}", status));
        }
        //        println!("{}", res.text().await?);
        let res = res.text().await?;
        let mut deserializer = serde_json::Deserializer::from_str(&res);

        let res: T = serde_path_to_error::deserialize(&mut deserializer).map_err(|e| {
            anyhow::anyhow!(
                "Error at path: {} {} - response: {}",
                e.path(),
                e.inner(),
                res
            )
        })?;
        Ok(res)
    }
}
