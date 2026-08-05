use crate::target_system::TargetSystemClient;
use futures_util::{stream, StreamExt};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

const MAX_DICTIONARY_ITEMS: usize = 10_000;
const MAX_CONCURRENT_REQUESTS: usize = 6;
const REQUIRED_DICTIONARIES: &[&str] = &["rbmh.base.med.articleType", "rbmh.base.med.doseType"];

/// Derived from Dictionary annotations on user-mappable HiBdMed fields. System-maintained
/// fgActive/fgPri are deliberately excluded because base medicine migration is tenant-wide.
const FIELD_BINDINGS: &[(&str, &str)] = &[
    ("sdMed", "rbmh.base.med.articleType"),
    ("sdChrgitmLv", "phis.medicareLevel"),
    ("sdAllergy", "rbmh.base.med.sdAllergy"),
    ("fgAntiAppr", "sys.sd.yesOrNo"),
    ("sdBasMed", "rbmh.base.med.baseMed"),
    ("sdSpeMed", "rbmh.base.med.sdSpeMed"),
    ("sdDose", "rbmh.base.med.doseType"),
    ("sdStorage", "phis.storageType"),
    ("sdProdPlac", "rbmh.base.med.drugGrade"),
    ("fgPois", "sys.sd.yesOrNo"),
    ("fgAnti", "sys.sd.yesOrNo"),
    ("sdRound", "rbmh.base.med.roundingStrategy"),
    ("sdDps", "rbmh.base.med.dispensingMethod"),
    ("fgMedRx", "rbmh.base.med.prescriptiondrugIdentification"),
    ("dftUsage", "rbmh.base.med.usage"),
    ("dftFreq", "rbmh.base.freq"),
    ("fgTcd", "sys.sd.yesOrNo"),
    ("fgSingle", "sys.sd.yesOrNo"),
    ("fgRegister", "phis.ifNeedRegister"),
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryBinding {
    pub target_field: String,
    pub dictionary_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetDictionaryCatalog {
    pub ok: bool,
    pub bindings: Vec<DictionaryBinding>,
    pub dictionaries: Vec<TargetDictionary>,
    pub warnings: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetDictionary {
    #[serde(default)]
    pub dic_id: String,
    #[serde(default)]
    pub last_modify: Option<i64>,
    #[serde(default)]
    pub items: Vec<TargetDictionaryItem>,
    #[serde(default)]
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetDictionaryItem {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub cd: String,
    #[serde(default)]
    pub na: String,
    #[serde(default)]
    pub py: String,
    #[serde(default)]
    pub wb: String,
    #[serde(default)]
    pub active: Option<bool>,
}

impl TargetDictionaryItem {
    fn target_value(&self) -> &str {
        if self.key.trim().is_empty() {
            self.cd.trim()
        } else {
            self.key.trim()
        }
    }

    fn is_active(&self) -> bool {
        self.active.unwrap_or(true)
    }
}

pub fn dictionary_id_for(target_field: &str) -> Option<&'static str> {
    FIELD_BINDINGS
        .iter()
        .find_map(|(field, dictionary)| (*field == target_field).then_some(*dictionary))
}

pub async fn load(client_state: &TargetSystemClient) -> Result<TargetDictionaryCatalog, String> {
    let (client, base_url) = client_state.authenticated_http()?;
    let mut dictionaries = Vec::new();
    let mut warnings = Vec::new();
    let mut required_failures = Vec::new();
    let unique = FIELD_BINDINGS
        .iter()
        .map(|(_, dictionary)| *dictionary)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(str::to_string);
    let requests = unique.map(|dictionary_id| {
        let client = client.clone();
        let base_url = base_url.clone();
        async move {
            let result = fetch_dictionary(&client, &base_url, &dictionary_id).await;
            (dictionary_id, result)
        }
    });
    let results = stream::iter(requests)
        .buffer_unordered(MAX_CONCURRENT_REQUESTS)
        .collect::<Vec<_>>()
        .await;

    for (dictionary_id, result) in results {
        match result {
            Ok(dictionary) => dictionaries.push(dictionary),
            Err(error) if REQUIRED_DICTIONARIES.contains(&dictionary_id.as_str()) => {
                required_failures.push(format!("{dictionary_id}：{error}"));
            }
            Err(error) => warnings.push(format!("{dictionary_id}：{error}")),
        }
    }
    dictionaries.sort_by(|left, right| left.dic_id.cmp(&right.dic_id));
    warnings.sort();
    required_failures.sort();

    if !required_failures.is_empty() {
        client_state.invalidate()?;
        return Err(format!(
            "必需的新系统药品字典读取失败：{}",
            required_failures.join("；")
        ));
    }

    let by_id = dictionaries
        .iter()
        .map(|dictionary| (dictionary.dic_id.as_str(), dictionary))
        .collect::<HashMap<_, _>>();
    let mut values = HashMap::new();
    for (target_field, dictionary_id) in FIELD_BINDINGS {
        if let Some(dictionary) = by_id.get(dictionary_id) {
            values.insert(
                (*target_field).to_string(),
                dictionary
                    .items
                    .iter()
                    .map(TargetDictionaryItem::target_value)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .collect::<HashSet<_>>(),
            );
        }
    }
    client_state.replace_dictionary_values(values)?;

    Ok(TargetDictionaryCatalog {
        ok: true,
        bindings: FIELD_BINDINGS
            .iter()
            .map(|(target_field, dictionary_id)| DictionaryBinding {
                target_field: (*target_field).into(),
                dictionary_id: (*dictionary_id).into(),
            })
            .collect(),
        message: format!(
            "已读取 {} 个新系统药品字典、{} 个有效字典项",
            dictionaries.len(),
            dictionaries
                .iter()
                .map(|dictionary| dictionary.items.len())
                .sum::<usize>()
        ),
        dictionaries,
        warnings,
    })
}

async fn fetch_dictionary(
    client: &Client,
    base_url: &Url,
    dictionary_id: &str,
) -> Result<TargetDictionary, String> {
    let endpoint = base_url
        .join(&format!("{dictionary_id}.dic"))
        .map_err(|_| "无法生成字典服务地址".to_string())?;
    let limit = MAX_DICTIONARY_ITEMS.to_string();
    let response = client
        .get(endpoint)
        .query(&[("start", "0"), ("limit", limit.as_str())])
        .send()
        .await
        .map_err(|error| format!("请求失败：{error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("HTTP {}", status.as_u16()));
    }
    let mut dictionary: TargetDictionary = response
        .json()
        .await
        .map_err(|_| "返回内容不是有效字典 JSON".to_string())?;
    if !dictionary.dic_id.is_empty() && dictionary.dic_id != dictionary_id {
        return Err(format!("返回了不匹配的字典 {}", dictionary.dic_id));
    }
    dictionary.dic_id = dictionary_id.into();
    let received_count = dictionary.items.len();
    if dictionary.total > received_count {
        return Err(format!(
            "字典返回不完整：声明{}项，实际仅收到{}项",
            dictionary.total, received_count
        ));
    }
    dictionary
        .items
        .retain(|item| item.is_active() && !item.target_value().is_empty());
    if received_count > MAX_DICTIONARY_ITEMS {
        return Err(format!(
            "字典项超过安全上限 {}，请联系管理员精简字典",
            MAX_DICTIONARY_ITEMS
        ));
    }
    if dictionary.items.is_empty() {
        return Err("没有可用字典项".into());
    }
    Ok(dictionary)
}

pub fn validate_values(
    data: &Map<String, Value>,
    dictionary_values: &HashMap<String, HashSet<String>>,
) -> Vec<String> {
    let mut errors = Vec::new();
    for (target_field, dictionary_id) in FIELD_BINDINGS {
        let value = data
            .get(*target_field)
            .map(crate::normalize::value_text)
            .unwrap_or_default();
        if value.is_empty() {
            continue;
        }
        let Some(allowed) = dictionary_values.get(*target_field) else {
            // Some dynamic dictionaries depend on a department context and are not exposed by the
            // generic .dic endpoint. Keep the visible warning, but do not reject a manually mapped
            // value when the target system could not provide an authoritative list.
            continue;
        };
        if !allowed.contains(&value) {
            let mut available = allowed.iter().cloned().collect::<Vec<_>>();
            available.sort();
            let sample = available.into_iter().take(8).collect::<Vec<_>>().join("、");
            errors.push(format!(
                "字段 {target_field} 的值“{value}”不在目标字典 {dictionary_id} 中{}",
                if sample.is_empty() {
                    String::new()
                } else {
                    format!("（可用编码示例：{sample}）")
                }
            ));
        }
    }
    errors
}

pub fn ensure_required_loaded(
    dictionary_values: &HashMap<String, HashSet<String>>,
) -> Result<(), String> {
    for (target_field, dictionary_id) in FIELD_BINDINGS {
        if REQUIRED_DICTIONARIES.contains(dictionary_id)
            && dictionary_values
                .get(*target_field)
                .is_none_or(HashSet::is_empty)
        {
            return Err(format!(
                "目标字典 {dictionary_id} 尚未加载，请重新执行 system 登录后再校验数据"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{dictionary_id_for, ensure_required_loaded, load, validate_values};
    use crate::target_system::{login, TargetSystemClient, TargetSystemLoginRequest};
    use serde_json::json;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn bindings_follow_hi_bd_med_dictionary_annotations() {
        assert_eq!(
            dictionary_id_for("sdMed"),
            Some("rbmh.base.med.articleType")
        );
        assert_eq!(dictionary_id_for("sdDose"), Some("rbmh.base.med.doseType"));
        assert_eq!(dictionary_id_for("unitPre"), None);
    }

    #[test]
    fn rejects_values_outside_loaded_target_dictionary() {
        let mut dictionaries = HashMap::new();
        dictionaries.insert(
            "sdMed".into(),
            HashSet::from(["1".into(), "2".into(), "3".into()]),
        );
        let data = json!({ "sdMed": "9" }).as_object().unwrap().clone();
        let errors = validate_values(&data, &dictionaries);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("rbmh.base.med.articleType"));
    }

    #[test]
    fn requires_core_dictionary_cache_before_preparing_data() {
        assert!(ensure_required_loaded(&HashMap::new())
            .unwrap_err()
            .contains("articleType"));
        let mut dictionaries = HashMap::new();
        dictionaries.insert("sdMed".into(), HashSet::from(["1".into()]));
        dictionaries.insert("sdDose".into(), HashSet::from(["1".into()]));
        assert!(ensure_required_loaded(&dictionaries).is_ok());
    }

    #[test]
    #[ignore = "requires an explicitly configured reachable target system"]
    fn live_catalog_uses_authenticated_tk_session() {
        let state = TargetSystemClient::new().unwrap();
        let request = TargetSystemLoginRequest {
            base_url: std::env::var("TARGET_SYSTEM_TEST_URL")
                .expect("TARGET_SYSTEM_TEST_URL is required for the ignored live test"),
            tenant_id: std::env::var("TARGET_SYSTEM_TEST_TENANT")
                .expect("TARGET_SYSTEM_TEST_TENANT is required for the ignored live test"),
            login_name: "system".into(),
            password: std::env::var("TARGET_SYSTEM_TEST_PASSWORD")
                .expect("TARGET_SYSTEM_TEST_PASSWORD is required for the ignored live test"),
        };
        tauri::async_runtime::block_on(login(&state, request)).unwrap();
        let catalog = tauri::async_runtime::block_on(load(&state)).unwrap();

        println!(
            "loaded dictionaries={}, items={}, warnings={:?}",
            catalog.dictionaries.len(),
            catalog
                .dictionaries
                .iter()
                .map(|dictionary| dictionary.items.len())
                .sum::<usize>(),
            catalog.warnings
        );

        assert!(catalog.ok);
        assert!(catalog
            .dictionaries
            .iter()
            .any(|dictionary| dictionary.dic_id == "rbmh.base.med.articleType"));
        assert!(catalog
            .dictionaries
            .iter()
            .any(|dictionary| dictionary.dic_id == "rbmh.base.med.doseType"));
        for required_dynamic in ["rbmh.base.med.usage", "rbmh.base.freq"] {
            let dictionary = catalog
                .dictionaries
                .iter()
                .find(|dictionary| dictionary.dic_id == required_dynamic)
                .unwrap_or_else(|| panic!("missing live dictionary {required_dynamic}"));
            println!(
                "live dictionary {} items={}",
                required_dynamic,
                dictionary.items.len()
            );
            assert!(!dictionary.items.is_empty());
        }
        assert!(!state.dictionary_values().unwrap().is_empty());
    }
}
