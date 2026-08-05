# 药品数据迁移助手

一个不依赖 Java 服务端、可直接交付给三方对接人员使用的 Tauri 桌面工具。界面负责引导配置，Rust 负责来源读取、数据转换、直接写表、逐行事务和本地审计。

## 当前能力

- 新系统门禁：验证目标系统访问地址，先通过 `/logon/myRoles` 取得并验证 `system` 的 `tenantSystem` 角色，再调用 `/logon/myApps` 完成角色登录；只有返回成功并下发非空 `tk` Cookie 后才允许进入和执行迁移。
- 任务入口：药品基础信息同步已开放；机构库房初始化作为独立任务展示，待库存目标表及机构、库房接口明确后接入。
- 来源：CSV、JSON、MySQL、Oracle、达梦 DM8、Gauss/openGauss、人大金仓 KingbaseES、PostgreSQL 只读查询；单批最多读取 10,000 行。
- PHIS27 适配：自动核对 `YK_TYPK`、`YK_YPCD`、`YK_CDDZ`、`YF_YPXX` 等药品表，提供“机构在用药品”和“机构全部配置药品”范围，并生成内置只读组合查询，无需手写 SQL。
- 映射：推荐映射、逐字段确认、完整映射专家模式；支持可视化“旧值 → 新值”字典、默认值、大小写、空白清洗、整数/小数、布尔和日期标准化。
- 在线字典：依据 `HiBdMed` 的 `@Dictionary(id=...)` 白名单，在 system 登录后复用 `tk` Cookie，以最多 6 路并发读取当前租户真实字典；界面展示编码与名称，可按编码、名称、拼音及常见组合格式生成安全映射，预校验会拒绝已加载字典范围外的编码。
- 费用归并：登录后调用 `api/base.tenantDicService/medicineCostMerge` 读取当前租户归并项目；按药品类型推荐“西药→西药费、草药→草药费、疫苗→疫苗费、耗材→卫生材料费、中药/民族药/院内制剂→成药费”，由对接人员逐项确认后写入 `hi_bd_med.id_cstmg`。
- 校验：必填、长度、枚举、数字、条件必填和业务冲突检查。
- 写入：通过 MySQL 原生驱动或企业数据库 ODBC 驱动，直接写入 `hi_bd_med`、`hi_bd_med_alias`、`hi_bd_med_unit`、`hi_bd_fac`、`hi_bd_med_pro`。
- 分层写入：没有厂家商品数据时仅写药品、单位和基础别名；存在厂家商品数据时才继续校验并写入厂家与商品表。
- 可靠性：每条来源记录使用独立数据库事务，单条失败不影响其它记录；可仅重试失败记录。
- 增量：本机 SQLite 维护“租户 + 来源 + 来源主键 → 目标主键与来源摘要”的迁移台账；来源未变化时自动跳过，检测到已迁移来源发生变化时阻止静默覆盖并提示进入覆盖流程。
- 幂等：药品和产品按业务唯一条件复用或跳过，避免重复执行产生重复数据。
- 审计：批次、行状态、目标主键、写入清单、前后数据摘要、错误和操作时间写入本机 SQLite。
- 安全撤销：覆盖记录先按字段级快照恢复，再删除审计中明确标记为本批次 `INSERT`、属于当前登录租户、且未被后续数据引用的记录；复用数据永不删除。撤销前必须核对执行时记录的目标库身份，存在后续引用时保留记录并标记为部分撤销；来源台账也同步恢复到覆盖前版本。
- 覆盖迁移：仅允许覆盖迁移台账确认由本工具创建或连续覆盖的药品、商品记录；正式执行前必须从目标库生成逐行字段差异，支持勾选部分记录，未勾选记录进入可审计的跳过状态。更新前保存实际数据库字段原值，更新与快照读取位于同一事务。厂家、包装单位采用匹配或新建，不直接覆盖共享主数据；暂不允许在“仅基础药品”和“含厂家商品”之间改变数据层级。
- 安全：新系统密码在 Rust 后端按 MD5 接口协议转换，`tk` 由内存 Cookie 会话维护并自动附加到后续请求，密码、`tk` 和数据库口令均不暴露给前端、不写入本地审计库；药品基础数据固定为租户级公共数据，`id_tet` 和操作人由后端登录会话强制赋值，不接收机构、科室上下文。当前按约定不迁移 `idSrv`。MD5 不是传输加密，正式环境仍建议使用 HTTPS。
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

Windows 安装包不会携带 macOS Oracle 动态库。Oracle 连接需要在目标 Windows 电脑上安装与应用同为 64 位的 Oracle Instant Client 19c Basic/Basic Light 与 ODBC 驱动；MySQL 无需额外驱动，达梦、高斯、人大金仓和 PostgreSQL 使用各厂商 Windows 64 位 ODBC 驱动。

Rust 单元测试：

```bash
cd src-tauri
cargo test --offline
```

前端转换规则测试：

```bash
npm test
```

## 数据库支持范围

MySQL 使用应用内置的原生 Rust 驱动。Oracle、达梦、Gauss/openGauss、人大金仓 KingbaseES 和 PostgreSQL 使用 64 位 ODBC 驱动；应用会自动列出本机已安装驱动，也允许填写带占位符的高级连接串。目标表结构或字段约束发生变化时，应同步调整 `src-tauri/src/target.rs` 与 `src-tauri/src/target_odbc.rs` 的写入语句，并分别使用对应数据库的测试实例验收。

## 驱动预置策略

驱动中心清单位于 `src-tauri/driver-packs/manifest.json`，编译时会嵌入桌面程序。它统一提供默认端口、推荐驱动名、版本、识别关键词、交付方式和许可提示。

| 数据库 | 预置方案 | 二进制交付 |
| --- | --- | --- |
| MySQL | 原生 Rust 驱动 | 已内置 |
| Oracle | Instant Client ODBC 19c；macOS Intel 使用官方 19.16 | macOS 包内置；Windows 安装包检测目标电脑已安装的 64 位 Oracle 19c ODBC 驱动 |
| 达梦 DM8 | 官方通用 ODBC 配置 | 从项目获授权的达梦客户端介质导入 |
| Gauss/openGauss | openGauss 6.0 LTS 通用配置 | 可按目标系统加入官方离线驱动包 |
| 人大金仓 KingbaseES | 8.6 通用 ODBC 配置 | 从项目获授权的金仓客户端介质导入 |
| PostgreSQL | psqlODBC 17.x Unicode 配置 | 可按目标系统加入官方离线驱动包 |

驱动二进制必须与目标操作系统和 CPU 架构一致。当前 macOS Intel 安装包已内置 Oracle Instant Client 19.16 Basic Light、ODBC 运行库及随包许可文件，无需另外安装 Oracle 客户端。达梦与人大金仓仍只嵌入清单和识别规则，其驱动应从项目获得授权的厂商介质导入。
