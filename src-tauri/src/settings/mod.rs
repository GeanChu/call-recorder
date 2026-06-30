//! Segredos no keychain do SO (crate `keyring`). Nunca em texto puro, nunca logado.
//!
//! As preferências não-secretas (idioma padrão, URL/modelo do provedor) ficam no
//! SQLite (ver `storage`).

use anyhow::{anyhow, Result};
use keyring::Entry;

const SERVICE: &str = "com.hicapital.callrecorder";
const KEY_USER: &str = "transcription_api_key";

fn entry() -> Result<Entry> {
    Entry::new(SERVICE, KEY_USER).map_err(|e| anyhow!("keychain: {e}"))
}

pub fn set_api_key(key: &str) -> Result<()> {
    entry()?.set_password(key).map_err(|e| anyhow!("keychain: {e}"))
}

pub fn get_api_key() -> Result<Option<String>> {
    match entry()?.get_password() {
        Ok(p) => Ok(Some(p)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(anyhow!("keychain: {e}")),
    }
}

pub fn has_api_key() -> bool {
    matches!(get_api_key(), Ok(Some(_)))
}
