use super::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod get {
    use crate::settings::ExtensionSettingsData;
    use serde::Serialize;
    use shared::{
        ApiError, GetState,
        models::user::GetPermissionManager,
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {
        settings: ExtensionSettingsData,
    }

    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = UNAUTHORIZED, body = ApiError),
    ))]
    pub async fn route(state: GetState, permissions: GetPermissionManager) -> ApiResponseResult {
        permissions.has_admin_permission("subdomains.read")?;

        let settings = state.settings.get().await?;
        let extension = settings.find_extension_settings::<ExtensionSettingsData>()?;

        ApiResponse::new_serialized(Response {
            settings: extension.clone(),
        })
        .ok()
    }
}

mod put {
    use crate::settings::ExtensionSettingsData;
    use axum::http::StatusCode;
    use serde::Serialize;
    use shared::{
        ApiError, GetState,
        models::{admin_activity::GetAdminActivityLogger, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use std::collections::HashSet;
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {}

    #[utoipa::path(put, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
    ), request_body = ExtensionSettingsData)]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        activity_logger: GetAdminActivityLogger,
        shared::Payload(data): shared::Payload<ExtensionSettingsData>,
    ) -> ApiResponseResult {
        if let Err(errors) = shared::utils::validate_data(&data) {
            return ApiResponse::new_serialized(ApiError::new_strings_value(errors))
                .with_status(StatusCode::BAD_REQUEST)
                .ok();
        }

        permissions.has_admin_permission("subdomains.manage")?;

        let mut invalid = Vec::new();
        for pattern in &data.blacklist {
            if let Err(err) = regex::RegexBuilder::new(pattern)
                .case_insensitive(true)
                .build()
            {
                invalid.push(format!(
                    "blacklist entry `{pattern}` is not a valid regex: {err}"
                ));
            }
        }
        if !invalid.is_empty() {
            return ApiResponse::new_serialized(ApiError::new_strings_value(invalid))
                .with_status(StatusCode::BAD_REQUEST)
                .ok();
        }

        let mut seen_eggs = HashSet::new();
        for entry in &data.egg_records {
            if !seen_eggs.insert(entry.egg_uuid) {
                return ApiResponse::error(format!(
                    "duplicate egg override for egg {}",
                    entry.egg_uuid
                ))
                .with_status(StatusCode::BAD_REQUEST)
                .ok();
            }
        }

        let mut settings = state.settings.get_mut().await?;
        *settings.find_mut_extension_settings::<ExtensionSettingsData>()? = data.clone();
        settings.save().await?;

        activity_logger
            .log(
                "subdomains:settings.update",
                serde_json::json!({
                    "blacklist": data.blacklist,
                    "default_limit": data.default_limit,
                    "default_records": data.default_records.len(),
                    "egg_records": data.egg_records.len(),
                }),
            )
            .await;

        ApiResponse::new_serialized(Response {}).ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .routes(routes!(put::route))
        .with_state(state.clone())
}
