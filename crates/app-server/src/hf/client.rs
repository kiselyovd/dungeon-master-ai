use std::time::Duration;

use reqwest::Client;

use super::types::{
    HfError, HfLicenseStatus, HfModel, HfModelDetail, HfSearchQuery, HfSort, SizeBucket,
};

const HF_BASE: &str = "https://huggingface.co";

pub struct HfClient {
    http: Client,
    token: Option<String>,
    base: String,
}

impl HfClient {
    pub fn new(token: Option<String>) -> Self {
        Self::new_with_base(token, HF_BASE.to_string())
    }

    pub fn new_with_base(token: Option<String>, base: String) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .expect("reqwest client");
        Self { http, token, base }
    }

    fn auth(&self, mut rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(t) = &self.token {
            rb = rb.bearer_auth(t);
        }
        rb
    }

    pub async fn search(&self, q: HfSearchQuery) -> Result<Vec<HfModel>, HfError> {
        let sort = match q.sort {
            HfSort::Downloads => "downloads",
            HfSort::Likes => "likes",
            HfSort::LastModified => "lastModified",
        };
        let url = format!("{}/api/models", self.base);
        let mut params: Vec<(String, String)> = vec![
            ("search".into(), q.q.clone()),
            ("filter".into(), "text-generation".into()),
            ("sort".into(), sort.into()),
            ("direction".into(), "-1".into()),
            ("limit".into(), "20".into()),
        ];
        if let Some(lic) = &q.license {
            params.push(("filter".into(), format!("license:{lic}")));
        }
        if let Some(arch) = &q.arch {
            params.push(("filter".into(), arch.clone()));
        }

        let req = self.auth(self.http.get(&url).query(&params));
        let resp = req
            .send()
            .await
            .map_err(|e| HfError::Network(e.to_string()))?;
        match resp.status().as_u16() {
            200 => {
                let body = resp
                    .text()
                    .await
                    .map_err(|e| HfError::Network(format!("read body: {e}")))?;
                let mut models: Vec<HfModel> = serde_json::from_str(&body)
                    .map_err(|e| HfError::Network(format!("parse: {e}; body={body}")))?;
                if let Some(sz) = q.size {
                    models.retain(|m| {
                        let total: u64 = m.siblings.iter().filter_map(|s| s.size).sum();
                        let gb = total as f32 / 1_000_000_000.0;
                        match sz {
                            SizeBucket::Small => gb < 4.0,
                            SizeBucket::Medium => (4.0..8.0).contains(&gb),
                            SizeBucket::Large => gb >= 8.0,
                        }
                    });
                }
                if let Some(qt) = q.quant {
                    let needle = qt.to_lowercase();
                    models.retain(|m| {
                        m.siblings
                            .iter()
                            .any(|s| s.filename.to_lowercase().contains(&needle))
                    });
                }
                Ok(models)
            }
            401 => Err(HfError::InvalidToken),
            429 => {
                let secs = resp
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(60);
                Err(HfError::RateLimited {
                    retry_after_secs: secs,
                })
            }
            other => Err(HfError::Unexpected(other)),
        }
    }

    pub async fn model_info(&self, repo_id: &str) -> Result<HfModelDetail, HfError> {
        let url = format!("{}/api/models/{}", self.base, repo_id);
        let req = self.auth(self.http.get(&url));
        let resp = req
            .send()
            .await
            .map_err(|e| HfError::Network(e.to_string()))?;
        match resp.status().as_u16() {
            200 => resp
                .json()
                .await
                .map_err(|e| HfError::Network(format!("parse: {e}"))),
            401 => Err(HfError::InvalidToken),
            404 => Err(HfError::NotFound),
            403 => {
                // gated repo not accepted
                Ok(HfModelDetail {
                    repo_id: repo_id.into(),
                    gated: true,
                    tags: vec![],
                    siblings: vec![],
                    card_data: serde_json::Value::Null,
                })
            }
            other => Err(HfError::Unexpected(other)),
        }
    }

    pub async fn check_license(&self, repo_id: &str) -> Result<HfLicenseStatus, HfError> {
        let url = format!("{}/api/models/{}", self.base, repo_id);
        let req = self.auth(self.http.get(&url));
        let resp = req
            .send()
            .await
            .map_err(|e| HfError::Network(e.to_string()))?;
        match resp.status().as_u16() {
            200 => {
                let detail: HfModelDetail = resp
                    .json()
                    .await
                    .map_err(|e| HfError::Network(format!("parse: {e}")))?;
                Ok(HfLicenseStatus {
                    gated: detail.gated,
                    accepted: true,
                })
            }
            403 => Ok(HfLicenseStatus {
                gated: true,
                accepted: false,
            }),
            401 => Err(HfError::InvalidToken),
            404 => Err(HfError::NotFound),
            other => Err(HfError::Unexpected(other)),
        }
    }
}
