use crate::target_system::TargetSystemClient;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

const MEDICINE_COST_MERGE_PATH: &str = "api/base.tenantDicService/medicineCostMerge";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MedicineCostMergeCatalog {
    pub items: Vec<MedicineCostMergeItem>,
    pub total: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MedicineCostMergeItem {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub leaf: bool,
    #[serde(default)]
    pub parent: String,
    #[serde(default)]
    pub index: i64,
    #[serde(default)]
    pub properties: Map<String, Value>,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub mcode: String,
}

#[derive(Debug, Deserialize)]
struct CostMergeResponse {
    #[serde(default)]
    code: i64,
    #[serde(default)]
    message: String,
    #[serde(default)]
    body: CostMergeBody,
}

#[derive(Debug, Default, Deserialize)]
struct CostMergeBody {
    #[serde(default)]
    total: usize,
    #[serde(default)]
    items: Vec<MedicineCostMergeItem>,
}

pub async fn load_medicine_cost_merges(
    state: &TargetSystemClient,
) -> Result<MedicineCostMergeCatalog, String> {
    let (client, base_url) = state.authenticated_http()?;
    let endpoint = base_url
        .join(MEDICINE_COST_MERGE_PATH)
        .map_err(|_| "无法生成费用归并服务地址".to_string())?;
    let random = (Utc::now().timestamp_millis().rem_euclid(1_000_000) as f64) / 1_000_000.0;
    let response = client
        .post(endpoint)
        .json(&[json!({
            "params": {
                "parentKey": "",
                "sliceType": 3,
                "start": 0,
                "limit": 100,
                "random": random
            },
            "start": 0,
            "limit": 100
        })])
        .send()
        .await
        .map_err(|error| format!("费用归并服务请求失败：{error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("费用归并服务返回 HTTP {}", status.as_u16()));
    }
    let payload: CostMergeResponse = response
        .json()
        .await
        .map_err(|_| "费用归并服务返回内容不是有效 JSON".to_string())?;
    if payload.code != 200 {
        return Err(format!(
            "费用归并服务返回失败：{}",
            if payload.message.is_empty() {
                format!("code={}", payload.code)
            } else {
                payload.message
            }
        ));
    }
    let mut items = payload
        .body
        .items
        .into_iter()
        .filter(|item| item.active && item.leaf && !item.key.is_empty())
        .collect::<Vec<_>>();
    items.sort_by(|left, right| {
        left.index
            .cmp(&right.index)
            .then(left.text.cmp(&right.text))
    });
    if items.is_empty() {
        return Err("费用归并服务没有返回可用项目".into());
    }
    Ok(MedicineCostMergeCatalog {
        total: payload.body.total,
        message: format!("已读取 {} 个可用费用归并项目", items.len()),
        items,
    })
}

#[cfg(test)]
mod tests {
    use super::load_medicine_cost_merges;
    use crate::target_system::{login, TargetSystemClient, TargetSystemLoginRequest};

    #[test]
    #[ignore = "requires an explicitly configured reachable target system"]
    fn live_cost_merge_service_uses_authenticated_tk_session() {
        let state = TargetSystemClient::new().unwrap();
        let request = TargetSystemLoginRequest {
            base_url: std::env::var("TARGET_SYSTEM_TEST_URL").expect("TARGET_SYSTEM_TEST_URL"),
            tenant_id: std::env::var("TARGET_SYSTEM_TEST_TENANT")
                .expect("TARGET_SYSTEM_TEST_TENANT"),
            login_name: "system".into(),
            password: std::env::var("TARGET_SYSTEM_TEST_PASSWORD")
                .expect("TARGET_SYSTEM_TEST_PASSWORD"),
        };
        tauri::async_runtime::block_on(login(&state, request)).unwrap();
        let catalog = tauri::async_runtime::block_on(load_medicine_cost_merges(&state)).unwrap();
        println!("cost merge items={}", catalog.items.len());
        assert!(catalog.items.iter().any(|item| item.text == "西药费"));
        assert!(catalog.items.iter().any(|item| item.text == "草药费"));
    }
}
