use super::{DnsProvider, DnsRecordInput, DnsRecordType, StoredRecord};
use std::time::Duration;

const BASE: &str = "https://api.cloudflare.com/client/v4";

pub struct CloudflareProvider {
    client: reqwest::Client,
    zone_id: String,
    token: String,
}

impl CloudflareProvider {
    pub fn new(zone_id: &str, token: &str) -> anyhow::Result<Self> {
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .build()?,
            zone_id: zone_id.to_string(),
            token: token.to_string(),
        })
    }

    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.client
            .request(method, format!("{BASE}{path}"))
            .bearer_auth(&self.token)
    }

    /// Extracts the `errors[].message` list from a failed Cloudflare response.
    async fn response_error(response: reqwest::Response) -> anyhow::Error {
        let status = response.status();
        let body: serde_json::Value = response.json().await.unwrap_or_default();

        let messages = body["errors"]
            .as_array()
            .map(|errors| {
                errors
                    .iter()
                    .filter_map(|e| e["message"].as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();

        anyhow::anyhow!(
            "cloudflare: {}",
            if messages.is_empty() {
                format!("request failed with status {status}")
            } else {
                messages
            }
        )
    }
}

/// Request body for `POST /zones/{zone}/dns_records`. Split out so it can be
/// unit-tested without a network.
pub(crate) fn create_body(record: &DnsRecordInput) -> serde_json::Value {
    // Cloudflare's "auto" ttl is 1
    let ttl = if record.ttl == 0 { 1 } else { record.ttl };

    match record.record_type {
        DnsRecordType::SRV => serde_json::json!({
            "type": "SRV",
            "name": record.name,
            "ttl": ttl,
            "data": {
                "priority": record.priority,
                "weight": record.weight,
                "port": record.port,
                "target": record.content,
            },
        }),
        record_type => {
            // `proxied` is only valid for A/AAAA/CNAME; sending it for TXT errors out
            let mut body = serde_json::json!({
                "type": format!("{record_type:?}"),
                "name": record.name,
                "content": record.content,
                "ttl": ttl,
            });
            if matches!(
                record_type,
                DnsRecordType::A | DnsRecordType::AAAA | DnsRecordType::CNAME
            ) {
                body["proxied"] = serde_json::Value::Bool(record.proxied);
            }
            body
        }
    }
}

#[async_trait::async_trait]
impl DnsProvider for CloudflareProvider {
    async fn verify(&self) -> anyhow::Result<String> {
        let response = self
            .request(reqwest::Method::GET, &format!("/zones/{}", self.zone_id))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Self::response_error(response).await);
        }

        let body: serde_json::Value = response.json().await?;
        body["result"]["name"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| anyhow::anyhow!("cloudflare: zone response missing name"))
    }

    async fn create_record(&self, record: &DnsRecordInput) -> anyhow::Result<StoredRecord> {
        let response = self
            .request(
                reqwest::Method::POST,
                &format!("/zones/{}/dns_records", self.zone_id),
            )
            .json(&create_body(record))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Self::response_error(response).await);
        }

        let body: serde_json::Value = response.json().await?;
        let id = body["result"]["id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("cloudflare: create response missing record id"))?;

        Ok(StoredRecord {
            id: id.to_string(),
            record_type: record.record_type,
            name: record.name.clone(),
            content: record.content.clone(),
        })
    }

    async fn delete_record(&self, id: &str) -> anyhow::Result<()> {
        let response = self
            .request(
                reqwest::Method::DELETE,
                &format!("/zones/{}/dns_records/{id}", self.zone_id),
            )
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND || response.status().is_success() {
            return Ok(());
        }

        Err(Self::response_error(response).await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(record_type: DnsRecordType) -> DnsRecordInput {
        DnsRecordInput {
            record_type,
            name: "mc.example.com".to_string(),
            content: "203.0.113.10".to_string(),
            ttl: 0,
            proxied: true,
            priority: 0,
            weight: 5,
            port: 25565,
        }
    }

    #[test]
    fn create_body_a_auto_ttl_and_proxy() {
        let body = create_body(&record(DnsRecordType::A));
        assert_eq!(body["type"], "A");
        assert_eq!(body["name"], "mc.example.com");
        assert_eq!(body["content"], "203.0.113.10");
        assert_eq!(body["ttl"], 1);
        assert_eq!(body["proxied"], true);
    }

    #[test]
    fn create_body_txt_omits_proxied() {
        let body = create_body(&record(DnsRecordType::TXT));
        assert_eq!(body["type"], "TXT");
        assert_eq!(body["ttl"], 1);
        assert!(body.get("proxied").is_none());
    }

    #[test]
    fn create_body_explicit_ttl() {
        let mut r = record(DnsRecordType::CNAME);
        r.ttl = 120;
        r.content = "target.example.com".to_string();
        let body = create_body(&r);
        assert_eq!(body["ttl"], 120);
        assert_eq!(body["proxied"], true);
    }

    #[test]
    fn create_body_srv_uses_data_object() {
        let mut r = record(DnsRecordType::SRV);
        r.name = "_minecraft._tcp.mc.example.com".to_string();
        r.content = "mc.example.com".to_string();
        r.priority = 10;
        let body = create_body(&r);
        assert_eq!(body["type"], "SRV");
        assert_eq!(body["name"], "_minecraft._tcp.mc.example.com");
        assert_eq!(body["data"]["priority"], 10);
        assert_eq!(body["data"]["weight"], 5);
        assert_eq!(body["data"]["port"], 25565);
        assert_eq!(body["data"]["target"], "mc.example.com");
        assert!(body.get("content").is_none());
        assert!(body.get("proxied").is_none());
    }
}
