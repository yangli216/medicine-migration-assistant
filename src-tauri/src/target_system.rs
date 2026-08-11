use md5::{Digest, Md5};
use reqwest::{redirect::Policy, Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    sync::{Mutex, RwLock},
    time::{Duration, Instant},
};

const ROLE_PATH: &str = "logon/myRoles";
const APP_PATH: &str = "logon/myApps";
const ORGANIZATION_PATH: &str = "api/bbp.organization/findByTenantId";
const REQUIRED_LOGIN_NAME: &str = "system";
const REQUIRED_ROLE_CODE: &str = "tenantSystem";

pub struct TargetSystemClient {
    client: RwLock<Client>,
    authorized_session: Mutex<Option<AuthorizedSession>>,
    dictionary_values: Mutex<HashMap<String, HashSet<String>>>,
}

#[derive(Debug, Clone)]
struct AuthorizedSession {
    base_url: String,
    tenant_id: String,
    user_id: String,
}

impl TargetSystemClient {
    pub fn new() -> Result<Self, String> {
        let client = build_http_client()?;
        Ok(Self {
            client: RwLock::new(client),
            authorized_session: Mutex::new(None),
            dictionary_values: Mutex::new(HashMap::new()),
        })
    }

    fn http_client(&self) -> Result<Client, String> {
        self.client
            .read()
            .map(|client| client.clone())
            .map_err(|_| "新系统网络会话不可用，请重新启动工具".to_string())
    }

    fn authorize(&self, client: Client, login: &TargetSystemLogin) -> Result<(), String> {
        *self
            .client
            .write()
            .map_err(|_| "新系统网络会话不可用，请重新启动工具".to_string())? = client;
        let mut session = self
            .authorized_session
            .lock()
            .map_err(|_| "新系统认证状态不可用，请重新启动工具".to_string())?;
        *session = Some(AuthorizedSession {
            base_url: login.base_url.clone(),
            tenant_id: login.tenant_id.clone(),
            user_id: login.user_id.clone(),
        });
        Ok(())
    }

    fn begin_authentication(&self, client: Client) -> Result<(), String> {
        *self
            .client
            .write()
            .map_err(|_| "新系统网络会话不可用，请重新启动工具".to_string())? = client;
        *self
            .authorized_session
            .lock()
            .map_err(|_| "新系统认证状态不可用，请重新启动工具".to_string())? = None;
        self.dictionary_values
            .lock()
            .map_err(|_| "新系统字典缓存不可用，请重新启动工具".to_string())?
            .clear();
        Ok(())
    }

    pub(crate) fn authenticated_http(&self) -> Result<(Client, Url), String> {
        // Keep the lock order consistent with authorize/begin_authentication.
        let client = self
            .client
            .read()
            .map_err(|_| "新系统网络会话不可用，请重新启动工具".to_string())?;
        let session = self
            .authorized_session
            .lock()
            .map_err(|_| "新系统认证状态不可用，请重新启动工具".to_string())?;
        let authorized = session
            .as_ref()
            .ok_or_else(|| "请先使用 system 租户管理员完成新系统认证".to_string())?;
        let base_url = normalize_base_url(&authorized.base_url)?;
        Ok((client.clone(), base_url))
    }

    pub(crate) fn replace_dictionary_values(
        &self,
        values: HashMap<String, HashSet<String>>,
    ) -> Result<(), String> {
        *self
            .dictionary_values
            .lock()
            .map_err(|_| "新系统字典缓存不可用，请重新启动工具".to_string())? = values;
        Ok(())
    }

    pub(crate) fn dictionary_values(&self) -> Result<HashMap<String, HashSet<String>>, String> {
        self.dictionary_values
            .lock()
            .map(|values| values.clone())
            .map_err(|_| "新系统字典缓存不可用，请重新启动工具".to_string())
    }

    pub(crate) fn invalidate(&self) -> Result<(), String> {
        self.begin_authentication(build_http_client()?)
    }

    pub fn execution_identity(&self) -> Result<(String, String), String> {
        let session = self
            .authorized_session
            .lock()
            .map_err(|_| "新系统认证状态不可用，请重新启动工具".to_string())?;
        let authorized = session
            .as_ref()
            .ok_or_else(|| "请先使用 system 租户管理员完成新系统认证".to_string())?;
        Ok((authorized.tenant_id.clone(), authorized.user_id.clone()))
    }
}

fn build_http_client() -> Result<Client, String> {
    Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .redirect(Policy::limited(5))
        .cookie_store(true)
        .build()
        .map_err(|error| format!("无法初始化新系统访问组件：{error}"))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSystemProbeRequest {
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSystemProbe {
    pub ok: bool,
    pub base_url: String,
    pub status_code: u16,
    pub latency_ms: u128,
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSystemLoginRequest {
    pub base_url: String,
    pub tenant_id: String,
    pub login_name: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetRole {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub role_id: String,
    #[serde(default)]
    pub tenant_id: String,
    #[serde(default)]
    pub tenant_name: String,
    #[serde(default)]
    pub login_name: String,
    #[serde(default)]
    pub role_name: String,
    #[serde(default)]
    pub role_cd: String,
    #[serde(default)]
    pub user_name: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub active: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSystemLogin {
    pub ok: bool,
    pub base_url: String,
    pub authorization_id: String,
    pub tenant_id: String,
    pub tenant_name: String,
    pub user_id: String,
    pub user_name: String,
    pub role_id: String,
    pub role_name: String,
    pub role_cd: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetOrganization {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub cd: String,
    #[serde(default)]
    pub org_id: String,
    #[serde(default)]
    pub tenant_id: String,
    #[serde(default)]
    pub org_type: String,
    #[serde(default)]
    pub org_type_text: String,
    #[serde(default)]
    pub full_name: String,
    #[serde(default)]
    pub parent: String,
    #[serde(default)]
    pub parent_text: String,
    #[serde(default)]
    pub active: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetOrganizationCatalog {
    pub organizations: Vec<TargetOrganization>,
    pub message: String,
}

pub async fn probe(
    state: &TargetSystemClient,
    request: TargetSystemProbeRequest,
) -> Result<TargetSystemProbe, String> {
    let base_url = normalize_base_url(&request.base_url)?;
    let started = Instant::now();
    let client = state.http_client()?;
    let first = client
        .get(base_url.clone())
        .send()
        .await
        .map_err(|error| connection_error(&base_url, &error))?;

    let mut status = first.status();
    if matches!(
        status,
        StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED
    ) {
        let endpoint = endpoint_url(&base_url, ROLE_PATH)?;
        status = client
            .get(endpoint)
            .send()
            .await
            .map_err(|error| connection_error(&base_url, &error))?
            .status();
    }

    if status.is_server_error() || status == StatusCode::NOT_FOUND {
        return Err(format!(
            "新系统地址已响应，但服务不可用（HTTP {}）",
            status.as_u16()
        ));
    }

    Ok(TargetSystemProbe {
        ok: true,
        base_url: base_url.to_string().trim_end_matches('/').to_string(),
        status_code: status.as_u16(),
        latency_ms: started.elapsed().as_millis(),
        message: "新系统地址可访问".into(),
    })
}

pub async fn login(
    state: &TargetSystemClient,
    request: TargetSystemLoginRequest,
) -> Result<TargetSystemLogin, String> {
    validate_login_request(&request)?;
    let base_url = normalize_base_url(&request.base_url)?;
    // A fresh cookie store prevents an earlier tenant's tk from leaking into a new login attempt.
    let client = build_http_client()?;
    // Invalidate the previous identity before any new login attempt. A failed re-login must not
    // leave the old session authorized.
    state.begin_authentication(client.clone())?;
    let endpoint = endpoint_url(&base_url, ROLE_PATH)?;
    let response = client
        .post(endpoint)
        .json(&json!({
            "loginName": REQUIRED_LOGIN_NAME,
            "pwd": password_digest(&request.password),
            "tenantId": request.tenant_id.trim(),
            "forAccessToken": true
        }))
        .send()
        .await
        .map_err(|error| connection_error(&base_url, &error))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!("登录服务返回 HTTP {}", status.as_u16()));
    }

    let payload: Value = response
        .json()
        .await
        .map_err(|_| "登录服务返回内容不是有效 JSON".to_string())?;
    let login = parse_login_response(
        base_url.to_string().trim_end_matches('/').to_string(),
        request.tenant_id.trim(),
        payload,
    )?;
    establish_application_session(&client, &base_url, &login.authorization_id).await?;
    state.authorize(client, &login)?;
    Ok(login)
}

pub async fn load_organizations(
    state: &TargetSystemClient,
) -> Result<TargetOrganizationCatalog, String> {
    let (tenant_id, _) = state.execution_identity()?;
    let (client, base_url) = state.authenticated_http()?;
    let endpoint = endpoint_url(&base_url, ORGANIZATION_PATH)?;
    let response = client
        .post(endpoint)
        .json(&[tenant_id.as_str()])
        .send()
        .await
        .map_err(|error| format!("新系统机构服务请求失败：{error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("新系统机构服务返回 HTTP {}", status.as_u16()));
    }
    let payload: Value = response
        .json()
        .await
        .map_err(|_| "新系统机构服务返回内容不是有效 JSON".to_string())?;
    let items = organization_items(&payload)?;
    let mut organizations = serde_json::from_value::<Vec<TargetOrganization>>(items.clone())
        .map_err(|_| "新系统机构服务返回的机构字段格式不兼容".to_string())?;
    organizations.retain(|item| {
        item.active
            && !item.id.trim().is_empty()
            && (item.tenant_id.trim().is_empty() || item.tenant_id == tenant_id)
    });
    organizations.sort_by(|left, right| {
        left.cd
            .cmp(&right.cd)
            .then(left.name.cmp(&right.name))
            .then(left.id.cmp(&right.id))
    });
    if organizations.is_empty() {
        return Err("新系统机构服务没有返回当前租户的有效机构".into());
    }
    Ok(TargetOrganizationCatalog {
        message: format!("已从新系统服务读取 {} 个有效机构", organizations.len()),
        organizations,
    })
}

fn organization_items(payload: &Value) -> Result<&Value, String> {
    if payload.is_array() {
        return Ok(payload);
    }
    if response_code(payload) != 0 && response_code(payload) != 200 {
        return Err(format!(
            "新系统机构服务返回失败：{}",
            response_message(payload).unwrap_or("未返回失败原因")
        ));
    }
    let body = payload
        .get("body")
        .ok_or_else(|| "新系统机构服务未返回机构清单".to_string())?;
    if body.is_array() {
        return Ok(body);
    }
    body.get("items")
        .filter(|items| items.is_array())
        .ok_or_else(|| "新系统机构服务返回的机构清单格式不兼容".to_string())
}

async fn establish_application_session(
    client: &Client,
    base_url: &Url,
    authorization_id: &str,
) -> Result<(), String> {
    if authorization_id.is_empty() {
        return Err("租户管理员角色缺少授权标识，无法建立登录会话".into());
    }
    let endpoint = endpoint_url(base_url, APP_PATH)?;
    let response = client
        .get(endpoint)
        .query(&[
            ("urt", authorization_id),
            ("deep", "3"),
            ("platform", "256"),
            ("login", "0"),
            ("urtDept", ""),
        ])
        .send()
        .await
        .map_err(|error| connection_error(base_url, &error))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("角色登录服务返回 HTTP {}", status.as_u16()));
    }
    let has_tk = response
        .cookies()
        .any(|cookie| cookie.name() == "tk" && !cookie.value().is_empty());
    let payload: Value = response
        .json()
        .await
        .map_err(|_| "角色登录服务返回内容不是有效 JSON".to_string())?;
    validate_application_login(&payload, has_tk)
}

fn validate_application_login(payload: &Value, has_tk: bool) -> Result<(), String> {
    if response_code(payload) != 200 {
        return Err(format!(
            "角色登录失败：{}",
            response_message(payload).unwrap_or("myApps 未返回成功状态")
        ));
    }
    if !has_tk {
        return Err("角色登录返回成功，但没有下发 tk 登录凭证".into());
    }
    Ok(())
}

fn parse_login_response(
    base_url: String,
    requested_tenant: &str,
    payload: Value,
) -> Result<TargetSystemLogin, String> {
    let code = response_code(&payload);
    if code != 200 {
        let message = response_message(&payload).unwrap_or("账号、密码或租户验证失败");
        return Err(format!("登录失败：{message}"));
    }

    let roles: Vec<TargetRole> = serde_json::from_value(
        payload
            .get("body")
            .cloned()
            .ok_or_else(|| "登录成功但未返回角色信息".to_string())?,
    )
    .map_err(|_| "登录成功但角色信息格式不兼容".to_string())?;
    let role = select_tenant_admin_role(&roles, requested_tenant)?;

    Ok(TargetSystemLogin {
        ok: true,
        base_url,
        authorization_id: role.id.clone(),
        tenant_id: role.tenant_id.clone(),
        tenant_name: role.tenant_name.clone(),
        user_id: role.user_id.clone(),
        user_name: if role.user_name.is_empty() {
            role.display_name.clone()
        } else {
            role.user_name.clone()
        },
        role_id: role.role_id.clone(),
        role_name: role.role_name.clone(),
        role_cd: role.role_cd.clone(),
        message: "system 租户管理员认证通过，tk 登录凭证已建立".into(),
    })
}

fn response_code(payload: &Value) -> i64 {
    payload
        .get("code")
        .and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_str()?.parse::<i64>().ok())
        })
        .unwrap_or_default()
}

fn response_message(payload: &Value) -> Option<&str> {
    payload
        .get("message")
        .or_else(|| payload.get("msg"))
        .and_then(Value::as_str)
}

fn select_tenant_admin_role<'a>(
    roles: &'a [TargetRole],
    requested_tenant: &str,
) -> Result<&'a TargetRole, String> {
    roles
        .iter()
        .find(|role| {
            role.active
                && role.login_name == REQUIRED_LOGIN_NAME
                && role.role_cd == REQUIRED_ROLE_CODE
                && role.tenant_id == requested_tenant
        })
        .ok_or_else(|| "当前账号不是该租户的有效 system（租户管理员），禁止使用迁移功能".into())
}

fn validate_login_request(request: &TargetSystemLoginRequest) -> Result<(), String> {
    if request.login_name.trim() != REQUIRED_LOGIN_NAME {
        return Err("本功能仅允许 system（租户管理员）登录".into());
    }
    if request.tenant_id.trim().is_empty() || request.tenant_id.chars().count() > 64 {
        return Err("租户编码不能为空且不能超过64个字符".into());
    }
    if request.password.is_empty() {
        return Err("请输入 system 密码".into());
    }
    Ok(())
}

fn normalize_base_url(raw: &str) -> Result<Url, String> {
    let mut url = Url::parse(raw.trim()).map_err(|_| "请输入有效的新系统访问地址".to_string())?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("新系统地址只支持 HTTP 或 HTTPS".into());
    }
    if url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() {
        return Err("新系统地址格式不正确，地址中不能包含账号密码".into());
    }
    url.set_query(None);
    url.set_fragment(None);
    let path = url.path().trim_end_matches('/').to_string();
    url.set_path(&format!("{path}/"));
    Ok(url)
}

fn endpoint_url(base_url: &Url, relative: &str) -> Result<Url, String> {
    base_url
        .join(relative)
        .map_err(|_| "无法生成新系统登录服务地址".to_string())
}

fn password_digest(password: &str) -> String {
    let trimmed = password.trim();
    if trimmed.len() == 32
        && trimmed
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return trimmed.to_ascii_lowercase();
    }
    format!("{:x}", Md5::digest(password.as_bytes()))
}

fn connection_error(base_url: &Url, error: &reqwest::Error) -> String {
    if error.is_timeout() {
        format!(
            "连接新系统超时：{}",
            base_url.as_str().trim_end_matches('/')
        )
    } else if error.is_connect() {
        format!(
            "无法连接新系统：{}",
            base_url.as_str().trim_end_matches('/')
        )
    } else {
        format!("访问新系统失败：{error}")
    }
}

#[cfg(test)]
mod tests {
    use super::{
        login, normalize_base_url, organization_items, parse_login_response, password_digest,
        validate_application_login, validate_login_request, TargetSystemClient,
        TargetSystemLoginRequest,
    };
    use serde_json::json;

    #[test]
    fn normalizes_context_path_and_removes_query() {
        let url = normalize_base_url(" http://10.17.18.88:8000/rbmh-phis/?x=1 ").unwrap();
        assert_eq!(url.as_str(), "http://10.17.18.88:8000/rbmh-phis/");
    }

    #[test]
    fn password_is_md5_encoded_once() {
        assert_eq!(
            password_digest("123456"),
            "e10adc3949ba59abbe56e057f20f883e"
        );
        assert_eq!(
            password_digest("E10ADC3949BA59ABBE56E057F20F883E"),
            "e10adc3949ba59abbe56e057f20f883e"
        );
    }

    #[test]
    fn rejects_non_system_login_before_network_call() {
        let request = TargetSystemLoginRequest {
            base_url: "http://example.test/rbmh-phis".into(),
            tenant_id: "tenant".into(),
            login_name: "admin".into(),
            password: "secret".into(),
        };
        assert!(validate_login_request(&request)
            .unwrap_err()
            .contains("仅允许 system"));
    }

    #[test]
    fn accepts_only_active_system_tenant_admin_role() {
        let payload = json!({
            "code": 200,
            "body": [{
                "id": "6045cda1d0081238ab8fd09b",
                "userId": "6045cda1d0081238ab8fd09a",
                "roleId": "6045cda1d0081238ab8fd099",
                "tenantId": "cszzyzh",
                "tenantName": "HIHIS（一体化版）",
                "loginName": "system",
                "roleName": "租户管理员",
                "roleCd": "tenantSystem",
                "userName": "管理员",
                "active": true
            }]
        });
        let login =
            parse_login_response("http://host/rbmh-phis".into(), "cszzyzh", payload).unwrap();
        assert_eq!(login.role_cd, "tenantSystem");
        assert_eq!(login.user_id, "6045cda1d0081238ab8fd09a");
    }

    #[test]
    fn rejects_non_tenant_admin_response() {
        let payload = json!({
            "code": 200,
            "body": [{
                "tenantId": "cszzyzh",
                "loginName": "system",
                "roleCd": "ordinary",
                "active": true
            }]
        });
        assert!(
            parse_login_response("http://host/rbmh-phis".into(), "cszzyzh", payload)
                .unwrap_err()
                .contains("禁止使用迁移功能")
        );
    }

    #[test]
    fn requires_successful_my_apps_response_and_tk_cookie() {
        assert!(validate_application_login(&json!({ "code": 200 }), true).is_ok());
        assert!(validate_application_login(&json!({ "code": 200 }), false)
            .unwrap_err()
            .contains("tk"));
        assert!(
            validate_application_login(&json!({ "code": 500, "message": "failed" }), true)
                .unwrap_err()
                .contains("failed")
        );
    }

    #[test]
    fn organization_service_accepts_direct_and_wrapped_lists() {
        let direct = json!([{"id":"org-1","name":"测试机构","active":true}]);
        assert_eq!(
            organization_items(&direct)
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let wrapped = json!({
            "code": 200,
            "body": {"items": [{"id":"org-2","name":"分院","active":true}]}
        });
        assert_eq!(
            organization_items(&wrapped).unwrap().as_array().unwrap()[0]["id"],
            "org-2"
        );
    }

    #[test]
    #[ignore = "requires an explicitly configured reachable target system"]
    fn live_two_stage_system_login_receives_tk() {
        let base_url = std::env::var("TARGET_SYSTEM_TEST_URL")
            .expect("TARGET_SYSTEM_TEST_URL is required for the ignored live test");
        let tenant_id = std::env::var("TARGET_SYSTEM_TEST_TENANT")
            .expect("TARGET_SYSTEM_TEST_TENANT is required for the ignored live test");
        let password = std::env::var("TARGET_SYSTEM_TEST_PASSWORD")
            .expect("TARGET_SYSTEM_TEST_PASSWORD is required for the ignored live test");
        let state = TargetSystemClient::new().unwrap();
        let result = tauri::async_runtime::block_on(login(
            &state,
            TargetSystemLoginRequest {
                base_url,
                tenant_id,
                login_name: "system".into(),
                password,
            },
        ))
        .unwrap();

        assert_eq!(result.role_cd, "tenantSystem");
        assert_eq!(
            state.execution_identity().unwrap(),
            (result.tenant_id, result.user_id)
        );
    }
}
