use std::time::Duration;

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_USER_AGENT: &str = concat!("QuietReader/", env!("CARGO_PKG_VERSION"));

pub fn user_agent(component: Option<&str>) -> String {
    match component {
        Some(comp) => format!("QuietReader/{APP_VERSION} ({comp})"),
        None => APP_USER_AGENT.to_string(),
    }
}

pub fn agent(timeout: Duration) -> ureq::Agent {
    let mut builder = ureq::Agent::config_builder().timeout_global(Some(timeout));
    if let Some(proxy) = ureq::Proxy::try_from_env() {
        builder = builder.proxy(Some(proxy));
    }
    builder.build().new_agent()
}

#[cfg(all(test, feature = "enable_tests"))]
mod test;
