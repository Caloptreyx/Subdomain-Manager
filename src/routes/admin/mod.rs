use super::State;
use utoipa_axum::router::OpenApiRouter;

mod domains;
mod settings;
mod subdomains;

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .nest("/settings", settings::router(state))
        .nest("/domains", domains::router(state))
        .nest("/subdomains", subdomains::router(state))
        .with_state(state.clone())
}
