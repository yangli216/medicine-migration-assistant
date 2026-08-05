use crate::normalize::value_text;
use rust_decimal::Decimal;
use serde_json::{Map, Value};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnPatch {
    pub column: &'static str,
    pub value: Option<String>,
}

pub fn column_label(table: &str, column: &str) -> String {
    match (table, column) {
        ("hi_bd_med", "na_med") => "医疗物品通用名",
        ("hi_bd_med", "sd_med") => "物品类型",
        ("hi_bd_med", "id_cstmg") => "费用归并",
        ("hi_bd_med", "unit_pre") => "制剂单位",
        ("hi_bd_med", "spec") => "制剂规格",
        ("hi_bd_med", "dose") => "制剂剂量",
        ("hi_bd_med", "unit_dose") => "剂量单位",
        ("hi_bd_med", "sd_dose") => "剂型",
        ("hi_bd_med", "dft_usage") => "默认用法",
        ("hi_bd_med", "dft_freq") => "默认频次",
        ("hi_bd_med", "limit_anti_day") => "抗菌药物限用天数",
        ("hi_bd_med", "fg_bas_med") => "基本药物标志",
        ("hi_bd_med", "fg_anti") => "抗菌药物标志",
        ("hi_bd_med", "fg_skintest") => "皮试标志",
        ("hi_bd_med_pro", "id_fac") => "生产厂家",
        ("hi_bd_med_pro", "id_med_unit") => "包装单位",
        ("hi_bd_med_pro", "unit_sale") => "零售包装单位",
        ("hi_bd_med_pro", "na_med_pro") => "商品名",
        ("hi_bd_med_pro", "spec_sale") => "销售规格",
        ("hi_bd_med_pro", "price_sale") => "零售价格",
        ("hi_bd_med_pro", "price_pur") => "进货价格",
        ("hi_bd_med_pro", "unit_sale_factor") => "包装系数",
        ("hi_bd_med_pro", "cd_appr") => "批准文号",
        ("hi_bd_med_pro", "cd_bar") => "条形码",
        ("hi_bd_med_pro", "cd_med_pro") => "三方货品码",
        ("hi_bd_med_pro", "per") => "加成比例",
        _ => column,
    }
    .to_string()
}

pub fn values_equal(column: &str, before: &Option<String>, after: &Option<String>) -> bool {
    if before.as_deref().unwrap_or_default().is_empty()
        && after.as_deref().unwrap_or_default().is_empty()
    {
        return true;
    }
    if matches!(
        column,
        "limit_anti_day" | "price_sale" | "price_pur" | "unit_sale_factor" | "per"
    ) {
        return before
            .as_deref()
            .and_then(|value| Decimal::from_str(value).ok())
            .zip(
                after
                    .as_deref()
                    .and_then(|value| Decimal::from_str(value).ok()),
            )
            .is_some_and(|(left, right)| left == right);
    }
    before == after
}

pub fn medicine_patch(data: &Map<String, Value>) -> Vec<ColumnPatch> {
    let mut patch = vec![
        required("na_med", data, "naMed"),
        required("sd_med", data, "sdMed"),
        required("id_cstmg", data, "idCstmg"),
        required("unit_pre", data, "unitPre"),
        ColumnPatch {
            column: "spec",
            value: Some(derived_spec(data)),
        },
        required("sd_dose", data, "sdDose"),
    ];
    for (column, key) in [
        ("dose", "dose"),
        ("unit_dose", "unitDose"),
        ("sd_dose_unit", "sdDoseUnit"),
        ("sd_chrgitm_lv", "sdChrgitmLv"),
        ("sd_allergy", "sdAllergy"),
        ("sd_anti_acl", "sdAntiAcl"),
        ("ddd", "ddd"),
        ("sd_bas_med", "sdBasMed"),
        ("sd_spe_med", "sdSpeMed"),
        ("sd_storage", "sdStorage"),
        ("sd_pharm", "sdPharm"),
        ("sd_value", "sdValue"),
        ("sd_prod_plac", "sdProdPlac"),
        ("sd_pois", "sdPois"),
        ("sd_anti", "sdAnti"),
        ("sd_round", "sdRound"),
        ("sd_dps", "sdDps"),
        ("sd_skintest", "sdSkintest"),
        ("drip_rate", "dripRate"),
        ("dft_usage", "dftUsage"),
        ("dft_freq", "dftFreq"),
    ] {
        push_text_if_present(&mut patch, column, data, key);
    }
    for (column, key) in [
        ("fg_anti_appr", "fgAntiAppr"),
        ("fg_pois", "fgPois"),
        ("fg_anti", "fgAnti"),
        ("fg_med_rx", "fgMedRx"),
        ("fg_bas_med", "fgBasMed"),
        ("fg_skintest", "fgSkintest"),
        ("fg_tcd", "fgTcd"),
        ("fg_single", "fgSingle"),
        ("fg_register", "fgRegister"),
    ] {
        push_text_if_present(&mut patch, column, data, key);
    }
    push_nullable_if_present(&mut patch, "limit_anti_day", data, "limitAntiDay");
    if data.contains_key("dftDoseOnce") || text(data, "sdMed") == "3" {
        let value = if text(data, "dftDoseOnce").is_empty() && text(data, "sdMed") == "3" {
            text(data, "dose")
        } else {
            text(data, "dftDoseOnce")
        };
        patch.push(ColumnPatch {
            column: "dft_dose_once",
            value: Some(value),
        });
    }
    patch
}

pub fn product_patch(
    data: &Map<String, Value>,
    id_med: &str,
    id_fac: &str,
    id_med_unit: &str,
) -> Vec<ColumnPatch> {
    let base_spec = derived_spec(data);
    let mut patch = vec![
        fixed("id_med", id_med),
        fixed("id_fac", id_fac),
        fixed("id_med_unit", id_med_unit),
        required("unit_sale", data, "unitSale"),
        ColumnPatch {
            column: "na_med_pro",
            value: Some(defaulted(data, "naMedPro", &text(data, "naMed"))),
        },
        ColumnPatch {
            column: "spec_sale",
            value: Some(derived_sale_spec(data, &base_spec)),
        },
        required("price_sale", data, "priceSale"),
        required("price_pur", data, "pricePur"),
        required("unit_sale_factor", data, "unitSaleFactor"),
    ];
    for (column, key) in [
        ("cd_appr", "cdAppr"),
        ("cd_bar", "cdBar"),
        ("cd_med_pro", "cdMedPro"),
        ("sd_per", "sdPer"),
        ("fg_coll_pur", "fgCollPur"),
        ("fg_import", "fgImport"),
    ] {
        push_text_if_present(&mut patch, column, data, key);
    }
    push_nullable_if_present(&mut patch, "per", data, "per");
    patch
}

pub fn restore_patch(table: &str, before: &Value) -> Result<Vec<ColumnPatch>, String> {
    let object = before
        .as_object()
        .ok_or_else(|| "覆盖审计缺少可恢复的字段快照".to_string())?;
    let allowed = match table {
        "hi_bd_med" => medicine_columns(),
        "hi_bd_med_pro" => product_columns(),
        _ => return Err(format!("目标表{table}不支持覆盖恢复")),
    };
    let mut patch = Vec::with_capacity(object.len());
    for (column, value) in object {
        let Some(allowed_column) = allowed.iter().find(|allowed| **allowed == column) else {
            return Err(format!("覆盖快照包含未授权字段：{table}.{column}"));
        };
        patch.push(ColumnPatch {
            column: allowed_column,
            value: if value.is_null() {
                None
            } else {
                Some(
                    value
                        .as_str()
                        .ok_or_else(|| format!("覆盖快照字段格式错误：{table}.{column}"))?
                        .to_string(),
                )
            },
        });
    }
    if patch.is_empty() {
        return Err("覆盖审计中的字段快照为空".into());
    }
    Ok(patch)
}

fn medicine_columns() -> &'static [&'static str] {
    &[
        "na_med",
        "sd_med",
        "id_cstmg",
        "unit_pre",
        "spec",
        "dose",
        "unit_dose",
        "sd_dose",
        "sd_dose_unit",
        "sd_chrgitm_lv",
        "sd_allergy",
        "sd_anti_acl",
        "fg_anti_appr",
        "ddd",
        "sd_bas_med",
        "sd_spe_med",
        "limit_anti_day",
        "sd_storage",
        "sd_pharm",
        "sd_value",
        "sd_prod_plac",
        "fg_pois",
        "sd_pois",
        "fg_anti",
        "sd_anti",
        "sd_round",
        "sd_dps",
        "fg_med_rx",
        "fg_bas_med",
        "fg_skintest",
        "sd_skintest",
        "drip_rate",
        "dft_dose_once",
        "dft_usage",
        "dft_freq",
        "fg_tcd",
        "fg_single",
        "fg_register",
    ]
}

fn product_columns() -> &'static [&'static str] {
    &[
        "id_med",
        "id_fac",
        "id_med_unit",
        "unit_sale",
        "na_med_pro",
        "spec_sale",
        "price_sale",
        "price_pur",
        "unit_sale_factor",
        "cd_appr",
        "cd_bar",
        "cd_med_pro",
        "sd_per",
        "per",
        "fg_coll_pur",
        "fg_import",
    ]
}

fn required(column: &'static str, data: &Map<String, Value>, key: &str) -> ColumnPatch {
    ColumnPatch {
        column,
        value: Some(text(data, key)),
    }
}

fn fixed(column: &'static str, value: &str) -> ColumnPatch {
    ColumnPatch {
        column,
        value: Some(value.to_string()),
    }
}

fn push_text_if_present(
    patch: &mut Vec<ColumnPatch>,
    column: &'static str,
    data: &Map<String, Value>,
    key: &str,
) {
    if data.contains_key(key) {
        patch.push(ColumnPatch {
            column,
            value: Some(text(data, key)),
        });
    }
}

fn push_nullable_if_present(
    patch: &mut Vec<ColumnPatch>,
    column: &'static str,
    data: &Map<String, Value>,
    key: &str,
) {
    if data.contains_key(key) {
        let value = text(data, key);
        patch.push(ColumnPatch {
            column,
            value: (!value.is_empty()).then_some(value),
        });
    }
}

fn text(data: &Map<String, Value>, key: &str) -> String {
    data.get(key).map(value_text).unwrap_or_default()
}

fn defaulted(data: &Map<String, Value>, key: &str, default: &str) -> String {
    let value = text(data, key);
    if value.is_empty() {
        default.into()
    } else {
        value
    }
}

fn derived_spec(data: &Map<String, Value>) -> String {
    let spec = text(data, "spec");
    if !spec.is_empty() {
        spec
    } else {
        format!(
            "{}{}/{}",
            text(data, "dose"),
            text(data, "unitDose"),
            text(data, "unitPre")
        )
    }
}

fn derived_sale_spec(data: &Map<String, Value>, base_spec: &str) -> String {
    let spec = text(data, "specSale");
    if !spec.is_empty() {
        return spec;
    }
    let factor = text(data, "unitSaleFactor").parse::<i64>().unwrap_or(1);
    if factor <= 1 {
        base_spec.into()
    } else {
        format!(
            "{}{}*{}{}/{}",
            text(data, "dose"),
            text(data, "unitDose"),
            factor,
            text(data, "unitPre"),
            text(data, "unitSale")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{medicine_patch, product_patch, restore_patch, values_equal};
    use serde_json::json;

    #[test]
    fn overwrite_patch_preserves_unmapped_optional_fields_and_can_clear_nullable_numbers() {
        let value = json!({
            "naMed":"阿莫西林","sdMed":"1","idCstmg":"63aa8b1b3c6f491981ba4221",
            "unitPre":"粒","spec":"0.25g","sdDose":"1","limitAntiDay":""
        });
        let data = value.as_object().unwrap();
        let patch = medicine_patch(data);
        assert!(patch.iter().all(|item| item.column != "sd_allergy"));
        assert_eq!(
            patch
                .iter()
                .find(|item| item.column == "limit_anti_day")
                .unwrap()
                .value,
            None
        );
    }

    #[test]
    fn product_patch_rebinds_existing_product_to_resolved_relations() {
        let value = json!({
            "naMed":"阿莫西林","dose":"0.25","unitDose":"g","unitPre":"粒",
            "unitSale":"盒","unitSaleFactor":"24","priceSale":"10","pricePur":"8"
        });
        let data = value.as_object().unwrap();
        let patch = product_patch(data, "med", "fac", "unit");
        assert_eq!(patch[0].value.as_deref(), Some("med"));
        assert_eq!(patch[1].value.as_deref(), Some("fac"));
        assert_eq!(patch[2].value.as_deref(), Some("unit"));
    }

    #[test]
    fn restore_patch_rejects_columns_outside_the_fixed_whitelist() {
        assert!(restore_patch("hi_bd_med", &json!({"na_med":"旧名称"})).is_ok());
        assert!(restore_patch("hi_bd_med", &json!({"id_tet":"other"})).is_err());
        assert!(restore_patch("hi_bd_fac", &json!({"na_fac":"旧厂家"})).is_err());
    }

    #[test]
    fn diff_comparison_normalizes_numeric_scale_and_empty_null() {
        assert!(values_equal(
            "price_sale",
            &Some("10.00".into()),
            &Some("10".into())
        ));
        assert!(values_equal("cd_bar", &None, &Some(String::new())));
        assert!(!values_equal(
            "na_med",
            &Some("药品A".into()),
            &Some("药品B".into())
        ));
    }
}
