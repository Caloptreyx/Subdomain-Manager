use super::State;
use crate::providers::{DnsProvider, DnsRecordInput, StoredRecord};
use std::sync::LazyLock;
use utoipa_axum::{router::OpenApiRouter, routes};

mod _subdomain_;

/// `^[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?$` - a single DNS label, 1-63 chars.
pub(crate) static NAME_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new("^[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?$").expect("invalid subdomain name regex")
});

/// Creates records sequentially; on failure deletes what was already created
/// (best effort) and propagates the provider error.
pub(crate) async fn create_dns_records(
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

mod get {
    use crate::{
        db::{ApiDomainRef, ApiSubdomain, Domain, Subdomain},
        model,
    };
    use serde::Serialize;
    use shared::{
        ApiError, GetState,
        models::{server::GetServer, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {
        subdomains: Vec<ApiSubdomain>,
        limit: i32,
        domains: Vec<ApiDomainRef>,
    }

    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = UNAUTHORIZED, body = ApiError),
    ), params(
        (
            "server" = uuid::Uuid,
            description = "The server ID",
            example = "123e4567-e89b-12d3-a456-426614174000",
        ),
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        server: GetServer,
    ) -> ApiResponseResult {
        permissions.has_server_permission("subdomains.read")?;

        let subdomains = Subdomain::all_by_server_uuid(&state.database, server.uuid).await?;
        let domains = Domain::all(&state.database, true).await?;
        let limit = model::subdomain_limit(&server)?;

        ApiResponse::new_serialized(Response {
            subdomains: subdomains
                .into_iter()
                .map(|subdomain| subdomain.into_subdomain_api())
                .collect(),
            limit,
            domains: domains
                .into_iter()
                .map(|domain| ApiDomainRef {
                    uuid: domain.uuid,
                    domain: domain.domain,
                })
                .collect(),
        })
        .ok()
    }
}

mod post {
    use crate::{
        db::{ApiSubdomain, Domain, Subdomain},
        model, records,
        routes::server::{NAME_REGEX, create_dns_records},
        settings::ExtensionSettingsData,
    };
    use axum::http::StatusCode;
    use garde::Validate;
    use serde::{Deserialize, Serialize};
    use shared::{
        ApiError, GetState,
        models::{
            server::{GetServer, GetServerActivityLogger},
            server_allocation::ServerAllocation,
            user::GetPermissionManager,
        },
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Validate, Deserialize)]
    pub struct Payload {
        #[garde(skip)]
        domain_uuid: uuid::Uuid,
        #[garde(length(chars, min = 1, max = 63))]
        #[schema(min_length = 1, max_length = 63)]
        name: String,
        #[garde(skip)]
        allocation_uuid: uuid::Uuid,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        subdomain: ApiSubdomain,
    }

    #[utoipa::path(post, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
        (status = CONFLICT, body = ApiError),
    ), params(
        (
            "server" = uuid::Uuid,
            description = "The server ID",
            example = "123e4567-e89b-12d3-a456-426614174000",
        ),
    ), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        server: GetServer,
        activity_logger: GetServerActivityLogger,
        shared::Payload(data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        if let Err(errors) = shared::utils::validate_data(&data) {
            return ApiResponse::new_serialized(ApiError::new_strings_value(errors))
                .with_status(StatusCode::BAD_REQUEST)
                .ok();
        }

        permissions.has_server_permission("subdomains.create")?;

        let name = data.name.trim().to_lowercase();
        if !NAME_REGEX.is_match(&name) {
            return ApiResponse::error(
                "subdomain name must be 1-63 characters of lowercase letters, numbers and dashes, and may not start or end with a dash",
            )
            .with_status(StatusCode::BAD_REQUEST)
            .ok();
        }

        let settings = state.settings.get().await?;
        let extension = settings
            .find_extension_settings::<ExtensionSettingsData>()
            .cloned()
            .unwrap_or_default();

        if extension
            .compiled_blacklist()
            .iter()
            .any(|pattern| pattern.is_match(&name))
        {
            return ApiResponse::error("this subdomain name is not allowed")
                .with_status(StatusCode::BAD_REQUEST)
                .ok();
        }

        let limit = model::subdomain_limit(&server)?;
        let count = Subdomain::count_by_server_uuid(&state.database, server.uuid).await?;
        if count >= limit as i64 {
            return ApiResponse::error("subdomain limit reached for this server")
                .with_status(StatusCode::BAD_REQUEST)
                .ok();
        }

        let domain = match Domain::by_uuid(&state.database, data.domain_uuid).await? {
            Some(domain) if domain.enabled => domain,
            _ => {
                return ApiResponse::error("domain not found")
                    .with_status(StatusCode::BAD_REQUEST)
                    .ok();
            }
        };

        let allocation = match ServerAllocation::by_server_uuid_uuid(
            &state.database,
            server.uuid,
            data.allocation_uuid,
        )
        .await?
        {
            Some(allocation) => allocation,
            None => {
                return ApiResponse::error("allocation does not belong to this server")
                    .with_status(StatusCode::BAD_REQUEST)
                    .ok();
            }
        };

        if Subdomain::by_domain_and_name(&state.database, domain.uuid, &name)
            .await?
            .is_some()
        {
            return ApiResponse::error("this subdomain is already taken")
                .with_status(StatusCode::CONFLICT)
                .ok();
        }

        let target = match records::resolve_target(&allocation.allocation) {
            Ok(target) => target,
            Err(err) => return ApiResponse::from(err).ok(),
        };

        let vars = records::Vars {
            name: name.clone(),
            domain: domain.domain.clone(),
            ip: target.content(),
            port: allocation.allocation.port,
            server: server.uuid,
            server_name: server.name.to_string(),
        };
        let record_inputs = match records::render_records(
            extension.records_for_egg(server.egg.uuid),
            &vars,
            &target,
        ) {
            Ok(records) => records,
            Err(err) => {
                return ApiResponse::error(err.to_string())
                    .with_status(StatusCode::BAD_REQUEST)
                    .ok();
            }
        };

        let provider = match domain.provider_client(&state.database).await {
            Ok(provider) => provider,
            Err(err) => return ApiResponse::from(err).ok(),
        };

        let stored_records = match create_dns_records(&*provider, &record_inputs).await {
            Ok(stored) => stored,
            Err(err) => {
                return ApiResponse::error(format!("failed to create dns records: {err}"))
                    .with_status(StatusCode::BAD_REQUEST)
                    .ok();
            }
        };

        let subdomain = match Subdomain::insert(
            &state.database,
            server.uuid,
            domain.uuid,
            Some(allocation.uuid),
            &name,
            &stored_records,
        )
        .await
        {
            Ok(subdomain) => subdomain,
            Err(err) => return ApiResponse::from(err).ok(),
        };

        let api_subdomain = Subdomain::by_uuid_joined(&state.database, subdomain.uuid)
            .await?
            .ok_or_else(|| anyhow::anyhow!("subdomain not found after creation"))?
            .into_subdomain_api();

        activity_logger
            .log(
                "server:subdomains.create",
                serde_json::json!({
                    "uuid": subdomain.uuid,
                    "name": name,
                    "fqdn": api_subdomain.fqdn,
                    "allocation_uuid": allocation.uuid,
                }),
            )
            .await;

        ApiResponse::new_serialized(Response {
            subdomain: api_subdomain,
        })
        .ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .routes(routes!(post::route))
        .nest("/{subdomain}", _subdomain_::router(state))
        .with_state(state.clone())
}

#[cfg(test)]
mod tests {
    use super::NAME_REGEX;

    #[test]
    fn subdomain_name_regex() {
        for valid in [
            "a",
            "mc",
            "my-server",
            "0",
            "a-b-c",
            &"x".repeat(63),
            "s1a2b3",
        ] {
            assert!(NAME_REGEX.is_match(valid), "{valid} should be valid");
        }

        for invalid in [
            "",
            "-a",
            "a-",
            "-",
            &"x".repeat(64),
            "a_b",
            "A",
            "a b",
            "a.b",
            "ä",
        ] {
            assert!(!NAME_REGEX.is_match(invalid), "{invalid} should be invalid");
        }
    }
}
