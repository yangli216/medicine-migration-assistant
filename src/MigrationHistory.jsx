import { useEffect, useMemo, useState } from "react";
import {
  ClockCounterClockwise,
  Info,
  MagnifyingGlass,
  Warning,
  X,
} from "@phosphor-icons/react";
import { SearchableSelect } from "./SearchableSelect";

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

export function migrationTaskLabel(sourceType = "") {
  return (
    {
      PHIS27: "二系列phis药品基础数据",
      PHIS27_INVENTORY: "二系列phis机构库存",
      CSV: "文件导入",
      JSON: "文件导入",
      DATABASE: "数据库导入",
    }[sourceType] || sourceType || "其他迁移任务"
  );
}

export function formatLocalDateTime(value) {
  if (!value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  }).format(date);
}

function formatHistoryListDate(value) {
  if (!value) return "时间未知";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).format(date);
}

function historyBatchTitle(sourceType = "") {
  if (sourceType === "PHIS27_INVENTORY") return "机构库存首次盘点";
  if (sourceType === "PHIS27") return "药品基础数据迁移";
  return migrationTaskLabel(sourceType);
}

function historyBatchScope(batch) {
  const text = `${batch.batchName || ""} ${batch.sourceDescription || ""}`;
  if (batch.sourceType === "PHIS27_INVENTORY") {
    const organizationCount = text.match(/(\d+)\s*个?机构/);
    return organizationCount
      ? `${organizationCount[1]} 个机构`
      : "机构库存";
  }
  if (/全部通用药品/.test(text)) return "全部通用药品";
  if (/机构全部配置药品/.test(text)) return "机构全部配置药品";
  if (/机构在用药品/.test(text)) return "机构在用药品";
  return batch.sourceType === "PHIS27" ? "药品主数据" : "迁移批次";
}

function historyBatchSource(batch) {
  const parts = `${batch.sourceName || ""}`
    .split("·")
    .map((part) => part.trim())
    .filter(Boolean);
  if (parts.length > 1 && /二系列phis/i.test(parts[0])) parts.shift();
  return parts.join(" · ") || "未记录数据源";
}

export function diffValue(value) {
  if (value === null || value === undefined || `${value}` === "") return "（空）";
  return `${value}`;
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


export function MigrationHistory({
  open,
  batches,
  detail,
  busy,
  onClose,
  onRefresh,
  onSelect,
}) {
  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState("ALL");
  const [taskFilter, setTaskFilter] = useState("ALL");
  const [detailTab, setDetailTab] = useState("overview");
  const [rowFilter, setRowFilter] = useState("ALL");
  const [expandedAuditId, setExpandedAuditId] = useState("");

  useEffect(() => {
    if (!open) return;
    setExpandedAuditId("");
    setDetailTab("overview");
    setRowFilter("ALL");
  }, [open, detail?.batch?.batchId]);

  const taskOptions = useMemo(() => {
    const values = [...new Set(batches.map((batch) => batch.sourceType))];
    return [
      { value: "ALL", label: "全部任务" },
      ...values.map((value) => ({
        value,
        label: migrationTaskLabel(value),
      })),
    ];
  }, [batches]);

  const filteredBatches = useMemo(() => {
    const keyword = search.trim().toLowerCase();
    return batches.filter((batch) => {
      const matchesStatus =
        statusFilter === "ALL" || batch.status === statusFilter;
      const matchesTask =
        taskFilter === "ALL" || batch.sourceType === taskFilter;
      const matchesKeyword =
        !keyword ||
        [
          batch.batchName,
          batch.batchId,
          batch.sourceName,
          batch.sourceDescription,
          migrationTaskLabel(batch.sourceType),
        ].some((value) => `${value || ""}`.toLowerCase().includes(keyword));
      return matchesStatus && matchesTask && matchesKeyword;
    });
  }, [batches, search, statusFilter, taskFilter]);

  if (!open) return null;
  const rowStatusCounts = Object.fromEntries(
    ["SUCCESS", "FAILED", "INVALID", "VALIDATED", "RUNNING", "SKIPPED"].map(
      (status) => [
        status,
        detail?.rows.filter((row) => row.status === status).length || 0,
      ],
    ),
  );
  const visibleRows =
    !detail || rowFilter === "ALL"
      ? detail?.rows || []
      : detail.rows.filter((row) => row.status === rowFilter);

  return (
    <div
      className="connection-manager-backdrop migration-history-backdrop"
      role="presentation"
      onMouseDown={onClose}
    >
      <section
        className="migration-history"
        role="dialog"
        aria-modal="true"
        aria-label="历史迁移日志"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <header className="connection-manager__header migration-history__header">
          <div>
            <span className="eyebrow">应用本地记录</span>
            <h2>历史迁移日志</h2>
            <p>查看每次迁移的批次结果、失败明细和完整审计轨迹，不需要重新登录新系统。</p>
          </div>
          <div className="migration-history__header-actions">
            <button
              className="button button--secondary"
              type="button"
              disabled={busy === "history-list"}
              onClick={onRefresh}
            >
              <ClockCounterClockwise size={16} />
              {busy === "history-list" ? "刷新中…" : "刷新记录"}
            </button>
            <button
              className="icon-button"
              type="button"
              onClick={onClose}
              aria-label="关闭历史迁移日志"
            >
              <X size={20} />
            </button>
          </div>
        </header>
        <div className="migration-history__body">
          <aside className="history-library">
            <label className="history-search">
              <MagnifyingGlass size={16} />
              <input
                value={search}
                onChange={(event) => setSearch(event.target.value)}
                placeholder="搜索批次名、来源或批次号"
              />
            </label>
            <div className="history-filter-grid">
              <SearchableSelect
                ariaLabel="筛选迁移任务"
                value={taskFilter}
                onChange={setTaskFilter}
                options={taskOptions}
                searchPlaceholder="按任务过滤"
              />
              <SearchableSelect
                ariaLabel="筛选迁移状态"
                value={statusFilter}
                onChange={setStatusFilter}
                options={[
                  { value: "ALL", label: "全部状态" },
                  { value: "SUCCESS", label: "成功" },
                  { value: "PARTIAL", label: "部分成功" },
                  { value: "RUNNING", label: "执行中断" },
                  { value: "FAILED", label: "失败" },
                  { value: "VALIDATED", label: "已校验 / 待执行" },
                  { value: "INVALID", label: "校验失败" },
                  { value: "UNDONE", label: "已撤销" },
                  { value: "UNDO_PARTIAL", label: "部分撤销" },
                ]}
                searchPlaceholder="按状态过滤"
              />
            </div>
            <div className="history-library__count">
              <span>迁移批次</span>
              <strong>{filteredBatches.length}</strong>
              <small>/ {batches.length}</small>
            </div>
            <div className="history-batch-list">
              {filteredBatches.map((batch) => (
                <button
                  type="button"
                  className={
                    detail?.batch.batchId === batch.batchId ? "active" : ""
                  }
                  onClick={() => onSelect(batch.batchId)}
                  key={batch.batchId}
                  title={batch.batchName}
                  aria-label={`${historyBatchTitle(batch.sourceType)}，${statusMeta(batch.status)[0]}，${formatLocalDateTime(batch.createdAt)}`}
                >
                  <div className="history-batch-card__top">
                    <strong>{historyBatchTitle(batch.sourceType)}</strong>
                    <span
                      className={`status-pill status-pill--${statusMeta(batch.status)[1]}`}
                    >
                      {statusMeta(batch.status)[0]}
                    </span>
                  </div>
                  <div className="history-batch-card__meta">
                    <time dateTime={batch.createdAt || undefined}>
                      {formatHistoryListDate(batch.createdAt)}
                    </time>
                    <span>{historyBatchScope(batch)}</span>
                  </div>
                  <p className="history-batch-card__source">
                    {historyBatchSource(batch)}
                  </p>
                  <div className="history-batch-card__results">
                    <span>
                      <em>总数</em>
                      <b>{batch.totalCount}</b>
                    </span>
                    {batch.validCount > 0 && (
                      <span className="ready">
                        <em>待执行</em>
                        <b>{batch.validCount}</b>
                      </span>
                    )}
                    {batch.successCount > 0 && (
                      <span className="success">
                        <em>成功</em>
                        <b>{batch.successCount}</b>
                      </span>
                    )}
                    {batch.failCount > 0 && (
                      <span className="danger">
                        <em>失败</em>
                        <b>{batch.failCount}</b>
                      </span>
                    )}
                    {batch.skipCount > 0 && (
                      <span>
                        <em>跳过</em>
                        <b>{batch.skipCount}</b>
                      </span>
                    )}
                  </div>
                </button>
              ))}
              {!filteredBatches.length && (
                <div className="history-library__empty">
                  <ClockCounterClockwise size={30} weight="duotone" />
                  <strong>{batches.length ? "没有符合条件的批次" : "还没有迁移记录"}</strong>
                  <span>
                    {batches.length
                      ? "调整任务、状态或关键词后再查看。"
                      : "完成试迁移或正式迁移后，批次会自动记录在这里。"}
                  </span>
                </div>
              )}
            </div>
          </aside>
          <div className="history-detail">
            {busy === "history-detail" && !detail ? (
              <div className="history-detail__empty">
                <ClockCounterClockwise size={32} weight="duotone" />
                <strong>正在读取批次详情…</strong>
              </div>
            ) : detail ? (
              <>
                <div className="history-detail__summary">
                  <div>
                    <span className="eyebrow">{migrationTaskLabel(detail.batch.sourceType)}</span>
                    <h3>{detail.batch.batchName}</h3>
                    <p>{detail.batch.sourceName}</p>
                  </div>
                  <span
                    className={`status-pill status-pill--${statusMeta(detail.batch.status)[1]}`}
                  >
                    {statusMeta(detail.batch.status)[0]}
                  </span>
                </div>
                <SummaryCards batch={detail.batch} />
                <div className="history-detail__tabs" role="tablist">
                  {[
                    ["overview", "批次概览"],
                    ["rows", `迁移明细 ${detail.rows.length}`],
                    ["audits", `审计日志 ${detail.audits.length}`],
                  ].map(([key, label]) => (
                    <button
                      type="button"
                      className={detailTab === key ? "active" : ""}
                      onClick={() => setDetailTab(key)}
                      key={key}
                    >
                      {label}
                    </button>
                  ))}
                </div>
                {detailTab === "overview" && (
                  <div className="history-overview">
                    <dl>
                      <div><dt>批次号</dt><dd><code>{detail.batch.batchId}</code></dd></div>
                      <div><dt>数据来源</dt><dd>{detail.batch.sourceName}</dd></div>
                      <div><dt>创建时间</dt><dd>{formatLocalDateTime(detail.batch.createdAt)}</dd></div>
                      <div><dt>完成时间</dt><dd>{formatLocalDateTime(detail.batch.finishedAt)}</dd></div>
                      <div><dt>冲突策略</dt><dd>{detail.batch.conflictStrategy}</dd></div>
                      <div><dt>来源说明</dt><dd>{detail.batch.sourceDescription || "—"}</dd></div>
                    </dl>
                    {detail.batch.failCount > 0 && (
                      <div className="history-overview__warning">
                        <Warning size={18} weight="fill" />
                        <span>
                          本批次有 {detail.batch.failCount} 条失败记录，可在“迁移明细”中查看完整原因。
                        </span>
                      </div>
                    )}
                  </div>
                )}
                {detailTab === "rows" && (
                  <div className="history-rows">
                    <div className="history-row-filters">
                      {[
                        ["ALL", "全部", detail.rows.length],
                        ["FAILED", "失败", rowStatusCounts.FAILED],
                        ["INVALID", "校验失败", rowStatusCounts.INVALID],
                        ["SUCCESS", "成功", rowStatusCounts.SUCCESS],
                        ["VALIDATED", "待执行", rowStatusCounts.VALIDATED],
                        ["RUNNING", "执行中断", rowStatusCounts.RUNNING],
                        ["SKIPPED", "跳过", rowStatusCounts.SKIPPED],
                      ]
                        .filter(([, , count]) => count > 0 || rowFilter === "ALL")
                        .map(([key, label, count]) => (
                          <button
                            type="button"
                            className={rowFilter === key ? "active" : ""}
                            onClick={() => setRowFilter(key)}
                            key={key}
                          >
                            {label} {count}
                          </button>
                        ))}
                    </div>
                    <div className="history-row-list">
                      {visibleRows.map((row) => (
                        <div key={row.rowId}>
                          <span>第 {row.rowNo} 行</span>
                          <code>{row.sourceKey}</code>
                          <span
                            className={`status-pill status-pill--${statusMeta(row.status)[1]}`}
                          >
                            {statusMeta(row.status)[0]}
                          </span>
                          <p>{row.errorMessage || (row.status === "SUCCESS" ? "迁移成功" : "等待执行")}</p>
                          <small>更新于 {formatLocalDateTime(row.updatedAt)}</small>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
                {detailTab === "audits" && (
                  <div className="history-audit-list">
                    {detail.audits.length ? (
                      detail.audits.map((audit) => (
                        <article key={audit.auditId}>
                          <header>
                            <div>
                              <strong>{audit.operation}</strong>
                              <span
                                className={`status-pill status-pill--${audit.result === "SUCCESS" ? "success" : "danger"}`}
                              >
                                {audit.result}
                              </span>
                            </div>
                            <time>{formatLocalDateTime(audit.operatedAt)}</time>
                          </header>
                          <p>{audit.message || "已记录迁移审计事件"}</p>
                          <div className="history-audit-list__meta">
                            <span>目标：{audit.targetTable}{audit.targetId ? ` · ${audit.targetId}` : ""}</span>
                            <span>操作人：{audit.operatorId || "—"}</span>
                            <code>Trace {audit.traceId}</code>
                          </div>
                          <button
                            type="button"
                            onClick={() =>
                              setExpandedAuditId((current) =>
                                current === audit.auditId ? "" : audit.auditId,
                              )
                            }
                          >
                            {expandedAuditId === audit.auditId
                              ? "收起数据快照"
                              : "查看前后数据快照"}
                          </button>
                          {expandedAuditId === audit.auditId && (
                            <div className="history-audit-snapshot">
                              <div><strong>变更前</strong><pre>{JSON.stringify(audit.beforeData, null, 2)}</pre></div>
                              <div><strong>变更后</strong><pre>{JSON.stringify(audit.afterData, null, 2)}</pre></div>
                            </div>
                          )}
                        </article>
                      ))
                    ) : (
                      <div className="history-detail__empty">
                        <Info size={28} weight="duotone" />
                        <strong>本批次暂时没有审计事件</strong>
                        <span>预校验批次通常在正式写入后才会产生目标库审计记录。</span>
                      </div>
                    )}
                  </div>
                )}
              </>
            ) : (
              <div className="history-detail__empty">
                <ClockCounterClockwise size={36} weight="duotone" />
                <strong>选择一个历史批次</strong>
                <span>右侧会展示迁移结果、逐行错误和审计轨迹。</span>
              </div>
            )}
          </div>
        </div>
      </section>
    </div>
  );
}
