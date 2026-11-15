#[path = "../../core/license_codec.rs"]
mod license_codec;
#[path = "../../core/license_sign.rs"]
mod license_sign;

#[cfg(all(test, feature = "enable_tests"))]
mod test;

use ed25519_dalek::SigningKey;
use license_codec::{LicenseClaims, decode_hex, verify_license_key_with_verifying_keys};
use license_sign::{encode_hex, sign_license_key};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_KEY_PATH: &str = ".keys/developer_ed25519.key";

enum Command {
    Help,
    InitKey {
        out: PathBuf,
        force: bool,
    },
    Pubkey {
        key_file: Option<PathBuf>,
        key_hex: Option<String>,
    },
    Issue {
        email: String,
        issued_at_unix: Option<u64>,
        key_file: Option<PathBuf>,
        key_hex: Option<String>,
    },
    Verify {
        token: String,
        pubkey_hex: String,
    },
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(err) = run(&args) {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let cmd = parse_args(args)?;
    match cmd {
        Command::Help => {
            print_help();
            Ok(())
        }
        Command::InitKey { out, force } => {
            let mut seed = [0u8; 32];
            rustls::crypto::ring::default_provider()
                .secure_random
                .fill(&mut seed)
                .map_err(|_| "failed to read OS cryptographic RNG".to_string())?;
            write_signing_seed(&out, &seed, force)?;
            let pub_bytes = SigningKey::from_bytes(&seed).verifying_key().to_bytes();
            println!("Private key saved to: {}", out.display());
            println!("Public key (hex): {}", encode_hex(&pub_bytes));
            println!(
                "Rust verifying key array:\n{}",
                format_rust_key_array(&pub_bytes)
            );
            Ok(())
        }
        Command::Pubkey { key_file, key_hex } => {
            let seed = resolve_signing_seed(key_file.as_deref(), key_hex.as_deref())?;
            let pub_bytes = SigningKey::from_bytes(&seed).verifying_key().to_bytes();
            println!("Public key (hex): {}", encode_hex(&pub_bytes));
            println!(
                "Rust verifying key array:\n{}",
                format_rust_key_array(&pub_bytes)
            );
            Ok(())
        }
        Command::Issue {
            email,
            issued_at_unix,
            key_file,
            key_hex,
        } => {
            let seed = resolve_signing_seed(key_file.as_deref(), key_hex.as_deref())?;
            let issued_at = issued_at_unix.unwrap_or_else(current_unix_secs);
            let token = issue_lifetime_token(&seed, &email, issued_at)?;
            println!("{token}");
            Ok(())
        }
        Command::Verify { token, pubkey_hex } => {
            let pub_bytes = parse_key_32(&pubkey_hex, "public key")?;
            let claims = verify_license_key_with_verifying_keys(&token, &[&pub_bytes])
                .map_err(|e| format!("verification failed: {e:?}"))?;
            println!(
                "Valid license key for {} (issued_at_unix={})",
                claims.email, claims.issued_at_unix
            );
            Ok(())
        }
    }
}

fn parse_args(args: &[String]) -> Result<Command, String> {
    let Some(sub) = args.first().map(|s| s.as_str()) else {
        return Ok(Command::Help);
    };
    if matches!(sub, "-h" | "--help" | "help") {
        return Ok(Command::Help);
    }
    let rest = &args[1..];
    match sub {
        "init-key" => {
            let mut out = PathBuf::from(DEFAULT_KEY_PATH);
            let mut force = false;
            let mut i = 0;
            while i < rest.len() {
                match rest[i].as_str() {
                    "--out" => {
                        i += 1;
                        let val = rest.get(i).ok_or("--out requires a path")?;
                        out = PathBuf::from(val);
                    }
                    "--force" => force = true,
                    other => return Err(format!("unknown flag for init-key: {other}")),
                }
                i += 1;
            }
            Ok(Command::InitKey { out, force })
        }
        "pubkey" => {
            let mut key_file = None;
            let mut key_hex = None;
            let mut i = 0;
            while i < rest.len() {
                match rest[i].as_str() {
                    "--key-file" => {
                        i += 1;
                        let val = rest.get(i).ok_or("--key-file requires a path")?;
                        key_file = Some(PathBuf::from(val));
                    }
                    "--key-hex" => {
                        i += 1;
                        let val = rest.get(i).ok_or("--key-hex requires a value")?;
                        key_hex = Some(val.clone());
                    }
                    other => return Err(format!("unknown flag for pubkey: {other}")),
                }
                i += 1;
            }
            Ok(Command::Pubkey { key_file, key_hex })
        }
        "issue" => {
            let mut email = None;
            let mut issued_at_unix = None;
            let mut key_file = None;
            let mut key_hex = None;
            let mut i = 0;
            while i < rest.len() {
                match rest[i].as_str() {
                    "--email" => {
                        i += 1;
                        let val = rest.get(i).ok_or("--email requires a value")?;
                        email = Some(val.clone());
                    }
                    "--issued-at" => {
                        i += 1;
                        let val = rest.get(i).ok_or("--issued-at requires a unix timestamp")?;
                        let parsed = val
                            .parse::<u64>()
                            .map_err(|_| format!("invalid unix timestamp: {val}"))?;
                        issued_at_unix = Some(parsed);
                    }
                    "--key-file" => {
                        i += 1;
                        let val = rest.get(i).ok_or("--key-file requires a path")?;
                        key_file = Some(PathBuf::from(val));
                    }
                    "--key-hex" => {
                        i += 1;
                        let val = rest.get(i).ok_or("--key-hex requires a value")?;
                        key_hex = Some(val.clone());
                    }
                    other => return Err(format!("unknown flag for issue: {other}")),
                }
                i += 1;
            }
            let email = email.ok_or("issue requires --email <email>")?;
            Ok(Command::Issue {
                email,
                issued_at_unix,
                key_file,
                key_hex,
            })
        }
        "verify" => {
            let mut token = None;
            let mut pubkey_hex = None;
            let mut i = 0;
            while i < rest.len() {
                match rest[i].as_str() {
                    "--key" => {
                        i += 1;
                        let val = rest.get(i).ok_or("--key requires a license key")?;
                        token = Some(val.clone());
                    }
                    "--pubkey-hex" => {
                        i += 1;
                        let val = rest.get(i).ok_or("--pubkey-hex requires a hex key")?;
                        pubkey_hex = Some(val.clone());
                    }
                    other => return Err(format!("unknown flag for verify: {other}")),
                }
                i += 1;
            }
            Ok(Command::Verify {
                token: token.ok_or("verify requires --key <QR1-...>")?,
                pubkey_hex: pubkey_hex.ok_or("verify requires --pubkey-hex <64-hex-chars>")?,
            })
        }
        other => Err(format!("unknown command: {other}")),
    }
}

fn print_help() {
    println!("QuietReader offline license key tool");
    println!("Usage:");
    println!("  qr-keygen init-key [--out <path>] [--force]");
    println!("  qr-keygen pubkey [--key-file <path> | --key-hex <hex>]");
    println!(
        "  qr-keygen issue --email <email> [--issued-at <unix>] [--key-file <path> | --key-hex <hex>]"
    );
    println!("  qr-keygen verify --key <QR1-...> --pubkey-hex <hex>");
}

pub(crate) fn issue_lifetime_token(
    seed: &[u8; 32],
    email: &str,
    issued_at_unix: u64,
) -> Result<String, String> {
    let trimmed = email.trim();
    if trimmed.is_empty() {
        return Err("email must not be empty".to_string());
    }
    let claims = LicenseClaims {
        email: trimmed.to_string(),
        issued_at_unix,
    };
    let token = sign_license_key(&claims, seed);
    let pub_bytes = SigningKey::from_bytes(seed).verifying_key().to_bytes();
    verify_license_key_with_verifying_keys(&token, &[&pub_bytes])
        .map_err(|e| format!("self-verification failed: {e:?}"))?;
    Ok(token)
}

pub(crate) fn write_signing_seed(path: &Path, seed: &[u8; 32], force: bool) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        create_restricted_dir(parent)?;
    }

    let mut opts = OpenOptions::new();
    opts.write(true);
    if force {
        opts.create(true).truncate(true);
    } else {
        opts.create_new(true);
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }

    let mut file = opts
        .open(path)
        .map_err(|e| format!("cannot create key file {}: {e}", path.display()))?;
    writeln!(file, "{}", encode_hex(seed))
        .map_err(|e| format!("failed to write key file {}: {e}", path.display()))?;
    file.sync_all()
        .map_err(|e| format!("failed to sync key file {}: {e}", path.display()))
}

fn create_restricted_dir(dir: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
            .map_err(|e| format!("cannot create directory {}: {e}", dir.display()))
    }
    #[cfg(not(unix))]
    {
        fs::create_dir_all(dir)
            .map_err(|e| format!("cannot create directory {}: {e}", dir.display()))
    }
}

fn resolve_signing_seed(
    key_file: Option<&Path>,
    key_hex: Option<&str>,
) -> Result<[u8; 32], String> {
    if let Some(hex) = key_hex {
        return parse_key_32(hex, "signing key");
    }
    if let Ok(env_hex) = std::env::var("QUIETREADER_SIGNING_KEY")
        && !env_hex.trim().is_empty()
    {
        return parse_key_32(&env_hex, "QUIETREADER_SIGNING_KEY");
    }
    let path = key_file.unwrap_or_else(|| Path::new(DEFAULT_KEY_PATH));
    let content = fs::read_to_string(path)
        .map_err(|e| format!("cannot read signing key file {}: {e}", path.display()))?;
    parse_key_32(&content, "signing key file")
}

pub(crate) fn parse_key_32(hex_str: &str, label: &str) -> Result<[u8; 32], String> {
    let bytes = decode_hex(hex_str).ok_or_else(|| format!("invalid hex in {label}"))?;
    let array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| format!("{label} must be exactly 32 bytes (64 hex chars)"))?;
    Ok(array)
}

pub(crate) fn format_rust_key_array(bytes: &[u8; 32]) -> String {
    let mut lines = Vec::with_capacity(4);
    lines.push("[".to_string());
    for chunk in bytes.chunks(16) {
        let items: Vec<String> = chunk.iter().map(|b| format!("0x{b:02x}")).collect();
        lines.push(format!("    {},", items.join(", ")));
    }
    lines.push("]".to_string());
    lines.join("\n")
}

fn current_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
