use garde::Validate;
use serde::{Deserialize, Serialize};
use shared::extensions::settings::{
    ExtensionSettings, SettingsDeserializeExt, SettingsDeserializer, SettingsSerializeExt,
    SettingsSerializer,
};
use utoipa::ToSchema;

use crate::providers::DnsRecordType;

/// A template describing one DNS record to create when a subdomain is added.
/// Strings may contain `{name}`, `{domain}`, `{fqdn}`, `{ip}`, `{port}`,
/// `{server}` and `{server_name}` placeholders, rendered at creation time.
/// A `ttl` of `0` means the provider's "auto" value.
#[derive(ToSchema, Validate, Serialize, Deserialize, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RecordTemplate {
    /// Creates an A, AAAA or CNAME record depending on what the resolved
    /// allocation target is (IPv4, IPv6 or hostname).
    Address {
        #[garde(length(chars, min = 1, max = 255))]
        #[schema(min_length = 1, max_length = 255)]
        name: String,
        #[garde(skip)]
        proxied: bool,
        #[garde(skip)]
        #[schema(minimum = 0)]
        ttl: u32,
    },
    /// Creates an SRV record named `{service}.{protocol}.{name}.{domain}`
    /// pointing at `{fqdn}` on the allocation's port.
    Srv {
        #[garde(length(chars, min = 1, max = 63))]
        #[schema(min_length = 1, max_length = 63)]
        service: String,
        #[garde(length(chars, min = 1, max = 63))]
        #[schema(min_length = 1, max_length = 63)]
        protocol: String,
        #[garde(skip)]
        priority: u16,
        #[garde(skip)]
        weight: u16,
        #[garde(skip)]
        #[schema(minimum = 0)]
        ttl: u32,
    },
    /// Creates a record of an explicit type with a rendered name and content.
    Custom {
        #[garde(skip)]
        record_type: DnsRecordType,
        #[garde(length(chars, min = 1, max = 255))]
        #[schema(min_length = 1, max_length = 255)]
        name: String,
        #[garde(length(chars, min = 1, max = 1024))]
        #[schema(min_length = 1, max_length = 1024)]
        content: String,
        #[garde(skip)]
        #[schema(minimum = 0)]
        ttl: u32,
    },
}

/// Per-egg record template overrides.
#[derive(ToSchema, Validate, Serialize, Deserialize, Clone)]
pub struct EggRecords {
    #[garde(skip)]
    pub egg_uuid: uuid::Uuid,
    #[garde(dive)]
    pub records: Vec<RecordTemplate>,
}

/// Per-nest record template overrides, used for servers whose egg has no
/// override of its own.
#[derive(ToSchema, Validate, Serialize, Deserialize, Clone)]
pub struct NestRecords {
    #[garde(skip)]
    pub nest_uuid: uuid::Uuid,
    #[garde(dive)]
    pub records: Vec<RecordTemplate>,
}

#[derive(ToSchema, Validate, Serialize, Deserialize, Clone)]
pub struct ExtensionSettingsData {
    /// Regexes matched case-insensitively against requested subdomain names.
    #[garde(inner(length(chars, min = 1, max = 255)))]
    pub blacklist: Vec<String>,
    /// Subdomain limit applied to new servers when the create payload does
    /// not carry `feature_limits.subdomains`.
    #[garde(range(min = 0))]
    #[schema(minimum = 0)]
    pub default_limit: i32,
    /// Record templates used for servers without an egg or nest override.
    #[garde(dive)]
    pub default_records: Vec<RecordTemplate>,
    /// Per-nest record template overrides.
    #[garde(dive)]
    #[serde(default)]
    pub nest_records: Vec<NestRecords>,
    /// Per-egg record template overrides; these win over nest overrides.
    #[garde(dive)]
    pub egg_records: Vec<EggRecords>,
}

impl Default for ExtensionSettingsData {
    fn default() -> Self {
        Self {
            blacklist: vec![
                "^www$".to_string(),
                "^mail$".to_string(),
                "^admin$".to_string(),
                "^panel$".to_string(),
            ],
            default_limit: 0,
            default_records: vec![
                RecordTemplate::Address {
                    name: "{name}".to_string(),
                    proxied: false,
                    ttl: 0,
                },
                RecordTemplate::Srv {
                    service: "_minecraft".to_string(),
                    protocol: "_tcp".to_string(),
                    priority: 0,
                    weight: 5,
                    ttl: 0,
                },
            ],
            nest_records: Vec::new(),
            egg_records: Vec::new(),
        }
    }
}

impl ExtensionSettingsData {
    /// Compiles the configured blacklist patterns. Invalid patterns are
    /// skipped here - they are rejected at write time by the settings route.
    pub fn compiled_blacklist(&self) -> Vec<regex::Regex> {
        self.blacklist
            .iter()
            .filter_map(|pattern| {
                regex::RegexBuilder::new(pattern)
                    .case_insensitive(true)
                    .build()
                    .ok()
            })
            .collect()
    }

    /// Record templates for a server: the egg override, else the nest
    /// override, else `default_records`.
    pub fn records_for(&self, egg_uuid: uuid::Uuid, nest_uuid: uuid::Uuid) -> &[RecordTemplate] {
        if let Some(entry) = self
            .egg_records
            .iter()
            .find(|entry| entry.egg_uuid == egg_uuid)
        {
            return &entry.records;
        }
        if let Some(entry) = self
            .nest_records
            .iter()
            .find(|entry| entry.nest_uuid == nest_uuid)
        {
            return &entry.records;
        }
        &self.default_records
    }
}

#[async_trait::async_trait]
impl SettingsSerializeExt for ExtensionSettingsData {
    async fn serialize(
        &self,
        serializer: SettingsSerializer,
    ) -> Result<SettingsSerializer, anyhow::Error> {
        Ok(serializer
            .write_serde_setting("blacklist", &self.blacklist)?
            .write_serde_setting("default_limit", &self.default_limit)?
            .write_serde_setting("default_records", &self.default_records)?
            .write_serde_setting("nest_records", &self.nest_records)?
            .write_serde_setting("egg_records", &self.egg_records)?)
    }
}

pub struct ExtensionSettingsDataDeserializer;

#[async_trait::async_trait]
impl SettingsDeserializeExt for ExtensionSettingsDataDeserializer {
    async fn deserialize_boxed(
        &self,
        deserializer: SettingsDeserializer<'_>,
    ) -> Result<ExtensionSettings, anyhow::Error> {
        let defaults = ExtensionSettingsData::default();

        Ok(Box::new(ExtensionSettingsData {
            blacklist: deserializer
                .read_serde_setting("blacklist")
                .unwrap_or(defaults.blacklist),
            default_limit: deserializer
                .read_serde_setting("default_limit")
                .unwrap_or(defaults.default_limit),
            default_records: deserializer
                .read_serde_setting("default_records")
                .unwrap_or(defaults.default_records),
            nest_records: deserializer
                .read_serde_setting("nest_records")
                .unwrap_or(defaults.nest_records),
            egg_records: deserializer
                .read_serde_setting("egg_records")
                .unwrap_or(defaults.egg_records),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn address(name: &str) -> RecordTemplate {
        RecordTemplate::Address {
            name: name.to_string(),
            proxied: false,
            ttl: 0,
        }
    }

    fn record_name(records: &[RecordTemplate]) -> &str {
        match records {
            [RecordTemplate::Address { name, .. }] => name,
            _ => panic!("expected a single address template"),
        }
    }

    #[test]
    fn records_for_prefers_egg_then_nest_then_default() {
        let nest = uuid::Uuid::from_u128(1);
        let egg = uuid::Uuid::from_u128(2);
        let sibling_egg = uuid::Uuid::from_u128(3);
        let other_nest = uuid::Uuid::from_u128(4);

        let settings = ExtensionSettingsData {
            default_records: vec![address("default")],
            nest_records: vec![NestRecords {
                nest_uuid: nest,
                records: vec![address("nest")],
            }],
            egg_records: vec![EggRecords {
                egg_uuid: egg,
                records: vec![address("egg")],
            }],
            ..Default::default()
        };

        assert_eq!(record_name(settings.records_for(egg, nest)), "egg");
        assert_eq!(record_name(settings.records_for(sibling_egg, nest)), "nest");
        assert_eq!(
            record_name(settings.records_for(sibling_egg, other_nest)),
            "default"
        );
    }

    #[test]
    fn records_for_keeps_an_empty_nest_override() {
        let nest = uuid::Uuid::from_u128(1);
        let settings = ExtensionSettingsData {
            default_records: vec![address("default")],
            nest_records: vec![NestRecords {
                nest_uuid: nest,
                records: Vec::new(),
            }],
            ..Default::default()
        };

        assert!(
            settings
                .records_for(uuid::Uuid::from_u128(2), nest)
                .is_empty()
        );
    }
}
