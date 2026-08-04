# 药品数据迁移助手

一个不依赖 Java 服务端、可直接交付给三方对接人员使用的 Tauri 桌面工具。界面负责引导配置，Rust 负责来源读取、数据转换、直接写表、逐行事务和本地审计。

## 当前能力

- 来源：CSV、JSON、MySQL、Oracle、达梦 DM8、Gauss/openGauss、人大金仓 KingbaseES、PostgreSQL 只读查询；单次预览最多 1000 行。
- 映射：推荐映射、逐字段确认、完整映射专家模式、转换与默认值。
- 校验：必填、长度、枚举、数字、条件必填和业务冲突检查。
- 写入：通过 MySQL 原生驱动或企业数据库 ODBC 驱动，直接写入 `hi_bd_med`、`hi_bd_med_alias`、`hi_bd_med_unit`、`hi_bd_fac`、`hi_bd_med_pro`。
- 可靠性：每条来源记录使用独立数据库事务，单条失败不影响其它记录；可仅重试失败记录。
- 幂等：药品和产品按业务唯一条件复用或跳过，避免重复执行产生重复数据。
- 审计：批次、行状态、目标主键、前后数据摘要、错误和操作时间写入本机 SQLite。
- 安全：数据库口令仅在运行时内存中使用，不写入本地审计库。
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
