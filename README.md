# 药品数据迁移助手

一个不依赖 Java 服务端、可直接交付给三方对接人员使用的 Tauri 桌面工具。界面负责引导配置，Rust 负责来源读取、数据转换、直接写表、逐行事务和本地审计。

## 当前能力

- 新系统门禁：验证目标系统访问地址，先通过 `/logon/myRoles` 取得并验证 `system` 的 `tenantSystem` 角色，再调用 `/logon/myApps` 完成角色登录；只有返回成功并下发非空 `tk` Cookie 后才允许进入和执行迁移。
- 任务入口：药品基础信息同步、机构库存首次盘点均已开放。库存任务支持机构/库房映射、逐行核对、金额汇总、空库防重、首次盘点写入和安全撤销。
- 来源：CSV、JSON、MySQL、Oracle、达梦 DM8、Gauss/openGauss、海量 Vastbase、南大通用 GBase 8c/8a/8s、人大金仓 KingbaseES、PostgreSQL 只读查询；单批最多读取 10,000 行。
- 二系列phis适配：以 `YK_TYPK`、`YK_YPCD` 为药品主表，厂家读取 `YK_CDDZ`；提供“机构在用药品”“机构全部配置药品”“全部通用药品”三个范围。“机构在用”仅以有效 `YK_CDXX` 判断，库存读取 `YK_KCMX` / `YF_KCMX`。标准药品、机构和库存查询会先读取实际字段，核心关联字段缺失时给出明确清单，批号、效期、来源金额合计及描述字段缺失时使用空值或安全计算值继续核对。
- 映射：推荐映射、逐字段确认、完整映射专家模式；支持可视化“旧值 → 新值”字典、默认值、大小写、空白清洗、整数/小数、布尔和日期标准化。
- 在线字典：依据 `HiBdMed` 的 `@Dictionary(id=...)` 白名单，在 system 登录后复用 `tk` Cookie，以最多 6 路并发读取当前租户真实字典；界面展示编码与名称，可按编码、名称、拼音及常见组合格式生成安全映射，预校验会拒绝已加载字典范围外的编码。
- 费用归并：登录后调用 `api/base.tenantDicService/medicineCostMerge` 读取当前租户归并项目；按药品类型推荐“西药→西药费、草药→草药费、疫苗→疫苗费、耗材→卫生材料费、中药/民族药/院内制剂→成药费”，由对接人员逐项确认后写入 `hi_bd_med.id_cstmg`。
- 校验：必填、长度、枚举、数字、条件必填和业务冲突检查。
- 写入：MySQL 和 PostgreSQL 兼容家族使用应用内置原生协议，其他企业数据库使用已安装的厂商 ODBC，直接写入 `hi_bd_med`、`hi_bd_med_alias`、`hi_bd_med_unit`、`hi_bd_fac`、`hi_bd_med_pro`。每条来源记录独立提交或整体回滚。
- 分层写入：每个药品都会先确保“最小单位 × 1”，销售包装不同时再补充包装单位；没有厂家商品数据时仅写药品、单位和基础别名，存在厂家商品数据时才继续校验并写入厂家与商品表。
- 可靠性：每条来源记录使用独立数据库事务，单条失败不影响其它记录；可仅重试失败记录。
- 增量：本机 SQLite 维护“租户 + 来源 + 来源主键 → 目标主键与来源摘要”的迁移台账；来源未变化时自动跳过，检测到已迁移来源发生变化时阻止静默覆盖并提示进入覆盖流程。
- 幂等：药品和产品按业务唯一条件复用或跳过，避免重复执行产生重复数据。
- 审计：批次、行状态、目标主键、写入清单、前后数据摘要、错误和操作时间写入本机 SQLite。
- 安全撤销：覆盖记录先按字段级快照恢复，再删除审计中明确标记为本批次 `INSERT`、属于当前登录租户、且未被后续数据引用的记录；复用数据永不删除。撤销前必须核对执行时记录的目标库身份，存在后续引用时保留记录并标记为部分撤销；来源台账也同步恢复到覆盖前版本。
- 覆盖迁移：仅允许覆盖迁移台账确认由本工具创建或连续覆盖的药品、商品记录；正式执行前必须从目标库生成逐行字段差异，支持勾选部分记录，未勾选记录进入可审计的跳过状态。更新前保存实际数据库字段原值，更新与快照读取位于同一事务。厂家、包装单位采用匹配或新建，不直接覆盖共享主数据；暂不允许在“仅基础药品”和“含厂家商品”之间改变数据层级。
- 安全：新系统密码在 Rust 后端按 MD5 接口协议转换，`tk` 仅保存在内存 Cookie 会话。选择“记住密码”时，数据库和新系统口令使用应用数据目录中的每安装实例主密钥进行 AES-256-GCM 本地加密，不依赖 macOS 钥匙串，也不写入审计记录。药品基础数据固定为租户级公共数据，`id_tet` 和操作人由后端登录会话强制赋值。MD5 不是传输加密，正式环境仍建议使用 HTTPS。
- 主键：所有新建记录使用 BSON ObjectId，固定为 24 位小写十六进制字符串。

## 使用方式

开发预览：

```bash
npm install
npm run desktop:dev
```

生成当前操作系统的免服务端桌面安装包：

```bash
npm run desktop:build
```

### Windows EXE

仓库提供 `.github/workflows/windows-build.yml`。进入 GitHub 仓库的 **Actions → Build Windows installer → Run workflow**，构建完成后下载 `medicine-migration-assistant-windows-x64`，其中包含可直接安装的 NSIS `.exe`。

Windows x64 是驱动准备和安装包验收的第一优先级，macOS Apple Silicon、macOS Intel 依次验证。MySQL 原生协议和 PostgreSQL 通用协议随应用内置；PostgreSQL、openGauss/GaussDB、Vastbase、GBase 8c、KingbaseES 默认无需安装 ODBC。Windows x64 安装包同时内置 Oracle Instant Client 19.31 完整 Basic + ODBC，安装时自动登记应用专用驱动，因此会申请管理员权限；达梦、GBase 8a/8s 等厂商 ODBC 仍由项目安装介质或厂商提供。

Windows 流水线从 Oracle 官方地址下载固定版本驱动并核对 SHA-256，不把大体积厂商文件提交到 Git。它不仅生成 NSIS 安装包，还会在干净的 Windows x64 Runner 中执行静默安装，核对程序为 x86-64 PE、Oracle 许可及运行文件齐全、64 位 ODBC 登记指向应用目录，并实际启动桌面应用，最后输出 `windows-install-acceptance.json`。本地 Windows 也可运行：

```powershell
npm run verify:windows
```

### macOS 签名与公证

`.github/workflows/macos-release.yml` 只接受正式 Apple Developer 凭据，构建后强制执行代码签名校验、Gatekeeper 评估和公证票据校验。仓库需配置以下 Secrets：

- `APPLE_CERTIFICATE`：Developer ID Application 的 Base64 `.p12`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_SIGNING_IDENTITY`
- `APPLE_ID`
- `APPLE_PASSWORD`：Apple 专用密码
- `APPLE_TEAM_ID`

缺少任一项时流水线会明确失败，不会把自签名或未公证产物作为发布包。开发机可使用自签名证书检查包结构和代码签名，但只有 Developer ID + Apple 公证结果才算正式验收：

```bash
npm run verify:macos
REQUIRE_NOTARIZATION=1 npm run verify:macos
```

Rust 单元测试：

```bash
cd src-tauri
cargo test --offline
```

前端转换规则测试与 Windows 驱动交付检查：

```bash
npm test
npm run test:drivers
```

## 数据库支持范围

MySQL 使用应用内置原生协议。PostgreSQL 兼容家族优先使用应用内置的 PostgreSQL 前端协议：PostgreSQL、openGauss/GaussDB、海量 Vastbase G100、南大通用 GBase 8c 和人大金仓 KingbaseES；连接测试、只读查询、表清单、目标结构检查、药品基础数据增量写入和安全撤销均不依赖本机 ODBC。每条药品来源记录在同一事务中完成药品、别名、包装单位、厂家和厂家商品写入，任一步失败整行回滚。特殊版本可在连接表单的高级区域显式配置厂商 ODBC 回退。

Oracle、达梦 DM8、南大通用 GBase 8a/8s 默认使用 64 位 ODBC。其中 Windows x64 的 Oracle 19.31 驱动已内置并自动登记；保存的 `Oracle 19 ODBC driver` 配置会自动解析为应用专用驱动，不覆盖电脑原有 Oracle 客户端。该内置客户端支持 Oracle Database 11.2.0.4 及以上；更早的 11g 小版本需要升级数据库补丁，或在 Windows 安装匹配的 64 位 Oracle ODBC 驱动并在连接中明确选择。`ORA-01017` 表示已经到达数据库服务但账号认证失败，应核对密码大小写、Service Name 和账号密码验证器，而不是重新安装驱动。GBase 产品必须选择具体分支，不能把 8a/8s 当作 8c 的 PostgreSQL 协议连接。PG 通用协议已覆盖药品基础数据增量写入与安全撤销，以及机构库存的库房读取、首次盘点和安全撤销；覆盖写入仍需显式使用已验收的厂商 ODBC。首次盘点在正式写入前会验证所有业务字段，ODBC 回退按数据库家族选择日期表达式，不再统一套用 Oracle 语法。

## 驱动预置策略

驱动中心清单位于 `src-tauri/driver-packs/manifest.json`，编译时会嵌入桌面程序。它统一提供默认端口、协议、推荐驱动名、版本、识别关键词、交付方式、官方地址、逐步安装指引和平台优先级。

| 数据库 | 预置方案 | 二进制交付 |
| --- | --- | --- |
| MySQL | 原生 Rust 驱动 | 已内置 |
| PostgreSQL | 通用 PG 前端协议 | 已内置 |
| Gauss/openGauss | 通用 PG 前端协议优先 | 已内置；特殊版本可显式回退厂商 ODBC |
| 海量 Vastbase G100 | 通用 PG 前端协议优先 | 已内置；项目专用驱动从安装介质或海量支持获取 |
| 南大通用 GBase 8c | 通用 PG 前端协议优先 | 已内置；厂商包从 GBase 8c 下载中心获取 |
| 人大金仓 KingbaseES | 通用 PG 前端协议优先 | 已内置；特殊兼容版本可回退厂商 ODBC |
| Oracle | Instant Client ODBC 19.31 64 位 | Windows x64 安装包内置完整 Basic + ODBC；自动登记和卸载应用专用驱动，保留 Oracle 许可文件 |
| 达梦 DM8 | 厂商 ODBC | 从达梦官方客户端或项目授权安装介质获取 |
| 南大通用 GBase 8a | 厂商 ODBC | 从官方下载中心查找；Windows 驱动缺失时按官方说明向技术支持申请 |
| 南大通用 GBase 8s | 厂商 ODBC | 从官方下载中心或项目安装介质获取 |

厂商驱动二进制必须与应用位数、操作系统和 CPU 架构一致，Windows 统一按 64 位准备。Oracle 包在构建时从官方地址取得、按固定 SHA-256 校验并保持原文件不变；其他未允许重新分发的厂商驱动只内置配置、检测规则和下载说明，不把授权二进制打入公共安装包。

## 代码结构

- `src/App.jsx`：迁移流程状态和命令编排，不再承载所有界面实现。
- `src/PrimaryFlowScreens.jsx`：登录、任务选择、数据来源和来源识别。
- `src/DatabaseConnections.jsx`：可复用数据库连接管理。
- `src/InventoryMigrationScreen.jsx` / `src/InventoryReview.jsx`：库存映射、核对、执行和撤销界面。
- `src/MigrationHistory.jsx` / `src/MigrationResults.jsx`：历史审计与校验结果。
- `src-tauri/src/inventory.rs`：库存模块入口；预检、撤销、正式写入和映射逻辑分别位于 `src-tauri/src/inventory/`。
