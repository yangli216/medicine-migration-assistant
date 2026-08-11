import {
  ArrowLeft,
  ArrowRight,
  Check,
  CheckCircle,
  Code,
  Database,
  FileCsv,
  FirstAidKit,
  FloppyDisk,
  Info,
  ListMagnifyingGlass,
  LockKey,
  Plugs,
  Rows,
  ShieldCheck,
  Table,
  UploadSimple,
  Warning,
} from "@phosphor-icons/react";
import { demoRows } from "./api";
import {
  ConnectionForm,
  ConnectionPicker,
  Field,
} from "./DatabaseConnections";
import { DataTable } from "./MigrationResults";
import { SearchableSelect } from "./SearchableSelect";

function isPhis27Source(description) {
  return description.startsWith("二系列phis内置模板:");
}

export function SystemConnectionScreen({ context }) {
  const {
    busy,
    costMergeCatalog,
    dictionaryCatalog,
    editingTargetConnection,
    forgetTargetSystemConnection,
    hasSavedTargetSystem,
    loginPassword,
    loginTargetSystem,
    loginTenantId,
    probeTargetSystem,
    rememberTargetSystemPassword,
    setEditingTargetConnection,
    setLoginPassword,
    setLoginTenantId,
    setRememberTargetSystemPassword,
    setStep,
    setTargetSystemProbe,
    setTargetSystemUrl,
    step,
    targetAuth,
    targetSystemProbe,
    targetSystemUrl,
  } = context;

  return (
    <>
{step === 0 && (
  <section className="screen access-screen">
    <div className="screen-heading">
      <span className="eyebrow">第 1 步 · 新系统身份确认</span>
      <h1>先连接要迁入的新系统</h1>
      <p>
        可直接使用 system 租户管理员账号认证；“验证地址”仅用于提前排查网络连通性。成功建立 tk 登录会话后开放数据迁移功能。
      </p>
    </div>

    <div className="access-grid">
      <form
        className="access-card access-card--combined"
        onSubmit={loginTargetSystem}
      >
        <div className="access-card__number">1</div>
        <div className="access-card__body">
          <div className="section-title">
            <LockKey size={21} weight="duotone" />
            <div>
              <strong>
                {targetAuth ? "新系统连接成功" : "新系统地址与租户登录"}
              </strong>
              <small>
                {targetAuth
                  ? "登录会话与迁移所需基础数据均已就绪"
                  : "地址验证是可选检查，填写租户和 system 密码后可直接认证"}
              </small>
            </div>
          </div>
          {targetAuth && (
            <div className="target-auth-summary">
              <div className="target-auth-summary__lead">
                <span className="target-auth-summary__icon">
                  <ShieldCheck size={23} weight="fill" />
                </span>
                <div>
                  <span className="target-auth-summary__status">连接成功</span>
                  <strong>新系统已连接</strong>
                  <small>tk 登录会话已建立，后续请求将自动携带认证信息</small>
                </div>
              </div>
              <button
                className="button button--secondary target-auth-summary__edit"
                type="button"
                aria-expanded={editingTargetConnection}
                onClick={() =>
                  setEditingTargetConnection((expanded) => !expanded)
                }
              >
                {editingTargetConnection ? "收起修改" : "修改连接"}
              </button>
              <dl className="target-auth-summary__details">
                <div>
                  <dt>访问地址</dt>
                  <dd>{targetAuth.baseUrl || targetSystemUrl}</dd>
                </div>
                <div>
                  <dt>登录租户</dt>
                  <dd>
                    {targetAuth.tenantId || "未返回编码"}
                    {targetAuth.tenantName && (
                      <span>名称：{targetAuth.tenantName}</span>
                    )}
                  </dd>
                </div>
                <div>
                  <dt>登录身份</dt>
                  <dd>
                    {targetAuth.userName || "system"}
                    <span>
                      {targetAuth.roleName} · {targetAuth.roleCd}
                    </span>
                  </dd>
                </div>
              </dl>
              <div className="target-auth-summary__sync">
                {dictionaryCatalog && (
                  <span>
                    <CheckCircle size={15} weight="fill" />
                    已同步 {dictionaryCatalog.dictionaries.length} 个药品标准字典
                  </span>
                )}
                {costMergeCatalog && (
                  <span>
                    <CheckCircle size={15} weight="fill" />
                    已同步 {costMergeCatalog.items.length} 个费用归并项目
                  </span>
                )}
                {dictionaryCatalog?.warnings?.length ? (
                  <span className="target-auth-summary__warning">
                    <Warning size={15} weight="fill" />
                    {dictionaryCatalog.warnings.length} 个可选字典待处理
                  </span>
                ) : null}
              </div>
            </div>
          )}
          {(!targetAuth || editingTargetConnection) && (
            <div className="target-connection-editor">
              <div className="access-inline-form">
                <Field label="新系统访问地址">
                  <input
                    value={targetSystemUrl}
                    onChange={(event) => {
                      setTargetSystemUrl(event.target.value);
                      setTargetSystemProbe(null);
                    }}
                    placeholder="http://服务器:端口/rbmh-phis"
                    spellCheck="false"
                  />
                </Field>
                <button
                  className="button button--secondary"
                  type="button"
                  disabled={busy === "target-system-probe"}
                  onClick={probeTargetSystem}
                >
                  <Plugs size={19} />
                  {busy === "target-system-probe"
                    ? "正在验证…"
                    : "验证地址（可选）"}
                </button>
              </div>
              {targetSystemProbe && (
                <div className="access-result access-result--success">
                  <CheckCircle size={19} weight="fill" />
                  <div>
                    <strong>地址可用</strong>
                    <span>
                      HTTP {targetSystemProbe.statusCode} · {targetSystemProbe.latencyMs}ms
                    </span>
                  </div>
                </div>
              )}
              {targetSystemUrl.trim().toLowerCase().startsWith("http://") && (
                <div className="access-result access-result--warning">
                  <Warning size={19} weight="fill" />
                  <div>
                    <strong>当前使用内网 HTTP</strong>
                    <span>MD5 仅符合登录接口协议，不等同于传输加密；正式环境建议启用 HTTPS。</span>
                  </div>
                </div>
              )}
              <div className="auth-form-grid access-auth-fields">
                <Field label="租户编码">
                  <input
                    value={loginTenantId}
                    onChange={(event) => setLoginTenantId(event.target.value)}
                    placeholder="例如 cszzyzh"
                    autoComplete="organization"
                  />
                </Field>
                <Field label="登录账号">
                  <input value="system" disabled aria-label="登录账号" />
                </Field>
                <Field label="system 密码" wide>
                  <div className="password-input">
                    <LockKey size={17} />
                    <input
                      type="password"
                      value={loginPassword}
                      onChange={(event) => setLoginPassword(event.target.value)}
                      placeholder={
                        targetAuth
                          ? "请输入密码以重新认证"
                          : "请输入 system 密码"
                      }
                      autoComplete="current-password"
                    />
                  </div>
                </Field>
              </div>
              <div className="credential-preference credential-preference--auth">
                <div className="credential-preference__summary">
                  <CheckCircle size={16} weight="fill" />
                  <span>
                    <strong>自动记住新系统地址和租户</strong>
                    <small>下次启动自动恢复，tk 会话仍需重新认证。</small>
                  </span>
                </div>
                <label className="credential-preference__password">
                  <input
                    type="checkbox"
                    checked={rememberTargetSystemPassword}
                    onChange={(event) =>
                      setRememberTargetSystemPassword(event.target.checked)
                    }
                  />
                  <span>同时记住密码（本地加密）</span>
                </label>
                {hasSavedTargetSystem && (
                  <button
                    className="button button--ghost"
                    type="button"
                    onClick={forgetTargetSystemConnection}
                  >
                    忘记已保存信息
                  </button>
                )}
              </div>
              <button
                className="button button--primary button--wide"
                type="submit"
                disabled={busy === "target-system-login"}
              >
                <ShieldCheck size={19} />
                {busy === "target-system-login"
                  ? "正在认证…"
                  : targetAuth
                    ? "重新认证并更新连接"
                    : "认证并建立 tk 登录会话"}
              </button>
            </div>
          )}
        </div>
      </form>
    </div>

    <div className="security-banner">
      <ShieldCheck size={20} weight="fill" />
      <div>
        <strong>双重门禁</strong>
        <span>
          前端固定 system，桌面后端会再次检查账号、租户、角色编码、启用状态和 tk 下发结果，后续服务请求自动携带 Cookie。
        </span>
      </div>
    </div>
    <div className="screen-actions screen-actions--end">
      <button
        className="button button--primary"
        disabled={!targetAuth}
        onClick={() => setStep(1)}
      >
        认证完成，选择迁移任务
        <ArrowRight />
      </button>
    </div>
  </section>
)}
    </>
  );
}

export function TaskSelectionScreen({ context }) {
  const {
    migrationType,
    setMigrationType,
    setStep,
    step,
    targetAuth,
  } = context;

  return (
    <>
{step === 1 && (
  <section className="screen task-screen">
    <div className="screen-heading screen-heading--row">
      <div>
        <span className="eyebrow">第 2 步 · 迁移任务</span>
        <h1>这次要迁移什么？</h1>
        <p>两类任务相互独立，库存初始化必须建立在药品基础信息已经同步的前提下。</p>
      </div>
      <div className="source-summary">
        <ShieldCheck size={24} />
        <div>
          <small>已认证新系统</small>
          <strong>
            租户 {targetAuth?.tenantId}
            {targetAuth?.tenantName
              ? `（${targetAuth.tenantName}）`
              : ""} · system
          </strong>
        </div>
      </div>
    </div>
    <div className="task-grid">
      <button
        className={`task-card ${migrationType === "MEDICINE_BASE" ? "task-card--selected" : ""}`}
        onClick={() => setMigrationType("MEDICINE_BASE")}
      >
        <span className="task-card__icon">
          <FirstAidKit size={28} weight="duotone" />
        </span>
        <span className="task-card__copy">
          <small>任务 A · 已开放</small>
          <strong>药品基础信息同步</strong>
          <p>
            同步通用药品、单位、别名、生产厂家和药品商品信息，为后续机构库存初始化建立主键对照。
          </p>
          <em>全局数据 · 支持预校验、幂等复用和失败重试</em>
        </span>
        <span className="task-card__check">
          {migrationType === "MEDICINE_BASE" && (
            <Check size={17} weight="bold" />
          )}
        </span>
      </button>
      <button
        className={`task-card ${migrationType === "INVENTORY" ? "task-card--selected" : ""}`}
        onClick={() => setMigrationType("INVENTORY")}
      >
        <span className="task-card__icon">
          <Database size={28} weight="duotone" />
        </span>
        <span className="task-card__copy">
          <small>任务 B · 已开放</small>
          <strong>机构库房初始化</strong>
          <p>
            选择具体机构、药库或药房，在基础药品匹配完整后按批次同步库存数量、价格、批号和效期。
          </p>
          <em>机构数据 · 需要库房映射和库存初始化防重锁</em>
        </span>
        <span className="task-card__check">
          {migrationType === "INVENTORY" && (
            <Check size={17} weight="bold" />
          )}
        </span>
      </button>
    </div>
    <div className="prerequisite-note">
      <Info size={19} />
      <span>
        库存初始化读取 YK_KCMX / YF_KCMX，并用已核实的
        YPXH:YPCD 台账检查每个库存药品；完成机构与库房映射后，按首次盘点建立初始账簿。
      </span>
    </div>
    <div className="screen-actions">
      <button
        className="button button--secondary"
        onClick={() => setStep(0)}
      >
        <ArrowLeft />
        返回认证
      </button>
      <button
        className="button button--primary"
        onClick={() => setStep(2)}
      >
        {migrationType === "INVENTORY"
          ? "进入机构库存初始化"
          : "进入药品基础信息同步"}
        <ArrowRight />
      </button>
    </div>
  </section>
)}
    </>
  );
}

export function MedicineSourceScreen({ context }) {
  const {
    acceptData,
    busy,
    databaseConnections,
    databaseDrivers,
    driverPacks,
    fileInput,
    forgetSourceConnection,
    hasSavedSourceConnection,
    inspectPhis27Source,
    legacyInspection,
    legacyScope,
    loadDatabase,
    loadFile,
    loadPhis27Medicine,
    migrationType,
    query,
    rememberSourcePassword,
    selectDatabaseConnection,
    selectedSourceConnectionId,
    setConnectionManagerOpen,
    setLegacyInspection,
    setLegacyScope,
    setQuery,
    setRememberSourcePassword,
    setSelectedSourceConnectionId,
    setShowCustomQuery,
    setSourceConnectionEditing,
    setSourceMode,
    setSourceProfile,
    setStep,
    showCustomQuery,
    sourceConnectionEditing,
    sourceMode,
    sourceProfile,
    step,
    testSourceConnection,
  } = context;

  return (
    <>
{step === 2 && migrationType === "MEDICINE_BASE" && (
  <section className="screen source-screen">
    <div className="screen-heading screen-heading--row">
      <div>
        <span className="eyebrow">第 3 步 · 数据来源</span>
        <h1>三方数据从哪里来？</h1>
        <p>
          可以直接连接老数据库，也可以导入 CSV / JSON
          文件。数据库模式只执行只读查询。
        </p>
      </div>
      <button
        className="button button--secondary button--compact"
        type="button"
        onClick={() => setStep(1)}
      >
        <ArrowLeft size={17} />
        返回选择任务
      </button>
    </div>
    <div className="source-tabs">
      <button
        className={sourceMode === "file" ? "active" : ""}
        onClick={() => setSourceMode("file")}
      >
        <FileCsv size={22} />
        文件导入
      </button>
      <button
        className={sourceMode === "database" ? "active" : ""}
        onClick={() => setSourceMode("database")}
      >
        <Database size={22} />
        数据库直连
      </button>
    </div>
    {sourceMode === "file" ? (
      <div
        className="upload-zone"
        onClick={() => fileInput.current?.click()}
      >
        <input
          ref={fileInput}
          type="file"
          accept=".csv,.json"
          hidden
          onChange={(event) =>
            event.target.files?.[0] && loadFile(event.target.files[0])
          }
        />
        <UploadSimple size={44} weight="duotone" />
        <h2>选择三方数据文件</h2>
        <p>支持 UTF-8 CSV、JSON；首行作为字段名，单批最多 10,000 行</p>
        <button className="button button--primary">选择文件</button>
      </div>
    ) : (
      <div className="source-db-layout">
        <ConnectionPicker
          purpose="SOURCE"
          entries={databaseConnections}
          selectedId={selectedSourceConnectionId}
          onSelect={(connectionId) =>
            selectDatabaseConnection(connectionId, "SOURCE")
          }
          onManage={() => setConnectionManagerOpen(true)}
          editing={sourceConnectionEditing}
          onToggleEditing={() =>
            setSourceConnectionEditing((current) => !current)
          }
        />
        {(!selectedSourceConnectionId || sourceConnectionEditing) && (
          <ConnectionForm
            value={sourceProfile}
            onChange={(next) => {
              setSourceProfile(next);
              setSelectedSourceConnectionId("");
              setSourceConnectionEditing(true);
              setLegacyInspection(null);
            }}
            title="老系统只读连接"
            drivers={databaseDrivers}
            driverPacks={driverPacks}
            rememberPassword={rememberSourcePassword}
            onRememberPasswordChange={setRememberSourcePassword}
            hasSavedConnection={hasSavedSourceConnection}
            onForgetSaved={forgetSourceConnection}
            showCredentialPreference={false}
          />
        )}
        {sourceProfile.kind === "oracle" && (
          <div className="legacy-adapter">
            <div className="legacy-adapter__heading">
              <div>
                <span className="eyebrow">内置适配器</span>
                <strong>自动识别二系列phis药品数据</strong>
                <small>
                  自动核对核心表、统计数据范围，并生成只读多表组合查询。
                </small>
              </div>
              <button
                className="button button--secondary"
                disabled={busy === "phis27-inspect"}
                onClick={inspectPhis27Source}
              >
                <ListMagnifyingGlass size={19} />
                {busy === "phis27-inspect"
                  ? "正在识别…"
                  : "识别二系列phis"}
              </button>
            </div>
            {legacyInspection && (
              <div className="legacy-inspection">
                <div
                  className={`access-result ${legacyInspection.detected ? "access-result--success" : "access-result--warning"}`}
                >
                  {legacyInspection.detected ? (
                    <CheckCircle size={19} weight="fill" />
                  ) : (
                    <Warning size={19} weight="fill" />
                  )}
                  <div>
                    <strong>{legacyInspection.message}</strong>
                    <span>
                      Schema {legacyInspection.schema} · 已核对 {legacyInspection.checkedTables.length} 张表
                    </span>
                  </div>
                </div>
                {legacyInspection.detected && (
                  <>
                    <div className="legacy-stats">
                      {[
                        ["通用药品", legacyInspection.totalMedicines],
                        ["机构配置", legacyInspection.configuredMedicines],
                        ["在用药品", legacyInspection.activeConfiguredMedicines],
                        ["厂家商品", legacyInspection.productRows],
                        ["有库存药品", legacyInspection.stockMedicines],
                      ].map(([label, value]) => (
                        <div key={label}>
                          <span>{label}</span>
                          <strong>{value}</strong>
                        </div>
                      ))}
                    </div>
                    <div className="legacy-scopes">
                      {legacyInspection.scopes.map((scope) => (
                        <label
                          className={`legacy-scope ${legacyScope === scope.id ? "legacy-scope--selected" : ""}`}
                          key={scope.id}
                        >
                          <input
                            type="radio"
                            name="legacy-scope"
                            checked={legacyScope === scope.id}
                            onChange={() => setLegacyScope(scope.id)}
                          />
                          <span>
                            <strong>
                              {scope.label}
                              {scope.recommended && <em>推荐</em>}
                            </strong>
                            <small>{scope.description}</small>
                          </span>
                          <b>
                            {scope.medicineCount} 种 / 约 {scope.estimatedRows} 行
                          </b>
                        </label>
                      ))}
                    </div>
                    <div className="legacy-base-note">
                      <CheckCircle size={17} weight="fill" />
                      <span>
                        <strong>主数据读取与自动合并</strong>
                        药品基础信息来自 YK_TYPK，厂家商品来自 YK_YPCD；机构在用范围只看
                        YK_CDXX，不关联 YK_YPXX、YF_YPXX。名称、规格、最小单位一致的多个 YPXH
                        会复用同一新药品，每个 YPXH:YPCD 仍分别保留迁移映射。
                      </span>
                    </div>
                    <div className="legacy-warnings">
                      {legacyInspection.warnings.map((warning) => (
                        <span key={warning}>
                          <Warning size={15} />
                          {warning}
                        </span>
                      ))}
                    </div>
                    <button
                      className="button button--primary button--wide"
                      disabled={busy === "phis27-load"}
                      onClick={loadPhis27Medicine}
                    >
                      <Rows size={19} />
                      {busy === "phis27-load"
                        ? "正在读取标准数据…"
                        : "按选定范围读取标准药品数据"}
                    </button>
                  </>
                )}
              </div>
            )}
          </div>
        )}
        {sourceProfile.kind === "oracle" && (
          <div className="custom-query-toggle">
            <button
              className="button button--ghost"
              type="button"
              aria-expanded={showCustomQuery}
              onClick={() => setShowCustomQuery((current) => !current)}
            >
              <Code size={17} />
              {showCustomQuery
                ? "收起自定义 SQL"
                : "高级选项：使用自定义只读 SQL"}
            </button>
            <span>
              仅用于临时核对其他表，不使用二系列范围选择和标准字段组装。
            </span>
          </div>
        )}
        {(sourceProfile.kind !== "oracle" || showCustomQuery) && (
          <div className="custom-query-panel">
            <Field label="自定义读取 SQL" wide>
              <textarea
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                rows={5}
                placeholder="请输入当前数据库中真实存在的表或视图，例如 SELECT * FROM 表名"
              />
              <small>
                仅允许 SELECT / WITH；自定义结果不会自动套用二系列标准多表映射。
              </small>
            </Field>
            <div className="source-actions">
              <button
                className="button button--secondary"
                type="button"
                disabled={["source", "source-test"].includes(busy)}
                onClick={testSourceConnection}
              >
                <Plugs size={20} />
                {busy === "source-test" ? "正在测试…" : "仅测试连接"}
              </button>
              <button
                className="button button--primary"
                type="button"
                disabled={
                  !query.trim() ||
                  ["source", "source-test"].includes(busy)
                }
                onClick={loadDatabase}
              >
                <Rows size={20} />
                {busy === "source"
                  ? "正在读取…"
                  : "执行自定义 SQL 并预览"}
              </button>
            </div>
          </div>
        )}
      </div>
    )}
    <div className="demo-line">
      <span>还没有数据？</span>
      <button
        onClick={() =>
          acceptData(demoRows(), "内置演示数据 · T_DRUG_INFO")
        }
      >
        使用示例药品数据体验完整流程 <ArrowRight size={16} />
      </button>
    </div>
  </section>
)}
    </>
  );
}

export function SourceRecognitionScreen({ context }) {
  const {
    beginMapping,
    columnMetadata,
    columns,
    medicinePreviewColumns,
    medicinePreviewLabels,
    phis27MappingStatus,
    rows,
    setSourceKey,
    setStep,
    sourceDescription,
    sourceKey,
    sourceName,
    step,
  } = context;

  return (
    <>
{step === 3 && (
  <section className="screen">
    <div className="screen-heading screen-heading--row">
      <div>
        <span className="eyebrow">第 4 步 · 识别数据</span>
        <h1>确认要迁移的数据范围</h1>
        <p>
          系统已读取数据。请选择能代表三方记录唯一性的字段，它只用于追溯，不会作为新表主键。
        </p>
      </div>
      <div className="source-summary">
        <Database size={24} />
        <div>
          <small>当前来源</small>
          <strong>{sourceName}</strong>
        </div>
      </div>
    </div>
    <div className="recognition-grid">
      <div className="stat-panel">
        <div>
          <span>读取行数</span>
          <strong>{rows.length}</strong>
        </div>
        <div>
          <span>来源字段</span>
          <strong>{columns.length}</strong>
        </div>
        <div>
          <span>空记录</span>
          <strong>
            {
              rows.filter((row) =>
                Object.values(row).every(
                  (value) => `${value ?? ""}`.trim() === "",
                ),
              ).length
            }
          </strong>
        </div>
      </div>
      <Field label="三方记录唯一标识">
        <SearchableSelect
          ariaLabel="三方记录唯一标识"
          disabled={isPhis27Source(sourceDescription)}
          value={sourceKey}
          onChange={setSourceKey}
          options={columns.map((column) => ({
            value: column,
            label: column,
          }))}
          searchPlaceholder="过滤来源字段"
        />
        <small>
          {isPhis27Source(sourceDescription)
            ? "二系列phis已固定使用 YPXH:YPCD，确保每个厂家规格药品独立对应 id_med_pro。"
            : "例如药品编码或三方主键。它会写入本地迁移日志，便于反查老系统。"}
        </small>
      </Field>
    </div>
    {isPhis27Source(sourceDescription) && (
      <div className="legacy-base-note mapping-profile-note">
        <FloppyDisk size={17} weight="fill" />
        <span>
          <strong>
            {phis27MappingStatus?.restored
              ? "已恢复固化映射"
              : "首次核对后自动固化"}
          </strong>
          {phis27MappingStatus?.restored
            ? `已恢复 ${phis27MappingStatus.restoredCount} 个来源字段及其转换规则${phis27MappingStatus.missingCount ? `；${phis27MappingStatus.missingCount} 个旧字段已失效并重新生成建议` : ""}。数据库注释仍按本次连接实时刷新。`
            : "完成字段映射或进入校验时会保存到应用本地，以后直接复用；数据库注释仍会实时刷新。"}
        </span>
      </div>
    )}
    <div className="table-heading">
      <div>
        <Table size={20} />
        <strong>数据预览</strong>
        <span>前 {Math.min(6, rows.length)} 行</span>
      </div>
      <span className="read-only">
        <LockKey size={15} />
        只读
      </span>
    </div>
    <DataTable
      columns={columns}
      rows={rows}
      preferredColumns={
        sourceName.startsWith("二系列phis")
          ? medicinePreviewColumns
          : []
      }
      columnLabels={
        sourceName.startsWith("二系列phis")
          ? medicinePreviewLabels
          : {}
      }
      columnMetadata={columnMetadata}
    />
    <div className="screen-actions">
      <button
        className="button button--secondary"
        onClick={() => setStep(2)}
      >
        <ArrowLeft />
        重新选择
      </button>
      <button className="button button--primary" onClick={beginMapping}>
        开始匹配新系统字段
        <ArrowRight />
      </button>
    </div>
  </section>
)}
    </>
  );
}
