use crate::models::{
    CertStatusUpdateRequest, ErrorResponse, HealthResponse, InterceptionUpdateRequest,
    RewriteDiagnosticsResponse, RewriteRule, VersionResponse,
};
use crate::rules::{parse_query, select_matching_rule, NormalizedRequest};
use crate::state::{AppState, PROTOCOL_VERSION};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<ErrorResponse>)>;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/version", get(version))
        .route("/status", get(status))
        .route("/status/interception", post(update_interception_status))
        .route("/status/cert", post(update_cert_status))
        .route("/rules", get(list_rules).post(create_rule))
        .route("/rules/:id", put(update_rule).delete(delete_rule))
        .route("/events", get(list_events).delete(clear_events))
        .route("/diagnostics/rewrite-check", post(rewrite_check))
        .with_state(state)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { ok: true })
}

async fn version() -> Json<VersionResponse> {
    Json(VersionResponse {
        name: "interceptkit-proxy",
        version: env!("CARGO_PKG_VERSION"),
        protocol_version: PROTOCOL_VERSION,
    })
}

async fn status(State(state): State<Arc<AppState>>) -> Json<crate::models::ProxyStatus> {
    Json(state.status_snapshot().await)
}

async fn update_interception_status(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<InterceptionUpdateRequest>,
) -> Json<crate::models::ProxyStatus> {
    state.flags.write().await.interception_enabled = payload.enabled;
    Json(state.status_snapshot().await)
}

async fn update_cert_status(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CertStatusUpdateRequest>,
) -> Json<crate::models::ProxyStatus> {
    let mut flags = state.flags.write().await;
    flags.cert_ready = payload.cert_ready;
    flags.mitm_ready = payload.mitm_ready;
    drop(flags);
    Json(state.status_snapshot().await)
}

async fn list_rules(State(state): State<Arc<AppState>>) -> Json<Vec<RewriteRule>> {
    let mut rules = state.rules.read().await.clone();
    rules.sort_by(|a, b| {
        b.priority
            .cmp(&a.priority)
            .then_with(|| a.id.cmp(&b.id))
    });
    Json(rules)
}

async fn create_rule(
    State(state): State<Arc<AppState>>,
    Json(mut rule): Json<RewriteRule>,
) -> ApiResult<RewriteRule> {
    if rule.name.trim().is_empty() {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "rule name is required".to_string(),
        ));
    }

    let now = Utc::now();
    if rule.id.trim().is_empty() {
        rule.id = Uuid::new_v4().to_string();
    }
    rule.updated_at = now;
    if rule.created_at > now {
        rule.created_at = now;
    }

    let mut rules = state.rules.write().await;
    if rules.iter().any(|existing| existing.id == rule.id) {
        return Err(error_response(
            StatusCode::CONFLICT,
            format!("rule {} already exists", rule.id),
        ));
    }

    rules.push(rule.clone());
    Ok(Json(rule))
}

async fn update_rule(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(mut payload): Json<RewriteRule>,
) -> ApiResult<RewriteRule> {
    let mut rules = state.rules.write().await;

    let position = rules
        .iter()
        .position(|existing| existing.id == id)
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, format!("rule {} not found", id)))?;

    payload.id = id;
    payload.updated_at = Utc::now();
    rules[position] = payload.clone();

    Ok(Json(payload))
}

async fn delete_rule(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let mut rules = state.rules.write().await;
    let existing_len = rules.len();
    rules.retain(|rule| rule.id != id);

    if rules.len() == existing_len {
        Err(error_response(
            StatusCode::NOT_FOUND,
            format!("rule {} not found", id),
        ))
    } else {
        state.sequence_counters.write().await.remove(&id);
        Ok(StatusCode::NO_CONTENT)
    }
}

async fn list_events(State(state): State<Arc<AppState>>) -> Json<Vec<crate::models::ProxyEvent>> {
    Json(state.events.read().await.iter().cloned().collect())
}

async fn clear_events(State(state): State<Arc<AppState>>) -> StatusCode {
    state.clear_events().await;
    StatusCode::NO_CONTENT
}

async fn rewrite_check(State(state): State<Arc<AppState>>) -> Json<RewriteDiagnosticsResponse> {
    let rules = state.rules.read().await;
    let enabled_rule_count = rules.iter().filter(|rule| rule.enabled).count();

    let sample_request = NormalizedRequest {
        method: "GET".to_string(),
        url: "http://example.com/health".to_string(),
        headers: std::collections::HashMap::new(),
        query: parse_query("http://example.com/health"),
        body_text: String::new(),
    };

    let matched_enabled_rule = select_matching_rule(&rules, &sample_request).is_some();

    Json(RewriteDiagnosticsResponse {
        ok: true,
        matched_enabled_rule,
        enabled_rule_count,
    })
}

fn error_response(status: StatusCode, message: String) -> (StatusCode, Json<ErrorResponse>) {
    (status, Json(ErrorResponse { error: message }))
}
