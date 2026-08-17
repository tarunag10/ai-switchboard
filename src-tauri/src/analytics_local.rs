use serde_json::Value;
use tauri::{AppHandle, Manager};

/// Local-free builds retain the analytics call surface so product code does
/// not need conditional branches, while compiling no remote dispatcher,
/// destination, credential, or payload code into the application binary.
pub struct AnalyticsClient;

impl AnalyticsClient {
    pub fn new(_app_version: String) -> Self {
        Self
    }

    pub fn set_headroom_ai_version(&self, _version: Option<String>) {}

    pub fn track_event(&self, _name: &str, _properties: Option<Value>) {}

    pub fn shutdown(&self) {}
}

pub fn track_event(app: &AppHandle, name: &str, properties: Option<Value>) {
    app.state::<AnalyticsClient>().track_event(name, properties);
}

pub fn set_headroom_ai_version(app: &AppHandle, version: Option<String>) {
    app.state::<AnalyticsClient>()
        .set_headroom_ai_version(version);
}

pub fn shutdown(app: &AppHandle) {
    app.state::<AnalyticsClient>().shutdown();
}

#[cfg(test)]
mod tests {
    use super::AnalyticsClient;

    #[test]
    fn local_free_client_is_an_explicit_no_op() {
        let client = AnalyticsClient::new("0.0.0-test".to_string());
        client.set_headroom_ai_version(Some("test".to_string()));
        client.track_event("test", None);
        client.shutdown();
    }
}
