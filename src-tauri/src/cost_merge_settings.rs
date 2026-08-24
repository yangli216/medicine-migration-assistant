use crate::local_store::LocalStore;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

const COST_MERGE_MAPPING_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostMergeMappingProfileScope {
    pub base_url: String,
    pub tenant_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveCostMergeMappingProfileRequest {
    pub base_url: String,
    pub tenant_id: String,
    #[serde(default)]
    pub mappings: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostMergeMappingProfile {
    pub version: u32,
    pub base_url: String,
    pub tenant_id: String,
    pub mappings: HashMap<String, String>,
    pub saved_at: String,
}

pub fn load(
    store: &LocalStore,
    scope: CostMergeMappingProfileScope,
) -> Result<Option<CostMergeMappingProfile>, String> {
    let (base_url, tenant_id) = normalize_scope(&scope.base_url, &scope.tenant_id)?;
    let Some(value) = store.load_setting(&setting_key(&base_url, &tenant_id))? else {
        return Ok(None);
    };
    let profile: CostMergeMappingProfile =
        serde_json::from_value(value).map_err(|error| format!("读取费用归并映射失败：{error}"))?;
    if profile.version != COST_MERGE_MAPPING_VERSION
        || profile.base_url != base_url
        || profile.tenant_id != tenant_id
    {
        return Ok(None);
    }
    Ok(Some(profile))
}

pub fn save(
    store: &LocalStore,
    request: SaveCostMergeMappingProfileRequest,
) -> Result<CostMergeMappingProfile, String> {
    let (base_url, tenant_id) = normalize_scope(&request.base_url, &request.tenant_id)?;
    let mappings = request
        .mappings
        .into_iter()
        .filter_map(|(article_type, cost_merge_id)| {
            let article_type = sanitize_article_type(&article_type)?;
            let cost_merge_id = cost_merge_id.trim();
            if cost_merge_id.is_empty() {
                return Some((article_type, String::new()));
            }
            is_object_id(cost_merge_id).then(|| (article_type, cost_merge_id.to_ascii_lowercase()))
        })
        .collect::<HashMap<_, _>>();
    let profile = CostMergeMappingProfile {
        version: COST_MERGE_MAPPING_VERSION,
        base_url: base_url.clone(),
        tenant_id: tenant_id.clone(),
        mappings,
        saved_at: Utc::now().to_rfc3339(),
    };
    store.save_setting(
        &setting_key(&base_url, &tenant_id),
        &serde_json::to_value(&profile).map_err(|error| error.to_string())?,
    )?;
    Ok(profile)
}

fn normalize_scope(base_url: &str, tenant_id: &str) -> Result<(String, String), String> {
    let base_url = base_url.trim().trim_end_matches('/').to_string();
    let tenant_id = tenant_id.trim().to_string();
    if base_url.is_empty() {
        return Err("缺少新系统地址，无法保存费用归并映射".into());
    }
    if tenant_id.is_empty() {
        return Err("缺少租户编码，无法保存费用归并映射".into());
    }
    Ok((base_url, tenant_id))
}

fn setting_key(base_url: &str, tenant_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(base_url.as_bytes());
    hasher.update([0]);
    hasher.update(tenant_id.as_bytes());
    format!("medicine_cost_merge_mapping:v1:{:x}", hasher.finalize())
}

fn sanitize_article_type(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty() && value.chars().count() <= 100 && !value.chars().any(char::is_control))
        .then(|| value.to_string())
}

fn is_object_id(value: &str) -> bool {
    value.len() == 24 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::{load, save, CostMergeMappingProfileScope, SaveCostMergeMappingProfileRequest};
    use crate::local_store::LocalStore;
    use std::collections::HashMap;

    #[test]
    fn mappings_round_trip_per_target_tenant_and_preserve_explicit_unset() {
        let directory = tempfile::tempdir().unwrap();
        let store = LocalStore::open(&directory.path().join("cost-merge.sqlite")).unwrap();
        let saved = save(
            &store,
            SaveCostMergeMappingProfileRequest {
                base_url: " http://his.example/rbmh-phis/ ".into(),
                tenant_id: " tenant-a ".into(),
                mappings: HashMap::from([
                    ("1".into(), "63AA8B1B3C6F491981BA4221".into()),
                    ("2".into(), "".into()),
                    ("3".into(), "not-an-object-id".into()),
                ]),
            },
        )
        .unwrap();
        assert_eq!(saved.base_url, "http://his.example/rbmh-phis");
        assert_eq!(saved.tenant_id, "tenant-a");
        assert_eq!(
            saved.mappings.get("1"),
            Some(&"63aa8b1b3c6f491981ba4221".into())
        );
        assert_eq!(saved.mappings.get("2"), Some(&String::new()));
        assert!(!saved.mappings.contains_key("3"));

        let loaded = load(
            &store,
            CostMergeMappingProfileScope {
                base_url: "http://his.example/rbmh-phis/".into(),
                tenant_id: "tenant-a".into(),
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(loaded.mappings, saved.mappings);

        assert!(load(
            &store,
            CostMergeMappingProfileScope {
                base_url: "http://his.example/rbmh-phis".into(),
                tenant_id: "tenant-b".into(),
            },
        )
        .unwrap()
        .is_none());
    }
}
