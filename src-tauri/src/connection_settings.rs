use crate::id::new_object_id;
use crate::local_store::LocalStore;
use crate::model::ConnectionProfile;
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine as _};
use ring::{
    aead,
    rand::{SecureRandom, SystemRandom},
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

const SOURCE_SETTING_KEY: &str = "last_source_connection";
const TARGET_DATABASE_SETTING_KEY: &str = "last_target_database_connection";
const TARGET_SYSTEM_SETTING_KEY: &str = "last_target_system_connection";
const DATABASE_CONNECTION_LIBRARY_KEY: &str = "database_connection_library_v1";
const CREDENTIAL_KEY_FILE: &str = "credentials.key";
const ENCRYPTED_VALUE_PREFIX: &str = "v1:";
const SOURCE_PASSWORD_ACCOUNT: &str = "last-source-database-password";
const TARGET_DATABASE_PASSWORD_ACCOUNT: &str = "last-target-database-password";
const TARGET_PASSWORD_ACCOUNT: &str = "last-target-system-password";
const DATABASE_CONNECTION_PASSWORD_PREFIX: &str = "database-connection-password";

pub struct LocalCredentialCipher {
    key: [u8; 32],
}

impl LocalCredentialCipher {
    pub fn open(app_data_dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(app_data_dir).map_err(|error| error.to_string())?;
        let key_path = app_data_dir.join(CREDENTIAL_KEY_FILE);
        let key = load_or_create_key(&key_path)?;
        Ok(Self { key })
    }

    fn encrypt(&self, account: &str, secret: &str) -> Result<String, String> {
        let unbound = aead::UnboundKey::new(&aead::AES_256_GCM, &self.key)
            .map_err(|_| "初始化本地凭据加密失败".to_string())?;
        let key = aead::LessSafeKey::new(unbound);
        let mut nonce_bytes = [0_u8; 12];
        SystemRandom::new()
            .fill(&mut nonce_bytes)
            .map_err(|_| "生成凭据随机数失败".to_string())?;
        let mut encrypted = secret.as_bytes().to_vec();
        key.seal_in_place_append_tag(
            aead::Nonce::assume_unique_for_key(nonce_bytes),
            aead::Aad::from(account.as_bytes()),
            &mut encrypted,
        )
        .map_err(|_| "加密本地凭据失败".to_string())?;
        let mut payload = nonce_bytes.to_vec();
        payload.extend_from_slice(&encrypted);
        Ok(format!(
            "{ENCRYPTED_VALUE_PREFIX}{}",
            STANDARD_NO_PAD.encode(payload)
        ))
    }

    fn decrypt(&self, account: &str, encrypted: &str) -> Result<String, String> {
        let encoded = encrypted
            .strip_prefix(ENCRYPTED_VALUE_PREFIX)
            .ok_or_else(|| "本地凭据版本无法识别".to_string())?;
        let payload = STANDARD_NO_PAD
            .decode(encoded)
            .map_err(|_| "本地凭据密文格式损坏".to_string())?;
        if payload.len() <= 12 {
            return Err("本地凭据密文长度无效".into());
        }
        let mut nonce_bytes = [0_u8; 12];
        nonce_bytes.copy_from_slice(&payload[..12]);
        let mut encrypted = payload[12..].to_vec();
        let unbound = aead::UnboundKey::new(&aead::AES_256_GCM, &self.key)
            .map_err(|_| "初始化本地凭据解密失败".to_string())?;
        let key = aead::LessSafeKey::new(unbound);
        let plain = key
            .open_in_place(
                aead::Nonce::assume_unique_for_key(nonce_bytes),
                aead::Aad::from(account.as_bytes()),
                &mut encrypted,
            )
            .map_err(|_| "本地凭据校验失败，可能已损坏或来自其他安装".to_string())?;
        String::from_utf8(plain.to_vec()).map_err(|_| "本地凭据不是有效文本".to_string())
    }
}

fn load_or_create_key(path: &Path) -> Result<[u8; 32], String> {
    if path.exists() {
        return read_key(path);
    }
    let mut key = [0_u8; 32];
    SystemRandom::new()
        .fill(&mut key)
        .map_err(|_| "生成本地凭据主密钥失败".to_string())?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|error| format!("创建本地凭据主密钥失败：{error}"))?;
    file.write_all(&key)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("保存本地凭据主密钥失败：{error}"))?;
    restrict_key_permissions(path)?;
    Ok(key)
}

fn read_key(path: &Path) -> Result<[u8; 32], String> {
    let bytes = fs::read(path).map_err(|error| format!("读取本地凭据主密钥失败：{error}"))?;
    let key: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "本地凭据主密钥长度无效".to_string())?;
    restrict_key_permissions(path)?;
    Ok(key)
}

#[cfg(unix)]
fn restrict_key_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|error| format!("限制本地凭据主密钥权限失败：{error}"))
}

#[cfg(not(unix))]
fn restrict_key_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSourceConnectionRequest {
    pub profile: ConnectionProfile,
    #[serde(default)]
    pub remember_password: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveTargetSystemConnectionRequest {
    pub base_url: String,
    pub tenant_id: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub remember_password: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveDatabaseConnectionRequest {
    #[serde(default)]
    pub connection_id: String,
    pub name: String,
    pub purpose: String,
    pub profile: ConnectionProfile,
    #[serde(default)]
    pub remember_password: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSourceConnection {
    pub profile: ConnectionProfile,
    pub remember_password: bool,
    pub password_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedTargetSystemConnection {
    pub base_url: String,
    pub tenant_id: String,
    pub password: String,
    pub remember_password: bool,
    pub password_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedDatabaseConnection {
    pub connection_id: String,
    pub name: String,
    pub purpose: String,
    pub profile: ConnectionProfile,
    pub remember_password: bool,
    pub password_available: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedConnections {
    pub source: Option<SavedSourceConnection>,
    pub target_database: Option<SavedSourceConnection>,
    pub target_system: Option<SavedTargetSystemConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredSourceConnection {
    profile: ConnectionProfile,
    remember_password: bool,
    #[serde(default)]
    encrypted_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredTargetSystemConnection {
    base_url: String,
    tenant_id: String,
    remember_password: bool,
    #[serde(default)]
    encrypted_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredDatabaseConnection {
    connection_id: String,
    name: String,
    purpose: String,
    profile: ConnectionProfile,
    remember_password: bool,
    #[serde(default)]
    encrypted_password: String,
    updated_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredDatabaseConnectionLibrary {
    #[serde(default = "database_connection_library_version")]
    version: u8,
    #[serde(default)]
    entries: Vec<StoredDatabaseConnection>,
}

fn database_connection_library_version() -> u8 {
    1
}

pub fn load(
    store: &LocalStore,
    cipher: &LocalCredentialCipher,
) -> Result<SavedConnections, String> {
    let source =
        load_database_connection(store, cipher, SOURCE_SETTING_KEY, SOURCE_PASSWORD_ACCOUNT)?;
    let target_database = load_database_connection(
        store,
        cipher,
        TARGET_DATABASE_SETTING_KEY,
        TARGET_DATABASE_PASSWORD_ACCOUNT,
    )?;
    let target_system = store
        .load_setting(TARGET_SYSTEM_SETTING_KEY)?
        .map(serde_json::from_value::<StoredTargetSystemConnection>)
        .transpose()
        .map_err(|error| format!("读取最近新系统连接失败：{error}"))?
        .map(|stored| {
            let password = if stored.remember_password {
                cipher
                    .decrypt(TARGET_PASSWORD_ACCOUNT, &stored.encrypted_password)
                    .unwrap_or_default()
            } else {
                String::new()
            };
            let password_available = !password.is_empty();
            SavedTargetSystemConnection {
                base_url: stored.base_url,
                tenant_id: stored.tenant_id,
                password,
                remember_password: stored.remember_password,
                password_available,
            }
        });
    Ok(SavedConnections {
        source,
        target_database,
        target_system,
    })
}

fn load_database_connection(
    store: &LocalStore,
    cipher: &LocalCredentialCipher,
    setting_key: &str,
    password_account: &str,
) -> Result<Option<SavedSourceConnection>, String> {
    store
        .load_setting(setting_key)?
        .map(serde_json::from_value::<StoredSourceConnection>)
        .transpose()
        .map_err(|error| format!("读取最近数据库连接失败：{error}"))
        .map(|stored| {
            stored.map(|stored| {
                let mut profile = stored.profile;
                let password = if stored.remember_password {
                    cipher
                        .decrypt(password_account, &stored.encrypted_password)
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                let password_available = !password.is_empty();
                profile.password = password;
                SavedSourceConnection {
                    profile,
                    remember_password: stored.remember_password,
                    password_available,
                }
            })
        })
}

pub fn list_database_connections(
    store: &LocalStore,
    cipher: &LocalCredentialCipher,
) -> Result<Vec<SavedDatabaseConnection>, String> {
    let mut library = load_database_connection_library(store)?;
    let mut changed = false;
    for (purpose, setting_key, password_account, label) in [
        (
            "SOURCE",
            SOURCE_SETTING_KEY,
            SOURCE_PASSWORD_ACCOUNT,
            "老系统数据库",
        ),
        (
            "TARGET",
            TARGET_DATABASE_SETTING_KEY,
            TARGET_DATABASE_PASSWORD_ACCOUNT,
            "新系统目标数据库",
        ),
    ] {
        let Some(saved) = load_database_connection(store, cipher, setting_key, password_account)?
        else {
            continue;
        };
        if library.entries.iter().any(|entry| {
            entry.purpose == purpose && same_database_profile(&entry.profile, &saved.profile)
        }) {
            continue;
        }
        let connection_id = new_object_id();
        let mut profile = saved.profile;
        let password = std::mem::take(&mut profile.password);
        let remember_password = saved.remember_password && !password.is_empty();
        let encrypted_password = if remember_password {
            cipher.encrypt(
                &database_connection_password_account(&connection_id),
                &password,
            )?
        } else {
            String::new()
        };
        library.entries.push(StoredDatabaseConnection {
            connection_id,
            name: default_connection_name(label, &profile),
            purpose: purpose.to_string(),
            profile,
            remember_password,
            encrypted_password,
            updated_at: chrono::Utc::now().to_rfc3339(),
        });
        changed = true;
    }
    if changed {
        save_database_connection_library(store, &library)?;
    }
    let mut entries = library
        .entries
        .into_iter()
        .map(|stored| decrypt_database_connection(cipher, stored))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Ok(entries)
}

pub fn save_database_connection(
    store: &LocalStore,
    cipher: &LocalCredentialCipher,
    request: SaveDatabaseConnectionRequest,
) -> Result<SavedDatabaseConnection, String> {
    let name = request.name.trim();
    if name.is_empty() {
        return Err("请填写连接名称，便于后续识别和复用".into());
    }
    let purpose = normalize_database_purpose(&request.purpose)?;
    validate_saved_database_profile(&request.profile)?;
    let mut library = load_database_connection_library(store)?;
    let connection_id = if request.connection_id.trim().is_empty() {
        new_object_id()
    } else {
        request.connection_id.trim().to_string()
    };
    let existing = library
        .entries
        .iter()
        .find(|entry| entry.connection_id == connection_id)
        .cloned();
    let mut profile = request.profile;
    let password = std::mem::take(&mut profile.password);
    profile.connection_string = sanitized_connection_string(&profile.connection_string, &password);
    let (remember_password, encrypted_password) = if request.remember_password {
        if !password.is_empty() {
            (
                true,
                cipher.encrypt(
                    &database_connection_password_account(&connection_id),
                    &password,
                )?,
            )
        } else if let Some(existing) = existing.as_ref().filter(|item| item.remember_password) {
            (true, existing.encrypted_password.clone())
        } else {
            (false, String::new())
        }
    } else {
        (false, String::new())
    };
    let stored = StoredDatabaseConnection {
        connection_id: connection_id.clone(),
        name: name.to_string(),
        purpose,
        profile,
        remember_password,
        encrypted_password,
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    if let Some(index) = library
        .entries
        .iter()
        .position(|entry| entry.connection_id == connection_id)
    {
        library.entries[index] = stored.clone();
    } else {
        library.entries.push(stored.clone());
    }
    save_database_connection_library(store, &library)?;
    decrypt_database_connection(cipher, stored)
}

pub fn delete_database_connection(store: &LocalStore, connection_id: &str) -> Result<(), String> {
    let mut library = load_database_connection_library(store)?;
    let before = library.entries.len();
    library
        .entries
        .retain(|entry| entry.connection_id != connection_id.trim());
    if library.entries.len() == before {
        return Err("没有找到要删除的数据库连接".into());
    }
    save_database_connection_library(store, &library)
}

fn load_database_connection_library(
    store: &LocalStore,
) -> Result<StoredDatabaseConnectionLibrary, String> {
    store
        .load_setting(DATABASE_CONNECTION_LIBRARY_KEY)?
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| format!("读取数据库连接库失败：{error}"))
        .map(|library| {
            library.unwrap_or(StoredDatabaseConnectionLibrary {
                version: database_connection_library_version(),
                entries: Vec::new(),
            })
        })
}

fn save_database_connection_library(
    store: &LocalStore,
    library: &StoredDatabaseConnectionLibrary,
) -> Result<(), String> {
    store.save_setting(
        DATABASE_CONNECTION_LIBRARY_KEY,
        &serde_json::to_value(library).map_err(|error| error.to_string())?,
    )
}

fn decrypt_database_connection(
    cipher: &LocalCredentialCipher,
    stored: StoredDatabaseConnection,
) -> Result<SavedDatabaseConnection, String> {
    let mut profile = stored.profile;
    let password = if stored.remember_password && !stored.encrypted_password.is_empty() {
        cipher
            .decrypt(
                &database_connection_password_account(&stored.connection_id),
                &stored.encrypted_password,
            )
            .unwrap_or_default()
    } else {
        String::new()
    };
    let password_available = !password.is_empty();
    profile.password = password;
    Ok(SavedDatabaseConnection {
        connection_id: stored.connection_id,
        name: stored.name,
        purpose: stored.purpose,
        profile,
        remember_password: stored.remember_password,
        password_available,
        updated_at: stored.updated_at,
    })
}

fn database_connection_password_account(connection_id: &str) -> String {
    format!("{DATABASE_CONNECTION_PASSWORD_PREFIX}:{connection_id}")
}

fn normalize_database_purpose(purpose: &str) -> Result<String, String> {
    match purpose.trim().to_ascii_uppercase().as_str() {
        "SOURCE" => Ok("SOURCE".into()),
        "TARGET" => Ok("TARGET".into()),
        "BOTH" => Ok("BOTH".into()),
        _ => Err("连接用途必须是老系统读取、新系统写入或两者通用".into()),
    }
}

fn validate_saved_database_profile(profile: &ConnectionProfile) -> Result<(), String> {
    if profile.host.trim().is_empty() {
        return Err("请填写数据库主机地址".into());
    }
    if profile.port == 0 {
        return Err("请填写有效的数据库端口".into());
    }
    if profile.database.trim().is_empty() && profile.service_name.trim().is_empty() {
        return Err("请填写数据库名或 Service Name".into());
    }
    if profile.username.trim().is_empty() {
        return Err("请填写数据库账号".into());
    }
    Ok(())
}

fn same_database_profile(left: &ConnectionProfile, right: &ConnectionProfile) -> bool {
    left.kind.eq_ignore_ascii_case(&right.kind)
        && left.host.eq_ignore_ascii_case(&right.host)
        && left.port == right.port
        && left.database.eq_ignore_ascii_case(&right.database)
        && left.service_name.eq_ignore_ascii_case(&right.service_name)
        && left.schema.eq_ignore_ascii_case(&right.schema)
        && left.username.eq_ignore_ascii_case(&right.username)
}

fn default_connection_name(prefix: &str, profile: &ConnectionProfile) -> String {
    let database = if profile.database.trim().is_empty() {
        profile.service_name.trim()
    } else {
        profile.database.trim()
    };
    format!("{prefix} · {database}@{}", profile.host.trim())
}

pub fn save_source(
    store: &LocalStore,
    cipher: &LocalCredentialCipher,
    request: SaveSourceConnectionRequest,
) -> Result<SavedSourceConnection, String> {
    save_last_database_connection(
        store,
        cipher,
        SOURCE_SETTING_KEY,
        SOURCE_PASSWORD_ACCOUNT,
        request,
    )
}

pub fn save_target_database(
    store: &LocalStore,
    cipher: &LocalCredentialCipher,
    request: SaveSourceConnectionRequest,
) -> Result<SavedSourceConnection, String> {
    save_last_database_connection(
        store,
        cipher,
        TARGET_DATABASE_SETTING_KEY,
        TARGET_DATABASE_PASSWORD_ACCOUNT,
        request,
    )
}

fn save_last_database_connection(
    store: &LocalStore,
    cipher: &LocalCredentialCipher,
    setting_key: &str,
    password_account: &str,
    request: SaveSourceConnectionRequest,
) -> Result<SavedSourceConnection, String> {
    let mut profile = request.profile;
    let password = std::mem::take(&mut profile.password);
    profile.connection_string = sanitized_connection_string(&profile.connection_string, &password);
    let encrypted_password = if request.remember_password && !password.is_empty() {
        cipher.encrypt(password_account, &password)?
    } else {
        String::new()
    };
    let stored = StoredSourceConnection {
        profile: profile.clone(),
        remember_password: request.remember_password && !password.is_empty(),
        encrypted_password,
    };
    store.save_setting(
        setting_key,
        &serde_json::to_value(&stored).map_err(|error| error.to_string())?,
    )?;
    profile.password = password;
    Ok(SavedSourceConnection {
        password_available: !profile.password.is_empty() && stored.remember_password,
        profile,
        remember_password: stored.remember_password,
    })
}

pub fn save_target_system(
    store: &LocalStore,
    cipher: &LocalCredentialCipher,
    request: SaveTargetSystemConnectionRequest,
) -> Result<SavedTargetSystemConnection, String> {
    let base_url = request.base_url.trim().trim_end_matches('/').to_string();
    let tenant_id = request.tenant_id.trim().to_string();
    let encrypted_password = if request.remember_password && !request.password.is_empty() {
        cipher.encrypt(TARGET_PASSWORD_ACCOUNT, &request.password)?
    } else {
        String::new()
    };
    let stored = StoredTargetSystemConnection {
        base_url: base_url.clone(),
        tenant_id: tenant_id.clone(),
        remember_password: request.remember_password && !request.password.is_empty(),
        encrypted_password,
    };
    store.save_setting(
        TARGET_SYSTEM_SETTING_KEY,
        &serde_json::to_value(&stored).map_err(|error| error.to_string())?,
    )?;
    Ok(SavedTargetSystemConnection {
        base_url,
        tenant_id,
        password_available: stored.remember_password,
        password: request.password,
        remember_password: stored.remember_password,
    })
}

pub fn forget_source(store: &LocalStore) -> Result<(), String> {
    store.delete_setting(SOURCE_SETTING_KEY)
}

pub fn forget_target_database(store: &LocalStore) -> Result<(), String> {
    store.delete_setting(TARGET_DATABASE_SETTING_KEY)
}

pub fn forget_target_system(store: &LocalStore) -> Result<(), String> {
    store.delete_setting(TARGET_SYSTEM_SETTING_KEY)
}

fn sanitized_connection_string(template: &str, _password: &str) -> String {
    let sanitized = template.trim().to_string();
    let lower = sanitized.to_ascii_lowercase();
    if (lower.contains("pwd=") || lower.contains("password=")) && !sanitized.contains("${PASSWORD}")
    {
        String::new()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::{
        delete_database_connection, list_database_connections, load, sanitized_connection_string,
        save_database_connection, save_source, LocalCredentialCipher,
        SaveDatabaseConnectionRequest, SaveSourceConnectionRequest,
        DATABASE_CONNECTION_LIBRARY_KEY, SOURCE_SETTING_KEY,
    };
    use crate::{local_store::LocalStore, model::ConnectionProfile};

    #[test]
    fn connection_string_never_persists_a_literal_password() {
        assert_eq!(
            sanitized_connection_string("UID=PHIS27;PWD=PHIS27", "PHIS27"),
            ""
        );
        assert_eq!(
            sanitized_connection_string("UID=user;PWD=other-secret", "known-secret"),
            ""
        );
    }

    #[test]
    fn local_credentials_are_encrypted_authenticated_and_install_specific() {
        let first_dir = tempfile::tempdir().unwrap();
        let second_dir = tempfile::tempdir().unwrap();
        let first = LocalCredentialCipher::open(first_dir.path()).unwrap();
        let second = LocalCredentialCipher::open(second_dir.path()).unwrap();
        let encrypted = first.encrypt("source", "PHIS27").unwrap();
        assert!(encrypted.starts_with("v1:"));
        assert!(!encrypted.contains("PHIS27"));
        assert_eq!(first.decrypt("source", &encrypted).unwrap(), "PHIS27");
        assert!(first.decrypt("target", &encrypted).is_err());
        assert!(second.decrypt("source", &encrypted).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn local_master_key_is_owner_only_on_unix() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        LocalCredentialCipher::open(dir.path()).unwrap();
        let mode = std::fs::metadata(dir.path().join("credentials.key"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn saved_connection_round_trip_never_writes_plaintext_password() {
        let dir = tempfile::tempdir().unwrap();
        let store = LocalStore::open(&dir.path().join("settings.sqlite")).unwrap();
        let cipher = LocalCredentialCipher::open(dir.path()).unwrap();
        save_source(
            &store,
            &cipher,
            SaveSourceConnectionRequest {
                profile: ConnectionProfile {
                    kind: "oracle".into(),
                    host: "127.0.0.1".into(),
                    port: 1521,
                    database: "phis".into(),
                    username: "readonly".into(),
                    password: "never-plaintext".into(),
                    schema: "PHIS27".into(),
                    service_name: "phis".into(),
                    driver: "Oracle 19 ODBC driver".into(),
                    connection_string: String::new(),
                },
                remember_password: true,
            },
        )
        .unwrap();
        let stored = store.load_setting(SOURCE_SETTING_KEY).unwrap().unwrap();
        let json = stored.to_string();
        assert!(!json.contains("never-plaintext"));
        assert!(json.contains("v1:"));
        let loaded = load(&store, &cipher).unwrap().source.unwrap();
        assert_eq!(loaded.profile.password, "never-plaintext");
        assert!(loaded.password_available);
    }

    #[test]
    fn named_database_connections_are_encrypted_reusable_and_deletable() {
        let dir = tempfile::tempdir().unwrap();
        let store = LocalStore::open(&dir.path().join("settings.sqlite")).unwrap();
        let cipher = LocalCredentialCipher::open(dir.path()).unwrap();
        let saved = save_database_connection(
            &store,
            &cipher,
            SaveDatabaseConnectionRequest {
                connection_id: String::new(),
                name: "二系列phis测试库".into(),
                purpose: "BOTH".into(),
                profile: ConnectionProfile {
                    kind: "oracle".into(),
                    host: "10.19.40.42".into(),
                    port: 1521,
                    database: "phis".into(),
                    username: "PHIS27".into(),
                    password: "library-secret".into(),
                    schema: "PHIS27".into(),
                    service_name: "phis".into(),
                    driver: "Oracle 19 ODBC driver".into(),
                    connection_string: String::new(),
                },
                remember_password: true,
            },
        )
        .unwrap();
        assert_eq!(saved.purpose, "BOTH");
        assert_eq!(saved.profile.password, "library-secret");

        let stored = store
            .load_setting(DATABASE_CONNECTION_LIBRARY_KEY)
            .unwrap()
            .unwrap()
            .to_string();
        assert!(!stored.contains("library-secret"));
        assert!(stored.contains("v1:"));

        let updated = save_database_connection(
            &store,
            &cipher,
            SaveDatabaseConnectionRequest {
                connection_id: saved.connection_id.clone(),
                name: "二系列phis正式连接".into(),
                purpose: "SOURCE".into(),
                profile: ConnectionProfile {
                    password: String::new(),
                    ..saved.profile.clone()
                },
                remember_password: true,
            },
        )
        .unwrap();
        assert_eq!(updated.profile.password, "library-secret");
        assert_eq!(updated.name, "二系列phis正式连接");
        assert_eq!(list_database_connections(&store, &cipher).unwrap().len(), 1);

        delete_database_connection(&store, &saved.connection_id).unwrap();
        assert!(list_database_connections(&store, &cipher)
            .unwrap()
            .is_empty());
    }
}
