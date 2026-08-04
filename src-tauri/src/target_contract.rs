pub const TARGET_TABLE_PROJECTIONS: &[(&str, &str)] = &[
    (
        "hi_bd_med",
        "id_med,na_med,sd_med,id_cstmg,unit_pre,spec,dose,unit_dose,sd_dose,\
         sd_dose_unit,sd_chrgitm_lv,sd_allergy,sd_anti_acl,fg_anti_appr,ddd,sd_bas_med,sd_spe_med,\
         limit_anti_day,sd_storage,sd_pharm,sd_value,sd_prod_plac,fg_pois,sd_pois,\
         fg_anti,sd_anti,sd_round,sd_dps,fg_med_rx,fg_bas_med,fg_skintest,sd_skintest,\
         drip_rate,dft_dose_once,dft_usage,dft_freq,fg_tcd,fg_single,fg_register,\
         fg_active,fg_pri,id_org_pri,id_tet,revision,insert_user,insert_time",
    ),
    (
        "hi_bd_med_alias",
        "id_med_alias,id_med,na_alias,fg_main,py,wb,instr,id_tet,fg_active,fg_pri,id_org",
    ),
    (
        "hi_bd_med_unit",
        "id_med_unit,id_med,na_unit,unit_factor,id_tet",
    ),
    (
        "hi_bd_fac",
        "id_fac,na_fac,na_fac_short,sd_prod_plac,py,wb,instr,fg_active,id_tet,\
         revision,insert_user,insert_time,sd_fac",
    ),
    (
        "hi_bd_med_pro",
        "id_med_pro,id_med,id_fac,id_med_unit,unit_sale,na_med_pro,spec_sale,price_sale,\
         price_pur,unit_sale_factor,cd_appr,cd_bar,cd_med_pro,id_tet,revision,insert_user,\
         insert_time,fg_active,fg_pri,id_org_pri,sd_per,per,fg_coll_pur,fg_import",
    ),
];

pub fn validate_schema_identifier(schema: &str) -> Result<String, String> {
    let schema = schema.trim();
    if schema.is_empty() {
        return Ok(String::new());
    }
    if schema.len() > 128
        || !schema
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return Err("目标 Schema 只能包含字母、数字和下划线".into());
    }
    Ok(schema.to_string())
}

pub fn validate_execution_context(
    tenant_id: &str,
    operator_id: &str,
    organization_id: &str,
    private_record: bool,
) -> Result<(), String> {
    if tenant_id.trim().is_empty() || tenant_id.chars().count() > 64 {
        return Err("租户主键不能为空且不能超过64个字符".into());
    }
    if operator_id.trim().is_empty() || operator_id.chars().count() > 64 {
        return Err("操作人主键不能为空且不能超过64个字符".into());
    }
    if private_record
        && (organization_id.len() != 24
            || !organization_id
                .chars()
                .all(|character| character.is_ascii_hexdigit()))
    {
        return Err("机构私有药品必须提供24位十六进制机构主键".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{validate_execution_context, validate_schema_identifier};

    #[test]
    fn schema_identifier_rejects_sql_fragments() {
        assert!(validate_schema_identifier("RBMH_PHIS_SHOW").is_ok());
        assert!(validate_schema_identifier("base; drop table x").is_err());
        assert!(validate_schema_identifier("a.b").is_err());
    }

    #[test]
    fn private_record_requires_object_id_organization() {
        assert!(validate_execution_context("tenant", "operator", "", false).is_ok());
        assert!(validate_execution_context("tenant", "operator", "", true).is_err());
        assert!(
            validate_execution_context("tenant", "operator", "66aa10244f0d4826ac110001", true)
                .is_ok()
        );
    }
}
