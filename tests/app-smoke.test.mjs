import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { transformWithEsbuild } from "vite";

test("main React flow compiles as JSX", async () => {
  const source = await readFile(new URL("../src/App.jsx", import.meta.url), "utf8");
  const result = await transformWithEsbuild(source, "App.jsx", {
    loader: "jsx",
    jsx: "automatic",
  });
  assert.match(result.code, /function App\(/);
});

test("all dropdowns use the searchable combobox and medicine previews use business labels", async () => {
  const uiSourceFiles = [
    "App.jsx",
    "AppChrome.jsx",
    "DatabaseConnections.jsx",
    "InventoryMigrationScreen.jsx",
    "InventoryReview.jsx",
    "MigrationHistory.jsx",
    "MigrationResults.jsx",
    "PrimaryFlowScreens.jsx",
    "migrationFields.js",
    "migrationPreview.js",
  ];
  const appSource = (
    await Promise.all(
      uiSourceFiles.map((file) =>
        readFile(new URL(`../src/${file}`, import.meta.url), "utf8"),
      ),
    )
  ).join("\n");
  const stylesSource = await readFile(
    new URL("../src/styles.css", import.meta.url),
    "utf8",
  );
  const selectSource = await readFile(
    new URL("../src/SearchableSelect.jsx", import.meta.url),
    "utf8",
  );
  const cargoSource = await readFile(
    new URL("../src-tauri/Cargo.toml", import.meta.url),
    "utf8",
  );
  const connectionSettingsSource = await readFile(
    new URL("../src-tauri/src/connection_settings.rs", import.meta.url),
    "utf8",
  );
  const rustAppSource = await readFile(
    new URL("../src-tauri/src/lib.rs", import.meta.url),
    "utf8",
  );
  assert.doesNotMatch(`${appSource}\n${selectSource}`, /<(?:select|datalist)\b/i);
  assert.match(appSource, /<SearchableSelect/);
  assert.match(appSource, /DRUG_NAME: "药品名称"/);
  assert.match(appSource, /SPEC: "制剂规格"/);
  assert.match(appSource, /主数据读取与自动合并/);
  assert.match(appSource, /机构在用范围只看/);
  assert.match(appSource, /YK_CDXX，不关联 YK_YPXX、YF_YPXX/);
  assert.match(appSource, /名称、规格、最小单位一致/);
  assert.match(appSource, /每个 YPXH:YPCD 仍分别保留迁移映射/);
  assert.match(appSource, /任务 B · 已开放/);
  assert.match(appSource, /inspect_phis27_inventory/);
  assert.match(appSource, /读取库存并核对台账/);
  assert.match(appSource, /load_inventory_target_storages/);
  assert.match(appSource, /load_inventory_target_organizations/);
  assert.match(appSource, /load_phis27_inventory_catalog/);
  assert.match(appSource, /load_inventory_organization_mappings/);
  assert.match(appSource, /load_inventory_location_mappings/);
  assert.match(appSource, /prepare_phis27_inventory/);
  assert.match(appSource, /读取机构与库房清单/);
  assert.match(appSource, /老系统机构 → 新系统机构/);
  assert.match(appSource, /老系统药库\/药房 → 新系统库房/);
  assert.match(appSource, /机构与库房对应关系/);
  assert.match(appSource, /同名自动匹配/);
  assert.match(appSource, /检查已完成机构/);
  assert.match(appSource, /本批只处理已完整映射的机构/);
  assert.match(appSource, /请至少完整映射一个机构及其全部药库\/药房/);
  assert.match(appSource, /第 3 步 · 数据来源[\s\S]*返回选择任务/);
  assert.match(appSource, /查看读取明细/);
  assert.match(appSource, /老系统库存已读取/);
  assert.match(appSource, /新系统机构与库房已读取/);
  assert.match(appSource, /storage\.storageType === expectedType/);
  assert.match(appSource, /storage\.organizationId === targetOrganizationId/);
  assert.match(appSource, /药库（sdSto=1）/);
  assert.match(appSource, /药房（sdSto=2）/);
  assert.match(appSource, /hi_sto_dept 未配置有效药库\/药房/);
  assert.match(selectSource, /disabled: Boolean\(option\.disabled\)/);
  assert.match(selectSource, /aria-disabled=\{option\.disabled\}/);
  assert.match(appSource, /保存映射并检查重复/);
  assert.match(appSource, /库房 \+ 药品商品 \+ 进销价格 \+ 批号 \+ 效期/);
  assert.match(appSource, /正式执行首次盘点/);
  assert.match(appSource, /当天日期 \+ 3 位流水/);
  assert.match(appSource, /目标库房已有盘点或库存，整库就会被阻止/);
  assert.match(appSource, /进货金额合计/);
  assert.match(appSource, /零售金额合计/);
  assert.match(appSource, /预计零售差额/);
  assert.match(appSource, /库房管理包装/);
  assert.match(appSource, /商品主档包装/);
  assert.match(appSource, /normalizedData\?\.unitSale/);
  assert.match(appSource, /packagingNotes/);
  assert.match(appSource, /inventoryExecutionLock/);
  assert.match(appSource, /已等待 \{inventoryExecutionSeconds\} 秒/);
  assert.match(appSource, /正在执行 · \$\{inventoryExecutionSeconds\}秒/);
  assert.match(appSource, /preview_phis27_inventory_undo/);
  assert.match(appSource, /undo_phis27_inventory/);
  assert.match(appSource, /安全撤销首次盘点/);
  assert.match(appSource, /发现后续业务，不能撤销/);
  assert.match(appSource, /库房药品配置 hi_sto_med 将保留/);
  assert.match(appSource, /药库数量取 YK_KCMX\.KCSL/);
  assert.match(appSource, /药房数量取 YF_KCMX\.YPSL/);
  assert.doesNotMatch(
    appSource,
    /<button className="task-card task-card--disabled" disabled>/,
  );
  assert.match(appSource, /高级选项：使用自定义只读 SQL/);
  assert.match(appSource, /执行自定义 SQL 并预览/);
  assert.match(appSource, /请先填写要执行的自定义只读 SQL/);
  assert.doesNotMatch(
    appSource,
    /const \[query, setQuery\] = useState\("SELECT \* FROM T_DRUG_INFO"\)/,
  );
  assert.match(appSource, /随机换一条/);
  assert.match(appSource, /指定预览药品/);
  assert.match(appSource, /新系统字典含义/);
  assert.match(appSource, /二系列来源值/);
  assert.match(appSource, /sourceDictionaryText/);
  assert.match(appSource, /columnMetadata=\{columnMetadata\}/);
  assert.match(appSource, /二系列字典 → 新系统字典/);
  assert.match(appSource, /来源为 NULL、空字符串或仅空格/);
  assert.match(appSource, /EMPTY_VALUE_MAPPING_SOURCE/);
  assert.match(appSource, /一键按含义匹配/);
  assert.match(appSource, /忽略此来源值/);
  assert.match(appSource, /已忽略，不写入新系统/);
  assert.match(appSource, /该来源值已确认忽略，不参与目标字典校验/);
  assert.match(appSource, /待补充来源含义/);
  assert.match(appSource, /currentSourceDictionary\.entry/);
  assert.match(appSource, /currentSourceDictionary\.loadMessage/);
  assert.match(appSource, /每日次数/);
  assert.match(appSource, /candidate\.score >= 70/);
  assert.match(appSource, /\.\.\.sourceFieldOptions/);
  assert.match(appSource, /metadataByName\[column\]\?\.mappingEligible !== false/);
  assert.match(appSource, /lengthSimilarity >= 0\.8/);
  assert.match(appSource, /label: sourceFieldDisplayName\(column, metadata\)/);
  assert.match(appSource, /sourceFieldPhysicalOrigin/);
  assert.match(appSource, /sourceFieldMatchScore/);
  assert.match(appSource, /"ZXDW"/);
  assert.doesNotMatch(appSource, /<strong>\{item\.column\}<\/strong>/);
  assert.match(appSource, /const targetFieldLocations =/);
  assert.match(appSource, /naMed: \["hi_bd_med\.na_med"\]/);
  assert.match(appSource, /cdAppr: \["hi_bd_med_pro\.cd_appr"\]/);
  assert.match(appSource, /hi_bd_fac\.na_fac/);
  assert.match(appSource, /新系统落点/);
  assert.match(appSource, /targetLocations\(currentField\)/);
  assert.match(appSource, /className="rule-target-location"/);
  assert.match(appSource, /targetPhysicalLocationText\(field\)/);
  assert.match(appSource, /const booleanTargetFieldKeys = new Set/);
  assert.match(appSource, /defaultTransformForField\(field\)/);
  assert.match(appSource, /含 1\/2、RX\/OTC/);
  assert.match(appSource, /新系统字段允许为空；老系统有默认频次时再映射/);
  assert.match(appSource, /验证地址（可选）/);
  assert.match(appSource, /新系统地址与租户登录/);
  assert.match(appSource, /access-card access-card--combined/);
  assert.match(appSource, /新系统连接成功/);
  assert.match(appSource, /<strong>新系统已连接<\/strong>/);
  assert.match(appSource, /<dt>登录租户<\/dt>/);
  assert.match(appSource, /名称：\{targetAuth\.tenantName\}/);
  assert.doesNotMatch(
    appSource,
    /target-auth-summary__status[\s\S]{0,220}<strong>\{targetAuth\.tenantName/,
  );
  assert.match(appSource, /修改连接/);
  assert.match(appSource, /重新认证并更新连接/);
  assert.match(appSource, /上次执行中断，可重试/);
  assert.match(appSource, /重新执行中断批次/);
  assert.match(appSource, /数据库连接管理/);
  assert.match(appSource, /海量 Vastbase G100/);
  assert.match(appSource, /南大通用 GBase 8c/);
  assert.match(appSource, /南大通用 GBase 8a/);
  assert.match(appSource, /南大通用 GBase 8s/);
  assert.match(appSource, /内置 PostgreSQL 通用协议/);
  assert.match(appSource, /厂商 ODBC 兼容回退（可选）/);
  assert.match(appSource, /通用 PG 协议已覆盖连接、读取和目标结构检查/);
  assert.match(appSource, /查看下载安装步骤/);
  assert.match(appSource, /Windows x64 → macOS Apple Silicon → macOS Intel/);
  assert.match(cargoSource, /sqlx-postgres/);
  assert.match(appSource, /历史迁移日志/);
  assert.match(appSource, /list_recent_batches/);
  assert.match(appSource, /load_migration_batch/);
  assert.match(appSource, /筛选迁移任务/);
  assert.match(appSource, /筛选迁移状态/);
  assert.match(appSource, /批次概览/);
  assert.match(appSource, /迁移明细 \$\{detail\.rows\.length\}/);
  assert.match(appSource, /审计日志 \$\{detail\.audits\.length\}/);
  assert.match(appSource, /查看前后数据快照/);
  assert.match(appSource, /不需要重新登录新系统/);
  assert.match(stylesSource, /\.migration-history\s*\{/);
  assert.match(appSource, /const createEntry = \(\) =>/);
  assert.match(appSource, /className="connection-editor__content"/);
  assert.match(appSource, /draft\.connectionId[\s\S]*\? "更新连接"[\s\S]*: "保存连接"/);
  assert.match(stylesSource, /\.searchable-select__menu\s*\{[\s\S]*?z-index: 200/);
  assert.match(stylesSource, /\.connection-editor__actions\s*\{[\s\S]*?flex: 0 0 auto/);
  const connectionActionStyles =
    stylesSource.match(/\.connection-editor__actions\s*\{([^}]*)\}/)?.[1] || "";
  assert.doesNotMatch(connectionActionStyles, /position:\s*sticky/);
  assert.doesNotMatch(connectionActionStyles, /backdrop-filter/);
  assert.match(appSource, /复用已保存连接/);
  assert.match(appSource, /用于老库读取/);
  assert.match(appSource, /用于目标库写入/);
  assert.match(appSource, /list_database_connections/);
  assert.match(appSource, /save_database_connection/);
  assert.match(appSource, /delete_database_connection/);
  assert.match(appSource, /!targetAuth \|\| editingTargetConnection/);
  assert.match(appSource, /setEditingTargetConnection\(false\)/);
  assert.match(appSource, /target-auth-summary__details/);
  assert.match(appSource, /function ValidationResults/);
  assert.match(appSource, /fieldsMentionedInValidationError/);
  assert.match(appSource, /维护“\{field\.label\}”映射/);
  assert.match(appSource, /function openValidationFieldMapping/);
  assert.match(appSource, /从校验失败定位：第/);
  assert.match(appSource, /返回校验结果/);
  assert.match(appSource, /setMappingSampleIndex\(row\.rowNo - 1\)/);
  assert.match(appSource, /detail\.batch\.failCount > 0 \? "INVALID" : "VALIDATED"/);
  assert.match(appSource, /当前显示 \{visibleRows\.length\} 条/);
  assert.doesNotMatch(appSource, /batchDetail\.rows\.slice\(0, 12\)/);
  assert.match(stylesSource, /\.issue-list__row--danger p/);
  assert.match(stylesSource, /\.validation-jump-context/);
  assert.match(stylesSource, /white-space: normal/);
  assert.match(stylesSource, /min-height: calc\(100vh - 136px\)/);
  assert.match(stylesSource, /min-height: calc\(100vh - 128px\)/);
  assert.doesNotMatch(appSource, /access-card__number">2/);
  assert.match(appSource, /targetSystemProbe\?\.baseUrl \|\| targetSystemUrl/);
  assert.doesNotMatch(appSource, /请先验证新系统地址/);
  assert.doesNotMatch(appSource, /access-card--locked/);
  assert.doesNotMatch(appSource, /disabled=\{!targetSystemProbe/);
  assert.doesNotMatch(cargoSource, /security-framework/);
  assert.doesNotMatch(connectionSettingsSource, /security_framework/);
  assert.match(connectionSettingsSource, /AES_256_GCM/);
  assert.match(connectionSettingsSource, /credentials\.key/);
  assert.match(connectionSettingsSource, /database_connection_library_v1/);
  assert.match(connectionSettingsSource, /DATABASE_CONNECTION_PASSWORD_PREFIX/);
  assert.match(rustAppSource, /list_database_connections/);
  assert.match(rustAppSource, /save_database_connection/);
  assert.match(rustAppSource, /delete_database_connection/);
  assert.match(appSource, /load_saved_connections/);
  assert.match(appSource, /load_phis27_mapping_profile/);
  assert.match(appSource, /save_phis27_mapping_profile/);
  assert.match(appSource, /已恢复固化映射/);
  assert.match(appSource, /数据库注释仍按本次连接实时刷新/);
  assert.match(appSource, /Object\.prototype\.hasOwnProperty\.call\(restoredMapping/);
  assert.match(appSource, /自动记住连接参数/);
  assert.match(appSource, /同时记住密码（本地加密）/);
  assert.match(appSource, /忘记已保存连接/);
  assert.match(appSource, /tk 会话仍需重新认证/);
  assert.match(selectSource, /option\.description/);
  assert.match(selectSource, /option\.meta/);
});
