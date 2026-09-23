use super::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod _subdomain_;

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
        db::{ApiSubdomain, Subdomain},
        service::{self, CreateOptions},
    };
    use axum::http::StatusCode;
    use garde::Validate;
    use serde::{Deserialize, Serialize};
    use shared::{
        ApiError, GetState,
        models::{
            server::{GetServer, GetServerActivityLogger},
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

        let subdomain = service::create(
            &state,
            &server,
            data.domain_uuid,
            &data.name,
            data.allocation_uuid,
            CreateOptions {
                enforce_limit: true,
                enforce_blacklist: true,
            },
        )
        .await?;

        let api_subdomain = Subdomain::by_uuid_joined(&state.database, subdomain.uuid)
            .await?
            .ok_or_else(|| anyhow::anyhow!("subdomain not found after creation"))?
            .into_subdomain_api();

        activity_logger
            .log(
                "server:subdomains.create",
                serde_json::json!({
                    "uuid": subdomain.uuid,
                    "name": subdomain.name,
                    "fqdn": api_subdomain.fqdn,
                    "allocation_uuid": subdomain.allocation_uuid,
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
