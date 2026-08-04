use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DriverPackDefinition {
    id: String,
    database_kind: String,
    title: String,
    version: String,
    default_driver: String,
    driver_keywords: Vec<String>,
    default_port: u16,
    delivery: String,
    license_note: String,
    official_url: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverPackStatus {
    id: String,
    database_kind: String,
    title: String,
    version: String,
    default_driver: String,
    default_port: u16,
    delivery: String,
    license_note: String,
    official_url: String,
    state: String,
    detected_driver: Option<String>,
    platform: String,
    architecture: String,
}

pub fn list(installed_drivers: &[String]) -> Result<Vec<DriverPackStatus>, String> {
    let definitions: Vec<DriverPackDefinition> =
        serde_json::from_str(include_str!("../driver-packs/manifest.json"))
            .map_err(|error| format!("驱动包清单解析失败：{error}"))?;

    Ok(definitions
        .into_iter()
        .map(|definition| {
            let detected_driver = installed_drivers.iter().find(|driver| {
                let normalized = driver.to_lowercase();
                definition
                    .driver_keywords
                    .iter()
                    .any(|keyword| normalized.contains(&keyword.to_lowercase()))
            });
            let state = if definition.delivery == "bundled"
                || (definition.delivery == "licensed-bundle" && detected_driver.is_some())
            {
                "bundled"
            } else if detected_driver.is_some() {
                "installed"
            } else {
                "profile-ready"
            };

            DriverPackStatus {
                id: definition.id,
                database_kind: definition.database_kind,
                title: definition.title,
                version: definition.version,
                default_driver: definition.default_driver,
                default_port: definition.default_port,
                delivery: definition.delivery,
                license_note: definition.license_note,
                official_url: definition.official_url,
                state: state.to_string(),
                detected_driver: detected_driver.cloned(),
                platform: std::env::consts::OS.to_string(),
                architecture: std::env::consts::ARCH.to_string(),
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::list;

    #[test]
    fn oracle_profile_is_pinned_to_19c() {
        let packs = list(&[]).expect("driver manifest");
        let oracle = packs
            .iter()
            .find(|pack| pack.database_kind == "oracle")
            .expect("oracle pack");
        assert_eq!(oracle.version, "19c · 按操作系统架构");
        assert_eq!(oracle.default_port, 1521);
        assert_eq!(oracle.state, "profile-ready");
    }

    #[test]
    fn installed_driver_is_detected_case_insensitively() {
        let packs = list(&["KINGBASEES 8.6 ODBC Driver".to_string()]).expect("driver manifest");
        let kingbase = packs
            .iter()
            .find(|pack| pack.database_kind == "kingbase")
            .expect("kingbase pack");
        assert_eq!(kingbase.state, "installed");
        assert_eq!(
            kingbase.detected_driver.as_deref(),
            Some("KINGBASEES 8.6 ODBC Driver")
        );
    }

    #[test]
    fn bundled_oracle_driver_is_reported_as_bundled() {
        let packs = list(&["Oracle 19 ODBC driver".to_string()]).expect("driver manifest");
        let oracle = packs
            .iter()
            .find(|pack| pack.database_kind == "oracle")
            .expect("oracle pack");
        assert_eq!(oracle.state, "bundled");
    }
}
