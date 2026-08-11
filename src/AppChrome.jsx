import {
  Check,
  Clock,
  ClockCounterClockwise,
  FirstAidKit,
  HardDrives,
  ShieldCheck,
  UserCircle,
} from "@phosphor-icons/react";

export const steps = [
  "连接新系统",
  "选择任务",
  "导入数据源",
  "识别数据",
  "匹配字段",
  "校验修正",
  "执行审计",
];

export function Stepper({ active }) {
  return (
    <nav className="stepper" aria-label="迁移进度">
      {steps.map((step, index) => (
        <div
          className={`step ${index < active ? "step--done" : ""} ${index === active ? "step--active" : ""}`}
          key={step}
        >
          <div className="step__node">
            {index < active ? <Check size={16} weight="bold" /> : index + 1}
          </div>
          <span>{step}</span>
          {index < steps.length - 1 && <div className="step__line" />}
        </div>
      ))}
    </nav>
  );
}

export function Header({
  runtime,
  auth,
  connectionCount = 0,
  historyCount = 0,
  onOpenConnections,
  onOpenHistory,
}) {
  return (
    <header className="topbar">
      <div className="brand">
        <FirstAidKit className="brand-mark" size={34} weight="duotone" />
        <span className="brand__title">数据迁移助手</span>
      </div>
      <div className="topbar__divider" />
      <span className="topbar__section">药品基础数据</span>
      <span className="desktop-chip">
        {runtime === "tauri-rust" ? "桌面版 · 本地运行" : "交互预览"}
      </span>
      <div className="topbar__spacer" />
      <button className="connection-manager-trigger" onClick={onOpenConnections}>
        <HardDrives size={18} weight="duotone" />
        <span>连接管理</span>
        <i>{connectionCount}</i>
      </button>
      <button className="connection-manager-trigger" onClick={onOpenHistory}>
        <ClockCounterClockwise size={18} weight="duotone" />
        <span>迁移历史</span>
        {historyCount > 0 && <i>{historyCount}</i>}
      </button>
      <div className="save-status">
        <ShieldCheck size={18} weight="fill" />
        审计记录保存在本机
      </div>
      <div className="topbar__time">
        <Clock size={18} />
        {new Date().toLocaleDateString("zh-CN")}
      </div>
      <button className="user-menu">
        <UserCircle size={27} weight="fill" />
        <span>{auth ? `${auth.userName} · ${auth.roleName}` : "未登录"}</span>
      </button>
    </header>
  );
}
