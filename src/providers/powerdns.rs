use super::{DnsProvider, DnsRecordInput, DnsRecordType, StoredRecord};
use serde::Deserialize;
use std::{net::IpAddr, time::Duration};

/// PowerDNS Authoritative only ever exposes the `localhost` server.
const SERVER_ID: &str = "localhost";

/// Used when a template asks for the provider's "auto" ttl.
const AUTO_TTL: u32 = 300;

/// Stored (encrypted) credential: the API is self-hosted, so the base URL is
/// part of the credential alongside the key.
#[derive(Deserialize)]
struct Credential {
    api_url: String,
    api_key: String,
}

/// PowerDNS has no per-record ids - records live in RRsets keyed by name and
/// type. Records are therefore addressed by `"{type} {name} {content}"`, where
/// `name` is canonical (trailing dot) and `content` is in PowerDNS format.
/// Only `content` may contain spaces.
struct RecordKey<'a> {
    record_type: DnsRecordType,
    name: &'a str,
    content: &'a str,
}

impl<'a> RecordKey<'a> {
    fn encode(&self) -> String {
        format!("{:?} {} {}", self.record_type, self.name, self.content)
    }

    fn decode(id: &'a str) -> anyhow::Result<Self> {
        let mut parts = id.splitn(3, ' ');
        let (Some(record_type), Some(name), Some(content)) =
            (parts.next(), parts.next(), parts.next())
        else {
            return Err(anyhow::anyhow!("powerdns: malformed record id `{id}`"));
        };

        let record_type = match record_type {
            "A" => DnsRecordType::A,
            "AAAA" => DnsRecordType::AAAA,
            "CNAME" => DnsRecordType::CNAME,
            "TXT" => DnsRecordType::TXT,
            "SRV" => DnsRecordType::SRV,
            _ => return Err(anyhow::anyhow!("powerdns: malformed record id `{id}`")),
        };

        Ok(Self {
            record_type,
            name,
            content,
        })
    }
}

pub struct PowerDnsProvider {
    client: reqwest::Client,
    /// `{api_url}/api/v1/servers/localhost/zones/{zone}.`
    zone_url: String,
    key: String,
}

impl PowerDnsProvider {
    pub fn new(zone: &str, credential: &str) -> anyhow::Result<Self> {
        let credential: Credential = serde_json::from_str(credential).map_err(|_| {
            anyhow::anyhow!("powerdns: credential must be JSON with `api_url` and `api_key`")
        })?;

        let api_url = normalize_api_url(&credential.api_url)?;
        if credential.api_key.is_empty() {
            return Err(anyhow::anyhow!("powerdns: api key is empty"));
        }

        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .build()?,
            zone_url: format!(
                "{api_url}/api/v1/servers/{SERVER_ID}/zones/{}",
                canonical(zone)
            ),
            key: credential.api_key,
        })
    }

    fn request(&self, method: reqwest::Method) -> reqwest::RequestBuilder {
        self.client
            .request(method, &self.zone_url)
            .header("X-API-Key", &self.key)
    }

    /// Current records (PowerDNS content strings) and ttl of one RRset.
    /// `Ok(None)` if the zone does not exist.
    async fn rrset(
        &self,
        name: &str,
        record_type: DnsRecordType,
    ) -> anyhow::Result<Option<(Vec<String>, Option<u64>)>> {
        let record_type = format!("{record_type:?}");
        // Older PowerDNS versions ignore the filter and return every RRset,
        // hence the client-side filter below.
        let response = self
            .request(reqwest::Method::GET)
            .query(&[("rrset_name", name), ("rrset_type", record_type.as_str())])
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(Self::response_error(response).await);
        }

        let body: serde_json::Value = response.json().await?;
        let Some(rrset) = body["rrsets"].as_array().and_then(|rrsets| {
            rrsets.iter().find(|rrset| {
                rrset["type"] == record_type.as_str()
                    && rrset["name"]
                        .as_str()
                        .is_some_and(|n| n.eq_ignore_ascii_case(name))
            })
        }) else {
            return Ok(Some((Vec::new(), None)));
        };

        let contents = rrset["records"]
            .as_array()
            .map(|records| {
                records
                    .iter()
                    .filter_map(|record| record["content"].as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();

        Ok(Some((contents, rrset["ttl"].as_u64())))
    }

    async fn patch(&self, rrset: serde_json::Value) -> anyhow::Result<()> {
        let response = self
            .request(reqwest::Method::PATCH)
            .json(&serde_json::json!({ "rrsets": [rrset] }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Self::response_error(response).await);
        }

        Ok(())
    }

    /// Extracts the `error` field from a failed PowerDNS response.
    async fn response_error(response: reqwest::Response) -> anyhow::Error {
        let status = response.status();
        let body: serde_json::Value = response.json().await.unwrap_or_default();

        let message = body["error"].as_str().unwrap_or_default();

        anyhow::anyhow!(
            "powerdns: {}",
            if message.is_empty() {
                format!("request failed with status {status}")
            } else {
                message.to_string()
            }
        )
    }
}

/// Accepts the webserver root with or without a trailing `/` or `/api/v1`.
fn normalize_api_url(url: &str) -> anyhow::Result<String> {
    let parsed = reqwest::Url::parse(url.trim())
        .map_err(|err| anyhow::anyhow!("powerdns: invalid api url: {err}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(anyhow::anyhow!("powerdns: api url must be http or https"));
    }

    let url = parsed.as_str().trim_end_matches('/');
    let url = url.strip_suffix("/api/v1").unwrap_or(url);
    Ok(url.trim_end_matches('/').to_string())
}

/// Absolute DNS name with the trailing dot PowerDNS requires.
fn canonical(name: &str) -> String {
    if name.ends_with('.') {
        name.to_string()
    } else {
        format!("{name}.")
    }
}

/// Quotes TXT content, splitting it into <=255 byte character-strings.
fn txt_content(value: &str) -> String {
    let mut chunks = Vec::new();
    let mut chunk = String::new();
    for c in value.chars() {
        if chunk.len() + c.len_utf8() > 255 {
            chunks.push(std::mem::take(&mut chunk));
        }
        chunk.push(c);
    }
    if !chunk.is_empty() || chunks.is_empty() {
        chunks.push(chunk);
    }

    chunks
        .iter()
        .map(|chunk| format!("\"{}\"", chunk.replace('\\', "\\\\").replace('"', "\\\"")))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Record content in PowerDNS zone-file format.
fn record_content(record: &DnsRecordInput) -> String {
    match record.record_type {
        DnsRecordType::A | DnsRecordType::AAAA => record.content.clone(),
        DnsRecordType::CNAME => canonical(&record.content),
        DnsRecordType::TXT => txt_content(&record.content),
        DnsRecordType::SRV => format!(
            "{} {} {} {}",
            record.priority,
            record.weight,
            record.port,
            canonical(&record.content)
        ),
    }
}

/// Whether two content strings denote the same record. PowerDNS normalizes
/// what it stores (IPv6 compression, name case), so compare semantically.
fn content_matches(record_type: DnsRecordType, a: &str, b: &str) -> bool {
    match record_type {
        DnsRecordType::A | DnsRecordType::AAAA => {
            match (a.parse::<IpAddr>(), b.parse::<IpAddr>()) {
                (Ok(a), Ok(b)) => a == b,
                _ => a == b,
            }
        }
        DnsRecordType::CNAME | DnsRecordType::SRV => a.eq_ignore_ascii_case(b),
        DnsRecordType::TXT => a == b,
    }
}

/// PATCH rrset replacing the whole RRset with `contents`.
fn replace_rrset(
    name: &str,
    record_type: DnsRecordType,
    ttl: u64,
    contents: &[String],
) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "type": format!("{record_type:?}"),
        "ttl": ttl,
        "changetype": "REPLACE",
        "records": contents
            .iter()
            .map(|content| serde_json::json!({ "content": content, "disabled": false }))
            .collect::<Vec<_>>(),
    })
}

#[async_trait::async_trait]
impl DnsProvider for PowerDnsProvider {
    async fn verify(&self) -> anyhow::Result<String> {
        let response = self
            .request(reqwest::Method::GET)
            .query(&[("rrsets", "false")])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Self::response_error(response).await);
        }

        let body: serde_json::Value = response.json().await?;
        body["name"]
            .as_str()
            .map(|name| name.trim_end_matches('.').to_string())
            .ok_or_else(|| anyhow::anyhow!("powerdns: zone response missing name"))
    }

    // RRsets can only be replaced as a whole, so creating and deleting are
    // read-modify-write. Concurrent changes to the same name+type may race.
    async fn create_record(&self, record: &DnsRecordInput) -> anyhow::Result<StoredRecord> {
        let name = canonical(&record.name);
        let content = record_content(record);

        let (mut contents, _) = self
            .rrset(&name, record.record_type)
            .await?
            .ok_or_else(|| anyhow::anyhow!("powerdns: zone not found"))?;
        if !contents
            .iter()
            .any(|existing| content_matches(record.record_type, existing, &content))
        {
            contents.push(content.clone());
        }

        let ttl = if record.ttl == 0 { AUTO_TTL } else { record.ttl };
        self.patch(replace_rrset(&name, record.record_type, ttl.into(), &contents))
            .await?;

        Ok(StoredRecord {
            id: RecordKey {
                record_type: record.record_type,
                name: &name,
                content: &content,
            }
            .encode(),
            record_type: record.record_type,
            name: record.name.clone(),
            content: record.content.clone(),
        })
    }

    async fn delete_record(&self, id: &str) -> anyhow::Result<()> {
        let key = RecordKey::decode(id)?;

        let Some((contents, ttl)) = self.rrset(key.name, key.record_type).await? else {
            return Ok(());
        };
        let remaining: Vec<String> = contents
            .iter()
            .filter(|existing| !content_matches(key.record_type, existing, key.content))
            .cloned()
            .collect();
        if remaining.len() == contents.len() {
            return Ok(());
        }

        let rrset = if remaining.is_empty() {
            serde_json::json!({
                "name": key.name,
                "type": format!("{:?}", key.record_type),
                "changetype": "DELETE",
            })
        } else {
            replace_rrset(
                key.name,
                key.record_type,
                ttl.unwrap_or(AUTO_TTL.into()),
                &remaining,
            )
        };

        self.patch(rrset).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(record_type: DnsRecordType, content: &str) -> DnsRecordInput {
        DnsRecordInput {
            record_type,
            name: "_minecraft._tcp.mc.example.com".to_string(),
            content: content.to_string(),
            ttl: 0,
            proxied: false,
            priority: 10,
            weight: 5,
            port: 25565,
        }
    }

    #[test]
    fn api_url_accepts_root_or_api_path() {
        for url in [
            "http://127.0.0.1:8081",
            "http://127.0.0.1:8081/",
            "http://127.0.0.1:8081/api/v1",
            "http://127.0.0.1:8081/api/v1/",
        ] {
            assert_eq!(normalize_api_url(url).unwrap(), "http://127.0.0.1:8081");
        }
        assert!(normalize_api_url("ftp://127.0.0.1").is_err());
        assert!(normalize_api_url("127.0.0.1:8081").is_err());
    }

    #[test]
    fn content_formats() {
        assert_eq!(
            record_content(&record(DnsRecordType::SRV, "mc.example.com")),
            "10 5 25565 mc.example.com."
        );
        assert_eq!(
            record_content(&record(DnsRecordType::CNAME, "node.example.net")),
            "node.example.net."
        );
        assert_eq!(
            record_content(&record(DnsRecordType::TXT, r#"say "hi" \o/"#)),
            r#""say \"hi\" \\o/""#
        );
    }

    #[test]
    fn long_txt_is_split_into_character_strings() {
        let content = txt_content(&"a".repeat(300));
        assert_eq!(
            content,
            format!("\"{}\" \"{}\"", "a".repeat(255), "a".repeat(45))
        );
        assert_eq!(txt_content(""), "\"\"");
    }

    #[test]
    fn content_matches_normalized_forms() {
        assert!(content_matches(
            DnsRecordType::AAAA,
            "2001:db8::1",
            "2001:0db8:0000::0001"
        ));
        assert!(content_matches(
            DnsRecordType::SRV,
            "0 5 25565 MC.example.com.",
            "0 5 25565 mc.example.com."
        ));
        assert!(!content_matches(DnsRecordType::TXT, "\"A\"", "\"a\""));
    }

    #[test]
    fn record_key_roundtrips_content_with_spaces() {
        let id = RecordKey {
            record_type: DnsRecordType::SRV,
            name: "_minecraft._tcp.mc.example.com.",
            content: "10 5 25565 mc.example.com.",
        }
        .encode();
        let key = RecordKey::decode(&id).unwrap();
        assert_eq!(key.record_type, DnsRecordType::SRV);
        assert_eq!(key.name, "_minecraft._tcp.mc.example.com.");
        assert_eq!(key.content, "10 5 25565 mc.example.com.");
        assert!(RecordKey::decode("MX example.com. 10 mail.").is_err());
    }
}
