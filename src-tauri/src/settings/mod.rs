//! Chaves de API por SO.
//!
//! - Windows/Linux: keychain do SO (crate `keyring`) — sem fricção.
//! - macOS: arquivo protegido (0600) na pasta de dados do app. Apps não
//!   assinados/notarizados sofrem prompts repetidos do chaveiro "login" no
//!   macOS; o arquivo evita isso. As chaves ficam só na pasta local do usuário.
//!
//! Preferências não-secretas (idioma, endpoints, etc.) ficam no SQLite.

use anyhow::Result;

const SERVICE: &str = "com.hicapital.hicorder";
const TRANSCRIPTION_KEY: &str = "transcription_api_key";
const SUMMARY_KEY: &str = "summary_api_key";
const ATTIO_KEY: &str = "attio_api_key";

// ---- macOS: arquivo protegido (sem keychain) ----
#[cfg(target_os = "macos")]
mod store {
    use super::SERVICE;
    use anyhow::{anyhow, Result};
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn secrets_path() -> Result<PathBuf> {
        let home = std::env::var_os("HOME").ok_or_else(|| anyhow!("HOME não definido"))?;
        Ok(PathBuf::from(home)
            .join("Library/Application Support")
            .join(SERVICE)
            .join("secrets.json"))
    }

    fn read_all() -> BTreeMap<String, String> {
        secrets_path()
            .ok()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn set(user: &str, key: &str) -> Result<()> {
        let path = secrets_path()?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut map = read_all();
        map.insert(user.to_string(), key.to_string());
        let json = serde_json::to_string(&map)?;
        std::fs::write(&path, json)?;
        // Apenas o dono lê/escreve.
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        Ok(())
    }

    pub fn get(user: &str) -> Result<Option<String>> {
        Ok(read_all().get(user).cloned())
    }
}

// ---- Windows/Linux: keychain do SO ----
#[cfg(not(target_os = "macos"))]
mod store {
    use super::SERVICE;
    use anyhow::{anyhow, Result};
    use keyring::Entry;

    const OLD_SERVICE: &str = "com.hicapital.callrecorder";

    fn entry(user: &str) -> Result<Entry> {
        Entry::new(SERVICE, user).map_err(|e| anyhow!("keychain: {e}"))
    }

    pub fn set(user: &str, key: &str) -> Result<()> {
        entry(user)?
            .set_password(key)
            .map_err(|e| anyhow!("keychain: {e}"))
    }

    pub fn get(user: &str) -> Result<Option<String>> {
        match entry(user)?.get_password() {
            Ok(p) => Ok(Some(p)),
            Err(keyring::Error::NoEntry) => migrate_old(user),
            Err(e) => Err(anyhow!("keychain: {e}")),
        }
    }

    /// Migração preguiçosa do serviço antigo (Call Recorder).
    fn migrate_old(user: &str) -> Result<Option<String>> {
        let old = Entry::new(OLD_SERVICE, user).map_err(|e| anyhow!("keychain: {e}"))?;
        match old.get_password() {
            Ok(p) => {
                let _ = set(user, &p);
                Ok(Some(p))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(anyhow!("keychain: {e}")),
        }
    }
}

fn set_key(user: &str, key: &str) -> Result<()> {
    store::set(user, key)
}
fn get_key(user: &str) -> Result<Option<String>> {
    store::get(user)
}

// Transcrição (Groq/Whisper).
pub fn set_api_key(key: &str) -> Result<()> {
    set_key(TRANSCRIPTION_KEY, key)
}
pub fn get_api_key() -> Result<Option<String>> {
    get_key(TRANSCRIPTION_KEY)
}
pub fn has_api_key() -> bool {
    matches!(get_api_key(), Ok(Some(_)))
}

// Resumo (MiniMax-M3, sk-cp).
pub fn set_summary_key(key: &str) -> Result<()> {
    set_key(SUMMARY_KEY, key)
}
pub fn get_summary_key() -> Result<Option<String>> {
    get_key(SUMMARY_KEY)
}
pub fn has_summary_key() -> bool {
    matches!(get_summary_key(), Ok(Some(_)))
}

// Attio (CRM).
pub fn set_attio_key(key: &str) -> Result<()> {
    set_key(ATTIO_KEY, key)
}
pub fn get_attio_key() -> Result<Option<String>> {
    get_key(ATTIO_KEY)
}
pub fn has_attio_key() -> bool {
    matches!(get_attio_key(), Ok(Some(_)))
}
