use crate::model::{FieldMapping, TargetField};
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
    "fgPri",
    "naFac",
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
            "1西药、2中成药、3草药、4保健品、5耗材、9其他",
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
            "草药、耗材可不填",
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
            true,
            "包装与价格",
            "text",
            "例如盒、瓶、支",
        ),
        field(
            "unitSaleFactor",
            "包装系数",
            true,
            "包装与价格",
            "integer",
            "零售包装单位相对制剂单位的正整数倍数",
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
            true,
            "包装与价格",
            "decimal",
            "允许为0，不允许负数",
        ),
        field(
            "priceSale",
            "零售价格",
            true,
            "包装与价格",
            "decimal",
            "允许为0，不允许负数",
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
            "标志",
            "boolean01",
            "0非处方、1处方",
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
            "fgPri",
            "机构私有标志",
            false,
            "可见范围",
            "boolean01",
            "1时仅当前机构可见",
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
    for mapping in mappings {
        if !allowed.contains(mapping.target_field.as_str()) {
            continue;
        }
        let mut value = source
            .get(&mapping.source_field)
            .cloned()
            .unwrap_or(Value::Null);
        if is_blank(&value) && !mapping.default_value.trim().is_empty() {
            value = Value::String(mapping.default_value.clone());
        }
        let lookup = value_text(&value);
        if let Some(mapped) = mapping.value_mappings.get(&lookup) {
            value = mapped.clone();
        }
        target.insert(
            mapping.target_field.clone(),
            transform(value, &mapping.transform),
        );
    }
    target
}

pub fn validate(data: &Map<String, Value>) -> Vec<String> {
    let mut errors = Vec::new();
    for (key, label) in [
        ("naMed", "医疗物品通用名"),
        ("sdMed", "物品类型编码"),
        ("idCstmg", "费用归并主键"),
        ("sdDose", "剂型编码"),
        ("unitPre", "制剂单位"),
        ("unitSale", "零售包装单位"),
        ("unitSaleFactor", "包装系数"),
        ("pricePur", "进货价格"),
        ("priceSale", "零售价格"),
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
        require(data, "dftFreq", "默认频次编码", &mut errors);
    }
    if blank_at(data, "idFac") && blank_at(data, "naFac") {
        errors.push("生产厂家主键或名称至少填写一项".into());
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
        ("fgMedRx", "处方药标志"),
        ("fgBasMed", "基本药物标志"),
        ("fgSkintest", "皮试标志"),
        ("fgTcd", "中药饮片标志"),
        ("fgSingle", "单品标志"),
        ("fgRegister", "注册证管理标志"),
        ("fgPri", "机构私有标志"),
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
        "INTEGER" => text
            .parse::<i64>()
            .map(Value::from)
            .unwrap_or(Value::String(text)),
        "DECIMAL" => text
            .parse::<f64>()
            .ok()
            .and_then(Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::String(text)),
        "BOOLEAN_01" => Value::String(
            if ["1", "true", "yes", "是", "y"].contains(&text.to_ascii_lowercase().as_str()) {
                "1"
            } else {
                "0"
            }
            .into(),
        ),
        _ => Value::String(text),
    }
}

fn require(data: &Map<String, Value>, key: &str, label: &str, errors: &mut Vec<String>) {
    if blank_at(data, key) {
        errors.push(format!("{}不能为空", label));
    }
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
    use super::{normalize, target_fields, validate, ALLOWED_TARGETS};
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
    fn validation_reports_conditional_fields() {
        let data = Map::new();
        let errors = validate(&data);
        assert!(errors.iter().any(|item| item.contains("医疗物品通用名")));
        assert!(errors.iter().any(|item| item.contains("生产厂家")));
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
            "pricePur":"0","priceSale":"0","fgMedRx":"是"
        })
        .as_object()
        .unwrap()
        .clone();
        let errors = validate(&data);
        assert!(errors.iter().any(|item| item.contains("24位十六进制")));
        assert!(errors.iter().any(|item| item.contains("必须为0或1")));
    }
}
