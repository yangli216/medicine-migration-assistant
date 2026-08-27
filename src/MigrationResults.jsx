import { useEffect, useMemo, useState } from "react";
import { CaretLeft, CaretRight, LinkSimple, ListMagnifyingGlass } from "@phosphor-icons/react";
import { fieldsMentionedInValidationError } from "./migrationFields";
import { sourceValueLabel } from "./migrationPreview";
import { statusMeta } from "./MigrationHistory";

function validationMedicineName(row) {
  const candidates = [
    row.normalizedData?.naMed,
    row.rawData?.DRUG_NAME,
    row.rawData?.drugName,
    row.rawData?.medicineName,
    row.rawData?.YPMC,
  ];
  const name = candidates.find(
    (value) => value !== null && value !== undefined && String(value).trim(),
  );
  return name ? String(name).trim() : "药品名称未读取";
}

export function DataTable({
  columns,
  rows,
  maxRows = 6,
  preferredColumns = [],
  columnLabels = {},
  columnMetadata = {},
}) {
  const prioritized = preferredColumns.filter((column) => columns.includes(column));
  const shown = prioritized.length ? prioritized : columns.slice(0, 7);
  return (
    <div className="data-table">
      <div
        className="data-table__row data-table__head"
        style={{
          gridTemplateColumns: `repeat(${shown.length}, minmax(140px, 1fr))`,
        }}
      >
        {shown.map((column) => (
          <span key={column} title={column}>
            {columnLabels[column] || column}
          </span>
        ))}
      </div>
      {rows.slice(0, maxRows).map((row, index) => (
        <div
          className="data-table__row"
          style={{
            gridTemplateColumns: `repeat(${shown.length}, minmax(140px, 1fr))`,
          }}
          key={index}
        >
          {shown.map((column) => (
            <span
              title={sourceValueLabel(row[column], columnMetadata[column])}
              key={column}
            >
              {row[column] === null || row[column] === undefined
                ? "—"
                : sourceValueLabel(row[column], columnMetadata[column])}
            </span>
          ))}
        </div>
      ))}
    </div>
  );
}


export function ValidationResults({
  detail,
  filter,
  onFilterChange,
  onEditField,
}) {
  const pageSize = 100;
  const [page, setPage] = useState(0);
  const filters = [
    ["INVALID", "校验失败", detail.batch.failCount, "danger"],
    ["VALIDATED", "可迁移", detail.batch.validCount, "ready"],
    ...(detail.batch.skipCount
      ? [["SKIPPED", "已跳过", detail.batch.skipCount, "muted"]]
      : []),
    ["ALL", "全部", detail.batch.totalCount, "neutral"],
  ];
  const visibleRows = useMemo(
    () =>
      filter === "ALL"
        ? detail.rows
        : detail.rows.filter((row) => row.status === filter),
    [detail.rows, filter],
  );
  const pageCount = Math.max(1, Math.ceil(visibleRows.length / pageSize));
  const safePage = Math.min(page, pageCount - 1);
  const pageRows = visibleRows.slice(
    safePage * pageSize,
    (safePage + 1) * pageSize,
  );
  const rangeStart = visibleRows.length ? safePage * pageSize + 1 : 0;
  const rangeEnd = Math.min((safePage + 1) * pageSize, visibleRows.length);
  const hasFailures = detail.batch.failCount > 0;

  useEffect(() => {
    setPage(0);
  }, [filter, detail.batch.batchId]);

  return (
    <div className="validation-result">
      <div className="table-heading validation-result__heading">
        <div>
          <ListMagnifyingGlass size={20} />
          <strong>逐行校验结果</strong>
          <small>
            当前显示 {rangeStart}–{rangeEnd}，筛选结果 {visibleRows.length} 条，共 {detail.batch.totalCount} 条
          </small>
        </div>
        <span
          className={`status-pill status-pill--${hasFailures ? "danger" : "ready"}`}
        >
          {hasFailures
            ? `${detail.batch.failCount} 条校验失败`
            : "全部校验通过"}
        </span>
      </div>
      <div className="validation-filters" aria-label="校验结果筛选">
        {filters.map(([key, label, count, tone]) => (
          <button
            className={`validation-filter validation-filter--${tone} ${filter === key ? "validation-filter--active" : ""}`}
            type="button"
            aria-pressed={filter === key}
            onClick={() => onFilterChange(key)}
            key={key}
          >
            <span>{label}</span>
            <strong>{count}</strong>
          </button>
        ))}
      </div>
      <div className="issue-list">
        {visibleRows.length ? (
          pageRows.map((row) => {
            const issueFields = fieldsMentionedInValidationError(
              row.errorMessage,
            );
            const medicineName = validationMedicineName(row);
            return (
              <div
                className={row.status === "INVALID" ? "issue-list__row--danger" : ""}
                key={row.rowId}
              >
                <span>第 {row.rowNo} 行</span>
                <code>{row.sourceKey}</code>
                <strong
                  className="issue-list__medicine"
                  title={medicineName}
                >
                  {medicineName}
                </strong>
                <span
                  className={`status-pill status-pill--${statusMeta(row.status)[1]}`}
                >
                  {statusMeta(row.status)[0]}
                </span>
                <div className="issue-list__message">
                  <p title={row.errorMessage || "字段、类型和条件规则均通过"}>
                    {row.errorMessage || "字段、类型和条件规则均通过"}
                  </p>
                  {issueFields.length > 0 && (
                    <div className="issue-list__actions">
                      {issueFields.map((field) => (
                        <button
                          type="button"
                          onClick={() => onEditField(field, row)}
                          key={field.key}
                        >
                          <LinkSimple size={13} />
                          维护“{field.label}”映射
                        </button>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            );
          })
        ) : (
          <div className="issue-list__empty">当前筛选下没有记录</div>
        )}
      </div>
      {pageCount > 1 && (
        <div className="validation-pagination" aria-label="校验结果分页">
          <span>每页最多 {pageSize} 条，所有结果均可逐页查看</span>
          <div>
            <button
              type="button"
              disabled={safePage === 0}
              onClick={() => setPage((current) => Math.max(0, current - 1))}
              aria-label="上一页"
            >
              <CaretLeft size={15} />
              上一页
            </button>
            <strong>{safePage + 1} / {pageCount}</strong>
            <button
              type="button"
              disabled={safePage >= pageCount - 1}
              onClick={() =>
                setPage((current) => Math.min(pageCount - 1, current + 1))
              }
              aria-label="下一页"
            >
              下一页
              <CaretRight size={15} />
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
