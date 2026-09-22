use super::State;
use axum::{
    extract::{Path, Request},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use shared::{GetState, models::user::GetPermissionManager, response::ApiResponse};
use utoipa_axum::{router::OpenApiRouter, routes};

mod verify;

pub type GetDomain = shared::extract::ConsumingExtension<crate::db::Domain>;

pub async fn auth(
    state: GetState,
    permissions: GetPermissionManager,
    Path(domain): Path<Vec<String>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let domain = match domain.first().map(|s| s.parse::<uuid::Uuid>()) {
        Some(Ok(uuid)) => uuid,
        _ => {
            return Ok(ApiResponse::error("invalid domain uuid")
                .with_status(StatusCode::BAD_REQUEST)
                .into_response());
        }
    };

    if let Err(err) = permissions.has_admin_permission("subdomains.read") {
        return Ok(err.into_response());
    }

    let domain = match crate::db::Domain::by_uuid(&state.database, domain).await {
        Ok(Some(domain)) => domain,
        Ok(None) => {
            return Ok(ApiResponse::error("domain not found")
                .with_status(StatusCode::NOT_FOUND)
                .into_response());
        }
        Err(err) => return Ok(ApiResponse::from(err).into_response()),
    };

    req.extensions_mut().insert(domain);

    Ok(next.run(req).await)
}

mod patch {
    use crate::db::{ApiDomain, Domain, Subdomain};
    use axum::http::StatusCode;
    use garde::Validate;
    use serde::{Deserialize, Serialize};
    use shared::{
        ApiError, GetState,
        models::{admin_activity::GetAdminActivityLogger, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    use super::GetDomain;

    #[derive(ToSchema, Validate, Deserialize)]
    pub struct Payload {
        #[garde(length(chars, min = 1, max = 255))]
        #[schema(min_length = 1, max_length = 255)]
        #[serde(default)]
        domain: Option<String>,
        #[garde(length(chars, min = 1, max = 31))]
        #[schema(min_length = 1, max_length = 31)]
        #[serde(default)]
        provider: Option<String>,
        #[garde(length(chars, min = 1, max = 255))]
        #[schema(min_length = 1, max_length = 255)]
        #[serde(default)]
        zone_id: Option<String>,
        #[garde(length(chars, min = 1, max = 1024))]
        #[schema(min_length = 1, max_length = 1024)]
        #[serde(default)]
        credential: Option<String>,
        #[garde(skip)]
        #[serde(default)]
        enabled: Option<bool>,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        domain: ApiDomain,
    }

    #[utoipa::path(patch, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
        (status = NOT_FOUND, body = ApiError),
    ), params(
        (
            "domain" = uuid::Uuid,
            description = "The domain ID",
            example = "123e4567-e89b-12d3-a456-426614174000",
        ),
    ), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        activity_logger: GetAdminActivityLogger,
        mut domain: GetDomain,
        shared::Payload(data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        if let Err(errors) = shared::utils::validate_data(&data) {
            return ApiResponse::new_serialized(ApiError::new_strings_value(errors))
                .with_status(StatusCode::BAD_REQUEST)
                .ok();
        }

        permissions.has_admin_permission("subdomains.manage")?;

        let new_domain = data
            .domain
            .as_deref()
            .map(|domain| domain.trim().trim_end_matches('.').to_lowercase());
        let provider = data
            .provider
            .as_deref()
            .map(|provider| provider.trim().to_lowercase())
            .unwrap_or_else(|| domain.provider.clone());
        let zone_id = data
            .zone_id
            .as_deref()
            .map(|zone_id| zone_id.trim().to_string())
            .unwrap_or_else(|| domain.zone_id.clone());

        // Re-verify the zone whenever provider credentials change.
        if data.provider.is_some() || data.zone_id.is_some() || data.credential.is_some() {
            let credential = match &data.credential {
                Some(credential) => credential.trim().to_string(),
                None => match state.database.decrypt_base64(&domain.credential).await {
                    Ok(credential) => credential.to_string(),
                    Err(err) => return ApiResponse::from(err).ok(),
                },
            };

            let client = match crate::providers::build(&provider, &zone_id, &credential) {
                Ok(client) => client,
                Err(err) => {
                    return ApiResponse::error(err.to_string())
                        .with_status(StatusCode::BAD_REQUEST)
                        .ok();
                }
            };
            if let Err(err) = client.verify().await {
                return ApiResponse::error(format!("failed to verify zone: {err}"))
                    .with_status(StatusCode::BAD_REQUEST)
                    .ok();
            }
        }

        if let Some(domain_name) = &new_domain
            && *domain_name != domain.domain
            && Domain::by_domain(&state.database, domain_name)
                .await?
                .is_some()
        {
            return ApiResponse::error("a domain with this name already exists")
                .with_status(StatusCode::CONFLICT)
                .ok();
        }

        let credential_provided = data.credential.is_some();
        domain
            .update(
                &state.database,
                new_domain.as_deref(),
                data.provider.is_some().then_some(provider.as_str()),
                data.zone_id.is_some().then_some(zone_id.as_str()),
                data.credential.as_deref().map(str::trim),
                data.enabled,
            )
            .await?;

        let subdomain_count = Subdomain::count_by_domain_uuid(&state.database, domain.uuid).await?;

        activity_logger
            .log(
                "subdomains:domain.update",
                serde_json::json!({
                    "uuid": domain.uuid,
                    "domain": domain.domain,
                    "provider": domain.provider,
                    "zone_id": domain.zone_id,
                    "enabled": domain.enabled,
                    "credential_changed": credential_provided,
                }),
            )
            .await;

        ApiResponse::new_serialized(Response {
            domain: domain.0.into_api(subdomain_count),
        })
        .ok()
    }
}

mod delete {
    use crate::db::Subdomain;
    use axum::{extract::Query, http::StatusCode};
    use serde::{Deserialize, Serialize};
    use shared::{
        ApiError, GetState,
        models::{admin_activity::GetAdminActivityLogger, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    use super::GetDomain;

    #[derive(ToSchema, Deserialize)]
    pub struct Params {
        /// Also delete all subdomains under this domain.
        #[serde(default)]
        force: bool,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {}

    #[utoipa::path(delete, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = UNAUTHORIZED, body = ApiError),
        (status = NOT_FOUND, body = ApiError),
        (status = CONFLICT, body = ApiError),
    ), params(
        (
            "domain" = uuid::Uuid,
            description = "The domain ID",
            example = "123e4567-e89b-12d3-a456-426614174000",
        ),
        (
            "force" = Option<bool>, Query,
            description = "Delete all subdomains under this domain",
        ),
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        activity_logger: GetAdminActivityLogger,
        domain: GetDomain,
        Query(params): Query<Params>,
    ) -> ApiResponseResult {
        permissions.has_admin_permission("subdomains.manage")?;

        let subdomain_count = Subdomain::count_by_domain_uuid(&state.database, domain.uuid).await?;
        if subdomain_count > 0 && !params.force {
            return ApiResponse::error(format!(
                "domain still has {subdomain_count} subdomain(s), pass force to delete them"
            ))
            .with_status(StatusCode::CONFLICT)
            .ok();
        }

        if subdomain_count > 0 {
            match domain.provider_client(&state.database).await {
                Ok(provider) => {
                    for subdomain in
                        Subdomain::all_by_domain_uuid(&state.database, domain.uuid).await?
                    {
                        for record in subdomain.records.iter() {
                            if let Err(err) = provider.delete_record(&record.id).await {
                                tracing::warn!(
                                    domain = %domain.domain,
                                    subdomain = %subdomain.name,
                                    record = %record.id,
                                    "failed to delete dns record during domain deletion: {err:?}"
                                );
                            }
                        }
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        domain = %domain.domain,
                        "failed to build dns provider for domain deletion: {err:?}"
                    );
                }
            }

            Subdomain::delete_by_domain_uuid(&state.database, domain.uuid).await?;
        }

        domain.delete(&state.database).await?;

        activity_logger
            .log(
                "subdomains:domain.delete",
                serde_json::json!({
                    "uuid": domain.uuid,
                    "domain": domain.domain,
                    "provider": domain.provider,
                    "zone_id": domain.zone_id,
                    "subdomains_deleted": subdomain_count,
                }),
            )
            .await;

        ApiResponse::new_serialized(Response {}).ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(patch::route))
        .routes(routes!(delete::route))
        .nest("/verify", verify::router(state))
        .route_layer(axum::middleware::from_fn_with_state(state.clone(), auth))
        .with_state(state.clone())
}
