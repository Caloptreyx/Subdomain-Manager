use super::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod _domain_;

mod get {
    use crate::db::{ApiDomain, Domain, Subdomain};
    use serde::Serialize;
    use shared::{
        ApiError, GetState,
        models::user::GetPermissionManager,
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {
        domains: Vec<ApiDomain>,
    }

    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = UNAUTHORIZED, body = ApiError),
    ))]
    pub async fn route(state: GetState, permissions: GetPermissionManager) -> ApiResponseResult {
        permissions.has_admin_permission("subdomains.read")?;

        let domains = Domain::all(&state.database, false).await?;
        let counts = Subdomain::counts_by_domain(&state.database).await?;

        ApiResponse::new_serialized(Response {
            domains: domains
                .into_iter()
                .map(|domain| {
                    let count = counts.get(&domain.uuid).copied().unwrap_or(0);
                    domain.into_api(count)
                })
                .collect(),
        })
        .ok()
    }
}

mod post {
    use crate::db::{ApiDomain, Domain};
    use axum::http::StatusCode;
    use garde::Validate;
    use serde::{Deserialize, Serialize};
    use shared::{
        ApiError, GetState,
        models::{admin_activity::GetAdminActivityLogger, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Validate, Deserialize)]
    pub struct Payload {
        #[garde(length(chars, min = 1, max = 255))]
        #[schema(min_length = 1, max_length = 255)]
        domain: String,
        #[garde(length(chars, min = 1, max = 31))]
        #[schema(min_length = 1, max_length = 31)]
        provider: String,
        #[garde(length(chars, min = 1, max = 255))]
        #[schema(min_length = 1, max_length = 255)]
        zone_id: String,
        #[garde(length(chars, min = 1, max = 1024))]
        #[schema(min_length = 1, max_length = 1024)]
        credential: String,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        domain: ApiDomain,
    }

    #[utoipa::path(post, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
        (status = CONFLICT, body = ApiError),
    ), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        activity_logger: GetAdminActivityLogger,
        shared::Payload(data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        if let Err(errors) = shared::utils::validate_data(&data) {
            return ApiResponse::new_serialized(ApiError::new_strings_value(errors))
                .with_status(StatusCode::BAD_REQUEST)
                .ok();
        }

        permissions.has_admin_permission("subdomains.manage")?;

        let domain = data.domain.trim().trim_end_matches('.').to_lowercase();
        let provider = data.provider.trim().to_lowercase();
        let zone_id = data.zone_id.trim().to_string();
        let credential = data.credential.trim().to_string();

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

        if Domain::by_domain(&state.database, &domain).await?.is_some() {
            return ApiResponse::error("a domain with this name already exists")
                .with_status(StatusCode::CONFLICT)
                .ok();
        }

        let domain = match Domain::insert(
            &state.database,
            &domain,
            &provider,
            &zone_id,
            &credential,
        )
        .await
        {
            Ok(domain) => domain,
            Err(err) => return ApiResponse::from(err).ok(),
        };

        activity_logger
            .log(
                "subdomains:domain.create",
                serde_json::json!({
                    "uuid": domain.uuid,
                    "domain": domain.domain,
                    "provider": domain.provider,
                    "zone_id": domain.zone_id,
                }),
            )
            .await;

        ApiResponse::new_serialized(Response {
            domain: domain.into_api(0),
        })
        .ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .routes(routes!(post::route))
        .nest("/{domain}", _domain_::router(state))
        .with_state(state.clone())
}
