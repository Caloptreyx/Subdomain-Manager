use garde::Validate;
use serde::{Deserialize, Serialize};
use shared::{
    database::DatabaseError,
    models::{
        BaseModel, ModelExtension, ModelExtensionMapType, SafeModelExtension, server::Server,
    },
};
use sqlx::{Row, postgres::PgRow};
use std::collections::BTreeMap;
use utoipa::ToSchema;

/// The extension's columns on `servers` - a per-server subdomain cap.
#[derive(Serialize, Deserialize)]
pub struct ServerExtensionData {
    pub subdomain_limit: i32,
}

pub struct ServerExtension;

impl SafeModelExtension for ServerExtension {
    type Value = ServerExtensionData;

    fn name() -> &'static str {
        ServerExtension.extension_name()
    }
}

impl ModelExtension for ServerExtension {
    fn extension_name(&self) -> &'static str {
        "dev.caloptreyx.subdomains"
    }

    fn extended_columns(&self, prefix: &str) -> BTreeMap<&'static str, compact_str::CompactString> {
        BTreeMap::from([(
            "servers.subdomain_limit",
            compact_str::format_compact!("{prefix}subdomain_limit"),
        )])
    }

    fn map_extended(
        &self,
        prefix: &str,
        row: &PgRow,
    ) -> Result<ModelExtensionMapType, DatabaseError> {
        Ok(Box::new(ServerExtensionData {
            subdomain_limit: row
                .try_get(compact_str::format_compact!("{prefix}subdomain_limit").as_str())?,
        }))
    }
}

/// Extension fields on `ApiServerFeatureLimits`. `subdomains` is optional so
/// clients unaware of the extension don't reset the limit on updates.
#[derive(ToSchema, Validate, Serialize, Deserialize)]
pub struct ExtendedApiServerFeatureLimits {
    #[garde(range(min = 0))]
    #[schema(minimum = 0)]
    pub subdomains: Option<i32>,
}

/// Reads a server's configured subdomain limit from its extension data.
pub fn subdomain_limit(server: &Server) -> Result<i32, DatabaseError> {
    Ok(server
        .parse_model_extension::<ServerExtension>()?
        .subdomain_limit)
}
