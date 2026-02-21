use crate::models::{ProxyEvent, ProxyStatus, RewriteRule};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

pub const PROTOCOL_VERSION: u16 = 1;
const MAX_EVENTS: usize = 400;

#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub proxy_port: u16,
    pub control_port: u16,
}

#[derive(Debug)]
pub struct RuntimeFlags {
    pub interception_enabled: bool,
    pub cert_ready: bool,
    pub mitm_ready: bool,
    pub last_error: Option<String>,
}

#[derive(Clone)]
pub struct AppState {
    pub config: ProxyConfig,
    pub rules: Arc<RwLock<Vec<RewriteRule>>>,
    pub sequence_counters: Arc<RwLock<HashMap<String, usize>>>,
    pub events: Arc<RwLock<VecDeque<ProxyEvent>>>,
    pub flags: Arc<RwLock<RuntimeFlags>>,
}

impl AppState {
    pub fn new(config: ProxyConfig) -> Self {
        Self {
            config,
            rules: Arc::new(RwLock::new(Vec::new())),
            sequence_counters: Arc::new(RwLock::new(HashMap::new())),
            events: Arc::new(RwLock::new(VecDeque::new())),
            flags: Arc::new(RwLock::new(RuntimeFlags {
                interception_enabled: true,
                cert_ready: false,
                mitm_ready: false,
                last_error: None,
            })),
        }
    }

    pub async fn record_event(&self, event: ProxyEvent) {
        let mut events = self.events.write().await;
        if events.len() >= MAX_EVENTS {
            events.pop_front();
        }
        events.push_back(event);
    }

    pub async fn clear_events(&self) {
        self.events.write().await.clear();
    }

    pub async fn status_snapshot(&self) -> ProxyStatus {
        let flags = self.flags.read().await;
        let rule_count = self.rules.read().await.len();
        let recent_event_count = self.events.read().await.len();

        ProxyStatus {
            interception_enabled: flags.interception_enabled,
            proxy_port: self.config.proxy_port,
            control_port: self.config.control_port,
            cert_ready: flags.cert_ready,
            mitm_ready: flags.mitm_ready,
            rule_count,
            recent_event_count,
            last_error: flags.last_error.clone(),
        }
    }

    pub async fn set_last_error(&self, message: impl Into<String>) {
        self.flags.write().await.last_error = Some(message.into());
    }
}
