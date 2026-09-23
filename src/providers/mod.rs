mod bunny;
mod cloudflare;
mod powerdns;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// DNS record types supported by every provider.
/// (variant names are literal DNS record type names)
#[allow(clippy::upper_case_acronyms)]
#[derive(ToSchema, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum DnsRecordType {
    A,
    AAAA,
    CNAME,
    TXT,
    SRV,
}

/// A fully-rendered DNS record ready to be sent to a provider.
/// `name` is always the FQDN; providers that want relative names strip the
/// zone suffix themselves. `ttl` of `0` means "provider auto".
#[derive(Debug, Clone)]
pub struct DnsRecordInput {
    pub record_type: DnsRecordType,
    pub name: String,
    /// Record value; for SRV this is the target host.
    pub content: String,
    pub ttl: u32,
    pub proxied: bool,
    pub priority: u16,
    pub weight: u16,
    pub port: u16,
}

/// A record as persisted in `subdomains.records` - enough to delete it again.
#[derive(ToSchema, Serialize, Deserialize, Clone, Debug)]
pub struct StoredRecord {
    pub id: String,
    pub record_type: DnsRecordType,
    pub name: String,
    pub content: String,
}

#[async_trait::async_trait]
pub trait DnsProvider: Send + Sync {
    /// Verifies the credentials and zone exist; returns the zone name.
    async fn verify(&self) -> anyhow::Result<String>;
    async fn create_record(&self, record: &DnsRecordInput) -> anyhow::Result<StoredRecord>;
    /// Deleting a record that no longer exists (404) counts as success.
    async fn delete_record(&self, id: &str) -> anyhow::Result<()>;
}

pub fn build(
    provider: &str,
    zone_id: &str,
    credential: &str,
) -> anyhow::Result<Box<dyn DnsProvider>> {
    match provider {
        "cloudflare" => Ok(Box::new(cloudflare::CloudflareProvider::new(
            zone_id, credential,
        )?)),
        "bunny" => Ok(Box::new(bunny::BunnyProvider::new(zone_id, credential)?)),
        "powerdns" => Ok(Box::new(powerdns::PowerDnsProvider::new(
            zone_id, credential,
        )?)),
        _ => Err(anyhow::anyhow!("unknown dns provider `{provider}`")),
    }
}
