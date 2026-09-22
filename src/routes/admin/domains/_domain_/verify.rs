use super::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod post {
    use axum::http::StatusCode;
    use serde::Serialize;
    use shared::{
        ApiError, GetState,
        models::{admin_activity::GetAdminActivityLogger, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    use super::super::GetDomain;

    #[derive(ToSchema, Serialize)]
    struct Response {
        zone_name: String,
    }

    #[utoipa::path(post, path = "/", responses(
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
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        activity_logger: GetAdminActivityLogger,
        domain: GetDomain,
    ) -> ApiResponseResult {
        permissions.has_admin_permission("subdomains.manage")?;

        let client = match domain.provider_client(&state.database).await {
            Ok(client) => client,
            Err(err) => return ApiResponse::from(err).ok(),
        };

        let zone_name = match client.verify().await {
            Ok(zone_name) => zone_name,
            Err(err) => {
                return ApiResponse::error(format!("failed to verify zone: {err}"))
                    .with_status(StatusCode::BAD_REQUEST)
                    .ok();
            }
        };

        activity_logger
            .log(
                "subdomains:domain.verify",
                serde_json::json!({
                    "uuid": domain.uuid,
                    "domain": domain.domain,
                    "zone_name": zone_name,
                }),
            )
            .await;

        ApiResponse::new_serialized(Response { zone_name }).ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(post::route))
        .with_state(state.clone())
}
