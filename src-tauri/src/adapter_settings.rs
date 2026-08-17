use crate::local_store::LocalStore;
use crate::model::SourceDictionaryItem;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

const PHIS27_MEDICINE_MAPPING_KEY: &str = "adapter_mapping:phis27:medicine:v1";
const PHIS27_MEDICINE_MAPPING_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavePhis27MappingProfileRequest {
    #[serde(default)]
    pub mapping: HashMap<String, String>,
    #[serde(default)]
    pub rules: HashMap<String, Value>,
    #[serde(default)]
    pub dictionary_overrides: HashMap<String, HashMap<String, Vec<SourceDictionaryItem>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Phis27MappingProfile {
    pub version: u32,
    pub mapping: HashMap<String, String>,
    pub rules: HashMap<String, Value>,
    #[serde(default)]
    pub dictionary_overrides: HashMap<String, HashMap<String, Vec<SourceDictionaryItem>>>,
    pub saved_at: String,
}

pub fn load(store: &LocalStore) -> Result<Option<Phis27MappingProfile>, String> {
    let Some(value) = store.load_setting(PHIS27_MEDICINE_MAPPING_KEY)? else {
        return Ok(None);
    };
    let profile: Phis27MappingProfile = serde_json::from_value(value)
        .map_err(|error| format!("读取二系列phis固化映射失败：{error}"))?;
    if !matches!(profile.version, 1 | PHIS27_MEDICINE_MAPPING_VERSION) {
        return Ok(None);
    }
    Ok(Some(Phis27MappingProfile {
        version: PHIS27_MEDICINE_MAPPING_VERSION,
        ..profile
    }))
}

pub fn save(
    store: &LocalStore,
    request: SavePhis27MappingProfileRequest,
) -> Result<Phis27MappingProfile, String> {
    let mapping = request
        .mapping
        .into_iter()
        .filter(|(target, source)| {
            valid_identifier(target) && (source.trim().is_empty() || valid_identifier(source))
        })
        .collect::<HashMap<_, _>>();
    let rules = request
        .rules
        .into_iter()
        .filter(|(target, rule)| valid_identifier(target) && rule.is_object())
        .collect::<HashMap<_, _>>();
    let dictionary_overrides = sanitize_dictionary_overrides(request.dictionary_overrides);
    let profile = Phis27MappingProfile {
        version: PHIS27_MEDICINE_MAPPING_VERSION,
        mapping,
        rules,
        dictionary_overrides,
        saved_at: Utc::now().to_rfc3339(),
    };
    store.save_setting(
        PHIS27_MEDICINE_MAPPING_KEY,
        &serde_json::to_value(&profile).map_err(|error| error.to_string())?,
    )?;
    Ok(profile)
}

fn sanitize_dictionary_overrides(
    overrides: HashMap<String, HashMap<String, Vec<SourceDictionaryItem>>>,
) -> HashMap<String, HashMap<String, Vec<SourceDictionaryItem>>> {
    overrides
        .into_iter()
        .filter_map(|(source, dictionaries)| {
            let source = source.trim().chars().take(500).collect::<String>();
            if source.is_empty() {
                return None;
            }
            let dictionaries = dictionaries
                .into_iter()
                .filter_map(|(dictionary_id, items)| {
                    let dictionary_id = dictionary_id.trim().chars().take(200).collect::<String>();
                    if dictionary_id.is_empty() {
                        return None;
                    }
                    let mut seen = std::collections::HashSet::new();
                    let items = items
                        .into_iter()
                        .filter_map(|mut item| {
                            item.key = item.key.trim().chars().take(200).collect();
                            item.text = item.text.trim().chars().take(500).collect();
                            if item.key.is_empty()
                                || item.text.is_empty()
                                || !seen.insert(item.key.clone())
                            {
                                return None;
                            }
                            Some(item)
                        })
                        .take(10_000)
                        .collect::<Vec<_>>();
                    Some((dictionary_id, items))
                })
                .collect::<HashMap<_, _>>();
            (!dictionaries.is_empty()).then_some((source, dictionaries))
        })
        .collect()
}

fn valid_identifier(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.len() <= 80
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

#[cfg(test)]
mod tests {
    use super::{load, save, SavePhis27MappingProfileRequest};
    use crate::local_store::LocalStore;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn verified_phis27_mapping_round_trips_and_rejects_invalid_entries() {
        let directory = tempfile::tempdir().unwrap();
        let store = LocalStore::open(&directory.path().join("mapping.sqlite")).unwrap();
        let saved = save(
            &store,
            SavePhis27MappingProfileRequest {
                mapping: HashMap::from([
                    ("unitPre".into(), "PRE_UNIT".into()),
                    ("sdRound".into(), "".into()),
                    ("bad target".into(), "DROP TABLE".into()),
                ]),
                rules: HashMap::from([
                    ("unitPre".into(), json!({"transform":"TRIM"})),
                    ("invalid".into(), json!("not-an-object")),
                ]),
                dictionary_overrides: HashMap::from([(
                    "oracle:127.0.0.1:1521/phis:PHIS27".into(),
                    HashMap::from([(
                        "phis.dictionary.gmywlb".into(),
                        vec![
                            crate::model::SourceDictionaryItem {
                                key: "3".into(),
                                text: "喹诺酮类".into(),
                                properties: Default::default(),
                            },
                            crate::model::SourceDictionaryItem {
                                key: "3".into(),
                                text: "重复值".into(),
                                properties: Default::default(),
                            },
                        ],
                    )]),
                )]),
            },
        )
        .unwrap();
        assert_eq!(saved.mapping.get("unitPre"), Some(&"PRE_UNIT".into()));
        assert_eq!(saved.mapping.get("sdRound"), Some(&String::new()));
        assert!(!saved.mapping.contains_key("bad target"));
        assert!(saved.rules.contains_key("unitPre"));
        assert!(!saved.rules.contains_key("invalid"));
        let customized = &saved.dictionary_overrides["oracle:127.0.0.1:1521/phis:PHIS27"]
            ["phis.dictionary.gmywlb"];
        assert_eq!(customized.len(), 1);
        assert_eq!(customized[0].text, "喹诺酮类");

        let loaded = load(&store).unwrap().unwrap();
        assert_eq!(loaded.mapping, saved.mapping);
        assert_eq!(loaded.rules, saved.rules);
        assert_eq!(loaded.dictionary_overrides, saved.dictionary_overrides);
        assert_eq!(loaded.version, 2);
    }
}
