use axum::{Router, routing::post};

mod bridge;
mod docs;
mod session;
mod setup;
mod state;
mod ui;

pub use state::AppState;

#[cfg(test)]
mod tests;

pub async fn build_router() -> anyhow::Result<Router> {
    let state = AppState::default();
    let router = Router::new()
        .route("/session/init", post(session::session_init))
        .route("/docs/read", post(docs::docs_read))
        .route("/ui/install", post(ui::ui_install))
        .route("/setup/run", post(setup::setup_run))
        .route("/bridge/scaffold", post(bridge::bridge_scaffold))
        .route("/bridge/start", post(bridge::bridge_start))
        .route("/bridge/status", post(bridge::bridge_status))
        .route("/bridge/stop", post(bridge::bridge_stop))
        .with_state(state);
    Ok(router)
}
