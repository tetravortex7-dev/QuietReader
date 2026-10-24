pub use crate::settings_config::{OpdsCatalog, default_opds_catalogs};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpdsAcquisitionLink {
    pub href: String,
    pub mime_type: String,
    pub title: String,
    pub format_label: String,
    pub extension: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpdsEntry {
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub summary: Option<String>,
    pub updated: Option<String>,
    pub categories: Vec<String>,
    pub thumbnail_url: Option<String>,
    pub acquisition_links: Vec<OpdsAcquisitionLink>,
    pub navigation_link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpdsLink {
    pub rel: String,
    pub href: String,
    pub link_type: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpdsFeed {
    pub title: String,
    pub id: String,
    pub icon: Option<String>,
    pub links: Vec<OpdsLink>,
    pub entries: Vec<OpdsEntry>,
    pub next_url: Option<String>,
    pub prev_url: Option<String>,
    pub search_template: Option<String>,
    pub search_descriptor_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadStatus {
    Queued,
    Downloading {
        downloaded: usize,
        total: Option<usize>,
    },
    Completed {
        file_path: PathBuf,
    },
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct DownloadItem {
    pub book_id: String,
    pub title: String,
    pub author: String,
    pub format_label: String,
    pub status: DownloadStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpdsBreadcrumb {
    pub title: String,
    pub url: String,
}
