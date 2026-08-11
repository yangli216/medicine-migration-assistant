import { useEffect, useState } from "react";
import {
  Database,
  FloppyDisk,
  HardDrives,
  Info,
  LockKey,
  MagnifyingGlass,
  PencilSimple,
  Plugs,
  Plus,
  ShieldCheck,
  Trash,
  X,
} from "@phosphor-icons/react";
import { SearchableSelect } from "./SearchableSelect";

export const databaseKinds = {
  mysql: {
    label: "MySQL",
    port: 3306,
    protocol: "原生 Rust 驱动",
    defaultDriver: "",
    nativeProtocol: true,
  },
  oracle: {
    label: "Oracle",
    port: 1521,
    protocol: "Instant Client ODBC 19c",
    defaultDriver: "Oracle 19 ODBC driver",
    nativeProtocol: false,
  },
  dameng: {
    label: "达梦 DM8",
    port: 5236,
    protocol: "官方通用 ODBC 驱动",
    defaultDriver: "DM8 ODBC DRIVER",
    nativeProtocol: false,
  },
  opengauss: {
    label: "Gauss / openGauss",
    port: 5432,
    protocol: "内置 PostgreSQL 通用协议",
    defaultDriver: "",
    nativeProtocol: true,
  },
  vastbase: {
    label: "海量 Vastbase G100",
    port: 5432,
    protocol: "内置 PostgreSQL 通用协议",
    defaultDriver: "",
    nativeProtocol: true,
  },
  gbase8c: {
    label: "南大通用 GBase 8c",
    port: 5432,
    protocol: "内置 PostgreSQL 通用协议",
    defaultDriver: "",
    nativeProtocol: true,
  },
  gbase8a: {
    label: "南大通用 GBase 8a",
    port: 5258,
    protocol: "厂商 ODBC 驱动",
    defaultDriver: "GBase ODBC 8.3 Driver",
    nativeProtocol: false,
  },
  gbase8s: {
    label: "南大通用 GBase 8s",
    port: 9088,
    protocol: "厂商 ODBC 驱动",
    defaultDriver: "GBase ODBC DRIVER",
    nativeProtocol: false,
  },
  kingbase: {
    label: "人大金仓 KingbaseES",
    port: 54321,
    protocol: "内置 PostgreSQL 通用协议",
    defaultDriver: "",
    nativeProtocol: true,
  },
  postgresql: {
    label: "PostgreSQL",
    port: 5432,
    protocol: "内置 PostgreSQL 通用协议",
    defaultDriver: "",
    nativeProtocol: true,
  },
};

const driverKeywords = {
  oracle: ["oracle"],
  dameng: ["dm8", "dameng", "达梦"],
  opengauss: ["opengauss", "gauss"],
  vastbase: ["vastbase", "海量"],
  gbase8c: ["gbase 8c", "gbase8c"],
  gbase8a: ["gbase", "8a"],
  gbase8s: ["gbase", "8s"],
  kingbase: ["kingbase", "金仓"],
  postgresql: ["postgresql", "psql"],
};

export const initialProfile = {
  kind: "mysql",
  host: "127.0.0.1",
  port: 3306,
  database: "",
  username: "",
  password: "",
  schema: "",
  serviceName: "",
  driver: "",
  connectionString: "",
};

const connectionPurposeOptions = [
  { value: "SOURCE", label: "老系统读取", description: "用于导入数据源" },
  { value: "TARGET", label: "新系统写入", description: "用于正式迁移目标库" },
  { value: "BOTH", label: "读取和写入", description: "两个环节都可复用" },
];

export function connectionPurposeLabel(purpose) {
  return (
    connectionPurposeOptions.find((item) => item.value === purpose)?.label ||
    purpose
  );
}

export function connectionSupports(entry, purpose) {
  return entry?.purpose === "BOTH" || entry?.purpose === purpose;
}

export function connectionEndpoint(profile = initialProfile) {
  const database = profile.database || profile.serviceName || "未填写数据库";
  return `${profile.host || "未填写主机"}:${profile.port || "—"}/${database}`;
}

export function profilesMatch(left, right) {
  if (!left || !right) return false;
  return [
    "kind",
    "host",
    "port",
    "database",
    "serviceName",
    "schema",
    "username",
  ].every(
    (key) => `${left[key] ?? ""}`.toLowerCase() === `${right[key] ?? ""}`.toLowerCase(),
  );
}

export function newConnectionDraft(purpose = "SOURCE") {
  return {
    connectionId: "",
    name: "",
    purpose,
    profile: { ...initialProfile },
    rememberPassword: true,
    passwordAvailable: false,
  };
}


export function Field({ label, children, wide }) {
  return (
    <label className={`form-field ${wide ? "form-field--wide" : ""}`}>
      <span>{label}</span>
      {children}
    </label>
  );
}

export function ConnectionForm({
  value,
  onChange,
  title,
  drivers = [],
  driverPacks = [],
  rememberPassword = true,
  onRememberPasswordChange,
  hasSavedConnection = false,
  onForgetSaved,
  showCredentialPreference = true,
  purpose = "SOURCE",
}) {
  const set = (key, next) =>
    onChange({ ...value, [key]: key === "port" ? Number(next) : next });
  const kind = databaseKinds[value.kind] || databaseKinds.mysql;
  const driverPack = driverPacks.find(
    (pack) => pack.databaseKind === value.kind,
  );
  const changeKind = (nextKind) => {
    const next = databaseKinds[nextKind];
    const keywords = driverKeywords[nextKind] || [];
    const recommended = drivers.find((driver) =>
      keywords.some((keyword) => driver.toLowerCase().includes(keyword)),
    );
    onChange({
      ...value,
      kind: nextKind,
      port: next.port,
      driver:
        next.nativeProtocol
          ? ""
          : recommended ||
            driverPacks.find((pack) => pack.databaseKind === nextKind)
              ?.defaultDriver ||
            next.defaultDriver,
      connectionString: "",
    });
  };
  return (
    <div className="connection-form">
      <div className="section-title">
        <Database size={21} weight="duotone" />
        <div>
          <strong>{title}</strong>
          <small>
            {kind.label} · {kind.protocol}
          </small>
        </div>
      </div>
      {driverPack && (
        <div className={`driver-preset driver-preset--${driverPack.state}`}>
          <div className="driver-preset__topline">
            <span className="driver-preset__name">
              <ShieldCheck size={17} weight="fill" />
              {driverPack.title} · {driverPack.version}
            </span>
            <span className="driver-preset__status">
              {driverPack.state === "installed"
                ? "本机已就绪"
                : driverPack.state === "bundled"
                  ? "应用内置"
                  : "驱动文件未安装"}
            </span>
          </div>
          <p>
            {driverPack.state === "bundled"
              ? driverPack.licenseNote
              : driverPack.detectedDriver
              ? `已识别驱动：${driverPack.detectedDriver}`
              : `当前只有连接配置，安装或导入驱动文件后才能连接。${driverPack.licenseNote}`}
          </p>
          <details className="driver-preset__guide">
            <summary>{driverPack.state === "bundled" ? "查看协议说明" : "查看下载安装步骤"}</summary>
            <div>
              <span>{driverPack.installGuide}</span>
              {driverPack.officialUrl && (
                <a href={driverPack.officialUrl} target="_blank" rel="noreferrer">
                  打开官方下载/文档
                </a>
              )}
              <small>打包优先级：Windows x64 → macOS Apple Silicon → macOS Intel</small>
            </div>
          </details>
        </div>
      )}
      {kind.nativeProtocol && value.kind !== "mysql" && ["TARGET", "BOTH"].includes(purpose) && (
        <div className="driver-write-boundary">
          <Info size={17} />
          <span>
            <strong>通用 PG 协议已覆盖连接、读取和目标结构检查</strong>
            <small>
              药品基础数据增量写入可直接使用内置通用协议；覆盖迁移和机构库存首次盘点仍需按目标版本验收，必要时可在下方配置厂商 ODBC 回退。
            </small>
          </span>
        </div>
      )}
      <div className="form-grid">
        <Field label="数据库类型">
          <SearchableSelect
            ariaLabel="数据库类型"
            value={value.kind}
            onChange={changeKind}
            options={Object.entries(databaseKinds).map(([key, item]) => ({
              value: key,
              label: item.label,
              keywords: `${item.label} ${item.protocol}`,
            }))}
            searchPlaceholder="搜索数据库类型"
          />
        </Field>
        <Field label="主机地址">
          <input
            value={value.host}
            onChange={(event) => set("host", event.target.value)}
            placeholder="例如 192.168.1.20"
          />
        </Field>
        <Field label="端口">
          <input
            type="number"
            value={value.port}
            onChange={(event) => set("port", event.target.value)}
          />
        </Field>
        <Field label="数据库名">
          <input
            value={value.database}
            onChange={(event) => set("database", event.target.value)}
            placeholder={
              value.kind === "oracle" ? "数据库或 PDB 名称" : "数据库名称"
            }
          />
        </Field>
        {value.kind === "oracle" && (
          <Field label="Service Name">
            <input
              value={value.serviceName}
              onChange={(event) => set("serviceName", event.target.value)}
              placeholder="例如 ORCLPDB1"
            />
          </Field>
        )}
        {value.kind !== "mysql" && (
          <Field label="Schema / 模式">
            <input
              value={value.schema}
              onChange={(event) => set("schema", event.target.value)}
              placeholder="不填则读取当前账号可见表"
            />
          </Field>
        )}
        {kind.nativeProtocol && value.kind !== "mysql" && (
          <Field label="厂商 ODBC 兼容回退（可选）" wide>
            <details className="odbc-fallback">
              <summary>仅在通用 PG 协议与项目数据库版本不兼容时配置</summary>
              <div className="odbc-fallback__fields">
                <SearchableSelect
                  allowCustom
                  ariaLabel="兼容回退数据库驱动"
                  value={value.driver}
                  onChange={(next) => set("driver", next)}
                  options={drivers.map((driver) => ({ value: driver, label: driver }))}
                  placeholder="选择厂商 ODBC 驱动"
                  searchPlaceholder="过滤驱动，或输入自定义驱动名"
                />
                <textarea
                  rows={2}
                  value={value.connectionString}
                  onChange={(event) => set("connectionString", event.target.value)}
                  placeholder="高级 ODBC 连接串（通常留空）"
                />
              </div>
            </details>
          </Field>
        )}
        <Field
          label={
            title.includes("老系统")
              ? "只读账号"
              : title.includes("目标")
                ? "写入账号"
                : "数据库账号"
          }
        >
          <input
            value={value.username}
            onChange={(event) => set("username", event.target.value)}
            autoComplete="off"
          />
        </Field>
        <Field label="密码">
          <div className="password-input">
            <LockKey size={17} />
            <input
              type="password"
              value={value.password}
              onChange={(event) => set("password", event.target.value)}
              autoComplete="new-password"
            />
          </div>
        </Field>
        {!kind.nativeProtocol && (
          <>
            <Field label="本机数据库驱动" wide>
              <SearchableSelect
                allowCustom
                ariaLabel="本机数据库驱动"
                value={value.driver}
                onChange={(next) => set("driver", next)}
                options={drivers.map((driver) => ({
                  value: driver,
                  label: driver,
                }))}
                placeholder={
                  drivers.length
                    ? `推荐：${kind.defaultDriver}`
                    : `已预置：${kind.defaultDriver}`
                }
                searchPlaceholder="过滤驱动，或输入自定义驱动名"
              />
              <small>
                {drivers.length
                  ? `已检测到 ${drivers.length} 个 ODBC 驱动`
                  : "未检测到本机驱动；连接测试时会给出安装或导入提示"}
              </small>
            </Field>
            <Field label="高级连接串（可选）" wide>
              <textarea
                rows={2}
                value={value.connectionString}
                onChange={(event) =>
                  set("connectionString", event.target.value)
                }
                placeholder="${HOST}、${PORT}、${DATABASE}、${SERVICE}、${SCHEMA}、${USER}、${PASSWORD}"
              />
            </Field>
          </>
        )}
      </div>
      {showCredentialPreference && <div className="credential-preference">
        <div className="credential-preference__summary">
          <CheckCircle size={16} weight="fill" />
          <span>
            <strong>自动记住连接参数</strong>
            <small>
              主机、端口和账号保存在本机，下次启动自动恢复。
            </small>
          </span>
        </div>
        <label className="credential-preference__password">
          <input
            type="checkbox"
            checked={rememberPassword}
            onChange={(event) =>
              onRememberPasswordChange?.(event.target.checked)
            }
          />
          <span>同时记住密码（本地加密）</span>
        </label>
        {hasSavedConnection && (
          <button
            className="button button--ghost"
            type="button"
            onClick={onForgetSaved}
          >
            忘记已保存连接
          </button>
        )}
      </div>}
    </div>
  );
}

export function ConnectionPicker({
  purpose,
  entries,
  selectedId,
  onSelect,
  onManage,
  editing,
  onToggleEditing,
}) {
  const available = entries.filter((entry) => connectionSupports(entry, purpose));
  const selected = available.find((entry) => entry.connectionId === selectedId);
  return (
    <div className="connection-picker">
      <div className="connection-picker__control">
        <div>
          <span className="eyebrow">复用已保存连接</span>
          <SearchableSelect
            ariaLabel={purpose === "SOURCE" ? "选择老系统连接" : "选择目标库连接"}
            value={selectedId}
            onChange={onSelect}
            options={available.map((entry) => ({
              value: entry.connectionId,
              label: entry.name,
              description: `${databaseKinds[entry.profile.kind]?.label || entry.profile.kind} · ${connectionEndpoint(entry.profile)}`,
              meta: `${entry.profile.username || "未填写账号"}${entry.passwordAvailable ? " · 已保存加密密码" : " · 使用时填写密码"}`,
              keywords: `${entry.name} ${entry.profile.host} ${entry.profile.database} ${entry.profile.schema} ${entry.profile.username}`,
            }))}
            placeholder={available.length ? "选择一条已保存连接" : "暂无可用连接"}
            searchPlaceholder="按名称、地址、数据库或账号过滤"
          />
        </div>
        <button className="button button--secondary" type="button" onClick={onManage}>
          <HardDrives size={17} />
          管理连接
        </button>
      </div>
      {selected && (
        <div className="connection-picker__summary">
          <CheckCircle size={18} weight="fill" />
          <div>
            <strong>{selected.name}</strong>
            <span>
              {connectionEndpoint(selected.profile)} · {selected.profile.username}
            </span>
          </div>
          <span className="connection-purpose-badge">
            {connectionPurposeLabel(selected.purpose)}
          </span>
          <button className="button button--ghost" type="button" onClick={onToggleEditing}>
            <PencilSimple size={15} />
            {editing ? "收起临时参数" : "修改本次参数"}
          </button>
        </div>
      )}
    </div>
  );
}

export function ConnectionManager({
  open,
  entries,
  drivers,
  driverPacks,
  busy,
  onClose,
  onSave,
  onDelete,
  onTest,
  onUse,
}) {
  const [search, setSearch] = useState("");
  const [draft, setDraft] = useState(() => newConnectionDraft());
  const [deleteConfirm, setDeleteConfirm] = useState(false);
  const [testResult, setTestResult] = useState(null);

  useEffect(() => {
    if (!open) return;
    setSearch("");
    setDeleteConfirm(false);
    setTestResult(null);
    setDraft(
      entries[0]
        ? { ...entries[0], profile: { ...entries[0].profile } }
        : newConnectionDraft(),
    );
  }, [open]);

  if (!open) return null;
  const keyword = search.trim().toLowerCase();
  const filtered = entries.filter((entry) =>
    [
      entry.name,
      entry.profile.kind,
      entry.profile.host,
      entry.profile.database,
      entry.profile.schema,
      entry.profile.username,
      connectionPurposeLabel(entry.purpose),
    ]
      .join(" ")
      .toLowerCase()
      .includes(keyword),
  );
  const selectEntry = (entry) => {
    setDraft({ ...entry, profile: { ...entry.profile } });
    setDeleteConfirm(false);
    setTestResult(null);
  };
  const createEntry = () => {
    setSearch("");
    setDraft(newConnectionDraft("SOURCE"));
    setDeleteConfirm(false);
    setTestResult(null);
  };
  const save = async () => {
    const saved = await onSave(draft);
    if (saved) selectEntry(saved);
  };
  const test = async () => {
    setTestResult(null);
    const result = await onTest(draft.profile);
    if (result) setTestResult(result);
  };
  const remove = async () => {
    if (!deleteConfirm) {
      setDeleteConfirm(true);
      return;
    }
    const removed = await onDelete(draft.connectionId);
    if (removed !== false) {
      const remaining = entries.filter(
        (entry) => entry.connectionId !== draft.connectionId,
      );
      setDraft(
        remaining[0]
          ? { ...remaining[0], profile: { ...remaining[0].profile } }
          : newConnectionDraft(),
      );
      setDeleteConfirm(false);
    }
  };
  return (
    <div className="connection-manager-backdrop" role="presentation" onMouseDown={onClose}>
      <section
        className="connection-manager"
        role="dialog"
        aria-modal="true"
        aria-label="数据库连接管理"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <header className="connection-manager__header">
          <div>
            <span className="eyebrow">本地连接库</span>
            <h2>数据库连接管理</h2>
            <p>集中维护连接并在老库读取、目标库写入时直接复用。</p>
          </div>
          <button className="icon-button" type="button" onClick={onClose} aria-label="关闭连接管理">
            <X size={20} />
          </button>
        </header>
        <div className="connection-manager__body">
          <aside className="connection-library">
            <div className="connection-library__tools">
              <label>
                <MagnifyingGlass size={16} />
                <input
                  value={search}
                  onChange={(event) => setSearch(event.target.value)}
                  placeholder="过滤连接"
                />
              </label>
              <button
                className="button button--primary"
                type="button"
                onClick={createEntry}
              >
                <Plus size={16} />
                新建连接
              </button>
            </div>
            <div className="connection-library__count">
              已保存 {entries.length} 条 · 当前显示 {filtered.length} 条
            </div>
            <div className="connection-library__list">
              {filtered.map((entry) => (
                <button
                  type="button"
                  key={entry.connectionId}
                  className={
                    draft.connectionId === entry.connectionId ? "active" : ""
                  }
                  onClick={() => selectEntry(entry)}
                >
                  <Database size={19} weight="duotone" />
                  <span>
                    <strong>{entry.name}</strong>
                    <small>{connectionEndpoint(entry.profile)}</small>
                    <em>
                      {connectionPurposeLabel(entry.purpose)} · {entry.profile.username}
                    </em>
                  </span>
                  {entry.passwordAvailable && <LockKey size={14} weight="fill" />}
                </button>
              ))}
              {!filtered.length && (
                <div className="connection-library__empty">
                  <HardDrives size={28} weight="duotone" />
                  <strong>{entries.length ? "没有匹配的连接" : "还没有保存连接"}</strong>
                  <span>新建后即可在迁移步骤中直接选择复用。</span>
                </div>
              )}
            </div>
          </aside>
          <div className="connection-editor">
            <div className="connection-editor__content">
              <div className="connection-editor__identity">
                <Field label="连接名称">
                  <input
                    value={draft.name}
                    onChange={(event) => setDraft({ ...draft, name: event.target.value })}
                    placeholder="例如：二系列phis正式库"
                  />
                </Field>
                <Field label="连接用途">
                  <SearchableSelect
                    ariaLabel="连接用途"
                    value={draft.purpose}
                    onChange={(purpose) => setDraft({ ...draft, purpose })}
                    options={connectionPurposeOptions}
                    searchPlaceholder="选择连接用途"
                  />
                </Field>
              </div>
              <ConnectionForm
                value={draft.profile}
                onChange={(profile) => setDraft({ ...draft, profile })}
                title={draft.purpose === "TARGET" ? "目标数据库参数" : "数据库连接参数"}
                drivers={drivers}
                driverPacks={driverPacks}
                showCredentialPreference={false}
                purpose={draft.purpose}
              />
              <div className="connection-editor__preference">
                <label>
                  <input
                    type="checkbox"
                    checked={draft.rememberPassword}
                    onChange={(event) =>
                      setDraft({ ...draft, rememberPassword: event.target.checked })
                    }
                  />
                  <span>
                    <strong>保存密码到应用本地</strong>
                    <small>AES-256-GCM 加密，不依赖操作系统钥匙串</small>
                  </span>
                </label>
                {draft.passwordAvailable && !draft.profile.password && (
                  <span className="password-preserved">已保留原加密密码</span>
                )}
              </div>
              {testResult && (
                <div className="connection-test-result">
                  <CheckCircle size={17} weight="fill" />
                  <span>
                    <strong>{testResult.message}</strong>
                    <small>{testResult.databaseVersion} · {testResult.latencyMs} ms</small>
                  </span>
                </div>
              )}
            </div>
            <footer className="connection-editor__actions">
              <div>
                {draft.connectionId && (
                  <button className="button button--ghost-danger" type="button" onClick={remove}>
                    <Trash size={16} />
                    {deleteConfirm ? "确认删除" : "删除连接"}
                  </button>
                )}
              </div>
              <button
                className="button button--secondary"
                type="button"
                disabled={busy === "connection-manager-test"}
                onClick={test}
              >
                <Plugs size={17} />
                {busy === "connection-manager-test" ? "测试中…" : "测试连接"}
              </button>
              <button
                className="button button--primary"
                type="button"
                disabled={busy === "connection-manager-save"}
                onClick={save}
              >
                <FloppyDisk size={17} />
                {busy === "connection-manager-save"
                  ? "保存中…"
                  : draft.connectionId
                    ? "更新连接"
                    : "保存连接"}
              </button>
              {draft.connectionId && connectionSupports(draft, "SOURCE") && (
                <button className="button button--secondary" type="button" onClick={() => onUse(draft, "SOURCE")}>
                  用于老库读取
                </button>
              )}
              {draft.connectionId && connectionSupports(draft, "TARGET") && (
                <button className="button button--secondary" type="button" onClick={() => onUse(draft, "TARGET")}>
                  用于目标库写入
                </button>
              )}
            </footer>
          </div>
        </div>
      </section>
    </div>
  );
}
