use crate::providers::{DnsProvider, DnsRecordType, StoredRecord};
use chrono::{DateTime, Utc};
use serde::Serialize;
use shared::database::Database;
use sqlx::types::Json;
use utoipa::ToSchema;
use uuid::Uuid;

/// A configured DNS zone (e.g. a Cloudflare zone or Bunny DNS zone) that
/// subdomains can be created under.
#[derive(sqlx::FromRow, Clone)]
pub struct Domain {
    pub uuid: Uuid,
    pub domain: String,
    pub provider: String,
    pub zone_id: String,
    /// Encrypted provider credential - never exposed via the API.
    pub credential: String,
    pub enabled: bool,
    pub created: DateTime<Utc>,
}

impl Domain {
    /// Decrypts the stored credential and builds a provider client.
    pub async fn provider_client(
        &self,
        database: &Database,
    ) -> Result<Box<dyn DnsProvider>, anyhow::Error> {
        let credential = database.decrypt_base64(&self.credential).await?;
        crate::providers::build(&self.provider, &self.zone_id, &credential)
    }

    pub async fn by_uuid(database: &Database, uuid: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM dev_caloptreyx_subdomains_domains WHERE uuid = $1")
            .bind(uuid)
            .fetch_optional(database.read())
            .await
    }

    pub async fn by_domain(database: &Database, domain: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM dev_caloptreyx_subdomains_domains WHERE domain = $1",
        )
        .bind(domain)
        .fetch_optional(database.read())
        .await
    }

    pub async fn all(database: &Database, only_enabled: bool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM dev_caloptreyx_subdomains_domains
             WHERE ($1::boolean IS NULL OR enabled = $1) ORDER BY created",
        )
        .bind(only_enabled.then_some(true))
        .fetch_all(database.read())
        .await
    }

    /// Encrypts `credential` and inserts the domain.
    pub async fn insert(
        database: &Database,
        domain: &str,
        provider: &str,
        zone_id: &str,
        credential: &str,
    ) -> Result<Self, anyhow::Error> {
        let credential = database.encrypt_base64(credential.to_string()).await?;

        Ok(sqlx::query_as::<_, Self>(
            "INSERT INTO dev_caloptreyx_subdomains_domains (domain, provider, zone_id, credential)
             VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(domain)
        .bind(provider)
        .bind(zone_id)
        .bind(credential.as_str())
        .fetch_one(database.write())
        .await?)
    }

    /// Updates provided fields; `credential` is plaintext and stored encrypted.
    pub async fn update(
        &mut self,
        database: &Database,
        domain: Option<&str>,
        provider: Option<&str>,
        zone_id: Option<&str>,
        credential: Option<&str>,
        enabled: Option<bool>,
    ) -> Result<(), anyhow::Error> {
        if let Some(domain) = domain {
            self.domain = domain.to_string();
        }
        if let Some(provider) = provider {
            self.provider = provider.to_string();
        }
        if let Some(zone_id) = zone_id {
            self.zone_id = zone_id.to_string();
        }
        if let Some(credential) = credential {
            self.credential = database
                .encrypt_base64(credential.to_string())
                .await?
                .to_string();
        }
        if let Some(enabled) = enabled {
            self.enabled = enabled;
        }

        sqlx::query(
            "UPDATE dev_caloptreyx_subdomains_domains
             SET domain = $2, provider = $3, zone_id = $4, credential = $5, enabled = $6
             WHERE uuid = $1",
        )
        .bind(self.uuid)
        .bind(&self.domain)
        .bind(&self.provider)
        .bind(&self.zone_id)
        .bind(&self.credential)
        .bind(self.enabled)
        .execute(database.write())
        .await?;

        Ok(())
    }

    pub async fn delete(&self, database: &Database) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM dev_caloptreyx_subdomains_domains WHERE uuid = $1")
            .bind(self.uuid)
            .execute(database.write())
            .await?;
        Ok(())
    }
}

/// A created subdomain row.
#[derive(sqlx::FromRow, Clone)]
pub struct Subdomain {
    pub uuid: Uuid,
    pub server_uuid: Uuid,
    pub domain_uuid: Uuid,
    /// NULL when the allocation was deleted or the server was transferred.
    pub allocation_uuid: Option<Uuid>,
    pub name: String,
    pub records: Json<Vec<StoredRecord>>,
    pub created: DateTime<Utc>,
}

/// Flat row for joined subdomain queries (domain, server and allocation data
/// in one SELECT).
#[derive(sqlx::FromRow)]
pub struct JoinedSubdomain {
    pub uuid: Uuid,
    pub server_uuid: Uuid,
    pub domain_uuid: Uuid,
    pub allocation_uuid: Option<Uuid>,
    pub name: String,
    pub records: Json<Vec<StoredRecord>>,
    pub created: DateTime<Utc>,

    pub domain_name: String,
    pub server_name: String,
    pub alloc_ip: Option<sqlx::types::ipnetwork::IpNetwork>,
    pub alloc_ip_alias: Option<String>,
    pub alloc_port: Option<i32>,
}

const SUBDOMAIN_JOIN_COLUMNS: &str = "
        s.uuid, s.server_uuid, s.domain_uuid, s.allocation_uuid, s.name, s.records, s.created,
        d.domain AS domain_name, sv.name AS server_name,
        na.ip AS alloc_ip, na.ip_alias AS alloc_ip_alias, na.port AS alloc_port";

const SUBDOMAIN_JOIN_FROM: &str = "
    FROM dev_caloptreyx_subdomains_subdomains s
    JOIN dev_caloptreyx_subdomains_domains d ON d.uuid = s.domain_uuid
    JOIN servers sv ON sv.uuid = s.server_uuid
    LEFT JOIN server_allocations sa ON sa.uuid = s.allocation_uuid
    LEFT JOIN node_allocations na ON na.uuid = sa.allocation_uuid";

impl Subdomain {
    pub async fn by_uuid(database: &Database, uuid: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM dev_caloptreyx_subdomains_subdomains WHERE uuid = $1",
        )
        .bind(uuid)
        .fetch_optional(database.read())
        .await
    }

    /// Joined lookup used by routes so domain/allocation data is available
    /// without extra queries.
    pub async fn by_uuid_joined(
        database: &Database,
        uuid: Uuid,
    ) -> Result<Option<JoinedSubdomain>, sqlx::Error> {
        sqlx::query_as::<_, JoinedSubdomain>(sqlx::AssertSqlSafe(format!(
            "SELECT {SUBDOMAIN_JOIN_COLUMNS} {SUBDOMAIN_JOIN_FROM} WHERE s.uuid = $1"
        )))
        .bind(uuid)
        .fetch_optional(database.read())
        .await
    }

    /// Joined listing for one server.
    pub async fn all_by_server_uuid(
        database: &Database,
        server_uuid: Uuid,
    ) -> Result<Vec<JoinedSubdomain>, sqlx::Error> {
        sqlx::query_as::<_, JoinedSubdomain>(sqlx::AssertSqlSafe(format!(
            "SELECT {SUBDOMAIN_JOIN_COLUMNS} {SUBDOMAIN_JOIN_FROM}
             WHERE s.server_uuid = $1 ORDER BY s.created"
        )))
        .bind(server_uuid)
        .fetch_all(database.read())
        .await
    }

    /// Paginated joined listing across all servers, searched by subdomain
    /// name, domain or server name.
    pub async fn all_with_pagination(
        database: &Database,
        page: i64,
        per_page: i64,
        search: Option<&str>,
    ) -> Result<shared::models::Pagination<ApiAdminSubdomain>, sqlx::Error> {
        struct Row {
            total: i64,
            inner: JoinedSubdomain,
        }

        impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for Row {
            fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
                use sqlx::Row;
                Ok(Self {
                    total: row.try_get("total")?,
                    inner: JoinedSubdomain::from_row(row)?,
                })
            }
        }

        let rows = sqlx::query_as::<_, Row>(sqlx::AssertSqlSafe(format!(
            "SELECT {SUBDOMAIN_JOIN_COLUMNS}, COUNT(*) OVER() AS total {SUBDOMAIN_JOIN_FROM}
             WHERE ($1::text IS NULL
                OR s.name ILIKE '%' || $1 || '%'
                OR d.domain ILIKE '%' || $1 || '%'
                OR sv.name ILIKE '%' || $1 || '%')
             ORDER BY s.created LIMIT $2 OFFSET $3"
        )))
        .bind(search)
        .bind(per_page)
        .bind(per_page * (page - 1))
        .fetch_all(database.read())
        .await?;

        Ok(shared::models::Pagination {
            total: rows.first().map(|row| row.total).unwrap_or(0),
            per_page,
            page,
            data: rows.into_iter().map(|row| row.inner.into_api()).collect(),
        })
    }

    pub async fn count_by_server_uuid(
        database: &Database,
        server_uuid: Uuid,
    ) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM dev_caloptreyx_subdomains_subdomains WHERE server_uuid = $1",
        )
        .bind(server_uuid)
        .fetch_one(database.read())
        .await
    }

    /// Subdomain count per domain, for the admin domain listing.
    pub async fn counts_by_domain(
        database: &Database,
    ) -> Result<std::collections::HashMap<Uuid, i64>, sqlx::Error> {
        let rows: Vec<(Uuid, i64)> = sqlx::query_as(
            "SELECT domain_uuid, COUNT(*) FROM dev_caloptreyx_subdomains_subdomains
             GROUP BY domain_uuid",
        )
        .fetch_all(database.read())
        .await?;
        Ok(rows.into_iter().collect())
    }

    pub async fn count_by_domain_uuid(
        database: &Database,
        domain_uuid: Uuid,
    ) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM dev_caloptreyx_subdomains_subdomains WHERE domain_uuid = $1",
        )
        .bind(domain_uuid)
        .fetch_one(database.read())
        .await
    }

    pub async fn all_by_domain_uuid(
        database: &Database,
        domain_uuid: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM dev_caloptreyx_subdomains_subdomains WHERE domain_uuid = $1",
        )
        .bind(domain_uuid)
        .fetch_all(database.read())
        .await
    }

    /// Plain rows for a server (used by the delete hook where join data is
    /// not needed).
    pub async fn all_by_server_uuid_plain(
        database: &Database,
        server_uuid: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM dev_caloptreyx_subdomains_subdomains WHERE server_uuid = $1",
        )
        .bind(server_uuid)
        .fetch_all(database.read())
        .await
    }

    pub async fn by_domain_and_name(
        database: &Database,
        domain_uuid: Uuid,
        name: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM dev_caloptreyx_subdomains_subdomains
             WHERE domain_uuid = $1 AND name = $2",
        )
        .bind(domain_uuid)
        .bind(name)
        .fetch_optional(database.read())
        .await
    }

    pub async fn insert(
        database: &Database,
        server_uuid: Uuid,
        domain_uuid: Uuid,
        allocation_uuid: Option<Uuid>,
        name: &str,
        records: &[StoredRecord],
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "INSERT INTO dev_caloptreyx_subdomains_subdomains
                 (server_uuid, domain_uuid, allocation_uuid, name, records)
             VALUES ($1, $2, $3, $4, $5) RETURNING *",
        )
        .bind(server_uuid)
        .bind(domain_uuid)
        .bind(allocation_uuid)
        .bind(name)
        .bind(Json(records))
        .fetch_one(database.write())
        .await
    }

    pub async fn update_allocation_and_records(
        &mut self,
        database: &Database,
        allocation_uuid: Option<Uuid>,
        records: &[StoredRecord],
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE dev_caloptreyx_subdomains_subdomains
             SET allocation_uuid = $2, records = $3 WHERE uuid = $1",
        )
        .bind(self.uuid)
        .bind(allocation_uuid)
        .bind(Json(records))
        .execute(database.write())
        .await?;

        self.allocation_uuid = allocation_uuid;
        self.records = Json(records.to_vec());
        Ok(())
    }

    pub async fn delete_by_domain_uuid(
        database: &Database,
        domain_uuid: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM dev_caloptreyx_subdomains_subdomains WHERE domain_uuid = $1")
            .bind(domain_uuid)
            .execute(database.write())
            .await?;
        Ok(())
    }

    pub async fn delete(&self, database: &Database) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM dev_caloptreyx_subdomains_subdomains WHERE uuid = $1")
            .bind(self.uuid)
            .execute(database.write())
            .await?;
        Ok(())
    }
}

#[derive(ToSchema, Serialize)]
pub struct ApiDomainRef {
    pub uuid: Uuid,
    pub domain: String,
}

/// Admin domain listing shape - credential is never exposed.
#[derive(ToSchema, Serialize)]
pub struct ApiDomain {
    pub uuid: Uuid,
    pub domain: String,
    pub provider: String,
    pub zone_id: String,
    pub enabled: bool,
    pub subdomain_count: i64,
    pub created: DateTime<Utc>,
}

impl Domain {
    pub fn into_api(self, subdomain_count: i64) -> ApiDomain {
        ApiDomain {
            uuid: self.uuid,
            domain: self.domain,
            provider: self.provider,
            zone_id: self.zone_id,
            enabled: self.enabled,
            subdomain_count,
            created: self.created,
        }
    }
}

#[derive(ToSchema, Serialize)]
pub struct ApiAllocationRef {
    pub uuid: Uuid,
    pub ip: String,
    pub ip_alias: Option<String>,
    pub port: i32,
}

#[derive(ToSchema, Serialize)]
pub struct ApiStoredRecord {
    pub record_type: DnsRecordType,
    pub name: String,
    pub content: String,
}

/// Public shape of a subdomain (client + embedded in admin responses).
#[derive(ToSchema, Serialize)]
pub struct ApiSubdomain {
    pub uuid: Uuid,
    pub name: String,
    pub fqdn: String,
    pub domain: ApiDomainRef,
    pub allocation: Option<ApiAllocationRef>,
    pub records: Vec<ApiStoredRecord>,
    pub created: DateTime<Utc>,
}

#[derive(ToSchema, Serialize)]
pub struct ApiServerRef {
    pub uuid: Uuid,
    pub name: String,
}

/// Admin listing shape: subdomain plus the owning server.
#[derive(ToSchema, Serialize)]
pub struct ApiAdminSubdomain {
    #[serde(flatten)]
    #[schema(inline)]
    pub subdomain: ApiSubdomain,
    pub server: ApiServerRef,
}

impl JoinedSubdomain {
    pub fn fqdn(&self) -> String {
        format!("{}.{}", self.name, self.domain_name)
    }

    fn allocation_ref(&self) -> Option<ApiAllocationRef> {
        match (self.allocation_uuid, self.alloc_ip) {
            (Some(uuid), Some(ip)) => Some(ApiAllocationRef {
                uuid,
                ip: ip.ip().to_string(),
                ip_alias: self.alloc_ip_alias.clone(),
                port: self.alloc_port.unwrap_or_default(),
            }),
            _ => None,
        }
    }

    fn api_records(&self) -> Vec<ApiStoredRecord> {
        self.records
            .iter()
            .map(|record| ApiStoredRecord {
                record_type: record.record_type,
                name: record.name.clone(),
                content: record.content.clone(),
            })
            .collect()
    }

    pub fn into_api(self) -> ApiAdminSubdomain {
        ApiAdminSubdomain {
            subdomain: ApiSubdomain {
                uuid: self.uuid,
                name: self.name.clone(),
                fqdn: self.fqdn(),
                domain: ApiDomainRef {
                    uuid: self.domain_uuid,
                    domain: self.domain_name.clone(),
                },
                allocation: self.allocation_ref(),
                records: self.api_records(),
                created: self.created,
            },
            server: ApiServerRef {
                uuid: self.server_uuid,
                name: self.server_name,
            },
        }
    }

    pub fn into_subdomain_api(self) -> ApiSubdomain {
        ApiSubdomain {
            uuid: self.uuid,
            name: self.name.clone(),
            fqdn: self.fqdn(),
            domain: ApiDomainRef {
                uuid: self.domain_uuid,
                domain: self.domain_name.clone(),
            },
            allocation: self.allocation_ref(),
            records: self.api_records(),
            created: self.created,
        }
    }
}
