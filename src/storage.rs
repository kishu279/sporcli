use std::fs::{self, OpenOptions};
use std::io::Write;

use crate::spotify::auth_handler::StoredToken;
use std::os::unix::fs::OpenOptionsExt;

pub fn save_credentials(
    auth: &StoredToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let home = std::env::var("HOME")?;
    let dir_path = format!("{}/.config/sporc", home);
    let file_path = format!("{}/credentials.json", dir_path);

    // Create directory if not exists
    fs::create_dir_all(&dir_path)?;

    let json = serde_json::to_string_pretty(auth)?;

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(file_path)?;

    file.write_all(json.as_bytes())?;

    Ok(())
}

pub fn load_credentials() -> Result<StoredToken, Box<dyn std::error::Error + Send + Sync>> {
    let home = std::env::var("HOME")?;
    let file_path = format!("{}/.config/sporc/credentials.json", home);

    let contents = fs::read_to_string(&file_path).map_err(|e| {
        // tracing::warn!("[storage] Failed to read {}: {}", file_path, e);
        e
    })?;

    let cred: StoredToken = serde_json::from_str(&contents)?;

    Ok(cred)
}
