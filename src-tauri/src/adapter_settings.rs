use crate::id::new_object_id;
use crate::local_store::LocalStore;
use crate::model::{
    sanitize_source_object_structures, SourceDictionaryItem, SourceObjectStructure,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};

const PHIS27_MEDICINE_MAPPING_KEY: &str = "adapter_mapping:phis27:medicine:v1";
const PHIS27_MEDICINE_MAPPING_VERSION: u32 = 2;
const SOURCE_MAPPING_PROFILE_VERSION: u32 = 1;
const SOURCE_MAPPING_PROFILE_PREFIX: &str = "source_mapping_profile:v1:";
const SOURCE_MAPPING_TEMPLATE_FORMAT: &str = "medicine-migration-source-template";
const SOURCE_MAPPING_TEMPLATE_VERSION: u32 = 1;
const SOURCE_MAPPING_TEMPLATE_CURRENT_SOURCE: &str = "__CURRENT_SOURCE__";
const MAX_SOURCE_KEY_FIELDS: usize = 3;
const SOURCE_ADAPTER_DIAGNOSTIC_PREFIX: &str = "source_adapter_diagnostics:v1:";
const SOURCE_ADAPTER_DIAGNOSTIC_LIMIT: usize = 20;
const SOURCE_ADAPTER_SUPPORT_FORMAT: &str = "medicine-migration-adapter-support";
const SOURCE_ADAPTER_SUPPORT_VERSION: u32 = 2;
const SOURCE_ADAPTER_TASK_MEDICINE: &str = "MEDICINE_BASE";
const SOURCE_ADAPTER_TASK_INVENTORY: &str = "INVENTORY";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceMappingProfileScope {
    pub adapter_id: String,
    pub source_identity: String,
    pub target_tenant_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadSourceMappingProfileRequest {
    pub scope: SourceMappingProfileScope,
    #[serde(default)]
    pub adapter_version: u32,
    #[serde(default)]
    pub template_compatible_from_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSourceMappingProfileRequest {
    pub scope: SourceMappingProfileScope,
    #[serde(default)]
    pub adapter_version: u32,
    #[serde(default)]
    pub source_key: String,
    #[serde(default)]
    pub source_key_fields: Vec<String>,
    #[serde(default)]
    pub source_query: String,
    #[serde(default)]
    pub mapping: HashMap<String, String>,
    #[serde(default)]
    pub rules: HashMap<String, Value>,
    #[serde(default)]
    pub dictionary_overrides: HashMap<String, HashMap<String, Vec<SourceDictionaryItem>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendSourceMappingProfileRequest {
    pub scope: SourceMappingProfileScope,
    #[serde(default)]
    pub columns: Vec<String>,
    #[serde(default)]
    pub adapter_version: u32,
    #[serde(default)]
    pub template_compatible_from_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceMappingProfileCandidate {
    pub profile: SourceMappingProfile,
    pub matched_mapping_count: usize,
    pub source_key_matched: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceMappingProfile {
    pub version: u32,
    pub adapter_id: String,
    #[serde(default)]
    pub adapter_version: u32,
    pub source_identity: String,
    pub target_tenant_id: String,
    pub source_key: String,
    #[serde(default)]
    pub source_key_fields: Vec<String>,
    #[serde(default)]
    pub source_query: String,
    pub mapping: HashMap<String, String>,
    pub rules: HashMap<String, Value>,
    #[serde(default)]
    pub dictionary_overrides: HashMap<String, HashMap<String, Vec<SourceDictionaryItem>>>,
    pub saved_at: String,
    #[serde(default)]
    pub compatibility: String,
    #[serde(default)]
    pub compatibility_message: String,
    #[serde(default)]
    pub requires_review: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSourceMappingTemplateRequest {
    pub scope: SourceMappingProfileScope,
    #[serde(default)]
    pub adapter_version: String,
    #[serde(default)]
    pub template_compatible_from_version: u32,
    #[serde(default)]
    pub dictionary_scope_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSourceMappingTemplateRequest {
    pub scope: SourceMappingProfileScope,
    pub content: String,
    #[serde(default)]
    pub adapter_version: u32,
    #[serde(default)]
    pub template_compatible_from_version: u32,
    #[serde(default)]
    pub dictionary_scope_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceMappingTemplateExchange {
    pub format: String,
    pub version: u32,
    pub adapter_id: String,
    #[serde(default)]
    pub adapter_version: String,
    #[serde(default)]
    pub source_key: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_key_fields: Vec<String>,
    #[serde(default)]
    pub source_query: String,
    #[serde(default)]
    pub mapping: BTreeMap<String, String>,
    #[serde(default)]
    pub rules: BTreeMap<String, Value>,
    #[serde(default)]
    pub dictionary_overrides: BTreeMap<String, BTreeMap<String, Vec<SourceDictionaryItem>>>,
    pub exported_at: String,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedSourceMappingTemplate {
    pub file_name: String,
    pub content: String,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceMappingTemplatePreview {
    pub adapter_id: String,
    pub adapter_version: String,
    pub source_key: String,
    pub source_key_fields: Vec<String>,
    pub has_source_query: bool,
    pub mapping_count: usize,
    pub rule_count: usize,
    pub dictionary_count: usize,
    pub dictionary_item_count: usize,
    pub exported_at: String,
    pub checksum: String,
    pub compatibility: String,
    pub compatibility_message: String,
    pub requires_review: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceAdapterDiagnosticMetric {
    pub id: String,
    pub label: String,
    pub value: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSourceAdapterDiagnosticRequest {
    pub scope: SourceMappingProfileScope,
    #[serde(default = "default_source_adapter_migration_task")]
    pub migration_task: String,
    pub adapter_version: u32,
    pub database_family: String,
    pub schema: String,
    pub detected: bool,
    #[serde(default)]
    pub checked_objects: Vec<String>,
    #[serde(default)]
    pub missing_objects: Vec<String>,
    #[serde(default)]
    pub object_structures: Vec<SourceObjectStructure>,
    #[serde(default)]
    pub metrics: Vec<SourceAdapterDiagnosticMetric>,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadSourceAdapterDiagnosticsRequest {
    pub scope: SourceMappingProfileScope,
    #[serde(default = "default_source_adapter_migration_task")]
    pub migration_task: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceAdapterDiagnosticRecord {
    pub diagnostic_id: String,
    pub adapter_id: String,
    #[serde(default = "default_source_adapter_migration_task")]
    pub migration_task: String,
    pub adapter_version: u32,
    pub source_fingerprint: String,
    pub database_family: String,
    pub schema: String,
    pub detected: bool,
    pub checked_objects: Vec<String>,
    pub missing_objects: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub object_structures: Vec<SourceObjectStructure>,
    pub metrics: Vec<SourceAdapterDiagnosticMetric>,
    pub warnings: Vec<String>,
    pub message: String,
    pub structure_hash: String,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSourceAdapterSupportPackageRequest {
    pub scope: SourceMappingProfileScope,
    #[serde(default = "default_source_adapter_migration_task")]
    pub migration_task: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceAdapterSupportPackageExchange {
    pub format: String,
    pub version: u32,
    pub adapter_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub migration_task: String,
    pub source_fingerprint: String,
    pub diagnostics: Vec<SourceAdapterDiagnosticRecord>,
    pub exported_at: String,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedSourceAdapterSupportPackage {
    pub file_name: String,
    pub content: String,
    pub checksum: String,
    pub diagnostic_count: usize,
}

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

pub fn load_source_mapping_profile(
    store: &LocalStore,
    request: LoadSourceMappingProfileRequest,
) -> Result<Option<SourceMappingProfile>, String> {
    let scope = normalize_scope(request.scope)?;
    let key = source_mapping_profile_key(&scope);
    let Some(value) = store.load_setting(&key)? else {
        return Ok(None);
    };
    let mut profile: SourceMappingProfile =
        serde_json::from_value(value).map_err(|error| format!("读取来源映射模板失败：{error}"))?;
    if profile.version != SOURCE_MAPPING_PROFILE_VERSION {
        return Ok(None);
    }
    if profile.adapter_id != scope.adapter_id
        || profile.source_identity != scope.source_identity
        || profile.target_tenant_id != scope.target_tenant_id
    {
        return Err("来源映射模板身份校验失败，请重新保存当前项目配置".into());
    }
    let (source_key, source_key_fields) = normalize_source_key_fields(
        &profile.source_key,
        std::mem::take(&mut profile.source_key_fields),
    )?;
    profile.source_key = source_key;
    profile.source_key_fields = source_key_fields;
    let compatibility = adapter_version_compatibility(
        profile.adapter_version,
        request.adapter_version,
        request.template_compatible_from_version,
        "本地来源模板",
    )?;
    profile.compatibility = compatibility.status;
    profile.compatibility_message = compatibility.message;
    profile.requires_review = compatibility.requires_review;
    Ok(Some(profile))
}

pub fn save_source_mapping_profile(
    store: &LocalStore,
    request: SaveSourceMappingProfileRequest,
) -> Result<SourceMappingProfile, String> {
    let scope = normalize_scope(request.scope)?;
    let (source_key, source_key_fields) =
        normalize_source_key_fields(&request.source_key, request.source_key_fields)?;
    let profile = SourceMappingProfile {
        version: SOURCE_MAPPING_PROFILE_VERSION,
        adapter_id: scope.adapter_id.clone(),
        adapter_version: request.adapter_version,
        source_identity: scope.source_identity.clone(),
        target_tenant_id: scope.target_tenant_id.clone(),
        source_key,
        source_key_fields,
        source_query: sanitize_source_query(request.source_query)?,
        mapping: sanitize_mapping(request.mapping),
        rules: sanitize_rules(request.rules),
        dictionary_overrides: sanitize_dictionary_overrides(request.dictionary_overrides),
        saved_at: Utc::now().to_rfc3339(),
        compatibility: "CURRENT".into(),
        compatibility_message: if request.adapter_version == 0 {
            "当前适配器未声明版本，保存后仍需在下次读取时核对".into()
        } else {
            format!("已按适配器 v{} 保存", request.adapter_version)
        },
        requires_review: request.adapter_version == 0,
    };
    store.save_setting(
        &source_mapping_profile_key(&scope),
        &serde_json::to_value(&profile).map_err(|error| error.to_string())?,
    )?;
    Ok(profile)
}

pub fn recommend_source_mapping_profile(
    store: &LocalStore,
    request: RecommendSourceMappingProfileRequest,
) -> Result<Option<SourceMappingProfileCandidate>, String> {
    let scope = normalize_scope(request.scope)?;
    let columns = request
        .columns
        .into_iter()
        .map(|column| column.trim().to_string())
        .filter(|column| valid_identifier(column))
        .collect::<HashSet<_>>();
    if columns.is_empty() {
        return Ok(None);
    }

    for value in store.list_settings_by_prefix(SOURCE_MAPPING_PROFILE_PREFIX)? {
        let Ok(mut profile) = serde_json::from_value::<SourceMappingProfile>(value) else {
            continue;
        };
        let Ok((source_key, source_key_fields)) = normalize_source_key_fields(
            &profile.source_key,
            std::mem::take(&mut profile.source_key_fields),
        ) else {
            continue;
        };
        profile.source_key = source_key;
        profile.source_key_fields = source_key_fields;
        if profile.version != SOURCE_MAPPING_PROFILE_VERSION
            || profile.adapter_id != scope.adapter_id
            || profile.target_tenant_id != scope.target_tenant_id
            || profile.source_identity == scope.source_identity
        {
            continue;
        }
        if adapter_version_compatibility(
            profile.adapter_version,
            request.adapter_version,
            request.template_compatible_from_version,
            "同结构本地模板",
        )
        .is_err()
        {
            continue;
        }
        let mapped_fields = profile
            .mapping
            .values()
            .map(|field| field.trim())
            .filter(|field| !field.is_empty())
            .map(str::to_string)
            .collect::<HashSet<_>>();
        if mapped_fields.len() < 3
            || mapped_fields.iter().any(|field| !columns.contains(field))
            || rule_source_fields(&profile.rules)
                .iter()
                .any(|field| !columns.contains(field))
        {
            continue;
        }
        let matched_mapping_count = mapped_fields.len();

        let source_key_matched = profile.source_key_fields.is_empty()
            || profile
                .source_key_fields
                .iter()
                .all(|field| columns.contains(field));
        if !source_key_matched {
            profile.source_key.clear();
            profile.source_key_fields.clear();
        }
        // A similar-source recommendation may reuse only the field decisions. Never carry an
        // executable query or project-specific dictionary meaning into another source identity.
        profile.source_query.clear();
        profile.dictionary_overrides.clear();
        profile.compatibility = "SIMILAR_SOURCE_REVIEW".into();
        profile.compatibility_message = format!(
            "发现同一目标租户中保存于 {} 的本地模板，{} 个已映射来源字段在当前数据中全部存在；仅复用字段和转换规则，不复用旧查询、连接或来源字典{}",
            profile.saved_at,
            matched_mapping_count,
            if source_key_matched {
                ""
            } else {
                "，原唯一标识不存在，已要求重新选择"
            }
        );
        profile.requires_review = true;
        return Ok(Some(SourceMappingProfileCandidate {
            profile,
            matched_mapping_count,
            source_key_matched,
            message: "已发现可复用的同结构本地模板建议，进入校验前请逐项核对".into(),
        }));
    }
    Ok(None)
}

fn rule_source_fields(rules: &HashMap<String, Value>) -> HashSet<String> {
    let mut fields = HashSet::new();
    for rule in rules.values() {
        if let Some(field) = rule.get("conditionField").and_then(Value::as_str) {
            let field = field.trim();
            if !field.is_empty() {
                fields.insert(field.to_string());
            }
        }
        if let Some(additional) = rule.get("additionalSourceFields").and_then(Value::as_array) {
            fields.extend(
                additional
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .filter(|field| !field.is_empty())
                    .map(str::to_string),
            );
        }
    }
    fields
}

pub fn export_source_mapping_template(
    store: &LocalStore,
    request: ExportSourceMappingTemplateRequest,
) -> Result<ExportedSourceMappingTemplate, String> {
    let scope = normalize_scope(request.scope)?;
    let profile = load_source_mapping_profile(
        store,
        LoadSourceMappingProfileRequest {
            scope: scope.clone(),
            adapter_version: parse_adapter_version(&request.adapter_version).unwrap_or(0),
            template_compatible_from_version: request.template_compatible_from_version,
        },
    )?
    .ok_or_else(|| "当前来源还没有已保存的字段映射，请先完成一次字段核对".to_string())?;
    let portable_dictionaries = if request.dictionary_scope_key.trim().is_empty() {
        (profile.dictionary_overrides.len() == 1)
            .then(|| profile.dictionary_overrides.values().next().cloned())
            .flatten()
            .unwrap_or_default()
    } else {
        profile
            .dictionary_overrides
            .get(request.dictionary_scope_key.trim())
            .cloned()
            .unwrap_or_default()
    };
    let mut exchange = SourceMappingTemplateExchange {
        format: SOURCE_MAPPING_TEMPLATE_FORMAT.into(),
        version: SOURCE_MAPPING_TEMPLATE_VERSION,
        adapter_id: profile.adapter_id,
        adapter_version: request.adapter_version.trim().chars().take(40).collect(),
        source_key: profile.source_key,
        source_key_fields: profile.source_key_fields,
        source_query: profile.source_query,
        mapping: profile.mapping.into_iter().collect(),
        rules: profile.rules.into_iter().collect(),
        dictionary_overrides: (!portable_dictionaries.is_empty())
            .then(|| {
                BTreeMap::from([(
                    SOURCE_MAPPING_TEMPLATE_CURRENT_SOURCE.into(),
                    portable_dictionaries
                        .into_iter()
                        .collect::<BTreeMap<_, _>>(),
                )])
            })
            .unwrap_or_default(),
        exported_at: Utc::now().to_rfc3339(),
        checksum: String::new(),
    };
    exchange.checksum = source_mapping_template_checksum(&exchange)?;
    let content = serde_json::to_string_pretty(&exchange)
        .map_err(|error| format!("生成来源模板文件失败：{error}"))?;
    let date = Utc::now().format("%Y%m%d");
    Ok(ExportedSourceMappingTemplate {
        file_name: format!(
            "{}-source-template-{date}.json",
            scope.adapter_id.to_ascii_lowercase()
        ),
        checksum: exchange.checksum,
        content,
    })
}

pub fn preview_source_mapping_template(
    request: ImportSourceMappingTemplateRequest,
) -> Result<SourceMappingTemplatePreview, String> {
    let scope = normalize_scope(request.scope)?;
    let exchange = parse_source_mapping_template(
        &scope,
        &request.content,
        request.adapter_version,
        request.template_compatible_from_version,
    )?;
    source_mapping_template_preview(
        &exchange,
        request.adapter_version,
        request.template_compatible_from_version,
    )
}

pub fn import_source_mapping_template(
    store: &LocalStore,
    request: ImportSourceMappingTemplateRequest,
) -> Result<SourceMappingProfile, String> {
    let scope = normalize_scope(request.scope)?;
    let exchange = parse_source_mapping_template(
        &scope,
        &request.content,
        request.adapter_version,
        request.template_compatible_from_version,
    )?;
    let dictionary_scope_key = request
        .dictionary_scope_key
        .trim()
        .chars()
        .take(500)
        .collect::<String>();
    let portable_dictionaries = exchange
        .dictionary_overrides
        .get(SOURCE_MAPPING_TEMPLATE_CURRENT_SOURCE)
        .cloned()
        .unwrap_or_default();
    let dictionary_overrides = if portable_dictionaries.is_empty() {
        HashMap::new()
    } else if dictionary_scope_key.is_empty() {
        return Err("当前适配器需要来源字典绑定信息，请重新读取老库后再导入".into());
    } else {
        HashMap::from([(
            dictionary_scope_key,
            portable_dictionaries.into_iter().collect::<HashMap<_, _>>(),
        )])
    };
    save_source_mapping_profile(
        store,
        SaveSourceMappingProfileRequest {
            scope,
            adapter_version: request.adapter_version,
            source_key: exchange.source_key,
            source_key_fields: exchange.source_key_fields,
            source_query: exchange.source_query,
            mapping: exchange.mapping.into_iter().collect(),
            rules: exchange.rules.into_iter().collect(),
            dictionary_overrides,
        },
    )
}

pub fn save_source_adapter_diagnostic(
    store: &LocalStore,
    request: SaveSourceAdapterDiagnosticRequest,
) -> Result<Vec<SourceAdapterDiagnosticRecord>, String> {
    let scope = normalize_scope(request.scope)?;
    let migration_task = normalize_source_adapter_migration_task(&request.migration_task)?;
    let source_fingerprint = source_adapter_source_fingerprint(&scope);
    let key =
        source_adapter_diagnostic_key(&scope.adapter_id, &source_fingerprint, &migration_task);
    let checked_objects = sanitize_diagnostic_list(request.checked_objects, 160, 120);
    let missing_objects = sanitize_diagnostic_list(request.missing_objects, 160, 120);
    let object_structures =
        sanitize_source_object_structures(request.object_structures, &checked_objects);
    let warnings = sanitize_diagnostic_list(request.warnings, 40, 500);
    let metrics = request
        .metrics
        .into_iter()
        .take(80)
        .filter_map(|metric| {
            let id = metric.id.trim().chars().take(80).collect::<String>();
            let label = metric.label.trim().chars().take(120).collect::<String>();
            (!id.is_empty() && !label.is_empty()).then_some(SourceAdapterDiagnosticMetric {
                id,
                label,
                value: metric.value,
            })
        })
        .collect::<Vec<_>>();
    let database_family = request
        .database_family
        .trim()
        .to_ascii_lowercase()
        .chars()
        .take(40)
        .collect::<String>();
    let schema = request
        .schema
        .trim()
        .to_ascii_uppercase()
        .chars()
        .take(128)
        .collect::<String>();
    let message = request.message.trim().chars().take(800).collect::<String>();
    let structure_hash = diagnostic_structure_hash(
        &migration_task,
        request.adapter_version,
        &database_family,
        &schema,
        request.detected,
        &checked_objects,
        &missing_objects,
        &object_structures,
        &metrics,
        &warnings,
        &message,
    )?;
    let mut history = load_source_adapter_diagnostics(
        store,
        LoadSourceAdapterDiagnosticsRequest {
            scope: scope.clone(),
            migration_task: migration_task.clone(),
        },
    )?;
    if history
        .first()
        .is_some_and(|record| record.structure_hash == structure_hash)
    {
        return Ok(history);
    }
    history.insert(
        0,
        SourceAdapterDiagnosticRecord {
            diagnostic_id: new_object_id(),
            adapter_id: scope.adapter_id,
            migration_task,
            adapter_version: request.adapter_version,
            source_fingerprint,
            database_family,
            schema,
            detected: request.detected,
            checked_objects,
            missing_objects,
            object_structures,
            metrics,
            warnings,
            message,
            structure_hash,
            recorded_at: Utc::now().to_rfc3339(),
        },
    );
    history.truncate(SOURCE_ADAPTER_DIAGNOSTIC_LIMIT);
    store.save_setting(
        &key,
        &serde_json::to_value(&history).map_err(|error| error.to_string())?,
    )?;
    Ok(history)
}

pub fn load_source_adapter_diagnostics(
    store: &LocalStore,
    request: LoadSourceAdapterDiagnosticsRequest,
) -> Result<Vec<SourceAdapterDiagnosticRecord>, String> {
    let scope = normalize_scope(request.scope)?;
    let migration_task = normalize_source_adapter_migration_task(&request.migration_task)?;
    let fingerprint = source_adapter_source_fingerprint(&scope);
    let key = source_adapter_diagnostic_key(&scope.adapter_id, &fingerprint, &migration_task);
    let mut value = store.load_setting(&key)?;
    if value.is_none() && migration_task == SOURCE_ADAPTER_TASK_MEDICINE {
        value = store.load_setting(&legacy_source_adapter_diagnostic_key(
            &scope.adapter_id,
            &fingerprint,
        ))?;
    }
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let mut history = serde_json::from_value::<Vec<SourceAdapterDiagnosticRecord>>(value)
        .map_err(|error| format!("读取来源适配器诊断历史失败：{error}"))?;
    history.retain(|record| {
        record.adapter_id == scope.adapter_id
            && record.source_fingerprint == fingerprint
            && record.migration_task == migration_task
    });
    history.truncate(SOURCE_ADAPTER_DIAGNOSTIC_LIMIT);
    Ok(history)
}

pub fn export_source_adapter_support_package(
    store: &LocalStore,
    request: ExportSourceAdapterSupportPackageRequest,
) -> Result<ExportedSourceAdapterSupportPackage, String> {
    let scope = normalize_scope(request.scope)?;
    let migration_task = normalize_source_adapter_migration_task(&request.migration_task)?;
    let diagnostics = load_source_adapter_diagnostics(
        store,
        LoadSourceAdapterDiagnosticsRequest {
            scope: scope.clone(),
            migration_task: migration_task.clone(),
        },
    )?;
    if diagnostics.is_empty() {
        return Err("当前来源还没有结构诊断记录，请先执行一次适配器检查".into());
    }
    let source_fingerprint = source_adapter_source_fingerprint(&scope);
    let mut exchange = SourceAdapterSupportPackageExchange {
        format: SOURCE_ADAPTER_SUPPORT_FORMAT.into(),
        version: SOURCE_ADAPTER_SUPPORT_VERSION,
        adapter_id: scope.adapter_id.clone(),
        migration_task: migration_task.clone(),
        source_fingerprint,
        diagnostics,
        exported_at: Utc::now().to_rfc3339(),
        checksum: String::new(),
    };
    exchange.checksum = source_adapter_support_checksum(&exchange)?;
    let diagnostic_count = exchange.diagnostics.len();
    let content = serde_json::to_string_pretty(&exchange)
        .map_err(|error| format!("生成适配器支持包失败：{error}"))?;
    let date = Utc::now().format("%Y%m%d");
    Ok(ExportedSourceAdapterSupportPackage {
        file_name: format!(
            "{}-{}-adapter-support-{date}.json",
            scope.adapter_id.to_ascii_lowercase(),
            migration_task.to_ascii_lowercase().replace('_', "-")
        ),
        content,
        checksum: exchange.checksum,
        diagnostic_count,
    })
}

fn source_adapter_source_fingerprint(scope: &SourceMappingProfileScope) -> String {
    let source = format!("{}\0{}", scope.adapter_id, scope.source_identity);
    format!("{:x}", Sha256::digest(source.as_bytes()))
        .chars()
        .take(20)
        .collect()
}

fn source_adapter_diagnostic_key(
    adapter_id: &str,
    fingerprint: &str,
    migration_task: &str,
) -> String {
    format!("{SOURCE_ADAPTER_DIAGNOSTIC_PREFIX}{adapter_id}:{fingerprint}:{migration_task}")
}

fn legacy_source_adapter_diagnostic_key(adapter_id: &str, fingerprint: &str) -> String {
    format!("{SOURCE_ADAPTER_DIAGNOSTIC_PREFIX}{adapter_id}:{fingerprint}")
}

fn default_source_adapter_migration_task() -> String {
    SOURCE_ADAPTER_TASK_MEDICINE.into()
}

fn normalize_source_adapter_migration_task(value: &str) -> Result<String, String> {
    let normalized = value.trim().to_ascii_uppercase();
    let normalized = if normalized.is_empty() {
        SOURCE_ADAPTER_TASK_MEDICINE
    } else {
        normalized.as_str()
    };
    if matches!(
        normalized,
        SOURCE_ADAPTER_TASK_MEDICINE | SOURCE_ADAPTER_TASK_INVENTORY
    ) {
        return Ok(normalized.into());
    }
    Err(format!(
        "来源适配器诊断任务仅支持 {SOURCE_ADAPTER_TASK_MEDICINE} 或 {SOURCE_ADAPTER_TASK_INVENTORY}"
    ))
}

fn sanitize_diagnostic_list(values: Vec<String>, limit: usize, length: usize) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    values
        .into_iter()
        .filter_map(|value| {
            let value = value.trim().chars().take(length).collect::<String>();
            (!value.is_empty() && seen.insert(value.clone())).then_some(value)
        })
        .take(limit)
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn diagnostic_structure_hash(
    migration_task: &str,
    adapter_version: u32,
    database_family: &str,
    schema: &str,
    detected: bool,
    checked_objects: &[String],
    missing_objects: &[String],
    object_structures: &[SourceObjectStructure],
    metrics: &[SourceAdapterDiagnosticMetric],
    warnings: &[String],
    message: &str,
) -> Result<String, String> {
    let canonical = canonicalize_json(serde_json::json!({
        "migrationTask": migration_task,
        "adapterVersion": adapter_version,
        "databaseFamily": database_family,
        "schema": schema,
        "detected": detected,
        "checkedObjects": checked_objects,
        "missingObjects": missing_objects,
        "objectStructures": object_structures,
        "metrics": metrics,
        "warnings": warnings,
        "message": message,
    }));
    let bytes =
        serde_json::to_vec(&canonical).map_err(|error| format!("计算诊断结构摘要失败：{error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn parse_source_mapping_template(
    scope: &SourceMappingProfileScope,
    content: &str,
    current_adapter_version: u32,
    template_compatible_from_version: u32,
) -> Result<SourceMappingTemplateExchange, String> {
    if content.len() > 5_000_000 {
        return Err("来源模板文件超过 5 MB，请检查是否选错文件".into());
    }
    let mut exchange: SourceMappingTemplateExchange =
        serde_json::from_str(content).map_err(|error| format!("来源模板文件格式无效：{error}"))?;
    if exchange.format != SOURCE_MAPPING_TEMPLATE_FORMAT {
        return Err("这不是数据迁移助手的来源模板文件".into());
    }
    if exchange.version != SOURCE_MAPPING_TEMPLATE_VERSION {
        return Err(format!(
            "来源模板版本 {} 暂不支持，请使用当前版本重新导出",
            exchange.version
        ));
    }
    if exchange.adapter_id.trim().to_ascii_uppercase() != scope.adapter_id {
        return Err(format!(
            "模板适配器 {} 与当前适配器 {} 不一致，不能直接套用",
            exchange.adapter_id, scope.adapter_id
        ));
    }
    adapter_version_compatibility(
        parse_adapter_version(&exchange.adapter_version).unwrap_or(0),
        current_adapter_version,
        template_compatible_from_version,
        "导入来源模板",
    )?;
    let expected = source_mapping_template_checksum(&exchange)?;
    if exchange.checksum.trim().to_ascii_lowercase() != expected {
        return Err("来源模板完整性校验失败，文件可能被修改或传输不完整".into());
    }
    sanitize_source_query(exchange.source_query.clone())?;
    let (source_key, source_key_fields) = normalize_source_key_fields(
        &exchange.source_key,
        std::mem::take(&mut exchange.source_key_fields),
    )
    .map_err(|error| format!("来源模板中的三方记录唯一标识无效：{error}"))?;
    exchange.source_key = source_key;
    exchange.source_key_fields = source_key_fields;
    Ok(exchange)
}

fn source_mapping_template_preview(
    exchange: &SourceMappingTemplateExchange,
    current_adapter_version: u32,
    template_compatible_from_version: u32,
) -> Result<SourceMappingTemplatePreview, String> {
    let compatibility = adapter_version_compatibility(
        parse_adapter_version(&exchange.adapter_version).unwrap_or(0),
        current_adapter_version,
        template_compatible_from_version,
        "导入来源模板",
    )?;
    let dictionary_count = exchange
        .dictionary_overrides
        .values()
        .map(BTreeMap::len)
        .sum();
    let dictionary_item_count = exchange
        .dictionary_overrides
        .values()
        .flat_map(BTreeMap::values)
        .map(Vec::len)
        .sum();
    Ok(SourceMappingTemplatePreview {
        adapter_id: exchange.adapter_id.clone(),
        adapter_version: exchange.adapter_version.clone(),
        source_key: exchange.source_key.clone(),
        source_key_fields: exchange.source_key_fields.clone(),
        has_source_query: !exchange.source_query.trim().is_empty(),
        mapping_count: exchange
            .mapping
            .values()
            .filter(|value| !value.is_empty())
            .count(),
        rule_count: exchange.rules.len(),
        dictionary_count,
        dictionary_item_count,
        exported_at: exchange.exported_at.clone(),
        checksum: exchange.checksum.clone(),
        compatibility: compatibility.status,
        compatibility_message: compatibility.message,
        requires_review: compatibility.requires_review,
    })
}

struct AdapterVersionCompatibility {
    status: String,
    message: String,
    requires_review: bool,
}

fn parse_adapter_version(value: &str) -> Option<u32> {
    value
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|version| *version > 0)
}

fn adapter_version_compatibility(
    saved_version: u32,
    current_version: u32,
    template_compatible_from_version: u32,
    subject: &str,
) -> Result<AdapterVersionCompatibility, String> {
    if current_version > 0 && saved_version > current_version {
        return Err(format!(
            "{subject}来自适配器 v{saved_version}，当前程序仅有 v{current_version}；请升级数据迁移助手后再使用，不能降级套用"
        ));
    }
    if saved_version == 0 || current_version == 0 {
        return Ok(AdapterVersionCompatibility {
            status: "LEGACY_REVIEW".into(),
            message: format!(
                "{subject}缺少可比较的适配器版本，已恢复配置但必须重新核对来源字段和字典"
            ),
            requires_review: true,
        });
    }
    if saved_version < current_version {
        let compatible_from = if template_compatible_from_version > 0
            && template_compatible_from_version <= current_version
        {
            template_compatible_from_version
        } else {
            current_version
        };
        if saved_version >= compatible_from {
            return Ok(AdapterVersionCompatibility {
                status: "COMPATIBLE_UPGRADE".into(),
                message: format!(
                    "{subject}由适配器 v{saved_version} 创建，当前 v{current_version} 声明兼容 v{compatible_from}–v{current_version}，可继续复用"
                ),
                requires_review: false,
            });
        }
        return Ok(AdapterVersionCompatibility {
            status: "UPGRADE_REVIEW".into(),
            message: format!(
                "{subject}由适配器 v{saved_version} 创建，当前为 v{current_version}；已按当前来源字段重新校验，确认后会升级保存"
            ),
            requires_review: true,
        });
    }
    Ok(AdapterVersionCompatibility {
        status: "CURRENT".into(),
        message: format!("{subject}与当前适配器 v{current_version} 一致"),
        requires_review: false,
    })
}

fn source_mapping_template_checksum(
    exchange: &SourceMappingTemplateExchange,
) -> Result<String, String> {
    let mut unsigned = serde_json::to_value(exchange)
        .map_err(|error| format!("计算来源模板校验值失败：{error}"))?;
    if let Some(object) = unsigned.as_object_mut() {
        object.remove("checksum");
    }
    let canonical = canonicalize_json(unsigned);
    let bytes = serde_json::to_vec(&canonical)
        .map_err(|error| format!("计算来源模板校验值失败：{error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn source_adapter_support_checksum(
    exchange: &SourceAdapterSupportPackageExchange,
) -> Result<String, String> {
    let mut unsigned = serde_json::to_value(exchange)
        .map_err(|error| format!("计算适配器支持包校验值失败：{error}"))?;
    if let Some(object) = unsigned.as_object_mut() {
        object.remove("checksum");
    }
    let canonical = canonicalize_json(unsigned);
    let bytes = serde_json::to_vec(&canonical)
        .map_err(|error| format!("计算适配器支持包校验值失败：{error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn canonicalize_json(value: Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, value)| (key, canonicalize_json(value)))
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.into_iter().map(canonicalize_json).collect()),
        other => other,
    }
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
    let mapping = sanitize_mapping(request.mapping);
    let rules = sanitize_rules(request.rules);
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

fn normalize_scope(scope: SourceMappingProfileScope) -> Result<SourceMappingProfileScope, String> {
    let adapter_id = scope.adapter_id.trim().to_ascii_uppercase();
    if !valid_identifier(&adapter_id) {
        return Err("来源适配器标识无效".into());
    }
    let source_identity = scope
        .source_identity
        .trim()
        .to_ascii_lowercase()
        .chars()
        .take(1_000)
        .collect::<String>();
    if source_identity.is_empty() {
        return Err("缺少来源数据库或文件身份，无法保存映射模板".into());
    }
    let target_tenant_id = scope
        .target_tenant_id
        .trim()
        .to_ascii_lowercase()
        .chars()
        .take(200)
        .collect::<String>();
    if target_tenant_id.is_empty() {
        return Err("缺少目标租户，无法隔离来源映射模板".into());
    }
    Ok(SourceMappingProfileScope {
        adapter_id,
        source_identity,
        target_tenant_id,
    })
}

fn source_mapping_profile_key(scope: &SourceMappingProfileScope) -> String {
    let canonical = format!(
        "{}\0{}\0{}",
        scope.adapter_id, scope.source_identity, scope.target_tenant_id
    );
    format!(
        "{SOURCE_MAPPING_PROFILE_PREFIX}{:x}",
        Sha256::digest(canonical.as_bytes())
    )
}

fn sanitize_mapping(mapping: HashMap<String, String>) -> HashMap<String, String> {
    mapping
        .into_iter()
        .filter(|(target, source)| {
            valid_identifier(target) && (source.trim().is_empty() || valid_identifier(source))
        })
        .collect()
}

fn sanitize_source_query(query: String) -> Result<String, String> {
    let query = query.trim().chars().take(50_001).collect::<String>();
    if query.is_empty() {
        return Ok(String::new());
    }
    if query.chars().count() > 50_000 {
        return Err("查询语句过长，请使用视图或拆分查询".into());
    }
    let normalized = query
        .trim_start_matches(|character: char| character.is_whitespace() || character == ';')
        .to_ascii_uppercase();
    if !(normalized.starts_with("SELECT") || normalized.starts_with("WITH")) {
        return Err("只允许保存 SELECT / WITH 开头的只读 SQL 模板".into());
    }
    let padded = format!(" {} ", normalized.replace(['\n', '\r', '\t'], " "));
    if [
        " INSERT ",
        " UPDATE ",
        " DELETE ",
        " DROP ",
        " ALTER ",
        " TRUNCATE ",
        " REPLACE ",
    ]
    .iter()
    .any(|keyword| padded.contains(keyword))
    {
        return Err("只读 SQL 模板中不能包含写操作关键字".into());
    }
    Ok(query)
}

fn sanitize_rules(rules: HashMap<String, Value>) -> HashMap<String, Value> {
    rules
        .into_iter()
        .filter(|(target, rule)| valid_identifier(target) && rule.is_object())
        .collect()
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

fn normalize_source_key_fields(
    legacy_source_key: &str,
    requested_fields: Vec<String>,
) -> Result<(String, Vec<String>), String> {
    let legacy_source_key = legacy_source_key
        .trim()
        .chars()
        .take(80)
        .collect::<String>();
    if !legacy_source_key.is_empty() && !valid_identifier(&legacy_source_key) {
        return Err("三方记录唯一标识只能使用字母、数字或下划线".into());
    }
    if requested_fields.len() > MAX_SOURCE_KEY_FIELDS {
        return Err(format!(
            "三方记录唯一标识最多由 {MAX_SOURCE_KEY_FIELDS} 个字段组成"
        ));
    }
    let candidates = if requested_fields.is_empty() && !legacy_source_key.is_empty() {
        vec![legacy_source_key.clone()]
    } else {
        requested_fields
    };
    let mut fields = Vec::with_capacity(candidates.len());
    let mut seen = HashSet::new();
    for field in candidates {
        let field = field.trim().chars().take(80).collect::<String>();
        if !valid_identifier(&field) {
            return Err("来源键字段只能使用字母、数字或下划线".into());
        }
        if !seen.insert(field.clone()) {
            return Err(format!("来源键字段 {field} 重复"));
        }
        fields.push(field);
    }
    if !legacy_source_key.is_empty()
        && (!fields.is_empty())
        && (fields.len() != 1 || fields[0] != legacy_source_key)
    {
        return Err("旧版单字段来源键与组合字段定义冲突".into());
    }
    let source_key = (fields.len() == 1)
        .then(|| fields[0].clone())
        .unwrap_or_default();
    Ok((source_key, fields))
}

#[cfg(test)]
mod tests {
    use super::{
        export_source_adapter_support_package, export_source_mapping_template,
        import_source_mapping_template, load, load_source_adapter_diagnostics,
        load_source_mapping_profile, preview_source_mapping_template,
        recommend_source_mapping_profile, save, save_source_adapter_diagnostic,
        save_source_mapping_profile, ExportSourceAdapterSupportPackageRequest,
        ExportSourceMappingTemplateRequest, ImportSourceMappingTemplateRequest,
        LoadSourceAdapterDiagnosticsRequest, LoadSourceMappingProfileRequest,
        RecommendSourceMappingProfileRequest, SavePhis27MappingProfileRequest,
        SaveSourceAdapterDiagnosticRequest, SaveSourceMappingProfileRequest,
        SourceAdapterDiagnosticMetric, SourceAdapterSupportPackageExchange,
        SourceMappingProfileScope,
    };
    use crate::local_store::LocalStore;
    use crate::model::{SourceObjectColumnStructure, SourceObjectStructure};
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

    #[test]
    fn generic_source_mapping_profiles_are_isolated_by_source_and_target_tenant() {
        let directory = tempfile::tempdir().unwrap();
        let store = LocalStore::open(&directory.path().join("mapping.sqlite")).unwrap();
        let scope = SourceMappingProfileScope {
            adapter_id: "generic_database".into(),
            source_identity: "Oracle:10.0.0.8:1521/legacy:HIS".into(),
            target_tenant_id: "Tenant-A".into(),
        };
        let saved = save_source_mapping_profile(
            &store,
            SaveSourceMappingProfileRequest {
                scope: scope.clone(),
                adapter_version: 1,
                source_key: "DRUG_ID".into(),
                source_key_fields: Vec::new(),
                source_query: "SELECT * FROM HIS_DRUG".into(),
                mapping: HashMap::from([
                    ("naMed".into(), "DRUG_NAME".into()),
                    ("invalid target".into(), "DROP TABLE".into()),
                ]),
                rules: HashMap::from([("naMed".into(), json!({"transform":"TRIM"}))]),
                dictionary_overrides: HashMap::new(),
            },
        )
        .unwrap();
        assert_eq!(saved.adapter_id, "GENERIC_DATABASE");
        assert_eq!(saved.source_key, "DRUG_ID");
        assert_eq!(saved.source_key_fields, vec!["DRUG_ID"]);
        assert_eq!(saved.source_query, "SELECT * FROM HIS_DRUG");
        assert_eq!(saved.mapping.get("naMed"), Some(&"DRUG_NAME".into()));
        assert!(!saved.mapping.contains_key("invalid target"));

        let loaded = load_source_mapping_profile(
            &store,
            LoadSourceMappingProfileRequest {
                scope: scope.clone(),
                adapter_version: 1,
                template_compatible_from_version: 1,
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(loaded.source_identity, "oracle:10.0.0.8:1521/legacy:his");
        assert_eq!(loaded.target_tenant_id, "tenant-a");
        assert_eq!(loaded.source_key_fields, vec!["DRUG_ID"]);

        let rejected_write_query = save_source_mapping_profile(
            &store,
            SaveSourceMappingProfileRequest {
                scope: scope.clone(),
                adapter_version: 1,
                source_key: String::new(),
                source_key_fields: Vec::new(),
                source_query: "DELETE FROM HIS_DRUG".into(),
                mapping: HashMap::new(),
                rules: HashMap::new(),
                dictionary_overrides: HashMap::new(),
            },
        );
        assert!(rejected_write_query.is_err());

        let another_tenant = load_source_mapping_profile(
            &store,
            LoadSourceMappingProfileRequest {
                scope: SourceMappingProfileScope {
                    target_tenant_id: "tenant-b".into(),
                    ..scope
                },
                adapter_version: 1,
                template_compatible_from_version: 1,
            },
        )
        .unwrap();
        assert!(another_tenant.is_none());
    }

    #[test]
    fn composite_source_key_definition_is_bounded_ordered_and_unambiguous() {
        let (legacy, legacy_fields) =
            super::normalize_source_key_fields("DRUG_ID", Vec::new()).unwrap();
        assert_eq!(legacy, "DRUG_ID");
        assert_eq!(legacy_fields, vec!["DRUG_ID"]);

        let (composite_legacy, composite_fields) = super::normalize_source_key_fields(
            "",
            vec!["DRUG_ID".into(), "FACTORY_ID".into(), "ORG_ID".into()],
        )
        .unwrap();
        assert!(composite_legacy.is_empty());
        assert_eq!(composite_fields, vec!["DRUG_ID", "FACTORY_ID", "ORG_ID"]);
        assert!(super::normalize_source_key_fields(
            "",
            vec!["A".into(), "B".into(), "C".into(), "D".into()]
        )
        .unwrap_err()
        .contains("最多"));
        assert!(
            super::normalize_source_key_fields("", vec!["DRUG_ID".into(), "DRUG_ID".into()])
                .unwrap_err()
                .contains("重复")
        );
    }

    #[test]
    fn source_mapping_profile_requires_review_after_adapter_upgrade_and_blocks_downgrade() {
        let directory = tempfile::tempdir().unwrap();
        let store = LocalStore::open(&directory.path().join("mapping.sqlite")).unwrap();
        let scope = SourceMappingProfileScope {
            adapter_id: "vendor_his".into(),
            source_identity: "oracle:project-a".into(),
            target_tenant_id: "tenant-a".into(),
        };
        save_source_mapping_profile(
            &store,
            SaveSourceMappingProfileRequest {
                scope: scope.clone(),
                adapter_version: 1,
                source_key: "DRUG_ID".into(),
                source_key_fields: Vec::new(),
                source_query: "SELECT DRUG_ID FROM DRUG_MASTER".into(),
                mapping: HashMap::from([("naMed".into(), "DRUG_NAME".into())]),
                rules: HashMap::new(),
                dictionary_overrides: HashMap::new(),
            },
        )
        .unwrap();

        let upgraded = load_source_mapping_profile(
            &store,
            LoadSourceMappingProfileRequest {
                scope: scope.clone(),
                adapter_version: 2,
                template_compatible_from_version: 2,
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(upgraded.compatibility, "UPGRADE_REVIEW");
        assert!(upgraded.requires_review);
        assert!(upgraded.compatibility_message.contains("v1"));
        assert!(upgraded.compatibility_message.contains("v2"));

        let compatible = load_source_mapping_profile(
            &store,
            LoadSourceMappingProfileRequest {
                scope: scope.clone(),
                adapter_version: 2,
                template_compatible_from_version: 1,
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(compatible.compatibility, "COMPATIBLE_UPGRADE");
        assert!(!compatible.requires_review);

        save_source_mapping_profile(
            &store,
            SaveSourceMappingProfileRequest {
                scope: scope.clone(),
                adapter_version: 3,
                source_key: "DRUG_ID".into(),
                source_key_fields: Vec::new(),
                source_query: "SELECT DRUG_ID FROM DRUG_MASTER".into(),
                mapping: HashMap::new(),
                rules: HashMap::new(),
                dictionary_overrides: HashMap::new(),
            },
        )
        .unwrap();
        let downgrade = load_source_mapping_profile(
            &store,
            LoadSourceMappingProfileRequest {
                scope,
                adapter_version: 2,
                template_compatible_from_version: 1,
            },
        );
        assert!(downgrade.unwrap_err().contains("不能降级套用"));
    }

    #[test]
    fn recommends_only_same_tenant_templates_whose_used_source_fields_still_exist() {
        let directory = tempfile::tempdir().unwrap();
        let store = LocalStore::open(&directory.path().join("mapping.sqlite")).unwrap();
        let saved_scope = SourceMappingProfileScope {
            adapter_id: "generic_database".into(),
            source_identity: "oracle:project-a:his-drug".into(),
            target_tenant_id: "tenant-a".into(),
        };
        save_source_mapping_profile(
            &store,
            SaveSourceMappingProfileRequest {
                scope: saved_scope,
                adapter_version: 1,
                source_key: "DRUG_ID".into(),
                source_key_fields: Vec::new(),
                source_query: "SELECT * FROM HIS_DRUG".into(),
                mapping: HashMap::from([
                    ("naMed".into(), "DRUG_NAME".into()),
                    ("spec".into(), "SPEC".into()),
                    ("unitPre".into(), "MIN_UNIT".into()),
                ]),
                rules: HashMap::from([(
                    "naMed".into(),
                    json!({
                        "transform": "TRIM",
                        "conditionField": "ACTIVE_FLAG",
                        "additionalSourceFields": ["PRODUCT_NAME"]
                    }),
                )]),
                dictionary_overrides: HashMap::from([(
                    "oracle:project-a".into(),
                    HashMap::from([(
                        "legacy.form".into(),
                        vec![crate::model::SourceDictionaryItem {
                            key: "1".into(),
                            text: "片剂".into(),
                            properties: Default::default(),
                        }],
                    )]),
                )]),
            },
        )
        .unwrap();

        let request = RecommendSourceMappingProfileRequest {
            scope: SourceMappingProfileScope {
                adapter_id: "GENERIC_DATABASE".into(),
                source_identity: "oracle:project-b:new-drug-view".into(),
                target_tenant_id: "tenant-a".into(),
            },
            columns: vec![
                "DRUG_NAME".into(),
                "SPEC".into(),
                "MIN_UNIT".into(),
                "ACTIVE_FLAG".into(),
                "PRODUCT_NAME".into(),
            ],
            adapter_version: 1,
            template_compatible_from_version: 1,
        };
        let candidate = recommend_source_mapping_profile(&store, request.clone())
            .unwrap()
            .unwrap();
        assert_eq!(candidate.matched_mapping_count, 3);
        assert!(!candidate.source_key_matched);
        assert!(candidate.profile.source_key.is_empty());
        assert!(candidate.profile.source_query.is_empty());
        assert!(candidate.profile.dictionary_overrides.is_empty());
        assert_eq!(candidate.profile.compatibility, "SIMILAR_SOURCE_REVIEW");
        assert!(candidate.profile.requires_review);

        let missing_rule_field = RecommendSourceMappingProfileRequest {
            columns: vec!["DRUG_NAME".into(), "SPEC".into(), "MIN_UNIT".into()],
            ..request.clone()
        };
        assert!(recommend_source_mapping_profile(&store, missing_rule_field)
            .unwrap()
            .is_none());

        let another_tenant = RecommendSourceMappingProfileRequest {
            scope: SourceMappingProfileScope {
                target_tenant_id: "tenant-b".into(),
                ..request.scope
            },
            ..request
        };
        assert!(recommend_source_mapping_profile(&store, another_tenant)
            .unwrap()
            .is_none());
    }

    #[test]
    fn adapter_diagnostic_history_is_deduplicated_scoped_and_credential_free() {
        let directory = tempfile::tempdir().unwrap();
        let store = LocalStore::open(&directory.path().join("mapping.sqlite")).unwrap();
        let scope = SourceMappingProfileScope {
            adapter_id: "vendor_his".into(),
            source_identity: "oracle:secret-host:1521/legacy:owner".into(),
            target_tenant_id: "private-tenant".into(),
        };
        let request = SaveSourceAdapterDiagnosticRequest {
            scope: scope.clone(),
            migration_task: "MEDICINE_BASE".into(),
            adapter_version: 1,
            database_family: "oracle".into(),
            schema: "owner".into(),
            detected: false,
            checked_objects: vec!["DRUG_MASTER".into()],
            missing_objects: vec!["DRUG_PRODUCT".into()],
            object_structures: vec![
                SourceObjectStructure {
                    name: "DRUG_MASTER".into(),
                    columns: vec![
                        SourceObjectColumnStructure {
                            name: "ID".into(),
                            data_type: "NUMBER".into(),
                        },
                        SourceObjectColumnStructure {
                            name: "NAME".into(),
                            data_type: "VARCHAR2".into(),
                        },
                    ],
                },
                SourceObjectStructure {
                    name: "PRIVATE_TABLE".into(),
                    columns: vec![SourceObjectColumnStructure {
                        name: "SECRET_VALUE".into(),
                        data_type: "VARCHAR2".into(),
                    }],
                },
            ],
            metrics: vec![SourceAdapterDiagnosticMetric {
                id: "MEDICINES".into(),
                label: "药品".into(),
                value: 12,
            }],
            warnings: vec!["缺少商品关系".into()],
            message: "结构未通过".into(),
        };
        let first = save_source_adapter_diagnostic(&store, request.clone()).unwrap();
        assert_eq!(first.len(), 1);
        let duplicate = save_source_adapter_diagnostic(&store, request).unwrap();
        assert_eq!(duplicate.len(), 1);

        let changed = save_source_adapter_diagnostic(
            &store,
            SaveSourceAdapterDiagnosticRequest {
                scope: scope.clone(),
                migration_task: "MEDICINE_BASE".into(),
                adapter_version: 2,
                database_family: "oracle".into(),
                schema: "owner".into(),
                detected: true,
                checked_objects: vec!["DRUG_MASTER".into(), "DRUG_PRODUCT".into()],
                missing_objects: Vec::new(),
                object_structures: vec![
                    SourceObjectStructure {
                        name: "DRUG_MASTER".into(),
                        columns: vec![
                            SourceObjectColumnStructure {
                                name: "ID".into(),
                                data_type: "NUMBER".into(),
                            },
                            SourceObjectColumnStructure {
                                name: "NAME".into(),
                                data_type: "NVARCHAR2".into(),
                            },
                            SourceObjectColumnStructure {
                                name: "PACK_FACTOR".into(),
                                data_type: "NUMBER".into(),
                            },
                        ],
                    },
                    SourceObjectStructure {
                        name: "DRUG_PRODUCT".into(),
                        columns: vec![SourceObjectColumnStructure {
                            name: "PRODUCT_ID".into(),
                            data_type: "NUMBER".into(),
                        }],
                    },
                ],
                metrics: Vec::new(),
                warnings: Vec::new(),
                message: "结构通过".into(),
            },
        )
        .unwrap();
        assert_eq!(changed.len(), 2);
        assert_eq!(changed[0].adapter_version, 2);

        let loaded = load_source_adapter_diagnostics(
            &store,
            LoadSourceAdapterDiagnosticsRequest {
                scope: scope.clone(),
                migration_task: "MEDICINE_BASE".into(),
            },
        )
        .unwrap();
        let serialized = serde_json::to_string(&loaded).unwrap();
        assert_eq!(loaded.len(), 2);
        assert!(!serialized.contains("secret-host"));
        assert!(!serialized.contains("private-tenant"));
        assert!(!serialized.contains("PRIVATE_TABLE"));
        assert!(!serialized.contains("SECRET_VALUE"));

        let inventory = save_source_adapter_diagnostic(
            &store,
            SaveSourceAdapterDiagnosticRequest {
                scope: scope.clone(),
                migration_task: "INVENTORY".into(),
                adapter_version: 2,
                database_family: "oracle".into(),
                schema: "owner".into(),
                detected: true,
                checked_objects: vec!["WAREHOUSE_STOCK".into()],
                missing_objects: Vec::new(),
                object_structures: vec![SourceObjectStructure {
                    name: "WAREHOUSE_STOCK".into(),
                    columns: vec![SourceObjectColumnStructure {
                        name: "QUANTITY".into(),
                        data_type: "NUMBER".into(),
                    }],
                }],
                metrics: vec![SourceAdapterDiagnosticMetric {
                    id: "STOCK_ROWS".into(),
                    label: "库存明细".into(),
                    value: 18,
                }],
                warnings: Vec::new(),
                message: "库存结构通过".into(),
            },
        )
        .unwrap();
        assert_eq!(inventory.len(), 1);
        assert_eq!(inventory[0].migration_task, "INVENTORY");
        let medicine_after_inventory = load_source_adapter_diagnostics(
            &store,
            LoadSourceAdapterDiagnosticsRequest {
                scope: scope.clone(),
                migration_task: "MEDICINE_BASE".into(),
            },
        )
        .unwrap();
        assert_eq!(medicine_after_inventory.len(), 2);

        let exported = export_source_adapter_support_package(
            &store,
            ExportSourceAdapterSupportPackageRequest {
                scope: scope.clone(),
                migration_task: "MEDICINE_BASE".into(),
            },
        )
        .unwrap();
        let exchange: SourceAdapterSupportPackageExchange =
            serde_json::from_str(&exported.content).unwrap();
        assert_eq!(exchange.format, super::SOURCE_ADAPTER_SUPPORT_FORMAT);
        assert_eq!(exchange.version, super::SOURCE_ADAPTER_SUPPORT_VERSION);
        assert_eq!(exchange.migration_task, "MEDICINE_BASE");
        assert_eq!(exported.diagnostic_count, 2);
        assert_eq!(
            exported.checksum,
            super::source_adapter_support_checksum(&exchange).unwrap()
        );
        assert!(!exported.content.contains("secret-host"));
        assert!(!exported.content.contains("private-tenant"));
        assert!(!exported.content.to_ascii_lowercase().contains("password"));
        assert!(exported.content.contains("PACK_FACTOR"));
        assert!(exported.content.contains("NVARCHAR2"));

        let inventory_export = export_source_adapter_support_package(
            &store,
            ExportSourceAdapterSupportPackageRequest {
                scope,
                migration_task: "INVENTORY".into(),
            },
        )
        .unwrap();
        assert_eq!(inventory_export.diagnostic_count, 1);
        assert!(inventory_export.file_name.contains("inventory"));
        assert!(inventory_export.content.contains("WAREHOUSE_STOCK"));
        assert!(!inventory_export.content.contains("DRUG_MASTER"));
    }

    #[test]
    fn support_package_checksum_matches_maintenance_tool_vector() {
        let exchange = SourceAdapterSupportPackageExchange {
            format: super::SOURCE_ADAPTER_SUPPORT_FORMAT.into(),
            version: super::SOURCE_ADAPTER_SUPPORT_VERSION,
            adapter_id: "VENDOR_HIS".into(),
            migration_task: String::new(),
            source_fingerprint: "1234567890abcdef1234".into(),
            diagnostics: Vec::new(),
            exported_at: "2026-08-27T08:05:00Z".into(),
            checksum: String::new(),
        };
        assert_eq!(
            super::source_adapter_support_checksum(&exchange).unwrap(),
            "9c0f922069e1f19b7d48d6595382c275316ee05550f9cd857297cfb7171e9761"
        );
    }

    #[test]
    fn source_template_exchange_is_portable_tamper_evident_and_credential_free() {
        let directory = tempfile::tempdir().unwrap();
        let store = LocalStore::open(&directory.path().join("mapping.sqlite")).unwrap();
        let original_scope = SourceMappingProfileScope {
            adapter_id: "generic_database".into(),
            source_identity: "oracle:secret-host:1521/legacy:his".into(),
            target_tenant_id: "private-tenant".into(),
        };
        save_source_mapping_profile(
            &store,
            SaveSourceMappingProfileRequest {
                scope: original_scope.clone(),
                adapter_version: 1,
                source_key: String::new(),
                source_key_fields: vec!["DRUG_ID".into(), "FACTORY_ID".into()],
                source_query: "SELECT DRUG_ID, FACTORY_ID, DRUG_NAME FROM HIS_DRUG".into(),
                mapping: HashMap::from([("naMed".into(), "DRUG_NAME".into())]),
                rules: HashMap::from([("naMed".into(), json!({"transform":"TRIM"}))]),
                dictionary_overrides: HashMap::from([(
                    "oracle:secret-host:1521/legacy:HIS".into(),
                    HashMap::from([(
                        "legacy.dosage-form".into(),
                        vec![crate::model::SourceDictionaryItem {
                            key: "1".into(),
                            text: "片剂".into(),
                            properties: Default::default(),
                        }],
                    )]),
                )]),
            },
        )
        .unwrap();

        let exported = export_source_mapping_template(
            &store,
            ExportSourceMappingTemplateRequest {
                scope: original_scope,
                adapter_version: "1".into(),
                template_compatible_from_version: 1,
                dictionary_scope_key: "oracle:secret-host:1521/legacy:HIS".into(),
            },
        )
        .unwrap();
        assert!(!exported.content.contains("secret-host"));
        assert!(!exported.content.contains("private-tenant"));
        assert!(!exported.content.to_ascii_lowercase().contains("password"));

        let imported_scope = SourceMappingProfileScope {
            adapter_id: "GENERIC_DATABASE".into(),
            source_identity: "oracle:new-host:1521/newhis:owner".into(),
            target_tenant_id: "tenant-b".into(),
        };
        let preview = preview_source_mapping_template(ImportSourceMappingTemplateRequest {
            scope: imported_scope.clone(),
            content: exported.content.clone(),
            adapter_version: 1,
            template_compatible_from_version: 1,
            dictionary_scope_key: "oracle:new-host:1521/newhis:OWNER".into(),
        })
        .unwrap();
        assert_eq!(preview.mapping_count, 1);
        assert_eq!(preview.rule_count, 1);
        assert_eq!(preview.source_key_fields, vec!["DRUG_ID", "FACTORY_ID"]);
        assert!(preview.has_source_query);
        assert_eq!(preview.compatibility, "CURRENT");
        assert!(!preview.requires_review);

        let mut legacy_exchange: super::SourceMappingTemplateExchange =
            serde_json::from_str(&exported.content).unwrap();
        legacy_exchange.source_key = "DRUG_ID".into();
        legacy_exchange.source_key_fields.clear();
        legacy_exchange.checksum.clear();
        legacy_exchange.checksum =
            super::source_mapping_template_checksum(&legacy_exchange).unwrap();
        let legacy_content = serde_json::to_string(&legacy_exchange).unwrap();
        assert!(!legacy_content.contains("sourceKeyFields"));
        let legacy_preview = preview_source_mapping_template(ImportSourceMappingTemplateRequest {
            scope: imported_scope.clone(),
            content: legacy_content,
            adapter_version: 1,
            template_compatible_from_version: 1,
            dictionary_scope_key: "oracle:new-host:1521/newhis:OWNER".into(),
        })
        .unwrap();
        assert_eq!(legacy_preview.source_key, "DRUG_ID");
        assert_eq!(legacy_preview.source_key_fields, vec!["DRUG_ID"]);

        let mut older_exchange: super::SourceMappingTemplateExchange =
            serde_json::from_str(&exported.content).unwrap();
        older_exchange.adapter_version = "1".into();
        older_exchange.checksum.clear();
        older_exchange.checksum = super::source_mapping_template_checksum(&older_exchange).unwrap();
        let older_preview = preview_source_mapping_template(ImportSourceMappingTemplateRequest {
            scope: imported_scope.clone(),
            content: serde_json::to_string(&older_exchange).unwrap(),
            adapter_version: 2,
            template_compatible_from_version: 2,
            dictionary_scope_key: "oracle:new-host:1521/newhis:OWNER".into(),
        })
        .unwrap();
        assert_eq!(older_preview.compatibility, "UPGRADE_REVIEW");
        assert!(older_preview.requires_review);

        let mut future_exchange = older_exchange.clone();
        future_exchange.adapter_version = "3".into();
        future_exchange.checksum.clear();
        future_exchange.checksum =
            super::source_mapping_template_checksum(&future_exchange).unwrap();
        let future_result = preview_source_mapping_template(ImportSourceMappingTemplateRequest {
            scope: imported_scope.clone(),
            content: serde_json::to_string(&future_exchange).unwrap(),
            adapter_version: 2,
            template_compatible_from_version: 1,
            dictionary_scope_key: "oracle:new-host:1521/newhis:OWNER".into(),
        });
        assert!(future_result.unwrap_err().contains("不能降级套用"));

        let imported = import_source_mapping_template(
            &store,
            ImportSourceMappingTemplateRequest {
                scope: imported_scope,
                content: exported.content.clone(),
                adapter_version: 1,
                template_compatible_from_version: 1,
                dictionary_scope_key: "oracle:new-host:1521/newhis:OWNER".into(),
            },
        )
        .unwrap();
        assert_eq!(
            imported.source_identity,
            "oracle:new-host:1521/newhis:owner"
        );
        assert_eq!(imported.target_tenant_id, "tenant-b");
        assert!(imported.source_key.is_empty());
        assert_eq!(imported.source_key_fields, vec!["DRUG_ID", "FACTORY_ID"]);
        assert_eq!(imported.mapping.get("naMed"), Some(&"DRUG_NAME".into()));
        assert_eq!(
            imported.dictionary_overrides["oracle:new-host:1521/newhis:OWNER"]
                ["legacy.dosage-form"][0]
                .text,
            "片剂"
        );

        let tampered = exported.content.replace("DRUG_NAME", "DRUG_TITLE");
        let tampered_result = preview_source_mapping_template(ImportSourceMappingTemplateRequest {
            scope: SourceMappingProfileScope {
                adapter_id: "GENERIC_DATABASE".into(),
                source_identity: "oracle:any".into(),
                target_tenant_id: "tenant-c".into(),
            },
            content: tampered,
            adapter_version: 1,
            template_compatible_from_version: 1,
            dictionary_scope_key: "oracle:any".into(),
        });
        assert!(tampered_result.unwrap_err().contains("完整性校验失败"));

        let mismatch = preview_source_mapping_template(ImportSourceMappingTemplateRequest {
            scope: SourceMappingProfileScope {
                adapter_id: "PHIS27".into(),
                source_identity: "oracle:any".into(),
                target_tenant_id: "tenant-c".into(),
            },
            content: exported.content,
            adapter_version: 1,
            template_compatible_from_version: 1,
            dictionary_scope_key: "oracle:any".into(),
        });
        assert!(mismatch.unwrap_err().contains("适配器"));
    }
}
