use std::time::Duration;

pub fn agent(timeout: Duration) -> ureq::Agent {
    let mut builder = ureq::Agent::config_builder().timeout_global(Some(timeout));
    if let Some(proxy) = ureq::Proxy::try_from_env() {
        builder = builder.proxy(Some(proxy));
    }
    builder.build().new_agent()
}
