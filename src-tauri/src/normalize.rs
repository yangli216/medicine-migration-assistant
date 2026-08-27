use crate::model::{FieldMapping, TargetField};
use chrono::NaiveDate;
use serde_json::{Map, Number, Value};
use std::collections::HashSet;

const ALLOWED_TARGETS: &[&str] = &[
    "naMed",
    "sdMed",
    "idCstmg",
    "unitPre",
    "spec",
    "dose",
    "unitDose",
    "sdDoseUnit",
    "sdDose",
    "sdChrgitmLv",
    "sdAllergy",
    "sdAntiAcl",
    "fgAntiAppr",
    "ddd",
    "sdBasMed",
    "sdSpeMed",
    "limitAntiDay",
    "sdStorage",
    "sdPharm",
    "sdValue",
    "sdProdPlac",
    "fgPois",
    "sdPois",
    "fgAnti",
    "sdAnti",
    "sdRound",
    "sdDps",
    "fgMedRx",
    "fgBasMed",
    "fgSkintest",
    "sdSkintest",
    "dripRate",
    "dftDoseOnce",
    "dftUsage",
    "dftFreq",
    "fgTcd",
    "fgSingle",
    "fgRegister",
    "naFac",
    "naFacShort",
    "pyFac",
    "idFac",
    "naMedPro",
    "unitSale",
    "unitSaleFactor",
    "specSale",
    "priceSale",
    "pricePur",
    "cdAppr",
    "cdBar",
    "cdMedPro",
    "sdPer",
    "per",
    "fgCollPur",
    "fgImport",
];

const EMPTY_VALUE_MAPPING_SOURCE: &str = "<空值>";

pub fn target_fields() -> Vec<TargetField> {
    vec![
        field(
            "naMed",
            "医疗物品通用名",
            true,
            "药品基本信息",
            "text",
            "写入 hi_bd_med.na_med",
        ),
        field(
            "sdMed",
            "物品类型编码",
            true,
            "药品基本信息",
            "dictionary",
            "以当前租户 rbmh.base.med.articleType 实时字典为准",
        ),
        field(
            "idCstmg",
            "费用归并主键",
            true,
            "药品基本信息",
            "id",
            "新系统费用归并记录的24位主键",
        ),
        field(
            "sdDose",
            "剂型编码",
            true,
            "药品基本信息",
            "dictionary",
            "新系统药品剂型编码",
        ),
        field(
            "unitPre",
            "制剂单位",
            true,
            "药品基本信息",
            "text",
            "最小制剂单位，例如片、粒、ml",
        ),
        field(
            "dose",
            "制剂剂量",
            false,
            "药品基本信息",
            "decimal",
            "耗材可不填",
        ),
        field(
            "unitDose",
            "剂量单位",
            false,
            "药品基本信息",
            "text",
            "耗材可不填",
        ),
        field(
            "spec",
            "制剂规格",
            false,
            "药品基本信息",
            "text",
            "为空时按剂量/制剂单位生成",
        ),
        field(
            "dftUsage",
            "默认给药方法编码",
            false,
            "用药规则",
            "dictionary",
            "草药、耗材可不填",
        ),
        field(
            "dftFreq",
            "默认频次编码",
            false,
            "用药规则",
            "dictionary",
            "目标字段允许为空；来源有默认频次时按新系统频次字典映射",
        ),
        field(
            "idFac",
            "生产厂家主键",
            false,
            "厂家与商品",
            "id",
            "已建立厂家对照时优先使用",
        ),
        field(
            "naFac",
            "生产厂家名称",
            false,
            "厂家与商品",
            "text",
            "厂家主键为空时用于精确匹配",
        ),
        field(
            "naFacShort",
            "生产厂家简称",
            false,
            "厂家与商品",
            "text",
            "新建厂家时写入 hi_bd_fac.na_fac_short",
        ),
        field(
            "pyFac",
            "生产厂家拼音码",
            false,
            "厂家与商品",
            "text",
            "新建厂家时写入 hi_bd_fac.py",
        ),
        field(
            "naMedPro",
            "商品名",
            false,
            "厂家与商品",
            "text",
            "为空时使用通用名",
        ),
        field(
            "unitSale",
            "零售包装单位",
            false,
            "包装与价格",
            "text",
            "存在商品信息时必填，例如盒、瓶、支",
        ),
        field(
            "unitSaleFactor",
            "包装系数",
            false,
            "包装与价格",
            "integer",
            "存在商品信息时必填；零售包装单位相对制剂单位的正整数倍数",
        ),
        field(
            "specSale",
            "零售包装规格",
            false,
            "包装与价格",
            "text",
            "为空时自动生成",
        ),
        field(
            "pricePur",
            "进货价格",
            false,
            "包装与价格",
            "decimal",
            "存在商品信息时必填；允许为0，不允许负数",
        ),
        field(
            "priceSale",
            "零售价格",
            false,
            "包装与价格",
            "decimal",
            "存在商品信息时必填；允许为0，不允许负数",
        ),
        field(
            "cdAppr",
            "批准文号",
            false,
            "监管编码",
            "text",
            "药品批准文号",
        ),
        field("cdBar", "条形码", false, "监管编码", "text", "商品条形码"),
        field(
            "cdMedPro",
            "货品码",
            false,
            "监管编码",
            "text",
            "三方商品编码",
        ),
        field(
            "fgMedRx",
            "处方药标志",
            false,
            "监管属性",
            "dictionary",
            "目标字典：1处方药、2非处方药",
        ),
        field(
            "fgCollPur",
            "集采标志",
            false,
            "标志",
            "boolean01",
            "0否、1是",
        ),
        field(
            "sdChrgitmLv",
            "医保等级",
            false,
            "标志",
            "dictionary",
            "1甲类、2乙类、3自费",
        ),
        field(
            "sdDoseUnit",
            "剂量单位编码",
            false,
            "用药规则",
            "dictionary",
            "新系统剂量单位字典编码",
        ),
        field(
            "sdAllergy",
            "过敏类别",
            false,
            "用药规则",
            "dictionary",
            "新系统过敏类别编码",
        ),
        field(
            "sdAntiAcl",
            "抗菌药物管理级别",
            false,
            "抗菌药物",
            "dictionary",
            "抗菌药物管理级别编码",
        ),
        field(
            "fgAntiAppr",
            "抗菌药物审批标志",
            false,
            "抗菌药物",
            "boolean01",
            "0否、1是",
        ),
        field("ddd", "DDD 值", false, "抗菌药物", "decimal", "限定日剂量"),
        field(
            "limitAntiDay",
            "抗菌药物限用天数",
            false,
            "抗菌药物",
            "decimal",
            "不得小于0",
        ),
        field(
            "sdBasMed",
            "基药类型",
            false,
            "监管属性",
            "dictionary",
            "新系统基药类型编码",
        ),
        field(
            "sdSpeMed",
            "特殊药品类型",
            false,
            "监管属性",
            "dictionary",
            "新系统特殊药品编码",
        ),
        field(
            "sdStorage",
            "药品储藏方式",
            false,
            "监管属性",
            "dictionary",
            "新系统储藏字典编码",
        ),
        field(
            "sdPharm",
            "药理分类",
            false,
            "监管属性",
            "dictionary",
            "新系统药理分类编码",
        ),
        field(
            "sdValue",
            "贵重等级",
            false,
            "监管属性",
            "dictionary",
            "新系统贵重等级编码",
        ),
        field(
            "sdProdPlac",
            "产地编码",
            false,
            "厂家与商品",
            "dictionary",
            "新系统产地字典编码",
        ),
        field(
            "fgPois",
            "毒性药品标志",
            false,
            "监管属性",
            "boolean01",
            "0否、1是",
        ),
        field(
            "sdPois",
            "毒性药品类型",
            false,
            "监管属性",
            "dictionary",
            "新系统毒性类型编码",
        ),
        field(
            "fgAnti",
            "抗菌药物标志",
            false,
            "抗菌药物",
            "boolean01",
            "0否、1是",
        ),
        field(
            "sdAnti",
            "抗菌药物分类",
            false,
            "抗菌药物",
            "dictionary",
            "新系统抗菌药物分类编码",
        ),
        field(
            "sdRound",
            "取整策略",
            false,
            "用药规则",
            "dictionary",
            "新系统取整策略编码",
        ),
        field(
            "sdDps",
            "发药方式",
            false,
            "用药规则",
            "dictionary",
            "新系统发药方式编码",
        ),
        field(
            "fgBasMed",
            "基本药物标志",
            false,
            "监管属性",
            "boolean01",
            "0否、1是",
        ),
        field(
            "fgSkintest",
            "皮试标志",
            false,
            "用药规则",
            "boolean01",
            "0否、1是",
        ),
        field(
            "sdSkintest",
            "皮试类型",
            false,
            "用药规则",
            "dictionary",
            "新系统皮试类型编码",
        ),
        field(
            "dripRate",
            "默认滴速",
            false,
            "用药规则",
            "text",
            "输液默认滴速",
        ),
        field(
            "dftDoseOnce",
            "默认一次用量",
            false,
            "用药规则",
            "decimal",
            "草药未填时默认使用制剂剂量",
        ),
        field(
            "fgTcd",
            "中药饮片标志",
            false,
            "标志",
            "boolean01",
            "0否、1是",
        ),
        field(
            "fgSingle",
            "单品标志",
            false,
            "标志",
            "boolean01",
            "0否、1是",
        ),
        field(
            "fgRegister",
            "注册证管理标志",
            false,
            "标志",
            "boolean01",
            "0否、1是",
        ),
        field(
            "sdPer",
            "加成类型",
            false,
            "包装与价格",
            "dictionary",
            "新系统加成类型编码",
        ),
        field(
            "per",
            "加成比例",
            false,
            "包装与价格",
            "decimal",
            "允许为空，不得小于0",
        ),
        field(
            "fgImport",
            "进口药品标志",
            false,
            "标志",
            "boolean01",
            "0否、1是",
        ),
    ]
}

fn field(
    key: &str,
    label: &str,
    required: bool,
    group: &str,
    value_type: &str,
    description: &str,
) -> TargetField {
    TargetField {
        key: key.into(),
        label: label.into(),
        required,
        group: group.into(),
        value_type: value_type.into(),
        description: description.into(),
        dictionary_id: crate::target_dictionary::dictionary_id_for(key)
            .unwrap_or_default()
            .into(),
    }
}

pub fn normalize(source: &Map<String, Value>, mappings: &[FieldMapping]) -> Map<String, Value> {
    let allowed = ALLOWED_TARGETS.iter().copied().collect::<HashSet<_>>();
    if mappings.is_empty() {
        return source
            .iter()
            .filter(|(key, _)| allowed.contains(key.as_str()))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
    }
    let mut target = Map::new();
    let mut explicitly_ignored = Vec::new();
    for mapping in mappings {
        if !allowed.contains(mapping.target_field.as_str()) {
            continue;
        }
        let mut value = composed_source_value(source, mapping);
        let mut ignored = false;
        let condition_applies = field_condition_matches(source, mapping);
        if !condition_applies {
            value = match mapping.condition_else.trim().to_ascii_uppercase().as_str() {
                "EMPTY" => Value::Null,
                "DEFAULT" if !mapping.default_value.trim().is_empty() => {
                    Value::String(mapping.default_value.clone())
                }
                "DEFAULT" => Value::Null,
                _ => value,
            };
        } else if is_blank(&value) {
            if let Some(mapped) = mapped_value(mapping, EMPTY_VALUE_MAPPING_SOURCE) {
                value = mapped.clone();
                ignored = mapped.is_null();
            } else if !mapping.default_value.trim().is_empty() {
                value = Value::String(mapping.default_value.clone());
            }
        } else {
            let lookup = value_text(&value);
            if let Some(mapped) = mapped_value(mapping, &lookup) {
                value = mapped.clone();
                ignored = mapped.is_null();
            }
        }
        if ignored && validation_ignore_allowed(&mapping.target_field) {
            explicitly_ignored.push(Value::String(mapping.target_field.clone()));
        }
        let normalized = if !condition_applies
            && !matches!(
                mapping.condition_else.trim().to_ascii_uppercase().as_str(),
                "DEFAULT"
            ) {
            if is_blank(&value) {
                Value::Null
            } else {
                Value::String(value_text(&value))
            }
        } else {
            truncate(transform(value, &mapping.transform), mapping)
        };
        target.insert(mapping.target_field.clone(), normalized);
    }
    if !explicitly_ignored.is_empty() {
        target.insert(
            "_ignoredValidationFields".into(),
            Value::Array(explicitly_ignored),
        );
    }
    target
}

pub fn apply_cost_merge_mapping(
    target: &mut Map<String, Value>,
    cost_merge_mappings: &Map<String, Value>,
) {
    if !target.get("idCstmg").map(is_blank).unwrap_or(true) {
        return;
    }
    let med_type = target.get("sdMed").map(value_text).unwrap_or_default();
    if let Some(cost_merge_id) = cost_merge_mappings.get(&med_type) {
        let value = value_text(cost_merge_id);
        if !value.is_empty() {
            target.insert("idCstmg".into(), Value::String(value));
        }
    }
}

pub fn validate(data: &Map<String, Value>) -> Vec<String> {
    let mut errors = Vec::new();
    for (key, label) in [
        ("naMed", "医疗物品通用名"),
        ("sdMed", "物品类型编码"),
        ("idCstmg", "费用归并主键"),
        ("sdDose", "剂型编码"),
        ("unitPre", "制剂单位"),
    ] {
        require(data, key, label, &mut errors);
    }
    let med_type = data.get("sdMed").map(value_text).unwrap_or_default();
    if !med_type.is_empty()
        && (med_type.len() > 4 || !med_type.chars().all(|character| character.is_ascii_digit()))
    {
        errors.push("物品类型必须使用新系统的数字字典编码（最多4位）".into());
    }
    if med_type != "5" {
        require(data, "dose", "制剂剂量", &mut errors);
        require(data, "unitDose", "剂量单位", &mut errors);
    }
    if med_type != "3" && med_type != "5" {
        require(data, "dftUsage", "默认给药方法编码", &mut errors);
    }
    if has_product_data(data) {
        for (key, label) in [
            ("unitSale", "零售包装单位"),
            ("unitSaleFactor", "包装系数"),
            ("pricePur", "进货价格"),
            ("priceSale", "零售价格"),
        ] {
            require(data, key, label, &mut errors);
        }
        if blank_at(data, "idFac") && blank_at(data, "naFac") {
            errors.push("生产厂家主键或名称至少填写一项".into());
        }
    }
    positive_number(data, "unitSaleFactor", "包装系数", false, &mut errors);
    positive_number(data, "pricePur", "进货价格", true, &mut errors);
    positive_number(data, "priceSale", "零售价格", true, &mut errors);
    positive_number(data, "limitAntiDay", "抗菌药物限用天数", true, &mut errors);
    positive_number(data, "per", "加成比例", true, &mut errors);
    for (key, label) in [("idCstmg", "费用归并主键"), ("idFac", "生产厂家主键")] {
        validate_reference_id(data, key, label, &mut errors);
    }
    for (key, label) in [
        ("fgAntiAppr", "抗菌药物审批标志"),
        ("fgPois", "毒性药品标志"),
        ("fgAnti", "抗菌药物标志"),
        ("fgBasMed", "基本药物标志"),
        ("fgSkintest", "皮试标志"),
        ("fgTcd", "中药饮片标志"),
        ("fgSingle", "单品标志"),
        ("fgRegister", "注册证管理标志"),
        ("fgCollPur", "集采标志"),
        ("fgImport", "进口药品标志"),
    ] {
        validate_boolean(data, key, label, &mut errors);
    }
    for (key, label, max) in [
        ("naMed", "医疗物品通用名", 180),
        ("unitPre", "制剂单位", 64),
        ("spec", "制剂规格", 180),
        ("unitDose", "剂量单位", 64),
        ("dftUsage", "默认给药方法编码", 64),
        ("dftFreq", "默认频次编码", 64),
        ("naFac", "生产厂家名称", 180),
        ("naFacShort", "生产厂家简称", 32),
        ("pyFac", "生产厂家拼音码", 64),
        ("naMedPro", "商品名", 180),
        ("unitSale", "零售包装单位", 64),
        ("specSale", "零售包装规格", 180),
        ("cdAppr", "批准文号", 64),
        ("cdBar", "条形码", 64),
        ("cdMedPro", "货品码", 64),
    ] {
        max_length(data, key, label, max, &mut errors);
    }
    errors
}

pub fn has_product_data(data: &Map<String, Value>) -> bool {
    [
        "idFac",
        "naFac",
        "naFacShort",
        "pyFac",
        "naMedPro",
        "unitSale",
        "unitSaleFactor",
        "specSale",
        "priceSale",
        "pricePur",
        "cdAppr",
        "cdBar",
        "cdMedPro",
    ]
    .iter()
    .any(|key| !blank_at(data, key))
}

fn validate_reference_id(
    data: &Map<String, Value>,
    key: &str,
    label: &str,
    errors: &mut Vec<String>,
) {
    let value = data.get(key).map(value_text).unwrap_or_default();
    if value.is_empty() {
        return;
    }
    if value.len() != 24 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        errors.push(format!("{label}必须是24位十六进制主键"));
    }
}

fn validate_boolean(data: &Map<String, Value>, key: &str, label: &str, errors: &mut Vec<String>) {
    let value = data.get(key).map(value_text).unwrap_or_default();
    if !value.is_empty() && value != "0" && value != "1" {
        errors.push(format!("{label}必须为0或1"));
    }
}

fn max_length(
    data: &Map<String, Value>,
    key: &str,
    label: &str,
    max: usize,
    errors: &mut Vec<String>,
) {
    let value = data.get(key).map(value_text).unwrap_or_default();
    if value.chars().count() > max {
        errors.push(format!("{label}不能超过{max}个字符"));
    }
}

pub fn value_text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.trim().to_string(),
        Value::Bool(flag) => {
            if *flag {
                "1".into()
            } else {
                "0".into()
            }
        }
        Value::Number(number) => number.to_string(),
        other => other.to_string(),
    }
}

fn transform(value: Value, operation: &str) -> Value {
    if is_blank(&value) {
        return Value::Null;
    }
    let text = value_text(&value);
    match operation.trim().to_ascii_uppercase().as_str() {
        "UPPER" => Value::String(text.to_uppercase()),
        "LOWER" => Value::String(text.to_lowercase()),
        "COLLAPSE_WHITESPACE" => {
            Value::String(text.split_whitespace().collect::<Vec<_>>().join(" "))
        }
        "REMOVE_WHITESPACE" => Value::String(
            text.chars()
                .filter(|character| !character.is_whitespace())
                .collect(),
        ),
        "INTEGER" => parse_integer(&text)
            .map(Value::from)
            .unwrap_or(Value::String(text)),
        "DECIMAL" => normalized_number(&text)
            .parse::<f64>()
            .ok()
            .and_then(Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::String(text)),
        "BOOLEAN_01" => boolean_01(&text)
            .map(|flag| Value::String(flag.into()))
            .unwrap_or(Value::String(text)),
        "DATE_YYYY_MM_DD" => normalize_date(&text)
            .map(Value::String)
            .unwrap_or(Value::String(text)),
        _ => Value::String(text),
    }
}

fn composed_source_value(source: &Map<String, Value>, mapping: &FieldMapping) -> Value {
    let mut fields = Vec::with_capacity(1 + mapping.additional_source_fields.len());
    if !mapping.source_field.is_empty() {
        fields.push(mapping.source_field.as_str());
    }
    fields.extend(
        mapping
            .additional_source_fields
            .iter()
            .filter(|field| !field.is_empty())
            .map(String::as_str),
    );
    if fields.len() <= 1 {
        return fields
            .first()
            .and_then(|field| source.get(*field))
            .cloned()
            .unwrap_or(Value::Null);
    }
    let parts = fields
        .into_iter()
        .filter_map(|field| source.get(field))
        .filter(|value| !is_blank(value))
        .map(value_text)
        .collect::<Vec<_>>();
    if parts.is_empty() {
        Value::Null
    } else {
        Value::String(parts.join(&mapping.join_separator))
    }
}

fn field_condition_matches(source: &Map<String, Value>, mapping: &FieldMapping) -> bool {
    let operator = mapping.condition_operator.trim().to_ascii_uppercase();
    if operator.is_empty() || operator == "ALWAYS" || mapping.condition_field.is_empty() {
        return true;
    }
    let actual = source
        .get(&mapping.condition_field)
        .cloned()
        .unwrap_or(Value::Null);
    let text = value_text(&actual);
    let expected = mapping.condition_value.trim();
    match operator.as_str() {
        "EMPTY" => is_blank(&actual),
        "NOT_EMPTY" => !is_blank(&actual),
        "EQUALS" => text == expected,
        "NOT_EQUALS" => text != expected,
        "CONTAINS" => text.contains(expected),
        _ => true,
    }
}

fn truncate(value: Value, mapping: &FieldMapping) -> Value {
    if mapping.max_length == 0 {
        return value;
    }
    let Value::String(text) = value else {
        return value;
    };
    let characters = text.chars().collect::<Vec<_>>();
    if characters.len() <= mapping.max_length {
        return Value::String(text);
    }
    let truncated = if mapping.truncate_mode.eq_ignore_ascii_case("KEEP_END") {
        characters[characters.len() - mapping.max_length..]
            .iter()
            .collect()
    } else {
        characters[..mapping.max_length].iter().collect()
    };
    Value::String(truncated)
}

fn mapped_value<'a>(mapping: &'a FieldMapping, lookup: &str) -> Option<&'a Value> {
    mapping.value_mappings.get(lookup).or_else(|| {
        mapping
            .value_mapping_case_insensitive
            .then(|| {
                mapping
                    .value_mappings
                    .iter()
                    .find(|(source, _)| source.to_lowercase() == lookup.to_lowercase())
                    .map(|(_, value)| value)
            })
            .flatten()
    })
}

fn normalized_number(text: &str) -> String {
    text.replace([',', '，'], "").replace(' ', "")
}

fn parse_integer(text: &str) -> Option<i64> {
    let normalized = normalized_number(text);
    normalized.parse::<i64>().ok().or_else(|| {
        normalized
            .parse::<f64>()
            .ok()
            .filter(|number| number.is_finite() && number.fract() == 0.0)
            .and_then(|number| {
                (number >= i64::MIN as f64 && number <= i64::MAX as f64).then_some(number as i64)
            })
    })
}

fn boolean_01(text: &str) -> Option<&'static str> {
    let normalized = text.trim().to_lowercase();
    if [
        "1",
        "true",
        "yes",
        "是",
        "y",
        "on",
        "启用",
        "有",
        "需要",
        "需",
        "有效",
        "正常",
        "rx",
        "处方药",
        "处方药品",
    ]
    .contains(&normalized.as_str())
        || (normalized.contains("处方") && !normalized.contains("非处方"))
    {
        Some("1")
    } else if [
        "0",
        "2",
        "false",
        "no",
        "否",
        "n",
        "off",
        "停用",
        "无",
        "不需要",
        "无需",
        "无效",
        "otc",
        "非处方药",
        "非处方药品",
    ]
    .contains(&normalized.as_str())
        || normalized.contains("非处方")
        || normalized.contains("otc")
    {
        Some("0")
    } else {
        None
    }
}

fn normalize_date(text: &str) -> Option<String> {
    let trimmed = text.trim();
    let date_part = trimmed.split([' ', 'T']).next().unwrap_or(trimmed);
    ["%Y-%m-%d", "%Y/%m/%d", "%Y.%m.%d", "%Y%m%d"]
        .iter()
        .find_map(|format| NaiveDate::parse_from_str(date_part, format).ok())
        .map(|date| date.format("%Y-%m-%d").to_string())
}

fn require(data: &Map<String, Value>, key: &str, label: &str, errors: &mut Vec<String>) {
    if blank_at(data, key) && !is_validation_ignored(data, key) {
        errors.push(format!("{}不能为空", label));
    }
}

fn validation_ignore_allowed(key: &str) -> bool {
    matches!(
        key,
        "dftUsage"
            | "dftFreq"
            | "sdRound"
            | "sdDps"
            | "sdChrgitmLv"
            | "sdAllergy"
            | "sdStorage"
            | "sdSpeMed"
            | "fgMedRx"
            | "fgAntiAppr"
            | "sdProdPlac"
            | "fgPois"
            | "fgAnti"
            | "fgTcd"
            | "fgSingle"
            | "fgRegister"
            | "fgCollPur"
            | "fgImport"
    )
}

fn is_validation_ignored(data: &Map<String, Value>, key: &str) -> bool {
    validation_ignore_allowed(key)
        && data
            .get("_ignoredValidationFields")
            .and_then(Value::as_array)
            .is_some_and(|fields| fields.iter().any(|field| field.as_str() == Some(key)))
}

fn positive_number(
    data: &Map<String, Value>,
    key: &str,
    label: &str,
    allow_zero: bool,
    errors: &mut Vec<String>,
) {
    if blank_at(data, key) {
        return;
    }
    match data
        .get(key)
        .map(value_text)
        .unwrap_or_default()
        .parse::<f64>()
    {
        Ok(number) if (allow_zero && number >= 0.0) || (!allow_zero && number > 0.0) => {}
        Ok(_) => errors.push(format!(
            "{}{}",
            label,
            if allow_zero {
                "不能小于0"
            } else {
                "必须大于0"
            }
        )),
        Err(_) => errors.push(format!("{}必须是数字", label)),
    }
}

fn blank_at(data: &Map<String, Value>, key: &str) -> bool {
    data.get(key).map(is_blank).unwrap_or(true)
}

fn is_blank(value: &Value) -> bool {
    matches!(value, Value::Null) || matches!(value, Value::String(text) if text.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::{
        apply_cost_merge_mapping, boolean_01, normalize, target_fields, validate, ALLOWED_TARGETS,
    };
    use crate::model::FieldMapping;
    use serde_json::{json, Map, Value};

    #[test]
    fn mapping_applies_trim_and_dictionary_values() {
        let source = json!({"TYPE_NAME":" 西药 "}).as_object().unwrap().clone();
        let mapping: FieldMapping = serde_json::from_value(json!({
            "sourceField":"TYPE_NAME","targetField":"sdMed","transform":"TRIM",
            "valueMappings":{"西药":"1"}
        }))
        .unwrap();
        assert_eq!(
            normalize(&source, &[mapping]).get("sdMed"),
            Some(&Value::String("1".into()))
        );
    }

    #[test]
    fn explicit_empty_dictionary_mapping_precedes_the_generic_default() {
        for source_value in [Value::Null, Value::String("   ".into())] {
            let source = Map::from_iter([("TYPE".into(), source_value)]);
            let mapping: FieldMapping = serde_json::from_value(json!({
                "sourceField":"TYPE","targetField":"sdMed","transform":"TRIM",
                "defaultValue":"2","valueMappings":{"<空值>":"1"}
            }))
            .unwrap();
            assert_eq!(
                normalize(&source, &[mapping]).get("sdMed"),
                Some(&Value::String("1".into()))
            );
        }
    }

    #[test]
    fn cost_merge_is_derived_from_normalized_medicine_type() {
        let mut target = json!({ "sdMed": "1" }).as_object().unwrap().clone();
        let mappings = json!({ "1": "63aa8b1b3c6f491981ba4221" })
            .as_object()
            .unwrap()
            .clone();
        apply_cost_merge_mapping(&mut target, &mappings);
        assert_eq!(
            target.get("idCstmg"),
            Some(&Value::String("63aa8b1b3c6f491981ba4221".into()))
        );
    }

    #[test]
    fn mapping_supports_case_insensitive_dictionary_and_common_transforms() {
        let source = json!({
            "FORM":"cap",
            "NAME":"  阿莫西林   胶囊  ",
            "FLAG":"未知",
            "COUNT":"1,024.0",
            "DATE":"2026/08/04 12:30:00"
        })
        .as_object()
        .unwrap()
        .clone();
        let mappings: Vec<FieldMapping> = serde_json::from_value(json!([
            {
                "sourceField":"FORM","targetField":"sdDose","transform":"UPPER",
                "valueMappings":{"CAP":"capsule"},"valueMappingCaseInsensitive":true
            },
            {"sourceField":"NAME","targetField":"naMed","transform":"COLLAPSE_WHITESPACE"},
            {"sourceField":"FLAG","targetField":"fgMedRx","transform":"BOOLEAN_01"},
            {"sourceField":"COUNT","targetField":"unitSaleFactor","transform":"INTEGER"},
            {"sourceField":"DATE","targetField":"cdAppr","transform":"DATE_YYYY_MM_DD"}
        ]))
        .unwrap();
        let normalized = normalize(&source, &mappings);
        assert_eq!(
            normalized.get("sdDose"),
            Some(&Value::String("CAPSULE".into()))
        );
        assert_eq!(
            normalized.get("naMed"),
            Some(&Value::String("阿莫西林 胶囊".into()))
        );
        assert_eq!(
            normalized.get("fgMedRx"),
            Some(&Value::String("未知".into()))
        );
        assert_eq!(normalized.get("unitSaleFactor"), Some(&Value::from(1024)));
        assert_eq!(
            normalized.get("cdAppr"),
            Some(&Value::String("2026-08-04".into()))
        );
    }

    #[test]
    fn mapping_combines_non_blank_fields_and_truncates_by_characters() {
        let source = json!({
            "NAME":"阿莫西林",
            "SPEC":"",
            "UNIT":"胶囊",
            "APPROVAL":"国药准字H123456"
        })
        .as_object()
        .unwrap()
        .clone();
        let mappings: Vec<FieldMapping> = serde_json::from_value(json!([
            {
                "sourceField":"NAME","additionalSourceFields":["SPEC","UNIT"],
                "joinSeparator":" / ","targetField":"naMed","transform":"TRIM",
                "maxLength":7,"truncateMode":"KEEP_START"
            },
            {
                "sourceField":"APPROVAL","targetField":"cdAppr","transform":"TRIM",
                "maxLength":6,"truncateMode":"KEEP_END"
            }
        ]))
        .unwrap();
        let normalized = normalize(&source, &mappings);
        assert_eq!(
            normalized.get("naMed"),
            Some(&Value::String("阿莫西林 / ".into()))
        );
        assert_eq!(
            normalized.get("cdAppr"),
            Some(&Value::String("123456".into()))
        );
    }

    #[test]
    fn mapping_condition_supports_empty_keep_and_default_fallbacks() {
        let source = json!({"NAME":"  青霉素  ","ACTIVE":"0"})
            .as_object()
            .unwrap()
            .clone();
        let mappings: Vec<FieldMapping> = serde_json::from_value(json!([
            {
                "sourceField":"NAME","targetField":"naMed","transform":"TRIM",
                "conditionField":"ACTIVE","conditionOperator":"EQUALS",
                "conditionValue":"1","conditionElse":"KEEP"
            },
            {
                "sourceField":"NAME","targetField":"naMedPro","transform":"TRIM",
                "conditionField":"ACTIVE","conditionOperator":"EQUALS",
                "conditionValue":"1","conditionElse":"EMPTY"
            },
            {
                "sourceField":"NAME","targetField":"naFac","transform":"TRIM",
                "defaultValue":"备用厂家","conditionField":"ACTIVE",
                "conditionOperator":"EQUALS","conditionValue":"1",
                "conditionElse":"DEFAULT"
            }
        ]))
        .unwrap();
        let normalized = normalize(&source, &mappings);
        assert_eq!(
            normalized.get("naMed"),
            Some(&Value::String("青霉素".into()))
        );
        assert_eq!(normalized.get("naMedPro"), Some(&Value::Null));
        assert_eq!(
            normalized.get("naFac"),
            Some(&Value::String("备用厂家".into()))
        );
    }

    #[test]
    fn validation_reports_conditional_fields() {
        let data = Map::new();
        let errors = validate(&data);
        assert!(errors.iter().any(|item| item.contains("医疗物品通用名")));
        assert!(!errors.iter().any(|item| item.contains("生产厂家")));

        let product = json!({
            "naMed":"测试药品","sdMed":"5","idCstmg":"66aa10244f0d4826ac110001",
            "sdDose":"1","unitPre":"个","naMedPro":"测试商品"
        })
        .as_object()
        .unwrap()
        .clone();
        let product_errors = validate(&product);
        assert!(product_errors.iter().any(|item| item.contains("生产厂家")));
        assert!(product_errors
            .iter()
            .any(|item| item.contains("零售包装单位")));
    }

    #[test]
    fn base_medicine_without_product_is_valid() {
        let data = json!({
            "naMed":"测试耗材","sdMed":"5","idCstmg":"66aa10244f0d4826ac110001",
            "sdDose":"1","unitPre":"个"
        })
        .as_object()
        .unwrap()
        .clone();
        assert!(validate(&data).is_empty());
    }

    #[test]
    fn regular_medicine_allows_blank_default_frequency() {
        let data = json!({
            "naMed":"测试药品","sdMed":"1","idCstmg":"66aa10244f0d4826ac110001",
            "sdDose":"1","unitPre":"片","dose":"1","unitDose":"mg","dftUsage":"1"
        })
        .as_object()
        .unwrap()
        .clone();
        let errors = validate(&data);
        assert!(!errors.iter().any(|item| item.contains("默认频次")));
    }

    #[test]
    fn explicitly_ignored_usage_is_left_blank_and_passes_conditional_validation() {
        let source = json!({"USAGE_CODE":"9"}).as_object().unwrap().clone();
        let mapping: FieldMapping = serde_json::from_value(json!({
            "sourceField":"USAGE_CODE","targetField":"dftUsage","transform":"TRIM",
            "valueMappings":{"9":null}
        }))
        .unwrap();
        let mut normalized = normalize(&source, &[mapping]);
        normalized.extend(
            json!({
                "naMed":"测试药品","sdMed":"1","idCstmg":"66aa10244f0d4826ac110001",
                "sdDose":"1","unitPre":"片","dose":"1","unitDose":"mg"
            })
            .as_object()
            .unwrap()
            .clone(),
        );
        assert_eq!(normalized.get("dftUsage"), Some(&Value::Null));
        assert!(validate(&normalized).is_empty());
    }

    #[test]
    fn hard_required_identity_field_cannot_be_validation_ignored() {
        let source = json!({"NAME":"9"}).as_object().unwrap().clone();
        let mapping: FieldMapping = serde_json::from_value(json!({
            "sourceField":"NAME","targetField":"naMed","transform":"TRIM",
            "valueMappings":{"9":null}
        }))
        .unwrap();
        let mut normalized = normalize(&source, &[mapping]);
        normalized.extend(
            json!({
                "sdMed":"5","idCstmg":"66aa10244f0d4826ac110001",
                "sdDose":"1","unitPre":"个"
            })
            .as_object()
            .unwrap()
            .clone(),
        );
        assert!(validate(&normalized)
            .iter()
            .any(|error| error.contains("医疗物品通用名")));
    }

    #[test]
    fn boolean_transform_accepts_common_legacy_flag_conventions() {
        for (source, expected) in [
            ("2", "0"),
            ("OTC", "0"),
            ("非处方药品（OTC）", "0"),
            ("不需要", "0"),
            ("RX", "1"),
            ("处方药品（RX）", "1"),
            ("需要", "1"),
        ] {
            assert_eq!(boolean_01(source), Some(expected));
        }
    }

    #[test]
    fn every_supported_write_field_is_visible_in_mapping_catalog() {
        let visible = target_fields()
            .into_iter()
            .map(|field| field.key)
            .collect::<std::collections::HashSet<_>>();
        let allowed = ALLOWED_TARGETS
            .iter()
            .map(|field| (*field).to_string())
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(visible, allowed);
    }

    #[test]
    fn validation_rejects_non_object_reference_and_invalid_flag() {
        let data = json!({
            "naMed":"测试药品","sdMed":"1","idCstmg":"bad-id","sdDose":"1",
            "unitPre":"片","dose":"1","unitDose":"mg","dftUsage":"1","dftFreq":"1",
            "naFac":"测试厂家","unitSale":"盒","unitSaleFactor":"10",
            "pricePur":"0","priceSale":"0","fgMedRx":"2","fgAntiAppr":"是"
        })
        .as_object()
        .unwrap()
        .clone();
        let errors = validate(&data);
        assert!(errors.iter().any(|item| item.contains("24位十六进制")));
        assert!(errors.iter().any(|item| item.contains("必须为0或1")));
    }
}
