use indexmap::IndexMap;
use shared::{
    Extendible, State,
    extensions::{
        Extension, ExtensionPermissionsBuilder, ExtensionRouteBuilder,
        settings::ExtensionSettingsDeserializer,
    },
    models::{
        BaseModel, CreatableModel, DeletableModel, EventEmittingModel, ListenerPriority,
        UpdatableModel,
        server::{ApiServerFeatureLimits, Server, ServerEvent},
        server_allocation::ServerAllocation,
    },
    permissions::PermissionGroup,
};
use std::{collections::HashMap, sync::Arc};

mod db;
mod model;
mod providers;
mod records;
mod routes;
mod service;
mod settings;

#[derive(Default)]
pub struct ExtensionStruct;

#[async_trait::async_trait]
impl Extension for ExtensionStruct {
    async fn initialize(&mut self, _state: State) {
        // SELECT-side: load servers.subdomain_limit with every Server.
        Server::register_model_extension(model::ServerExtension);

        // CREATE: take `feature_limits.subdomains` from the payload, else the
        // extension's configured default limit.
        Server::register_create_handler(
            ListenerPriority::Normal,
            |options, query_builder, state, _transaction| {
                Box::pin(async move {
                    let limit = match options
                        .feature_limits
                        .parse_extended::<model::ExtendedApiServerFeatureLimits>()
                        .ok()
                        .and_then(|extended| extended.subdomains)
                    {
                        Some(limit) => limit,
                        None => state
                            .settings
                            .get()
                            .await
                            .and_then(|settings| {
                                settings
                                    .find_extension_settings::<settings::ExtensionSettingsData>()
                                    .map(|extension| extension.default_limit)
                            })
                            .unwrap_or(0),
                    };

                    // the column is NOT NULL - always write a value
                    query_builder.set("subdomain_limit", limit.max(0));
                    Ok(())
                })
            },
        );

        // UPDATE: only write when the client actually sent `subdomains` -
        // Option<i32> keeps "absent" distinct from "set to 0".
        Server::register_update_handler(
            ListenerPriority::Normal,
            |_server, options, query_builder, _state, _transaction| {
                Box::pin(async move {
                    if let Some(feature_limits) = &options.feature_limits
                        && let Ok(extended) =
                            feature_limits.parse_extended::<model::ExtendedApiServerFeatureLimits>()
                        && let Some(value) = extended.subdomains
                    {
                        query_builder.set("subdomain_limit", Some(value.max(0)));
                    }
                    Ok(())
                })
            },
        );

        // DELETE: best-effort cleanup of remote DNS records; the rows are
        // removed by the ON DELETE CASCADE. Never fail the deletion.
        Server::register_delete_handler(
            ListenerPriority::Normal,
            |server, _options, state, _transaction| {
                Box::pin(async move {
                    match db::Subdomain::all_by_server_uuid_plain(&state.database, server.uuid)
                        .await
                    {
                        Ok(subdomains) => {
                            delete_dns_records(state, &subdomains).await;
                        }
                        Err(err) => tracing::warn!(
                            server = %server.uuid,
                            "failed to load subdomains for dns cleanup: {err:?}"
                        ),
                    }

                    Ok(())
                })
            },
        );

        ServerAllocation::register_delete_handler(
            ListenerPriority::Normal,
            |allocation, _options, state, _transaction| {
                Box::pin(async move {
                    match db::Subdomain::all_by_allocation_uuid(&state.database, allocation.uuid)
                        .await
                    {
                        Ok(subdomains) => release_subdomains(state, subdomains).await,
                        Err(err) => tracing::warn!(
                            allocation = %allocation.uuid,
                            "failed to load subdomains for dns cleanup: {err:?}"
                        ),
                    }

                    Ok(())
                })
            },
        );

        Server::register_event_handler(|state, event| async move {
            if let ServerEvent::TransferCompleted {
                server,
                successful: true,
                ..
            } = &*event
            {
                match db::Subdomain::all_by_server_uuid_plain(&state.database, server.uuid).await {
                    Ok(subdomains) => release_subdomains(&state, subdomains).await,
                    Err(err) => tracing::warn!(
                        server = %server.uuid,
                        "failed to load subdomains after transfer: {err:?}"
                    ),
                }
            }
            Ok(())
        });

        // Expose the column on `feature_limits.subdomains` in server API JSON.
        ApiServerFeatureLimits::extend_validated(
            |server, _state| {
                Box::pin(
                    async move { Ok(server.parse_model_extension::<model::ServerExtension>()?) },
                )
            },
            |_limits, extension, _state| model::ExtendedApiServerFeatureLimits {
                subdomains: Some(extension.subdomain_limit),
            },
        );
    }

    async fn initialize_router(
        &mut self,
        state: State,
        builder: ExtensionRouteBuilder,
    ) -> ExtensionRouteBuilder {
        builder
            .add_admin_api_router(|router| {
                router.nest(
                    "/extensions/dev.caloptreyx.subdomains",
                    routes::admin::router(&state),
                )
            })
            .add_client_server_api_router(|router| {
                router.nest("/subdomains", routes::server::router(&state))
            })
    }

    async fn initialize_permissions(
        &mut self,
        _state: State,
        mut builder: ExtensionPermissionsBuilder,
    ) -> ExtensionPermissionsBuilder {
        builder.server_permissions.insert(
            "subdomains",
            PermissionGroup {
                description: "Permissions that control the ability to manage subdomains for this server.",
                permissions: IndexMap::from([
                    (
                        "read",
                        "Allows viewing the server's subdomains, its limit and the available domains.",
                    ),
                    (
                        "create",
                        "Allows creating new subdomains for the server.",
                    ),
                    (
                        "update",
                        "Allows changing which allocation a subdomain points at.",
                    ),
                    (
                        "delete",
                        "Allows deleting the server's subdomains.",
                    ),
                ]),
            },
        );

        builder.admin_permissions.insert(
            "subdomains",
            PermissionGroup {
                description: "Permissions that control the ability to manage the subdomain manager extension.",
                permissions: IndexMap::from([
                    (
                        "read",
                        "Allows viewing the extension's settings, domains and all created subdomains.",
                    ),
                    (
                        "manage",
                        "Allows changing settings and managing DNS domains and subdomains.",
                    ),
                ]),
            },
        );

        builder
    }

    async fn settings_deserializer(&self, _state: State) -> ExtensionSettingsDeserializer {
        Arc::new(settings::ExtensionSettingsDataDeserializer)
    }
}

async fn delete_dns_records(
    state: &State,
    subdomains: &[db::Subdomain],
) -> HashMap<uuid::Uuid, Vec<providers::StoredRecord>> {
    let mut domains: HashMap<uuid::Uuid, Option<db::Domain>> = HashMap::new();
    let mut remaining = HashMap::new();

    for subdomain in subdomains {
        let domain = match domains.entry(subdomain.domain_uuid) {
            std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::hash_map::Entry::Vacant(entry) => entry.insert(
                db::Domain::by_uuid(&state.database, subdomain.domain_uuid)
                    .await
                    .unwrap_or(None),
            ),
        };
        let Some(domain) = domain else {
            remaining.insert(subdomain.uuid, subdomain.records.to_vec());
            continue;
        };

        let provider = match domain.provider_client(&state.database).await {
            Ok(provider) => provider,
            Err(err) => {
                tracing::warn!(
                    server = %subdomain.server_uuid,
                    domain = %domain.domain,
                    "failed to build dns provider for subdomain cleanup: {err:?}"
                );
                remaining.insert(subdomain.uuid, subdomain.records.to_vec());
                continue;
            }
        };

        let mut left = Vec::new();
        for record in subdomain.records.iter() {
            if let Err(err) = provider.delete_record(&record.id).await {
                tracing::warn!(
                    server = %subdomain.server_uuid,
                    subdomain = %subdomain.name,
                    record = %record.id,
                    "failed to delete dns record: {err:?}"
                );
                left.push(record.clone());
            }
        }
        remaining.insert(subdomain.uuid, left);
    }

    remaining
}

async fn release_subdomains(state: &State, subdomains: Vec<db::Subdomain>) {
    let mut remaining = delete_dns_records(state, &subdomains).await;

    for mut subdomain in subdomains {
        let left = remaining.remove(&subdomain.uuid).unwrap_or_default();
        if let Err(err) = subdomain
            .update_allocation_and_records(&state.database, None, &left)
            .await
        {
            tracing::warn!(
                server = %subdomain.server_uuid,
                subdomain = %subdomain.name,
                "failed to reset subdomain after its allocation went away: {err:?}"
            );
        }
    }
}
