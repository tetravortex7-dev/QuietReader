use crate::core::library::{LibraryEntry, LibraryReadingStatus};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryExportReport {
    pub exported: usize,
    pub skipped: usize,
    pub destination: PathBuf,
}

pub fn export_library(
    destination: &Path,
    entries: &[LibraryEntry],
) -> Result<LibraryExportReport, String> {
    std::fs::create_dir_all(destination).map_err(|error| {
        format!(
            "failed to create export folder {}: {error}",
            destination.display()
        )
    })?;

    let mut exported = 0usize;
    let mut skipped = 0usize;
    let mut rows = String::from("Title,Author,Status,Progress,File\n");

    for entry in entries {
        if !entry.path.is_file() {
            skipped += 1;
            continue;
        }
        let Some(file_name) = entry.path.file_name().and_then(|name| name.to_str()) else {
            skipped += 1;
            continue;
        };
        let target = unique_target(destination, file_name);
        match std::fs::copy(&entry.path, &target) {
            Ok(_) => {
                exported += 1;
                let progress = entry.progress_percent();
                let file_label = target
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or(file_name);
                rows.push_str(&format!(
                    "{},{},{},{:.1}%,{}\n",
                    crate::core::csv_field(&entry.title),
                    crate::core::csv_field(&entry.author),
                    status_label(entry.status),
                    progress,
                    crate::core::csv_field(file_label)
                ));
            }
            Err(_) => skipped += 1,
        }
    }

    std::fs::write(destination.join("library.csv"), rows)
        .map_err(|error| format!("failed to write library.csv: {error}"))?;

    Ok(LibraryExportReport {
        exported,
        skipped,
        destination: destination.to_path_buf(),
    })
}

fn status_label(status: LibraryReadingStatus) -> &'static str {
    match status {
        LibraryReadingStatus::Unread => "Unread",
        LibraryReadingStatus::Reading => "Reading",
        LibraryReadingStatus::Finished => "Finished",
    }
}

fn unique_target(directory: &Path, file_name: &str) -> PathBuf {
    let path = Path::new(file_name);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("book");
    let extension = path.extension().and_then(|value| value.to_str());
    let mut candidate = directory.join(file_name);
    let mut index = 2usize;
    while candidate.exists() {
        let name = match extension {
            Some(extension) => format!("{stem}-{index}.{extension}"),
            None => format!("{stem}-{index}"),
        };
        candidate = directory.join(name);
        index += 1;
    }
    candidate
}

#[cfg(all(test, feature = "enable_tests"))]
mod test;
