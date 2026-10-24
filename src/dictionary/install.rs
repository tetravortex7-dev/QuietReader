use flate2::read::MultiGzDecoder;
use sha2::{Digest, Sha512};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use super::catalog::CatalogEntry;

const MAX_DOWNLOAD_BYTES: u64 = 256 * 1024 * 1024;
const MAX_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_UNPACKED_BYTES: u64 = 512 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 2000;

pub fn install(
    entry: &CatalogEntry,
    dir: &Path,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<PathBuf, String> {
    let safe_id: String = entry
        .id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect();
    if safe_id.is_empty() {
        return Err("invalid dictionary id".to_string());
    }
    fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    crate::platform::permissions::create_private_dir(dir);

    let archive_path = dir.join(format!(".{safe_id}.download"));
    let tar_path = dir.join(format!(".{safe_id}.tar"));
    let temp_dir = dir.join(format!(".{safe_id}.install"));
    let final_dir = dir.join(&safe_id);
    let _ = fs::remove_file(&archive_path);
    let _ = fs::remove_file(&tar_path);
    let _ = fs::remove_dir_all(&temp_dir);

    let (_, checksum) = download(&entry.url, &archive_path, entry.size, progress)?;
    if !checksum.eq_ignore_ascii_case(&entry.checksum) {
        let _ = fs::remove_file(&archive_path);
        return Err("dictionary checksum mismatch".to_string());
    }

    let result = (|| -> Result<(), String> {
        let archive = File::open(&archive_path).map_err(|error| error.to_string())?;
        let mut tar_writer =
            BufWriter::new(File::create(&tar_path).map_err(|error| error.to_string())?);
        lzma_rs::xz_decompress(&mut BufReader::new(archive), &mut tar_writer)
            .map_err(|error| format!("dictionary archive: {error}"))?;
        tar_writer.flush().map_err(|error| error.to_string())?;

        extract_tar(&tar_path, &temp_dir)?;
        decompress_members(&temp_dir)
    })();

    let _ = fs::remove_file(&archive_path);
    let _ = fs::remove_file(&tar_path);
    if let Err(error) = result {
        let _ = fs::remove_dir_all(&temp_dir);
        return Err(error);
    }

    if final_dir.exists() {
        fs::remove_dir_all(&final_dir).map_err(|error| error.to_string())?;
    }
    fs::rename(&temp_dir, &final_dir).map_err(|error| error.to_string())?;
    Ok(final_dir)
}

fn download(
    url: &str,
    path: &Path,
    expected_size: u64,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<(u64, String), String> {
    if !url.starts_with("https://") {
        return Err("dictionary url must use https".to_string());
    }
    let agent = crate::net::agent(Duration::from_secs(300));
    let mut response = agent
        .get(url)
        .header("User-Agent", "QuietReader/0.13")
        .call()
        .map_err(|error| error.to_string())?;
    let mut reader = response.body_mut().as_reader();
    let mut writer = BufWriter::new(File::create(path).map_err(|error| error.to_string())?);
    let mut hasher = Sha512::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut received = 0u64;
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        received = received.saturating_add(read as u64);
        if received > MAX_DOWNLOAD_BYTES {
            return Err("dictionary download limit exceeded".to_string());
        }
        hasher.update(&buffer[..read]);
        writer
            .write_all(&buffer[..read])
            .map_err(|error| error.to_string())?;
        progress(received, expected_size);
    }
    writer.flush().map_err(|error| error.to_string())?;
    let digest = hasher.finalize();
    Ok((received, hex_string(&digest)))
}

fn hex_string(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn extract_tar(path: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut archive = tar::Archive::new(BufReader::new(file));
    let mut total = 0u64;
    let mut count = 0usize;
    for entry in archive.entries().map_err(|error| error.to_string())? {
        let mut entry = entry.map_err(|error| error.to_string())?;
        count += 1;
        if count > MAX_ARCHIVE_ENTRIES {
            return Err("dictionary archive has too many files".to_string());
        }
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let size = entry.size();
        total = total.saturating_add(size);
        if total > MAX_UNPACKED_BYTES {
            return Err("dictionary archive is too large".to_string());
        }
        let entry_path = entry.path().map_err(|error| error.to_string())?;
        if entry_path.is_absolute()
            || entry_path.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err("unsafe path in dictionary archive".to_string());
        }
        entry
            .unpack_in(destination)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn decompress_members(root: &Path) -> Result<(), String> {
    let mut ifo_files = Vec::new();
    collect_ifo_files(root, 0, &mut ifo_files)?;
    if ifo_files.is_empty() {
        return Err("dictionary archive has no StarDict data".to_string());
    }
    for ifo in ifo_files {
        let Some(folder) = ifo.parent() else {
            continue;
        };
        let idx_gz = folder.join(format!(
            "{}.idx.gz",
            ifo.file_stem().and_then(|stem| stem.to_str()).unwrap_or("")
        ));
        let dict_dz = folder.join(format!(
            "{}.dict.dz",
            ifo.file_stem().and_then(|stem| stem.to_str()).unwrap_or("")
        ));
        let plain_idx = with_new_extension(&idx_gz, "idx");
        let plain_dict = with_new_extension(&dict_dz, "dict");
        if idx_gz.exists() {
            gunzip_to(&idx_gz, &plain_idx)?;
            let _ = fs::remove_file(&idx_gz);
        }
        if dict_dz.exists() {
            gunzip_to(&dict_dz, &plain_dict)?;
            let _ = fs::remove_file(&dict_dz);
        }
        if !plain_idx.exists() || !plain_dict.exists() {
            return Err("dictionary archive is incomplete".to_string());
        }
    }
    Ok(())
}

fn with_new_extension(path: &Path, extension: &str) -> PathBuf {
    let stem = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .trim_end_matches(".gz")
        .trim_end_matches(".dz");
    path.with_file_name(stem).with_extension(extension)
}

fn collect_ifo_files(dir: &Path, depth: usize, found: &mut Vec<PathBuf>) -> Result<(), String> {
    if depth > 3 {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_ifo_files(&path, depth + 1, found)?;
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("ifo") {
            found.push(path);
        }
    }
    Ok(())
}

fn gunzip_to(source: &Path, destination: &Path) -> Result<(), String> {
    let file = File::open(source).map_err(|error| error.to_string())?;
    let mut decoder = MultiGzDecoder::new(BufReader::new(file));
    let mut writer = BufWriter::new(File::create(destination).map_err(|error| error.to_string())?);
    let mut buffer = [0u8; 64 * 1024];
    let mut written = 0u64;
    loop {
        let read = decoder
            .read(&mut buffer)
            .map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        written = written.saturating_add(read as u64);
        if written > MAX_ARCHIVE_BYTES {
            return Err("dictionary data is too large".to_string());
        }
        writer
            .write_all(&buffer[..read])
            .map_err(|error| error.to_string())?;
    }
    writer.flush().map_err(|error| error.to_string())?;
    Ok(())
}
