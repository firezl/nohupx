use anyhow::{Context, Result};
use keyring_core::{set_default_store, Entry, Error as KeyringError};

const SERVICE: &str = "nohupx";
const INDEX_KEY: &str = "__nohupx_secret_index";

pub fn set(key: &str, value: &str) -> Result<()> {
    validate_key(key)?;
    entry(key)?
        .set_password(value)
        .with_context(|| format!("failed to store secret {key:?}"))?;
    add_to_index(key)?;
    Ok(())
}

pub fn get(key: &str) -> Result<String> {
    validate_key(key)?;
    entry(key)?
        .get_password()
        .with_context(|| format!("failed to read secret {key:?}"))
}

pub fn delete(key: &str) -> Result<()> {
    validate_key(key)?;
    match entry(key)?.delete_credential() {
        Ok(()) => {
            remove_from_index(key)?;
            Ok(())
        }
        Err(KeyringError::NoEntry) => Ok(()),
        Err(err) => Err(err).with_context(|| format!("failed to delete secret {key:?}")),
    }
}

pub fn list() -> Result<Vec<String>> {
    let mut keys = read_index()?;
    keys.sort();
    keys.dedup();
    Ok(keys)
}

fn entry(key: &str) -> Result<Entry> {
    init_store()?;
    Entry::new(SERVICE, key).with_context(|| format!("failed to create keyring entry {key:?}"))
}

fn init_store() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        use std::collections::HashMap;

        let store =
            zbus_secret_service_keyring_store::Store::new_with_configuration(&HashMap::new())
                .context("failed to initialize Linux Secret Service keyring store")?;
        set_default_store(store);
    }

    #[cfg(target_os = "macos")]
    {
        use std::collections::HashMap;

        let store =
            apple_native_keyring_store::keychain::Store::new_with_configuration(&HashMap::new())
                .context("failed to initialize macOS Keychain store")?;
        set_default_store(store);
    }

    #[cfg(target_os = "windows")]
    {
        use std::collections::HashMap;

        let store = windows_native_keyring_store::Store::new_with_configuration(&HashMap::new())
            .context("failed to initialize Windows Credential Manager store")?;
        set_default_store(store);
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        anyhow::bail!("system keyring is not supported on this platform");
    }

    Ok(())
}

fn validate_key(key: &str) -> Result<()> {
    anyhow::ensure!(!key.trim().is_empty(), "secret key must not be empty");
    anyhow::ensure!(
        !key.starts_with("__nohupx_"),
        "secret key prefix __nohupx_ is reserved"
    );
    Ok(())
}

fn add_to_index(key: &str) -> Result<()> {
    let mut keys = read_index()?;
    if !keys.iter().any(|existing| existing == key) {
        keys.push(key.to_string());
    }
    write_index(&keys)
}

fn remove_from_index(key: &str) -> Result<()> {
    let mut keys = read_index()?;
    keys.retain(|existing| existing != key);
    write_index(&keys)
}

fn read_index() -> Result<Vec<String>> {
    match Entry::new(SERVICE, INDEX_KEY)
        .context("failed to create keyring index entry")?
        .get_password()
    {
        Ok(value) => Ok(serde_json::from_str(&value).unwrap_or_default()),
        Err(KeyringError::NoEntry) => Ok(Vec::new()),
        Err(err) => Err(err).context("failed to read keyring index"),
    }
}

fn write_index(keys: &[String]) -> Result<()> {
    let value = serde_json::to_string(keys).context("failed to serialize keyring index")?;
    Entry::new(SERVICE, INDEX_KEY)
        .context("failed to create keyring index entry")?
        .set_password(&value)
        .context("failed to write keyring index")
}
