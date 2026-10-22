use super::parser::base64_encode;
use super::types::OpdsCatalog;
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpdsAuthError {
    InvalidUrl,
    UnsupportedScheme,
    HttpsRequired,
}

#[derive(Clone, PartialEq, Eq)]
pub struct OpdsAuth {
    username: String,
    password: String,
    origin: Url,
}

impl std::fmt::Debug for OpdsAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpdsAuth")
            .field("username", &self.username)
            .field("origin", &self.origin.as_str())
            .finish_non_exhaustive()
    }
}

impl OpdsAuth {
    pub fn from_catalog(catalog: &OpdsCatalog) -> Result<Option<Self>, OpdsAuthError> {
        let username = catalog.username.as_deref().map(str::trim).unwrap_or("");
        if username.is_empty() {
            return Ok(None);
        }

        let origin = parse_catalog_url(&catalog.url)?;
        if origin.scheme() != "https" {
            return Err(OpdsAuthError::HttpsRequired);
        }

        Ok(Some(Self {
            username: username.to_string(),
            password: catalog.password.as_deref().unwrap_or("").to_string(),
            origin,
        }))
    }

    pub fn header_for(&self, url: &str) -> Option<String> {
        let target = Url::parse(url).ok()?;
        if target.scheme() != "https" || !same_origin(&self.origin, &target) {
            return None;
        }
        Some(format!(
            "Basic {}",
            base64_encode(format!("{}:{}", self.username, self.password).as_bytes())
        ))
    }
}

pub fn validate_catalog_url(url: &str, has_auth: bool) -> Result<(), OpdsAuthError> {
    let parsed = parse_catalog_url(url)?;
    if has_auth && parsed.scheme() != "https" {
        return Err(OpdsAuthError::HttpsRequired);
    }
    Ok(())
}

fn parse_catalog_url(url: &str) -> Result<Url, OpdsAuthError> {
    let parsed = Url::parse(url.trim()).map_err(|_| OpdsAuthError::InvalidUrl)?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(OpdsAuthError::UnsupportedScheme);
    }
    if parsed.host_str().is_none() {
        return Err(OpdsAuthError::InvalidUrl);
    }
    Ok(parsed)
}

fn same_origin(a: &Url, b: &Url) -> bool {
    a.scheme() == b.scheme()
        && a.host_str() == b.host_str()
        && a.port_or_known_default() == b.port_or_known_default()
}
