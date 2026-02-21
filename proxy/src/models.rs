use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RewriteRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub priority: i32,
    pub matcher: RuleMatcher,
    pub action: RuleAction,
    #[serde(default)]
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleMatcher {
    pub methods: Option<Vec<String>>,
    pub url_pattern: String,
    #[serde(default)]
    pub header_equals: HashMap<String, String>,
    #[serde(default)]
    pub query_equals: HashMap<String, String>,
    pub body_contains: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RuleAction {
    MockResponse(MockResponseAction),
    RewritePassThrough(RewritePassThroughAction),
    Sequence(SequenceAction),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MockResponseAction {
    pub status: u16,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub content_type: Option<String>,
    pub delay_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RewritePassThroughAction {
    pub request: Option<TransformOps>,
    pub response: Option<TransformOps>,
    pub delay_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformOps {
    #[serde(default)]
    pub set_headers: HashMap<String, String>,
    #[serde(default)]
    pub remove_headers: Vec<String>,
    pub replace_body: Option<String>,
    #[serde(default)]
    pub json_set: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceAction {
    pub steps: Vec<SequenceStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceStep {
    pub action: SequenceStepAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SequenceStepAction {
    MockResponse(MockResponseAction),
    RewritePassThrough(RewritePassThroughAction),
}

#[derive(Debug, Clone)]
pub enum ResolvedAction {
    MockResponse(MockResponseAction),
    RewritePassThrough(RewritePassThroughAction),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyEvent {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub method: String,
    pub url: String,
    pub matched_rule_id: Option<String>,
    pub matched_rule_name: Option<String>,
    pub decision: String,
    pub status: Option<u16>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyStatus {
    pub interception_enabled: bool,
    pub proxy_port: u16,
    pub control_port: u16,
    pub cert_ready: bool,
    pub mitm_ready: bool,
    pub rule_count: usize,
    pub recent_event_count: usize,
    pub last_error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterceptionUpdateRequest {
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CertStatusUpdateRequest {
    pub cert_ready: bool,
    pub mitm_ready: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionResponse {
    pub name: &'static str,
    pub version: &'static str,
    pub protocol_version: u16,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub ok: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RewriteDiagnosticsResponse {
    pub ok: bool,
    pub matched_enabled_rule: bool,
    pub enabled_rule_count: usize,
}
