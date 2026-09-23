use super::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod get {
    use crate::db::{ApiAdminSubdomain, Subdomain};
    use axum::{extract::Query, http::StatusCode};
    use serde::Serialize;
    use shared::{
        ApiError, GetState,
        models::{Pagination, PaginationParamsWithSearch, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {
        #[schema(inline)]
        subdomains: Pagination<ApiAdminSubdomain>,
    }

    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
    ), params(
        (
            "page" = i64, Query,
            description = "The page number",
            example = "1",
        ),
        (
            "per_page" = i64, Query,
            description = "The number of items per page",
            example = "10",
        ),
        (
            "search" = Option<String>, Query,
            description = "Search term for items",
        ),
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        Query(params): Query<PaginationParamsWithSearch>,
    ) -> ApiResponseResult {
        if let Err(errors) = shared::utils::validate_data(&params) {
            return ApiResponse::new_serialized(ApiError::new_strings_value(errors))
                .with_status(StatusCode::BAD_REQUEST)
                .ok();
        }

        permissions.has_admin_permission("subdomains.read")?;

        let subdomains = Subdomain::all_with_pagination(
            &state.database,
            params.page,
            params.per_page,
            params.search.as_deref(),
        )
        .await?;

        ApiResponse::new_serialized(Response { subdomains }).ok()
    }
}

/// Loads a subdomain and its server for the admin routes.
async fn load(
    state: &State,
    uuid: uuid::Uuid,
) -> Result<(crate::db::Subdomain, shared::models::server::Server), shared::response::ApiResponse> {
    use shared::models::{ByUuid, server::Server};

    let not_found = || {
        shared::response::ApiResponse::error("subdomain not found")
            .with_status(axum::http::StatusCode::NOT_FOUND)
    };
    let subdomain = crate::db::Subdomain::by_uuid(&state.database, uuid)
        .await?
        .ok_or_else(not_found)?;
    let server = Server::by_uuid_optional(&state.database, subdomain.server_uuid)
        .await?
        .ok_or_else(not_found)?;
    Ok((subdomain, server))
}

mod post {
    use crate::{
        db::{ApiAdminSubdomain, Subdomain},
        service::{self, CreateOptions},
    };
    use axum::http::StatusCode;
    use garde::Validate;
    use serde::{Deserialize, Serialize};
    use shared::{
        ApiError, GetState,
        models::{
            ByUuid, admin_activity::GetAdminActivityLogger, server::Server,
            user::GetPermissionManager,
        },
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Validate, Deserialize)]
    pub struct Payload {
        #[garde(skip)]
        server_uuid: uuid::Uuid,
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
        subdomain: ApiAdminSubdomain,
    }

    /// Admins are not bound by the server's subdomain limit or the name
    /// blacklist.
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

        let server = match Server::by_uuid_optional(&state.database, data.server_uuid).await? {
            Some(server) => server,
            None => {
                return ApiResponse::error("server not found")
                    .with_status(StatusCode::BAD_REQUEST)
                    .ok();
            }
        };

        let subdomain = service::create(
            &state,
            &server,
            data.domain_uuid,
            &data.name,
            data.allocation_uuid,
            CreateOptions {
                enforce_limit: false,
                enforce_blacklist: false,
            },
        )
        .await?;

        let api_subdomain = Subdomain::by_uuid_joined(&state.database, subdomain.uuid)
            .await?
            .ok_or_else(|| anyhow::anyhow!("subdomain not found after creation"))?
            .into_api();

        activity_logger
            .log(
                "settings:extensions:subdomains.create",
                serde_json::json!({
                    "uuid": subdomain.uuid,
                    "fqdn": api_subdomain.subdomain.fqdn,
                    "server_uuid": server.uuid,
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

mod patch {
    use crate::{
        db::{ApiAdminSubdomain, Subdomain},
        service,
    };
    use axum::extract::Path;
    use serde::{Deserialize, Serialize};
    use shared::{
        ApiError, GetState,
        models::{admin_activity::GetAdminActivityLogger, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Deserialize)]
    pub struct Payload {
        allocation_uuid: uuid::Uuid,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        subdomain: ApiAdminSubdomain,
    }

    #[utoipa::path(patch, path = "/{subdomain}", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
        (status = NOT_FOUND, body = ApiError),
    ), params(("subdomain" = uuid::Uuid, description = "The subdomain ID")), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        activity_logger: GetAdminActivityLogger,
        Path(uuid): Path<uuid::Uuid>,
        shared::Payload(data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        permissions.has_admin_permission("subdomains.manage")?;

        let (mut subdomain, server) = super::load(&state, uuid).await?;
        let previous = subdomain.allocation_uuid;
        service::change_allocation(&state, &server, &mut subdomain, data.allocation_uuid).await?;

        let api_subdomain = Subdomain::by_uuid_joined(&state.database, subdomain.uuid)
            .await?
            .ok_or_else(|| anyhow::anyhow!("subdomain not found after update"))?
            .into_api();

        activity_logger
            .log(
                "settings:extensions:subdomains.update",
                serde_json::json!({
                    "uuid": subdomain.uuid,
                    "fqdn": api_subdomain.subdomain.fqdn,
                    "server_uuid": server.uuid,
                    "previous_allocation_uuid": previous,
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
    use axum::extract::{Path, Query};
    use serde::{Deserialize, Serialize};
    use shared::{
        ApiError, GetState,
        models::{admin_activity::GetAdminActivityLogger, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Deserialize)]
    pub struct Params {
        /// Delete the row even if remote record deletion fails.
        #[serde(default)]
        force: bool,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {}

    #[utoipa::path(delete, path = "/{subdomain}", responses(
        (status = OK, body = inline(Response)),
        (status = NOT_FOUND, body = ApiError),
        (status = BAD_GATEWAY, body = ApiError),
    ), params(
        ("subdomain" = uuid::Uuid, description = "The subdomain ID"),
        ("force" = Option<bool>, Query, description = "Delete even when remote record deletion fails"),
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        activity_logger: GetAdminActivityLogger,
        Path(uuid): Path<uuid::Uuid>,
        Query(params): Query<Params>,
    ) -> ApiResponseResult {
        permissions.has_admin_permission("subdomains.manage")?;

        let (subdomain, server) = super::load(&state, uuid).await?;
        service::delete(&state, &subdomain, params.force).await?;

        activity_logger
            .log(
                "settings:extensions:subdomains.delete",
                serde_json::json!({
                    "uuid": subdomain.uuid,
                    "name": subdomain.name,
                    "domain_uuid": subdomain.domain_uuid,
                    "server_uuid": server.uuid,
                    "forced": params.force,
                }),
            )
            .await;

        ApiResponse::new_serialized(Response {}).ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .routes(routes!(post::route))
        .routes(routes!(patch::route))
        .routes(routes!(delete::route))
        .with_state(state.clone())
}
