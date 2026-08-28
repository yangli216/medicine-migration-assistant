import { useEffect, useMemo, useState } from "react";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  CheckCircle,
  Code,
  Database,
  DownloadSimple,
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
import { ConnectionForm, ConnectionPicker, Field } from "./DatabaseConnections";
import { DataTable } from "./MigrationResults";
import { targetFields } from "./migrationFields";
import { SearchableSelect } from "./SearchableSelect";
import { SourceAdapterDiagnosticPanel } from "./SourceAdapterDiagnosticPanel";
import { sourceObjectSuitability } from "./sourceFieldMatching";
import { sourceObjectOptions } from "./sourceDatabaseObjects";
import {
  assessSourceKeyColumns,
  assessSourceKeyFields,
  MAX_SOURCE_KEY_FIELDS,
  sourceKeyComponentOptions,
} from "./sourceKeyAssessment";
import {
  adapterProvidesSourceKey,
  medicineWorkflowFor,
  sourceKeyDisplayLabel,
} from "./sourceAdapterWorkflow";
import { sourceAdapterCapabilityAssessment } from "./sourceAdapterCapabilities";

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
              可直接使用 system
              租户管理员账号认证；“验证地址”仅用于提前排查网络连通性。成功建立
              tk 登录会话后开放数据迁移功能。
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
                        <span className="target-auth-summary__status">
                          连接成功
                        </span>
                        <strong>新系统已连接</strong>
                        <small>
                          tk 登录会话已建立，后续请求将自动携带认证信息
                        </small>
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
                          已同步 {dictionaryCatalog.dictionaries.length}{" "}
                          个药品标准字典
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
                            HTTP {targetSystemProbe.statusCode} ·{" "}
                            {targetSystemProbe.latencyMs}ms
                          </span>
                        </div>
                      </div>
                    )}
                    {targetSystemUrl
                      .trim()
                      .toLowerCase()
                      .startsWith("http://") && (
                      <div className="access-result access-result--warning">
                        <Warning size={19} weight="fill" />
                        <div>
                          <strong>当前使用内网 HTTP</strong>
                          <span>
                            MD5
                            仅符合登录接口协议，不等同于传输加密；正式环境建议启用
                            HTTPS。
                          </span>
                        </div>
                      </div>
                    )}
                    <div className="auth-form-grid access-auth-fields">
                      <Field label="租户编码">
                        <input
                          value={loginTenantId}
                          onChange={(event) =>
                            setLoginTenantId(event.target.value)
                          }
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
                            onChange={(event) =>
                              setLoginPassword(event.target.value)
                            }
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
                            setRememberTargetSystemPassword(
                              event.target.checked,
                            )
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
                前端固定
                system，桌面后端会再次检查账号、租户、角色编码、启用状态和 tk
                下发结果，后续服务请求自动携带 Cookie。
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
    sourceAdapters,
    step,
    targetAuth,
  } = context;
  const capability = useMemo(
    () => sourceAdapterCapabilityAssessment(sourceAdapters, migrationType),
    [migrationType, sourceAdapters],
  );
  const capabilityIcon = (tone) => {
    if (tone === "success") return <CheckCircle size={19} weight="fill" />;
    if (tone === "warning") return <Warning size={19} weight="fill" />;
    return <Code size={19} weight="duotone" />;
  };

  return (
    <>
      {step === 1 && (
        <section className="screen task-screen">
          <div className="screen-heading screen-heading--row">
            <div>
              <span className="eyebrow">第 2 步 · 迁移任务</span>
              <h1>这次要迁移什么？</h1>
              <p>
                两类任务相互独立，库存初始化必须建立在药品基础信息已经同步的前提下。
              </p>
            </div>
            <div className="source-summary">
              <ShieldCheck size={24} />
              <div>
                <small>已认证新系统</small>
                <strong>
                  租户 {targetAuth?.tenantId}
                  {targetAuth?.tenantName
                    ? `（${targetAuth.tenantName}）`
                    : ""}{" "}
                  · system
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
          <section className="task-capability" aria-live="polite">
            <header className="task-capability__heading">
              <div>
                <small>接入能力判断</small>
                <strong>{capability.headline}</strong>
              </div>
              <p>{capability.summary}</p>
            </header>
            <div className="task-capability__routes">
              {capability.routes.map((route) => (
                <article
                  className={`task-capability__route task-capability__route--${route.tone}`}
                  key={route.key}
                >
                  <span className="task-capability__icon">
                    {capabilityIcon(route.tone)}
                  </span>
                  <div>
                    <small>{route.label}</small>
                    <strong>{route.title}</strong>
                    <p>{route.detail}</p>
                    {route.compatibility?.map((item) => (
                      <span className="task-capability__policy" key={item}>
                        <ShieldCheck size={12} weight="fill" />
                        {item}
                      </span>
                    ))}
                  </div>
                </article>
              ))}
            </div>
          </section>
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
    inspectMedicineSourceAdapter,
    exportSourceAdapterDiagnostic,
    exportSourceAdapterSupportPackage,
    exportSourceObjectDiagnostic,
    sourceAdapterInspection,
    sourceAdapterDiagnostics = [],
    legacyScope,
    loadDatabase,
    loadDatabaseObject,
    previewDatabaseObject,
    loadFile,
    loadMedicineSourceAdapter,
    loadSourceObjects,
    migrationType,
    query,
    rememberSourcePassword,
    selectDatabaseConnection,
    selectedMedicineAdapterId,
    selectedSourceObject,
    sourceObjectPreview,
    sourceObjectCount,
    sourceObjectCountBusy,
    sourceObjectSurvey,
    selectedSourceConnectionId,
    setConnectionManagerOpen,
    setSourceAdapterInspection,
    setLegacyScope,
    setQuery,
    setRememberSourcePassword,
    setSelectedSourceConnectionId,
    setSelectedMedicineAdapterId,
    setSelectedSourceObject,
    setShowCustomQuery,
    setSourceConnectionEditing,
    setSourceMode,
    setSourceProfile,
    setStep,
    showCustomQuery,
    sourceConnectionEditing,
    sourceAdapters,
    sourceMode,
    sourceObjects,
    sourceProfile,
    step,
    surveyLikelySourceObjects,
    testSourceConnection,
  } = context;
  const automaticMedicineAdapters = sourceAdapters.filter(
    (adapter) =>
      adapter.automaticDetection &&
      adapter.migrationTasks.includes("MEDICINE_BASE") &&
      adapter.databaseFamilies.includes(sourceProfile.kind),
  );
  const selectedMedicineAdapter = automaticMedicineAdapters.find(
    (adapter) => adapter.id === selectedMedicineAdapterId,
  );
  const selectedMedicineAdapterChange = selectedMedicineAdapter?.changes?.find(
    (change) => change.version === selectedMedicineAdapter.version,
  );
  const activeSourceObjectPreview =
    sourceObjectPreview?.objectName === selectedSourceObject
      ? sourceObjectPreview.preview
      : null;
  const activeSourceObjectSuitability = activeSourceObjectPreview
    ? sourceObjectSuitability({
        columns: activeSourceObjectPreview.columns,
        columnMetadata: activeSourceObjectPreview.columnMetadata,
        rows: activeSourceObjectPreview.rows,
        targetFields,
      })
    : null;
  const activeSourceObjectCount =
    sourceObjectCount?.objectName === selectedSourceObject
      ? sourceObjectCount
      : null;
  const sourceObjectOverBatchLimit =
    Number(activeSourceObjectCount?.rowCount || 0) > 10_000;
  const rankedSourceObjectOptions = useMemo(
    () => sourceObjectOptions(sourceObjects),
    [sourceObjects],
  );
  const likelyMedicineObjectCount = rankedSourceObjectOptions.filter(
    (option) => option.data.medicineClueScore > 0,
  ).length;

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
                    setSourceAdapterInspection(null);
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
              {selectedMedicineAdapter && (
                <div className="legacy-adapter">
                  <div className="legacy-adapter__heading">
                    <div>
                      <span className="eyebrow">自动来源适配器</span>
                      <strong>
                        自动识别{selectedMedicineAdapter.name}药品数据
                      </strong>
                      <small>
                        {selectedMedicineAdapter.summary} · 适配器版本 v
                        {selectedMedicineAdapter.version}
                      </small>
                    </div>
                    <button
                      className="button button--secondary"
                      disabled={busy === "medicine-adapter-inspect"}
                      onClick={inspectMedicineSourceAdapter}
                    >
                      <ListMagnifyingGlass size={19} />
                      {busy === "medicine-adapter-inspect"
                        ? "正在识别…"
                        : `识别${selectedMedicineAdapter.name}`}
                    </button>
                  </div>
                  {automaticMedicineAdapters.length > 1 && (
                    <div className="adapter-selector-row">
                      <span>当前适配器</span>
                      <SearchableSelect
                        ariaLabel="自动药品来源适配器"
                        value={selectedMedicineAdapterId}
                        onChange={setSelectedMedicineAdapterId}
                        options={automaticMedicineAdapters.map((adapter) => ({
                          value: adapter.id,
                          label: `${adapter.name} · v${adapter.version}`,
                          description: adapter.summary,
                        }))}
                        searchPlaceholder="搜索 HIS 适配器"
                      />
                    </div>
                  )}
                  <div className="adapter-health-strip">
                    <span>
                      <CheckCircle size={14} weight="fill" />
                      包已注册
                    </span>
                    <span>
                      <CheckCircle size={14} weight="fill" />
                      支持 {sourceProfile.kind}
                    </span>
                    <span>
                      <CheckCircle size={14} weight="fill" />
                      自动结构识别
                    </span>
                    <span>
                      {selectedMedicineAdapter.reusableMappingProfiles ? (
                        <CheckCircle size={14} weight="fill" />
                      ) : (
                        <Warning size={14} weight="fill" />
                      )}
                      {selectedMedicineAdapter.reusableMappingProfiles
                        ? `模板兼容 v${selectedMedicineAdapter.templateCompatibleFromVersion}–v${selectedMedicineAdapter.version}`
                        : "不保存来源模板"}
                    </span>
                  </div>
                  {selectedMedicineAdapterChange && (
                    <div className="adapter-release-note">
                      <strong>
                        当前版本说明 · v{selectedMedicineAdapterChange.version}
                      </strong>
                      <span>{selectedMedicineAdapterChange.summary}</span>
                    </div>
                  )}
                  {sourceAdapterInspection && (
                    <div className="legacy-inspection">
                      <div
                        className={`access-result ${sourceAdapterInspection.detected ? "access-result--success" : "access-result--warning"}`}
                      >
                        {sourceAdapterInspection.detected ? (
                          <CheckCircle size={19} weight="fill" />
                        ) : (
                          <Warning size={19} weight="fill" />
                        )}
                        <div>
                          <strong>{sourceAdapterInspection.message}</strong>
                          <span>
                            Schema {sourceAdapterInspection.schema} · 已核对{" "}
                            {sourceAdapterInspection.checkedObjects.length}{" "}
                            个来源对象 · {sourceAdapterInspection.adapterName} v
                            {sourceAdapterInspection.adapterVersion}
                          </span>
                        </div>
                      </div>
                      <SourceAdapterDiagnosticPanel
                        diagnostics={sourceAdapterDiagnostics}
                        inspection={sourceAdapterInspection}
                        onExportReport={exportSourceAdapterDiagnostic}
                        onExportSupportPackage={
                          exportSourceAdapterSupportPackage
                        }
                      />
                      {sourceAdapterInspection.detected && (
                        <>
                          <div className="legacy-stats">
                            {sourceAdapterInspection.metrics.map((metric) => (
                              <div key={metric.id}>
                                <span>{metric.label}</span>
                                <strong>{metric.value}</strong>
                              </div>
                            ))}
                          </div>
                          <div className="legacy-scopes">
                            {sourceAdapterInspection.scopes.map((scope) => (
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
                                  {scope.medicineCount} 种 / 约{" "}
                                  {scope.estimatedRows} 行
                                </b>
                              </label>
                            ))}
                          </div>
                          {(sourceAdapterInspection.guidance || []).map(
                            (guidance) => (
                              <div
                                className={`legacy-base-note legacy-base-note--${`${guidance.tone || "INFO"}`.toLowerCase()}`}
                                key={guidance.id}
                              >
                                {guidance.tone === "WARNING" ? (
                                  <Warning size={17} weight="fill" />
                                ) : (
                                  <CheckCircle size={17} weight="fill" />
                                )}
                                <span>
                                  <strong>{guidance.title}</strong>
                                  {guidance.body}
                                </span>
                              </div>
                            ),
                          )}
                          <div className="legacy-warnings">
                            {sourceAdapterInspection.warnings.map((warning) => (
                              <span key={warning}>
                                <Warning size={15} />
                                {warning}
                              </span>
                            ))}
                          </div>
                          <button
                            className="button button--primary button--wide"
                            disabled={busy === "medicine-adapter-load"}
                            onClick={loadMedicineSourceAdapter}
                          >
                            <Rows size={19} />
                            {busy === "medicine-adapter-load"
                              ? "正在读取标准数据…"
                              : "按选定范围读取标准药品数据"}
                          </button>
                        </>
                      )}
                    </div>
                  )}
                </div>
              )}
              {!selectedMedicineAdapter && (
                <div className="adapter-unavailable-note">
                  <Info size={17} />
                  <span>
                    当前没有支持 {sourceProfile.kind}{" "}
                    的自动药品适配器。可以先从当前账号可见的表或视图中直接选择，无需编写
                    SQL；完成字段核对后即可保存和导出来源模板。
                  </span>
                </div>
              )}
              {(!selectedMedicineAdapter || showCustomQuery) && (
                <div className="source-object-panel">
                  <div className="source-object-panel__heading">
                    <div>
                      <span className="eyebrow">免写 SQL 读取</span>
                      <strong>选择一个药品表或已整理好的业务视图</strong>
                      <small>
                        仅显示当前只读账号可见的对象；读取前桌面后端会重新核对对象仍然存在。
                      </small>
                    </div>
                    <div className="source-object-panel__heading-actions">
                      {sourceObjects.length > 0 &&
                        likelyMedicineObjectCount > 0 && (
                          <button
                            className="button button--secondary button--compact"
                            type="button"
                            disabled={[
                              "source",
                              "source-object-preview",
                              "source-object-survey",
                              "source-objects",
                              "source-test",
                            ].includes(busy)}
                            onClick={surveyLikelySourceObjects}
                          >
                            <Rows size={17} />
                            {busy === "source-object-survey"
                              ? "正在核对候选…"
                              : `快速核对前 ${Math.min(likelyMedicineObjectCount, 5)} 个候选`}
                          </button>
                        )}
                      <button
                        className="button button--secondary button--compact"
                        type="button"
                        disabled={[
                          "source",
                          "source-object-preview",
                          "source-object-survey",
                          "source-objects",
                          "source-test",
                        ].includes(busy)}
                        onClick={loadSourceObjects}
                      >
                        <ListMagnifyingGlass size={17} />
                        {busy === "source-objects"
                          ? "正在读取清单…"
                          : sourceObjects.length
                            ? "刷新清单"
                            : "读取表/视图清单"}
                      </button>
                    </div>
                  </div>
                  <div className="source-object-panel__picker">
                    <SearchableSelect
                      ariaLabel="选择来源表或视图"
                      value={selectedSourceObject}
                      onChange={setSelectedSourceObject}
                      options={rankedSourceObjectOptions}
                      disabled={
                        !sourceObjects.length ||
                        ["source-objects", "source-object-survey"].includes(
                          busy,
                        )
                      }
                      placeholder={
                        sourceObjects.length
                          ? "搜索并选择来源表或视图"
                          : "先读取当前账号可见清单"
                      }
                      searchPlaceholder="按表名或视图名搜索"
                      emptyText="没有匹配的来源对象"
                      maxVisibleOptions={150}
                    />
                    <span>
                      {sourceObjects.length
                        ? likelyMedicineObjectCount
                          ? `共 ${sourceObjects.length} 个可见对象，名称线索优先显示 ${likelyMedicineObjectCount} 个`
                          : `共 ${sourceObjects.length} 个可见对象，未发现明确药品名称线索`
                        : "尚未读取来源对象"}
                    </span>
                    <button
                      className="button button--primary"
                      type="button"
                      disabled={
                        !sourceObjects.includes(selectedSourceObject) ||
                        [
                          "source",
                          "source-object-preview",
                          "source-object-survey",
                          "source-objects",
                          "source-test",
                        ].includes(busy)
                      }
                      onClick={() => previewDatabaseObject()}
                    >
                      <Rows size={19} />
                      {busy === "source-object-preview"
                        ? "正在读取样例…"
                        : "预览字段与样例"}
                    </button>
                  </div>
                  {sourceObjectSurvey && (
                    <div className="source-object-survey">
                      <div className="source-object-survey__heading">
                        <div>
                          <span className="eyebrow">候选结构快速核对</span>
                          <strong>
                            {sourceObjectSurvey.status === "RUNNING"
                              ? `正在核对 ${sourceObjectSurvey.totalCount} 个候选`
                              : `已核对 ${sourceObjectSurvey.completedCount}/${sourceObjectSurvey.totalCount} 个候选`}
                          </strong>
                          <small>
                            仅检查名称线索最强的前 5 个对象，每个最多读取 2
                            行；按实际药品字段完整度排序，不会自动选表。
                          </small>
                        </div>
                        {sourceObjectSurvey.status === "RUNNING" && (
                          <span className="source-object-survey__progress">
                            只读核对中…
                          </span>
                        )}
                      </div>
                      {sourceObjectSurvey.results.length > 0 && (
                        <div className="source-object-survey__list">
                          {sourceObjectSurvey.results.map((result, index) => {
                            const foundLabels = result.clues
                              .filter((clue) => clue.column)
                              .map((clue) => clue.label);
                            return (
                              <div
                                className={`source-object-survey__item source-object-survey__item--${result.status === "FAILED" ? "failed" : result.tone}`}
                                key={result.objectName}
                              >
                                <span className="source-object-survey__rank">
                                  {index + 1}
                                </span>
                                <div>
                                  <strong>{result.objectName}</strong>
                                  <small>
                                    {result.status === "FAILED"
                                      ? result.title
                                      : `${result.title} · ${result.columnCount} 个字段 · ${result.sampledRowCount} 行轻量样例`}
                                  </small>
                                  <span>
                                    {foundLabels.length
                                      ? `识别：${foundLabels.join("、")}`
                                      : result.description}
                                  </span>
                                </div>
                                <button
                                  className="button button--ghost button--compact"
                                  type="button"
                                  disabled={
                                    sourceObjectSurvey.status === "RUNNING" ||
                                    result.status === "FAILED"
                                  }
                                  onClick={() =>
                                    previewDatabaseObject(result.objectName)
                                  }
                                >
                                  选择并完整预览
                                </button>
                              </div>
                            );
                          })}
                        </div>
                      )}
                    </div>
                  )}
                  {activeSourceObjectPreview && (
                    <div className="source-object-preview">
                      <div className="source-object-preview__heading">
                        <div>
                          <span className="eyebrow">读取前核对</span>
                          <strong>{selectedSourceObject}</strong>
                          <small>
                            {activeSourceObjectPreview.columns.length} 个字段 ·
                            当前显示 {activeSourceObjectPreview.rows.length}{" "}
                            行样例
                            {activeSourceObjectPreview.truncated
                              ? " · 还有更多数据未在样例中展示"
                              : ""}
                          </small>
                          <span
                            className={`source-object-count ${sourceObjectOverBatchLimit ? "source-object-count--danger" : ""}`}
                          >
                            {sourceObjectCountBusy
                              ? "正在后台统计本批总行数…"
                              : activeSourceObjectCount?.error
                                ? activeSourceObjectCount.error
                                : activeSourceObjectCount
                                  ? sourceObjectOverBatchLimit
                                    ? `${activeSourceObjectCount.isExact === false ? "至少" : "共"} ${Number(activeSourceObjectCount.rowCount).toLocaleString()} 行，超过单批 10,000 行上限`
                                    : `本批共 ${Number(activeSourceObjectCount.rowCount).toLocaleString()} 行，可直接读取`
                                  : "行数统计准备中；正式读取仍会校验单批上限"}
                          </span>
                        </div>
                        <div className="source-object-preview__actions">
                          <button
                            className="button button--secondary button--compact"
                            type="button"
                            title="导出字段、注释、样例形态和识别线索，不包含任何原始样例值"
                            onClick={() =>
                              exportSourceObjectDiagnostic(
                                activeSourceObjectSuitability,
                              )
                            }
                          >
                            <DownloadSimple size={17} />
                            导出脱敏结构诊断
                          </button>
                          <button
                            className="button button--primary button--compact"
                            type="button"
                            disabled={
                              !activeSourceObjectPreview.rows.length ||
                              busy === "source" ||
                              sourceObjectOverBatchLimit
                            }
                            onClick={loadDatabaseObject}
                          >
                            <Check size={17} weight="bold" />
                            {busy === "source"
                              ? "正在读取本批…"
                              : sourceObjectOverBatchLimit
                                ? "超过单批上限"
                                : "确认使用并读取本批"}
                          </button>
                        </div>
                      </div>
                      {activeSourceObjectPreview.columns.length ? (
                        <>
                          <div className="source-object-preview__columns">
                            {activeSourceObjectPreview.columns
                              .slice(0, 18)
                              .map((column) => (
                                <span key={column} title={column}>
                                  {column}
                                </span>
                              ))}
                            {activeSourceObjectPreview.columns.length > 18 && (
                              <small>
                                另有{" "}
                                {activeSourceObjectPreview.columns.length - 18}{" "}
                                个字段
                              </small>
                            )}
                          </div>
                          <DataTable
                            columns={activeSourceObjectPreview.columns}
                            rows={activeSourceObjectPreview.rows}
                            maxRows={3}
                          />
                        </>
                      ) : (
                        <div className="source-object-preview__empty">
                          当前对象没有返回字段或数据，请选择其他表/视图，或检查只读权限。
                        </div>
                      )}
                      <div className="source-object-preview__hint">
                        {sourceObjectPreview.metadataMessage ||
                          "字段注释不可用时将按字段名和样例辅助识别"}
                        ；样例仅用于核对字段和内容，确认后会重新读取正式批次，单批仍限制
                        10,000 行。
                      </div>
                      <div className="source-object-preview__privacy">
                        <ShieldCheck size={15} weight="fill" />
                        <span>
                          遇到不确定结构时可把诊断 JSON
                          交给维护人员；文件仅含字段结构和样例形态，不含连接信息、租户或原始数据值。
                        </span>
                      </div>
                      {activeSourceObjectSuitability && (
                        <div
                          className={`source-object-suitability source-object-suitability--${activeSourceObjectSuitability.tone}`}
                        >
                          <div className="source-object-suitability__summary">
                            {activeSourceObjectSuitability.tone === "ready" ? (
                              <CheckCircle size={17} weight="fill" />
                            ) : (
                              <Warning size={17} weight="fill" />
                            )}
                            <div>
                              <strong>
                                {activeSourceObjectSuitability.title}
                              </strong>
                              <small>
                                {activeSourceObjectSuitability.description}
                              </small>
                            </div>
                          </div>
                          <div className="source-object-suitability__clues">
                            {activeSourceObjectSuitability.clues.map((clue) => (
                              <span
                                className={
                                  clue.column ? "is-found" : "is-missing"
                                }
                                key={clue.key}
                                title={
                                  clue.column
                                    ? `${clue.label} ← ${clue.column}${clue.hasSample ? "" : "（当前样例为空）"}`
                                    : `${clue.label}：未识别`
                                }
                              >
                                {clue.label}
                                {clue.column
                                  ? ` ← ${clue.column}`
                                  : " · 未识别"}
                                {clue.column && !clue.hasSample
                                  ? " · 样例为空"
                                  : ""}
                              </span>
                            ))}
                          </div>
                          <small className="source-object-suitability__note">
                            以上只依据字段名、数据库注释和当前样例给出线索，不代表字段映射或业务校验已经通过。
                          </small>
                        </div>
                      )}
                    </div>
                  )}
                  <div className="source-object-panel__safety">
                    <ShieldCheck size={15} weight="fill" />
                    <span>
                      对象名称来自数据库元数据并由后端再次验证，不会把输入内容直接拼成可执行
                      SQL。
                    </span>
                  </div>
                </div>
              )}
              <div className="custom-query-toggle">
                <button
                  className="button button--ghost"
                  type="button"
                  aria-expanded={showCustomQuery}
                  onClick={() => setShowCustomQuery((current) => !current)}
                >
                  <Code size={17} />
                  {showCustomQuery
                    ? "收起高级 SQL"
                    : selectedMedicineAdapter
                      ? "高级选项：使用自定义只读 SQL"
                      : "高级方式：编写多表只读 SQL"}
                </button>
                <span>
                  {selectedMedicineAdapter
                    ? "用于临时核对其他表，不使用专用适配器的范围和标准字段组装。"
                    : "只有药品信息分散在多个表时才需要；查询仍限定为 SELECT / WITH。"}
                </span>
              </div>
              {showCustomQuery && (
                <div className="custom-query-panel">
                  <Field label="自定义读取 SQL" wide>
                    <textarea
                      value={query}
                      onChange={(event) => setQuery(event.target.value)}
                      rows={5}
                      placeholder="请输入当前数据库中真实存在的表或视图，例如 SELECT * FROM 表名"
                    />
                    <small>
                      {sourceProfile.kind === "oracle"
                        ? "仅允许 SELECT / WITH；使用自定义结果时进入通用字段映射流程，不套用二系列标准多表规则。"
                        : "仅允许 SELECT / WITH；读取结果进入通用字段映射流程，核对后会保存为当前来源模板。"}
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
    activeSourceAdapterId,
    beginMapping,
    columnMetadata,
    columns,
    medicinePreviewColumns,
    medicinePreviewLabels,
    mappingProfileStatus,
    sourceTemplateImport,
    cancelSourceTemplateImport,
    confirmSourceMappingTemplateImport,
    exportSourceMappingTemplate,
    previewSourceMappingTemplateFile,
    rows,
    setSourceKeyFields,
    setSourceKeyConfirmed,
    setStep,
    sourceDescription,
    sourceAdapters,
    sourceKeyFields,
    sourceKeyConfirmed,
    sourceName,
    step,
  } = context;
  const activeSourceAdapter = sourceAdapters.find(
    (adapter) => adapter.id === activeSourceAdapterId,
  );
  const sourceWorkflow = medicineWorkflowFor(activeSourceAdapter);
  const adapterProvidedSourceKey = adapterProvidesSourceKey(activeSourceAdapter);
  const sourceKeyReview = useMemo(
    () =>
      assessSourceKeyColumns({
        rows,
        columns,
        columnMetadata,
      }),
    [rows, columns, columnMetadata],
  );
  const selectedSourceKeyFields = useMemo(
    () => sourceKeyFields.filter(Boolean).slice(0, MAX_SOURCE_KEY_FIELDS),
    [sourceKeyFields],
  );
  const [sourceKeyFieldCount, setSourceKeyFieldCount] = useState(
    Math.max(1, selectedSourceKeyFields.length),
  );
  useEffect(() => {
    setSourceKeyFieldCount(Math.max(1, selectedSourceKeyFields.length));
  }, [sourceDescription, selectedSourceKeyFields.length]);
  const selectedKeyAssessment = assessSourceKeyFields({
    rows,
    fields: selectedSourceKeyFields,
  });
  const hasEmptySourceKeySlot =
    sourceKeyFieldCount > selectedSourceKeyFields.length;
  const sourceKeyReady =
    !hasEmptySourceKeySlot && selectedKeyAssessment.completeUnique;
  const sourceKeyConfirmedReady = adapterProvidedSourceKey || sourceKeyConfirmed;
  const sourceKeyLabel =
    sourceKeyDisplayLabel(activeSourceAdapter, selectedSourceKeyFields) ||
    "尚未选择";
  const visibleSourceKeyFields = Array.from(
    {
      length: Math.max(sourceKeyFieldCount, selectedSourceKeyFields.length, 1),
    },
    (_, index) => selectedSourceKeyFields[index] || "",
  );

  function updateSourceKeyField(index, value) {
    const next = [...visibleSourceKeyFields];
    next[index] = value;
    setSourceKeyFields(next.filter(Boolean));
  }

  function removeSourceKeyField(index) {
    setSourceKeyFields(
      selectedSourceKeyFields.filter((_, fieldIndex) => fieldIndex !== index),
    );
    setSourceKeyFieldCount((count) => Math.max(1, count - 1));
  }

  return (
    <>
      {step === 3 && (
        <section className="screen">
          <div className="screen-heading screen-heading--row">
            <div>
              <span className="eyebrow">第 4 步 · 识别数据</span>
            <h1>确认要迁移的数据范围</h1>
            <p>
              {adapterProvidedSourceKey
                ? `${activeSourceAdapter?.name || "来源适配器"}已提供稳定来源键，请核对读取范围和样例；该键只用于追溯，不会作为新表主键。`
                : "系统已读取数据。请选择一至三个能稳定识别三方记录的字段，它只用于追溯，不会作为新表主键。"}
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
              <div className="source-key-fields">
                {visibleSourceKeyFields.map((field, index) => (
                  <div className="source-key-field" key={`${index}-${field}`}>
                    <span>
                      {visibleSourceKeyFields.length > 1
                        ? `第 ${index + 1} 项`
                        : "来源字段"}
                    </span>
                    <SearchableSelect
                      ariaLabel={`三方记录唯一标识第 ${index + 1} 项`}
                      disabled={adapterProvidedSourceKey}
                      value={field}
                      onChange={(value) => updateSourceKeyField(index, value)}
                      options={sourceKeyComponentOptions(
                        sourceKeyReview,
                        selectedSourceKeyFields,
                        field,
                      )}
                      placeholder="选择稳定编号字段"
                      searchPlaceholder="过滤来源字段"
                    />
                    {!adapterProvidedSourceKey &&
                      visibleSourceKeyFields.length > 1 && (
                      <button
                        className="source-key-field__remove"
                        type="button"
                        onClick={() => removeSourceKeyField(index)}
                        aria-label={`移除唯一标识第 ${index + 1} 项`}
                      >
                        移除
                      </button>
                    )}
                  </div>
                ))}
                {!adapterProvidedSourceKey &&
                  selectedSourceKeyFields.length > 0 &&
                  sourceKeyFieldCount < MAX_SOURCE_KEY_FIELDS && (
                    <button
                      className="source-key-field__add"
                      type="button"
                      onClick={() => {
                        setSourceKeyFieldCount((count) =>
                          Math.min(MAX_SOURCE_KEY_FIELDS, count + 1),
                        );
                        setSourceKeyConfirmed(false);
                      }}
                    >
                      ＋ 增加组合字段
                    </button>
                  )}
              </div>
              <small>
                {adapterProvidedSourceKey
                  ? `${activeSourceAdapter?.name || "当前适配器"}已固定使用 ${sourceWorkflow.sourceKeyLabel || sourceWorkflow.sourceKeyField}；该稳定键由适配器读取契约负责生成和校验。`
                  : "每一项都必须整批非空；单字段不唯一时可增加第二或第三项。系统按字段顺序生成无碰撞组合键，仍需确认这些字段不会复用或变化。"}
              </small>
              <div
                className={`source-key-assessment source-key-assessment--${sourceKeyReady ? "ready" : "danger"}`}
                role="status"
              >
                {sourceKeyReady ? (
                  <CheckCircle size={16} weight="fill" />
                ) : (
                  <Warning size={16} weight="fill" />
                )}
                <div>
                  <strong>
                    {sourceKeyReady
                      ? `${sourceKeyLabel}：${rows.length} 行组合后均非空且唯一`
                      : selectedSourceKeyFields.length
                        ? `${sourceKeyLabel}：当前组合仍有空值或重复`
                        : "请选择稳定来源字段"}
                  </strong>
                  <span>
                    {sourceKeyReady
                      ? adapterProvidedSourceKey
                        ? `由${activeSourceAdapter?.name || "当前适配器"}读取契约提供并通过整批唯一性检查`
                        : selectedSourceKeyFields.length > 1
                        ? `已使用 ${selectedSourceKeyFields.length} 个字段组成稳定来源键；顺序会随模板保存`
                        : selectedSourceKeyFields[0] ===
                            sourceKeyReview.recommendedColumn
                          ? "字段名包含稳定编号线索；这是系统当前推荐项"
                          : "当前单字段已通过整批唯一性检查"
                      : hasEmptySourceKeySlot
                        ? "请为新增的组合位置选择一个稳定编号字段"
                        : selectedKeyAssessment.emptyCount
                          ? `${selectedKeyAssessment.emptyCount} 行的组合字段存在空值`
                          : selectedKeyAssessment.duplicateCount
                            ? `${selectedKeyAssessment.duplicateCount} 行组合后仍重复，可继续增加编号字段`
                            : "优先选择药品序号、厂家序号、机构编码等稳定编号；不能使用行号代替"}
                  </span>
                </div>
              </div>
              {!adapterProvidedSourceKey && (
                <label
                  className={`source-key-confirm ${sourceKeyReady ? "" : "is-disabled"}`}
                >
                  <input
                    type="checkbox"
                    checked={sourceKeyConfirmed}
                    disabled={!sourceKeyReady}
                    onChange={(event) =>
                      setSourceKeyConfirmed(event.target.checked)
                    }
                  />
                  <span>
                    我确认以上字段组合是老系统稳定业务键，后续不会复用或随名称、价格等业务属性变化
                  </span>
                </label>
              )}
            </Field>
          </div>
          <div
            className={`legacy-base-note mapping-profile-note ${mappingProfileStatus?.requiresReview || mappingProfileStatus?.storageWarning ? "mapping-profile-note--review" : ""}`}
          >
            {mappingProfileStatus?.requiresReview ||
            mappingProfileStatus?.storageWarning ? (
              <Warning size={17} weight="fill" />
            ) : (
              <FloppyDisk size={17} weight="fill" />
            )}
            <span>
              <strong>
                {mappingProfileStatus?.storageWarning
                  ? "旧模板已恢复，但对象级固化失败"
                  : mappingProfileStatus?.compatibility ===
                      "SIMILAR_SOURCE_REVIEW"
                    ? "已套用同结构本地模板建议"
                    : mappingProfileStatus?.requiresReview
                      ? "适配器版本已变化，请重新核对"
                      : mappingProfileStatus?.restored
                        ? "已恢复当前来源模板"
                        : "首次核对后自动保存来源模板"}
              </strong>
              {mappingProfileStatus?.storageWarning
                ? `${mappingProfileStatus.storageWarning}。当前数据仍可继续核对；请完成一次字段映射保存后再导出模板。`
                : mappingProfileStatus?.compatibility ===
                    "SIMILAR_SOURCE_REVIEW"
                  ? `${mappingProfileStatus.compatibilityMessage}。已恢复 ${mappingProfileStatus.restoredCount} 项字段建议；进入校验前请逐项核对，确认后才会保存为当前来源自己的模板。`
                  : mappingProfileStatus?.requiresReview
                    ? `${mappingProfileStatus.compatibilityMessage}；当前仍会逐项检查真实来源字段，确认映射后才会按新版本保存。`
                    : mappingProfileStatus?.restored
                      ? `已恢复 ${mappingProfileStatus.restoredCount} 个来源字段及其转换规则${mappingProfileStatus.missingCount ? `；${mappingProfileStatus.missingCount} 个旧字段已失效并重新生成建议` : ""}。数据库模板按具体表/视图或只读查询、来源实例和目标租户隔离。${activeSourceAdapter?.automaticDetection ? " 来源字典和数据库注释仍会在连接时刷新。" : ""}`
                      : `完成字段映射或进入校验时会保存到应用本地，下次读取同一来源时直接复用。${activeSourceAdapter?.automaticDetection ? ` ${activeSourceAdapter.name}来源字典仍按老库实例独立维护。` : ""}`}
            </span>
            <div className="mapping-profile-note__actions">
              <button
                type="button"
                className="template-action"
                disabled={
                  !mappingProfileStatus?.restored ||
                  Boolean(mappingProfileStatus?.storageWarning) ||
                  mappingProfileStatus?.compatibility ===
                    "SIMILAR_SOURCE_REVIEW"
                }
                title={
                  mappingProfileStatus?.storageWarning
                    ? "请先重新保存当前来源映射，再导出对象级模板"
                    : mappingProfileStatus?.compatibility ===
                        "SIMILAR_SOURCE_REVIEW"
                      ? "请先核对并保存为当前来源模板，再执行导出"
                      : mappingProfileStatus?.restored
                        ? "导出不含连接信息和凭据的来源模板"
                        : "完成一次字段核对后即可导出"
                }
                onClick={exportSourceMappingTemplate}
              >
                <DownloadSimple size={15} />
                导出模板
              </button>
              <label
                className="template-action"
                title="导入同类适配器的来源模板"
              >
                <UploadSimple size={15} />
                导入模板
                <input
                  type="file"
                  accept=".json,application/json"
                  onChange={(event) => {
                    previewSourceMappingTemplateFile(event.target.files?.[0]);
                    event.target.value = "";
                  }}
                />
              </label>
            </div>
          </div>
          {sourceTemplateImport && (
            <div className="source-template-preview" role="status">
              <div>
                <strong>确认导入“{sourceTemplateImport.fileName}”</strong>
                <span>
                  {sourceTemplateImport.preview.mappingCount} 项字段映射 ·{" "}
                  {sourceTemplateImport.preview.ruleCount} 项转换规则
                  {sourceTemplateImport.preview.dictionaryCount
                    ? ` · ${sourceTemplateImport.preview.dictionaryCount} 组字典覆盖`
                    : ""}
                </span>
                <small>
                  {sourceTemplateImport.preview.requiresReview
                    ? `${sourceTemplateImport.preview.compatibilityMessage}；导入后必须按当前来源重新核对。`
                    : "将覆盖当前来源的本地模板并按现有字段重新核对；数据库连接和目标租户不会改变。"}
                </small>
              </div>
              <div className="source-template-preview__actions">
                <button type="button" onClick={cancelSourceTemplateImport}>
                  取消
                </button>
                <button
                  type="button"
                  className="source-template-preview__confirm"
                  onClick={confirmSourceMappingTemplateImport}
                >
                  {sourceTemplateImport.preview.requiresReview
                    ? "导入并重新核对"
                    : "确认导入"}
                </button>
              </div>
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
              sourceName.startsWith("二系列phis") ? medicinePreviewColumns : []
            }
            columnLabels={
              sourceName.startsWith("二系列phis") ? medicinePreviewLabels : {}
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
            <button
              className="button button--primary"
              disabled={!sourceKeyReady || !sourceKeyConfirmedReady}
              title={
                !sourceKeyReady
                  ? "请先选择一至三个字段组成全批非空且唯一的来源键"
                  : !sourceKeyConfirmedReady
                    ? "请确认该字段是稳定业务主键"
                    : "开始匹配新系统字段"
              }
              onClick={beginMapping}
            >
              开始匹配新系统字段
              <ArrowRight />
            </button>
          </div>
        </section>
      )}
    </>
  );
}
