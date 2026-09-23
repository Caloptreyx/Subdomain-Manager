use super::State;
use axum::{
    extract::{Path, Request},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use shared::{
    GetState,
    models::{server::GetServer, user::GetPermissionManager},
    response::ApiResponse,
};
use utoipa_axum::{router::OpenApiRouter, routes};

pub type GetSubdomain = shared::extract::ConsumingExtension<crate::db::Subdomain>;

pub async fn auth(
    state: GetState,
    permissions: GetPermissionManager,
    server: GetServer,
    Path(subdomain): Path<Vec<String>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let subdomain = match subdomain.get(1).map(|s| s.parse::<uuid::Uuid>()) {
        Some(Ok(uuid)) => uuid,
        _ => {
            return Ok(ApiResponse::error("invalid subdomain uuid")
                .with_status(StatusCode::BAD_REQUEST)
                .into_response());
        }
    };

    if let Err(err) = permissions.has_server_permission("subdomains.read") {
        return Ok(err.into_response());
    }

    let subdomain = match crate::db::Subdomain::by_uuid(&state.database, subdomain).await {
        Ok(Some(subdomain)) if subdomain.server_uuid == server.uuid => subdomain,
        Ok(_) => {
            return Ok(ApiResponse::error("subdomain not found")
                .with_status(StatusCode::NOT_FOUND)
                .into_response());
        }
        Err(err) => return Ok(ApiResponse::from(err).into_response()),
    };

    req.extensions_mut().insert(server.0);
    req.extensions_mut().insert(subdomain);

    Ok(next.run(req).await)
}

mod patch {
    use crate::{
        db::{ApiSubdomain, Subdomain},
        service,
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

    use super::GetSubdomain;

    #[derive(ToSchema, Validate, Deserialize)]
    pub struct Payload {
        #[garde(skip)]
        allocation_uuid: uuid::Uuid,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        subdomain: ApiSubdomain,
    }

    #[utoipa::path(patch, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
        (status = NOT_FOUND, body = ApiError),
    ), params(
        (
            "server" = uuid::Uuid,
            description = "The server ID",
            example = "123e4567-e89b-12d3-a456-426614174000",
        ),
        (
            "subdomain" = uuid::Uuid,
            description = "The subdomain ID",
            example = "123e4567-e89b-12d3-a456-426614174000",
        ),
    ), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        server: GetServer,
        mut subdomain: GetSubdomain,
        activity_logger: GetServerActivityLogger,
        shared::Payload(data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        if let Err(errors) = shared::utils::validate_data(&data) {
            return ApiResponse::new_serialized(ApiError::new_strings_value(errors))
                .with_status(StatusCode::BAD_REQUEST)
                .ok();
        }

        permissions.has_server_permission("subdomains.update")?;

        service::change_allocation(&state, &server, &mut subdomain, data.allocation_uuid).await?;

        let api_subdomain = Subdomain::by_uuid_joined(&state.database, subdomain.uuid)
            .await?
            .ok_or_else(|| anyhow::anyhow!("subdomain not found after update"))?
            .into_subdomain_api();

        activity_logger
            .log(
                "server:subdomains.update",
                serde_json::json!({
                    "uuid": subdomain.uuid,
                    "name": subdomain.name,
                    "fqdn": api_subdomain.fqdn,
                    "allocation_uuid": data.allocation_uuid,
                }),
            )
            .await;

        ApiResponse::new_serialized(Response {
            subdomain: api_subdomain,
        })
        .ok()
    }
}

mod delete {
    use crate::service;
    use axum::extract::Query;
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

    use super::GetSubdomain;

    #[derive(ToSchema, Deserialize)]
    pub struct Params {
        /// Delete the row even if remote record deletion fails.
        #[serde(default)]
        force: bool,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {}

    #[utoipa::path(delete, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = UNAUTHORIZED, body = ApiError),
        (status = NOT_FOUND, body = ApiError),
        (status = BAD_GATEWAY, body = ApiError),
    ), params(
        (
            "server" = uuid::Uuid,
            description = "The server ID",
            example = "123e4567-e89b-12d3-a456-426614174000",
        ),
        (
            "subdomain" = uuid::Uuid,
            description = "The subdomain ID",
            example = "123e4567-e89b-12d3-a456-426614174000",
        ),
        (
            "force" = Option<bool>, Query,
            description = "Delete even when remote record deletion fails",
        ),
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        _server: GetServer,
        subdomain: GetSubdomain,
        activity_logger: GetServerActivityLogger,
        Query(params): Query<Params>,
    ) -> ApiResponseResult {
        permissions.has_server_permission("subdomains.delete")?;

        service::delete(&state, &subdomain, params.force).await?;

        activity_logger
            .log(
                "server:subdomains.delete",
                serde_json::json!({
                    "uuid": subdomain.uuid,
                    "name": subdomain.name,
                    "domain_uuid": subdomain.domain_uuid,
                    "created": subdomain.created,
                    "forced": params.force,
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
        .route_layer(axum::middleware::from_fn_with_state(state.clone(), auth))
        .with_state(state.clone())
}
