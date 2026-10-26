use crate::net::{APP_USER_AGENT, APP_VERSION, user_agent};

#[test]
fn test_app_version_is_valid() {
    assert_eq!(APP_VERSION, env!("CARGO_PKG_VERSION"));
    assert!(!APP_VERSION.is_empty());
}

#[test]
fn test_app_user_agent_format() {
    let expected = format!("QuietReader/{}", env!("CARGO_PKG_VERSION"));
    assert_eq!(APP_USER_AGENT, expected);
}

#[test]
fn test_user_agent_with_and_without_component() {
    let base = user_agent(None);
    assert_eq!(base, format!("QuietReader/{}", env!("CARGO_PKG_VERSION")));

    let with_comp = user_agent(Some("OPDS Client"));
    assert_eq!(
        with_comp,
        format!("QuietReader/{} (OPDS Client)", env!("CARGO_PKG_VERSION"))
    );
}
