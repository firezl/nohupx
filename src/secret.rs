use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, CHACHA20_POLY1305};
use serde::{Deserialize, Serialize};

const SECRET_KEY_FILE: &str = ".secret_key";
const SECRETS_FILE: &str = "secrets.json";
const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;

pub fn set(key: &str, value: &str) -> Result<()> {
    default_store()?.set(key, value)
}

pub fn get(key: &str) -> Result<String> {
    default_store()?.get(key)
}

pub fn delete(key: &str) -> Result<()> {
    default_store()?.delete(key)
}

pub fn list() -> Result<Vec<String>> {
    default_store()?.list()
}

fn default_store() -> Result<FileSecrets> {
    Ok(FileSecrets::new(default_secret_dir()?))
}

fn default_secret_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("failed to resolve home directory")?;
    Ok(home.join(".config").join("nohupx"))
}

struct FileSecrets {
    dir: PathBuf,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct SecretFile {
    version: u8,
    #[serde(default)]
    secrets: HashMap<String, SecretEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SecretEntry {
    nonce: String,
    ciphertext: String,
}

impl FileSecrets {
    fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn set(&self, key: &str, value: &str) -> Result<()> {
        validate_key(key)?;
        let master_key = self.read_or_create_master_key()?;
        let mut file = self.read_secret_file()?;
        file.version = 1;
        file.secrets
            .insert(key.to_string(), encrypt_secret(&master_key, key, value)?);
        self.write_secret_file(&file)
            .with_context(|| format!("failed to store secret {key:?}"))
    }

    fn get(&self, key: &str) -> Result<String> {
        validate_key(key)?;
        let master_key = self.read_master_key()?;
        let file = self.read_secret_file()?;
        let entry = file
            .secrets
            .get(key)
            .with_context(|| format!("secret {key:?} does not exist"))?;
        decrypt_secret(&master_key, key, entry)
            .with_context(|| format!("failed to read secret {key:?}"))
    }

    fn delete(&self, key: &str) -> Result<()> {
        validate_key(key)?;
        let mut file = self.read_secret_file()?;
        if file.secrets.remove(key).is_some() {
            self.write_secret_file(&file)
                .with_context(|| format!("failed to delete secret {key:?}"))?;
        }
        Ok(())
    }

    fn list(&self) -> Result<Vec<String>> {
        let mut keys: Vec<String> = self.read_secret_file()?.secrets.into_keys().collect();
        keys.sort();
        Ok(keys)
    }

    fn key_path(&self) -> PathBuf {
        self.dir.join(SECRET_KEY_FILE)
    }

    fn secrets_path(&self) -> PathBuf {
        self.dir.join(SECRETS_FILE)
    }

    fn ensure_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.dir)
            .with_context(|| format!("failed to create secret directory {}", self.dir.display()))?;
        set_private_dir_permissions(&self.dir)?;
        Ok(())
    }

    fn read_or_create_master_key(&self) -> Result<[u8; KEY_LEN]> {
        let path = self.key_path();
        if path.exists() {
            return self.read_master_key();
        }

        self.ensure_dir()?;
        let mut key = [0_u8; KEY_LEN];
        getrandom::fill(&mut key)
            .map_err(|err| anyhow!("failed to generate secret encryption key: {err:?}"))?;
        write_private_file(&path, &STANDARD.encode(key))?;
        Ok(key)
    }

    fn read_master_key(&self) -> Result<[u8; KEY_LEN]> {
        let path = self.key_path();
        let encoded = fs::read_to_string(&path)
            .with_context(|| format!("failed to read secret key {}", path.display()))?;
        let bytes = STANDARD
            .decode(encoded.trim())
            .with_context(|| format!("failed to decode secret key {}", path.display()))?;
        bytes.try_into().map_err(|_| {
            anyhow!(
                "secret key {} must decode to {KEY_LEN} bytes",
                path.display()
            )
        })
    }

    fn read_secret_file(&self) -> Result<SecretFile> {
        let path = self.secrets_path();
        if !path.exists() {
            return Ok(SecretFile {
                version: 1,
                secrets: HashMap::new(),
            });
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read secrets file {}", path.display()))?;
        let file: SecretFile = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse secrets file {}", path.display()))?;
        anyhow::ensure!(
            file.version == 1,
            "unsupported secrets file version {}",
            file.version
        );
        Ok(file)
    }

    fn write_secret_file(&self, file: &SecretFile) -> Result<()> {
        self.ensure_dir()?;
        let content =
            serde_json::to_string_pretty(file).context("failed to serialize secrets file")?;
        write_private_file(&self.secrets_path(), &content)
    }
}

fn validate_key(key: &str) -> Result<()> {
    anyhow::ensure!(!key.trim().is_empty(), "secret key must not be empty");
    anyhow::ensure!(
        !key.starts_with("__nohupx_"),
        "secret key prefix __nohupx_ is reserved"
    );
    Ok(())
}

fn encrypt_secret(master_key: &[u8; KEY_LEN], key: &str, value: &str) -> Result<SecretEntry> {
    let cipher = cipher_key(master_key)?;
    let mut nonce = [0_u8; NONCE_LEN];
    getrandom::fill(&mut nonce)
        .map_err(|err| anyhow!("failed to generate secret nonce: {err:?}"))?;
    let mut ciphertext = value.as_bytes().to_vec();
    cipher
        .seal_in_place_append_tag(
            Nonce::assume_unique_for_key(nonce),
            Aad::from(key.as_bytes()),
            &mut ciphertext,
        )
        .map_err(|_| anyhow!("failed to encrypt secret"))?;

    Ok(SecretEntry {
        nonce: STANDARD.encode(nonce),
        ciphertext: STANDARD.encode(ciphertext),
    })
}

fn decrypt_secret(master_key: &[u8; KEY_LEN], key: &str, entry: &SecretEntry) -> Result<String> {
    let nonce = STANDARD
        .decode(&entry.nonce)
        .context("invalid secret nonce")?;
    let nonce: [u8; NONCE_LEN] = nonce
        .try_into()
        .map_err(|_| anyhow!("invalid secret nonce length"))?;
    let mut ciphertext = STANDARD
        .decode(&entry.ciphertext)
        .context("invalid secret ciphertext")?;
    let cipher = cipher_key(master_key)?;
    let plaintext = cipher
        .open_in_place(
            Nonce::assume_unique_for_key(nonce),
            Aad::from(key.as_bytes()),
            &mut ciphertext,
        )
        .map_err(|_| anyhow!("failed to decrypt secret"))?;
    String::from_utf8(plaintext.to_vec()).context("secret is not valid UTF-8")
}

fn cipher_key(master_key: &[u8; KEY_LEN]) -> Result<LessSafeKey> {
    let unbound = UnboundKey::new(&CHACHA20_POLY1305, master_key)
        .map_err(|_| anyhow!("failed to initialize secret cipher"))?;
    Ok(LessSafeKey::new(unbound))
}

fn write_private_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
        set_private_dir_permissions(parent)?;
    }

    let tmp_path = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&tmp_path, content)
        .with_context(|| format!("failed to write temporary file {}", tmp_path.display()))?;
    set_private_file_permissions(&tmp_path)?;
    replace_file(&tmp_path, path)
        .with_context(|| format!("failed to replace file {}", path.display()))?;
    set_private_file_permissions(path)?;
    Ok(())
}

#[cfg(windows)]
fn replace_file(from: &Path, to: &Path) -> Result<()> {
    if to.exists() {
        fs::remove_file(to).with_context(|| format!("failed to remove {}", to.display()))?;
    }
    fs::rename(from, to)
        .with_context(|| format!("failed to rename {} to {}", from.display(), to.display()))
}

#[cfg(not(windows))]
fn replace_file(from: &Path, to: &Path) -> Result<()> {
    fs::rename(from, to)
        .with_context(|| format!("failed to rename {} to {}", from.display(), to.display()))
}

#[cfg(unix)]
fn set_private_dir_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .with_context(|| format!("failed to set permissions on {}", path.display()))
}

#[cfg(not(unix))]
fn set_private_dir_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
fn set_private_file_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .with_context(|| format!("failed to set permissions on {}", path.display()))
}

#[cfg(not(unix))]
fn set_private_file_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_reads_lists_and_deletes_secret() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileSecrets::new(tmp.path().to_path_buf());

        store.set("email/password", "smtp-secret").unwrap();
        assert_eq!(store.get("email/password").unwrap(), "smtp-secret");
        assert_eq!(store.list().unwrap(), vec!["email/password".to_string()]);

        store.delete("email/password").unwrap();
        assert!(store.list().unwrap().is_empty());
        assert!(store.get("email/password").is_err());
    }

    #[test]
    fn binds_ciphertext_to_secret_key_name() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileSecrets::new(tmp.path().to_path_buf());

        store.set("email/password", "smtp-secret").unwrap();
        let mut file = store.read_secret_file().unwrap();
        let entry = file.secrets.remove("email/password").unwrap();
        file.secrets.insert("other/password".to_string(), entry);
        store.write_secret_file(&file).unwrap();

        assert!(store.get("other/password").is_err());
    }
}
