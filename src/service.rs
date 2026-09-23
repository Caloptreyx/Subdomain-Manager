//! Subdomain operations shared by the server and admin routes.
use crate::{
    db::{Domain, Subdomain},
    providers::{DnsProvider, DnsRecordInput, StoredRecord},
    records,
    settings::ExtensionSettingsData,
};
use axum::http::StatusCode;
use shared::{
    State,
    models::{server::Server, server_allocation::ServerAllocation},
    response::DisplayError,
};
use std::sync::LazyLock;

/// `^[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?$` - a single DNS label, 1-63 chars.
static NAME_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new("^[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?$").expect("invalid subdomain name regex")
});

pub fn user_error(message: impl Into<String>, status: StatusCode) -> anyhow::Error {
    DisplayError::new(message.into()).with_status(status).into()
}

fn bad_request(message: impl Into<String>) -> anyhow::Error {
    user_error(message, StatusCode::BAD_REQUEST)
}

async fn settings(state: &State) -> Result<ExtensionSettingsData, anyhow::Error> {
    Ok(state
        .settings
        .get()
        .await?
        .find_extension_settings::<ExtensionSettingsData>()
        .cloned()
        .unwrap_or_default())
}

/// Creates records sequentially; on failure deletes what was already created
/// (best effort) and propagates the provider error.
async fn create_dns_records(
    provider: &dyn DnsProvider,
    records: &[DnsRecordInput],
) -> Result<Vec<StoredRecord>, anyhow::Error> {
    let mut created = Vec::with_capacity(records.len());

    for record in records {
        match provider.create_record(record).await {
            Ok(stored) => created.push(stored),
            Err(err) => {
                for stored in &created {
                    if let Err(err) = provider.delete_record(&stored.id).await {
                        tracing::warn!(
                            record = %stored.id,
                            "failed to roll back dns record: {err:?}"
                        );
                    }
                }
                return Err(err);
            }
        }
    }

    Ok(created)
}

/// Whether the Reverse Proxy Manager extension is installed (its table
/// exists).
async fn reverse_proxy_installed(state: &State) -> Result<bool, anyhow::Error> {
    let table: Option<String> =
        sqlx::query_scalar("SELECT to_regclass('dev_caloptreyx_reverseproxy_proxies')::text")
            .fetch_one(state.database.read())
            .await?;
    Ok(table.is_some())
}

/// Whether the Reverse Proxy Manager extension already serves `fqdn`.
async fn used_by_reverse_proxy(state: &State, fqdn: &str) -> Result<bool, anyhow::Error> {
    if !reverse_proxy_installed(state).await? {
        return Ok(false);
    }
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM dev_caloptreyx_reverseproxy_proxies WHERE domain = $1)",
    )
    .bind(fqdn)
    .fetch_one(state.database.read())
    .await?)
}

/// Number of Reverse Proxy Manager proxies created on a managed domain.
pub async fn reverse_proxies_on_domain(
    state: &State,
    domain_uuid: uuid::Uuid,
) -> Result<i64, anyhow::Error> {
    if !reverse_proxy_installed(state).await? {
        return Ok(0);
    }
    Ok(sqlx::query_scalar(
        "SELECT COUNT(*) FROM dev_caloptreyx_reverseproxy_proxies WHERE managed_domain_uuid = $1",
    )
    .bind(domain_uuid)
    .fetch_one(state.database.read())
    .await?)
}

async fn server_allocation(
    state: &State,
    server: &Server,
    allocation_uuid: uuid::Uuid,
) -> Result<ServerAllocation, anyhow::Error> {
    ServerAllocation::by_server_uuid_uuid(&state.database, server.uuid, allocation_uuid)
        .await?
        .ok_or_else(|| bad_request("allocation does not belong to this server"))
}

/// Renders the egg's record templates for an allocation.
async fn render_records(
    state: &State,
    server: &Server,
    domain: &Domain,
    name: &str,
    allocation: &ServerAllocation,
) -> Result<Vec<DnsRecordInput>, anyhow::Error> {
    let target = records::resolve_target(&allocation.allocation)?;
    let vars = records::Vars {
        name: name.to_string(),
        domain: domain.domain.clone(),
        ip: target.content(),
        port: allocation.allocation.port,
        server: server.uuid,
        server_name: server.name.to_string(),
    };
    records::render_records(
        settings(state).await?.records_for_egg(server.egg.uuid),
        &vars,
        &target,
    )
    .map_err(|err| bad_request(err.to_string()))
}

async fn create_records_at(
    provider: &dyn DnsProvider,
    record_inputs: &[DnsRecordInput],
) -> Result<Vec<StoredRecord>, anyhow::Error> {
    create_dns_records(provider, record_inputs)
        .await
        .map_err(|err| bad_request(format!("failed to create dns records: {err}")))
}

pub struct CreateOptions {
    /// Users are bound by the server's limit; admins are not.
    pub enforce_limit: bool,
    /// Users can't pick blacklisted names; admins can.
    pub enforce_blacklist: bool,
}

pub async fn create(
    state: &State,
    server: &Server,
    domain_uuid: uuid::Uuid,
    name: &str,
    allocation_uuid: uuid::Uuid,
    options: CreateOptions,
) -> Result<Subdomain, anyhow::Error> {
    let name = name.trim().to_lowercase();
    if !NAME_REGEX.is_match(&name) {
        return Err(bad_request(
            "subdomain name must be 1-63 characters of lowercase letters, numbers and dashes, and may not start or end with a dash",
        ));
    }

    if options.enforce_blacklist
        && settings(state)
            .await?
            .compiled_blacklist()
            .iter()
            .any(|pattern| pattern.is_match(&name))
    {
        return Err(bad_request("this subdomain name is not allowed"));
    }

    if options.enforce_limit {
        let limit = crate::model::subdomain_limit(server)?;
        if Subdomain::count_by_server_uuid(&state.database, server.uuid).await? >= limit as i64 {
            return Err(bad_request("subdomain limit reached for this server"));
        }
    }

    let domain = Domain::by_uuid(&state.database, domain_uuid)
        .await?
        .filter(|domain| domain.enabled)
        .ok_or_else(|| bad_request("domain not found"))?;
    let allocation = server_allocation(state, server, allocation_uuid).await?;

    if Subdomain::by_domain_and_name(&state.database, domain.uuid, &name)
        .await?
        .is_some()
    {
        return Err(user_error("this subdomain is already taken", StatusCode::CONFLICT));
    }
    if used_by_reverse_proxy(state, &format!("{name}.{}", domain.domain)).await? {
        return Err(user_error(
            "this name is already used by a reverse proxy",
            StatusCode::CONFLICT,
        ));
    }

    let record_inputs = render_records(state, server, &domain, &name, &allocation).await?;
    let provider = domain.provider_client(&state.database).await?;
    let stored_records = create_records_at(&*provider, &record_inputs).await?;

    Ok(Subdomain::insert(
        &state.database,
        server.uuid,
        domain.uuid,
        Some(allocation.uuid),
        &name,
        &stored_records,
    )
    .await?)
}

/// Points a subdomain at another allocation of its server, replacing its
/// DNS records.
pub async fn change_allocation(
    state: &State,
    server: &Server,
    subdomain: &mut Subdomain,
    allocation_uuid: uuid::Uuid,
) -> Result<(), anyhow::Error> {
    let allocation = server_allocation(state, server, allocation_uuid).await?;
    let domain = Domain::by_uuid(&state.database, subdomain.domain_uuid)
        .await?
        .ok_or_else(|| bad_request("domain no longer exists"))?;

    // render first so a broken template doesn't leave the subdomain without
    // records
    let record_inputs = render_records(state, server, &domain, &subdomain.name, &allocation).await?;

    let provider = domain.provider_client(&state.database).await?;
    for record in subdomain.records.iter() {
        if let Err(err) = provider.delete_record(&record.id).await {
            tracing::warn!(
                subdomain = %subdomain.name,
                record = %record.id,
                "failed to delete old dns record: {err:?}"
            );
        }
    }

    let stored_records = create_records_at(&*provider, &record_inputs).await?;

    subdomain
        .update_allocation_and_records(&state.database, Some(allocation.uuid), &stored_records)
        .await?;
    Ok(())
}

/// Deletes the DNS records and the subdomain. Without `force`, a failing
/// provider aborts with a 502 so the caller can offer a forced delete.
pub async fn delete(state: &State, subdomain: &Subdomain, force: bool) -> Result<(), anyhow::Error> {
    if let Some(domain) = Domain::by_uuid(&state.database, subdomain.domain_uuid).await? {
        match domain.provider_client(&state.database).await {
            Ok(provider) => {
                for record in subdomain.records.iter() {
                    if let Err(err) = provider.delete_record(&record.id).await {
                        if !force {
                            return Err(user_error(
                                format!("failed to delete dns record: {err}"),
                                StatusCode::BAD_GATEWAY,
                            ));
                        }
                        tracing::warn!(
                            subdomain = %subdomain.name,
                            record = %record.id,
                            "force-deleting subdomain despite dns record failure: {err:?}"
                        );
                    }
                }
            }
            Err(err) if !force => return Err(err),
            Err(err) => tracing::warn!(
                subdomain = %subdomain.name,
                "force-deleting subdomain despite provider failure: {err:?}"
            ),
        }
    }

    subdomain.delete(&state.database).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::NAME_REGEX;

    #[test]
    fn subdomain_name_regex() {
        for valid in ["a", "mc", "my-server", "0", "a-b-c", &"x".repeat(63), "s1a2b3"] {
            assert!(NAME_REGEX.is_match(valid), "{valid} should be valid");
        }

        for invalid in ["", "-a", "a-", "-", &"x".repeat(64), "a_b", "A", "a b", "a.b", "ä"] {
            assert!(!NAME_REGEX.is_match(invalid), "{invalid} should be invalid");
        }
    }
}
