export function statusMeta(status) {
  return (
    {
      VALIDATED: ["可迁移", "ready"],
      INVALID: ["校验失败", "danger"],
      RUNNING: ["上次执行中断，可重试", "warning"],
      SUCCESS: ["成功", "success"],
      FAILED: ["失败", "danger"],
      PARTIAL: ["部分完成", "warning"],
      SKIPPED: ["已跳过", "muted"],
      UNDONE: ["已撤销", "muted"],
      UNDO_PARTIAL: ["部分撤销", "warning"],
    }[status] || [status || "未开始", "muted"]
  );
}

export function SummaryCards({ batch }) {
  if (!batch) return null;
  const cards = [
    ["总行数", batch.totalCount, "neutral"],
    ["可迁移", batch.validCount, "ready"],
    ["成功", batch.successCount, "success"],
    ["跳过", batch.skipCount, "muted"],
    ["失败", batch.failCount, "danger"],
  ];
  return (
    <div className="summary-cards">
      {cards.map(([label, value, tone]) => (
        <div className={`summary-card summary-card--${tone}`} key={label}>
          <span>{label}</span>
          <strong>{value}</strong>
        </div>
      ))}
    </div>
  );
}
