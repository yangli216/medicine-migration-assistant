import {
  ArrowCounterClockwise,
  ArrowRight,
  CheckCircle,
  CircleNotch,
  LinkSimple,
  ListMagnifyingGlass,
  MagnifyingGlass,
  PencilSimple,
  ShieldCheck,
  Table,
  Warning,
} from "@phosphor-icons/react";
import { SearchableSelect } from "./SearchableSelect";
import { completeInventoryOrganizationIds } from "./inventoryMapping";
import {
  formatInventoryMoney,
  inventoryFinancialTotals,
} from "./migrationPreview";

export function createInventoryRenderers(context) {
  const {
    autoMatchInventoryMappings,
    busy,
    inventoryBatchDetail,
    inventoryExecutionSeconds,
    inventoryLocationMappings,
    inventoryOrganizationMappings,
    inventoryReadiness,
    inventoryResolvedLocations,
    inventoryReviewSearch,
    inventoryReviewStatus,
    inventoryReviewStorage,
    inventoryUndoConfirmed,
    inventoryUndoPreview,
    legacyInventoryCatalog,
    preparePhis27Inventory,
    previewPhis27InventoryUndo,
    setInventoryBatchDetail,
    setInventoryLocationMappings,
    setInventoryMappingExpanded,
    setInventoryOrganizationMappings,
    setInventoryResolvedLocations,
    setInventoryReviewSearch,
    setInventoryReviewStatus,
    setInventoryReviewStorage,
    setInventoryUndoConfirmed,
    targetOrganizationCatalog,
    targetStorageCatalog,
    undoPhis27Inventory,
  } = context;

function renderInventoryMappingBoard() {
  if (
    !inventoryReadiness ||
    !legacyInventoryCatalog ||
    !targetOrganizationCatalog ||
    !targetStorageCatalog
  )
    return null;
  const sourceOrganizationIds = [
    ...new Set(
      inventoryReadiness.locations
        .map((location) => location.organizationId)
        .filter(Boolean),
    ),
  ];
  const completedOrganizationCount = completeInventoryOrganizationIds(
    inventoryReadiness.locations,
    inventoryOrganizationMappings,
    inventoryLocationMappings,
    inventoryResolvedLocations,
  ).length;
  const completedLocationCount = inventoryReadiness.locations.filter(
    (location) =>
      inventoryLocationMappings[location.sourceLocationKey] &&
      (location.mappingStatus !== "SOURCE_LOCATION_AMBIGUOUS" ||
        inventoryResolvedLocations[location.sourceLocationKey]),
  ).length;
  const organizationOptions = targetOrganizationCatalog.organizations.map(
    (organization) => {
      const organizationStorages = targetStorageCatalog.storages.filter(
        (storage) =>
          storage.organizationId === organization.id &&
          ["1", "2"].includes(storage.storageType),
      );
      const warehouseCount = organizationStorages.filter(
        (storage) => storage.storageType === "1",
      ).length;
      const pharmacyCount = organizationStorages.filter(
        (storage) => storage.storageType === "2",
      ).length;
      return {
      value: organization.id,
      label: organization.name || organization.fullName,
      description: `${organization.cd || "无编码"} · 药库 ${warehouseCount} · 药房 ${pharmacyCount}`,
      meta:
        organizationStorages.length > 0
          ? organization.parentText
            ? `上级：${organization.parentText}`
            : organization.orgTypeText || "当前租户机构"
          : "hi_sto_dept 未配置有效药库/药房，暂不可用于库存迁移",
      keywords: `${organization.name} ${organization.fullName} ${organization.cd} ${organization.parentText} 药库${warehouseCount} 药房${pharmacyCount}`,
      disabled: organizationStorages.length === 0,
    };
    },
  );
  return (
    <div className="inventory-mapping-board">
      <div className="inventory-mapping-board__heading">
        <div>
          <strong>机构与库房对应关系</strong>
          <span>先选目标机构，再为其下药库和药房选择同类型仓储。</span>
        </div>
        <div className="inventory-mapping-progress">
          <span>
            可迁移机构 {completedOrganizationCount}/{sourceOrganizationIds.length}
          </span>
          <span>
            库房 {completedLocationCount}/{inventoryReadiness.locations.length}
          </span>
          <button
            className="button button--secondary button--compact"
            onClick={autoMatchInventoryMappings}
          >
            <LinkSimple size={16} />
            同名自动匹配
          </button>
          {inventoryBatchDetail && (
            <button
              className="button button--secondary button--compact"
              type="button"
              onClick={() => setInventoryMappingExpanded(false)}
            >
              <Table size={16} />
              返回核对明细
            </button>
          )}
        </div>
      </div>
      <div className="inventory-org-groups">
        {sourceOrganizationIds.map((sourceOrganizationId) => {
          const sourceOrganization =
            legacyInventoryCatalog.organizations.find(
              (item) => item.id === sourceOrganizationId,
            );
          const targetOrganizationId =
            inventoryOrganizationMappings[sourceOrganizationId] || "";
          const locations = inventoryReadiness.locations.filter(
            (location) => location.organizationId === sourceOrganizationId,
          );
          const selectedOrganizationStorages =
            targetStorageCatalog.storages.filter(
              (storage) =>
                storage.organizationId === targetOrganizationId &&
                ["1", "2"].includes(storage.storageType),
            );
          const organizationCompleted = locations.filter(
            (location) =>
              inventoryLocationMappings[location.sourceLocationKey] &&
              (location.mappingStatus !== "SOURCE_LOCATION_AMBIGUOUS" ||
                inventoryResolvedLocations[location.sourceLocationKey]),
          ).length;
          return (
            <section className="inventory-org-group" key={sourceOrganizationId}>
              <div className="inventory-org-group__header">
                <div className="inventory-org-source">
                  <span className="inventory-location__kind">机构</span>
                  <div>
                    <strong>
                      {sourceOrganization?.name || `机构 ${sourceOrganizationId}`}
                    </strong>
                    <small>{sourceOrganizationId}</small>
                  </div>
                </div>
                <ArrowRight size={17} />
                <SearchableSelect
                  ariaLabel={`选择${sourceOrganization?.name || sourceOrganizationId}对应的新系统机构`}
                  value={targetOrganizationId}
                  onChange={(nextTargetOrganizationId) => {
                    setInventoryOrganizationMappings((current) => ({
                      ...current,
                      [sourceOrganizationId]: nextTargetOrganizationId,
                    }));
                    setInventoryLocationMappings((current) =>
                      Object.fromEntries(
                        Object.entries(current).filter(
                          ([sourceLocationKey, targetStorageId]) => {
                            const sourceLocation =
                              inventoryReadiness.locations.find(
                                (item) =>
                                  item.sourceLocationKey === sourceLocationKey,
                              );
                            const targetStorage =
                              targetStorageCatalog.storages.find(
                                (item) => item.idSto === targetStorageId,
                              );
                            return (
                              sourceLocation?.organizationId !==
                                sourceOrganizationId ||
                              targetStorage?.organizationId ===
                                nextTargetOrganizationId
                            );
                          },
                        ),
                      ),
                    );
                    setInventoryBatchDetail(null);
                  }}
                  options={organizationOptions}
                  placeholder="选择新系统机构"
                  searchPlaceholder="按机构名称、编码或上级机构过滤"
                />
                <span
                  className={`inventory-org-group__status ${organizationCompleted === locations.length && targetOrganizationId ? "is-complete" : ""}`}
                >
                  {organizationCompleted === locations.length && targetOrganizationId
                    ? "本批可迁移"
                    : targetOrganizationId
                      ? `还差 ${locations.length - organizationCompleted} 项`
                      : "待映射"}
                </span>
              </div>
              {targetOrganizationId &&
                selectedOrganizationStorages.length === 0 && (
                  <div className="inventory-org-storage-warning">
                    <Warning size={16} weight="fill" />
                    <span>
                      该机构在 hi_sto_dept 中没有启用的药库或药房，请改选已配置仓储的目标机构，或先在新系统补充仓储。
                    </span>
                  </div>
                )}
              <div className="inventory-org-locations">
                {locations.map((location) => {
                  const expectedType =
                    location.sourceKind === "WAREHOUSE" ? "1" : "2";
                  const sourceLocationOptions =
                    legacyInventoryCatalog.locations
                      .filter(
                        (item) =>
                          item.organizationId === sourceOrganizationId &&
                          item.sourceKind === location.sourceKind &&
                          item.active,
                      )
                      .map((item) => ({
                        value: item.sourceLocationKey,
                        label: item.name,
                        description: `${item.sourceKind === "WAREHOUSE" ? "药库" : "药房"} · ${item.sourceLocationKey}`,
                        meta: item.category
                          ? `类别编码：${item.category}`
                          : "老系统有效位置",
                        keywords: `${item.name} ${item.id} ${item.organizationId}`,
                      }));
                  const targetOptions = targetStorageCatalog.storages
                    .filter(
                      (storage) =>
                        storage.storageType === expectedType &&
                        storage.organizationId === targetOrganizationId,
                    )
                    .map((storage) => ({
                      value: storage.idSto,
                      label: storage.name,
                      description: storage.storageTypeName,
                      meta: storage.productTypes
                        ? `适用：${storage.productTypes}`
                        : "未限制物品类型",
                      keywords: `${storage.name} ${storage.idSto}`,
                    }));
                  return (
                    <div
                      className="inventory-compact-location-row"
                      key={location.sourceLocationKey}
                    >
                      <div className="inventory-compact-location-source">
                        <span className="inventory-location__kind">
                          {location.sourceKind === "WAREHOUSE" ? "药库" : "药房"}
                        </span>
                        <div>
                          <strong>
                            {location.mappingStatus ===
                            "SOURCE_LOCATION_AMBIGUOUS"
                              ? "药库库存总账"
                              : location.sourceLocationName}
                          </strong>
                          <small>
                            {location.medicineCount} 种药品 · {location.stockRowCount} 条
                          </small>
                        </div>
                        {location.mappingStatus ===
                          "SOURCE_LOCATION_AMBIGUOUS" && (
                          <SearchableSelect
                            className="inventory-source-location-select"
                            ariaLabel="指定药库库存总账所属的老系统药库"
                            value={
                              inventoryResolvedLocations[
                                location.sourceLocationKey
                              ] || ""
                            }
                            onChange={(sourceLocationKey) => {
                              setInventoryResolvedLocations((current) => ({
                                ...current,
                                [location.sourceLocationKey]: sourceLocationKey,
                              }));
                              setInventoryBatchDetail(null);
                            }}
                            options={sourceLocationOptions}
                            placeholder="指定实际老药库"
                            searchPlaceholder="按老药库名称或识别码过滤"
                          />
                        )}
                      </div>
                      <ArrowRight size={16} />
                      <SearchableSelect
                        ariaLabel={`选择${location.sourceLocationName}对应的新系统库房`}
                        value={
                          inventoryLocationMappings[
                            location.sourceLocationKey
                          ] || ""
                        }
                        onChange={(targetIdSto) => {
                          setInventoryLocationMappings((current) => ({
                            ...current,
                            [location.sourceLocationKey]: targetIdSto,
                          }));
                          setInventoryBatchDetail(null);
                        }}
                        options={targetOptions}
                        disabled={!targetOrganizationId}
                        placeholder={
                          !targetOrganizationId
                            ? "先选择上方目标机构"
                            : targetOptions.length
                              ? "选择新系统库房"
                              : `该机构未配置${location.sourceKind === "WAREHOUSE" ? "药库（sdSto=1）" : "药房（sdSto=2）"}`
                        }
                        searchPlaceholder="按库房名称或主键过滤"
                      />
                    </div>
                  );
                })}
              </div>
            </section>
          );
        })}
      </div>
      <div className="inventory-mapping-actions inventory-mapping-actions--compact">
        <div>
          <ShieldCheck size={18} weight="fill" />
          <span>
            本批只处理已完整映射的机构，其他机构可在后续批次继续配置。
          </span>
        </div>
        <button
          className="button button--primary"
          disabled={busy === "inventory-prepare" || completedOrganizationCount === 0}
          onClick={preparePhis27Inventory}
        >
          <ListMagnifyingGlass size={18} />
          {busy === "inventory-prepare"
            ? "正在检查…"
            : `检查已完成机构（${completedOrganizationCount}）`}
        </button>
      </div>
    </div>
  );
}

function renderInventoryBatchScopeSummary() {
  if (!inventoryBatchDetail) return null;
  const sourceOrganizationIds = [
    ...new Set(
      inventoryBatchDetail.rows
        .map((row) => row.rawData?.sourceOrganizationId)
        .filter(Boolean),
    ),
  ];
  const totalOrganizationCount = new Set(
    (inventoryReadiness?.locations || [])
      .map((location) => location.organizationId)
      .filter(Boolean),
  ).size;
  const locationPairs = [
    ...new Map(
      inventoryBatchDetail.rows.map((row) => [
        `${row.rawData?.sourceLocationKey || ""}:${row.normalizedData?.idSto || ""}`,
        {
          source: row.rawData?.sourceLocationName || "未命名来源库房",
          target: row.normalizedData?.naSto || "未命名目标库房",
          kind: row.rawData?.sourceKind === "WAREHOUSE" ? "药库" : "药房",
        },
      ]),
    ).values(),
  ];
  return (
    <section className="inventory-batch-scope">
      <div className="inventory-batch-scope__heading">
        <div>
          <CheckCircle size={20} weight="fill" />
          <div>
            <strong>本批迁移范围已锁定</strong>
            <span>
              {sourceOrganizationIds.length} 个机构 · {locationPairs.length} 个药库/药房；
              其余 {Math.max(totalOrganizationCount - sourceOrganizationIds.length, 0)} 个机构留待后续批次。
            </span>
          </div>
        </div>
        <button
          className="button button--secondary button--compact"
          type="button"
          onClick={() => setInventoryMappingExpanded(true)}
        >
          <PencilSimple size={16} />
          修改本批范围
        </button>
      </div>
      <div className="inventory-batch-scope__locations">
        {locationPairs.map((location) => (
          <div key={`${location.source}-${location.target}`}>
            <span>{location.kind}</span>
            <strong>{location.source}</strong>
            <ArrowRight size={14} />
            <strong>{location.target}</strong>
          </div>
        ))}
      </div>
    </section>
  );
}

function renderInventoryBatchReview() {
  if (!inventoryBatchDetail) return null;
  const rows = inventoryBatchDetail.rows || [];
  const storageOptions = [{
    value: "",
    label: "全部目标库房",
    description: "不限制目标库房",
    keywords: "全部",
  },
    ...new Map(
      rows
        .filter((row) => row.normalizedData?.idSto)
        .map((row) => [
          row.normalizedData.idSto,
          {
            value: row.normalizedData.idSto,
            label: row.normalizedData.naSto || row.normalizedData.idSto,
            description: `目标库房 · ${row.normalizedData.idSto}`,
            keywords: `${row.normalizedData.naSto || ""} ${row.normalizedData.idSto}`,
          },
        ]),
    ).values(),
  ];
  const normalizedSearch = inventoryReviewSearch.trim().toLocaleLowerCase("zh-CN");
  const matchesStatus = (row) => {
    if (inventoryReviewStatus === "ALL") return true;
    if (inventoryReviewStatus === "READY") {
      return ["VALIDATED", "SUCCESS", "SKIPPED", "UNDONE"].includes(row.status);
    }
    return ["INVALID", "FAILED"].includes(row.status);
  };
  const filteredRows = rows.filter((row) => {
    const searchable = [
      row.sourceKey,
      row.rawData?.sourceProductKey,
      row.rawData?.drugName,
      row.rawData?.productName,
      row.rawData?.specification,
      row.rawData?.saleSpecification,
      row.rawData?.saleUnit,
      row.rawData?.unitSaleFactor,
      row.rawData?.factoryName,
      row.rawData?.sourceLocationName,
      row.normalizedData?.naSto,
      row.normalizedData?.batchCode,
      row.normalizedData?.effectiveDate,
    ]
      .filter(Boolean)
      .join(" ")
      .toLocaleLowerCase("zh-CN");
    return (
      matchesStatus(row) &&
      (!inventoryReviewStorage ||
        row.normalizedData?.idSto === inventoryReviewStorage) &&
      (!normalizedSearch || searchable.includes(normalizedSearch))
    );
  });
  const sourceRecordCount = rows.reduce(
    (sum, row) => sum + (row.rawData?.sourceRecordIds?.length || 1),
    0,
  );
  const medicineCount = new Set(
    rows.map((row) => row.rawData?.sourceProductKey).filter(Boolean),
  ).size;
  const storageCount = new Set(
    rows.map((row) => row.normalizedData?.idSto).filter(Boolean),
  ).size;
  const readyCount = rows.filter((row) =>
    ["VALIDATED", "SUCCESS", "SKIPPED", "UNDONE"].includes(row.status),
  ).length;
  const failedCount = rows.filter((row) =>
    ["INVALID", "FAILED"].includes(row.status),
  ).length;
  const batchFinancials = inventoryFinancialTotals(rows);
  const filteredFinancials = inventoryFinancialTotals(filteredRows);
  const statusLabel = (status) => {
    if (status === "SUCCESS") return "已写入";
    if (status === "SKIPPED") return "已存在";
    if (status === "UNDONE") return "已撤销";
    if (status === "VALIDATED") return "可迁移";
    return "异常";
  };
  return (
    <section className="inventory-review-workbench">
      <div className="inventory-review-workbench__heading">
        <div>
          <span className="eyebrow">正式写入前核对</span>
          <h3>逐组核对待迁移库存</h3>
          <p>
            相同库房、商品、管理包装、进销价、批号和效期的老库存已合并为一组；当前 {rows.length} 组来自 {sourceRecordCount} 条原始记录。
          </p>
        </div>
        <div className="inventory-review-counts">
          <span>{medicineCount} 种药品</span>
          <span>{storageCount} 个目标库房</span>
        </div>
      </div>
      <div className="inventory-review-summary" aria-label="库存预检摘要">
        {[
          ["待核对库存组", rows.length, ""],
          ["原始库存明细", sourceRecordCount, ""],
          ["涉及药品", medicineCount, ""],
          [inventoryBatchDetail.batch.status === "UNDONE" ? "已撤销" : "可迁移", readyCount, "ready"],
          ["异常", failedCount, failedCount ? "danger" : ""],
        ].map(([label, value, tone]) => (
          <div className={tone ? `is-${tone}` : ""} key={label}>
            <span>{label}</span>
            <strong>{value}</strong>
          </div>
        ))}
      </div>
      <div className="inventory-financial-summary" aria-label="本批库存金额合计">
        <div>
          <span>进货金额合计</span>
          <strong>¥ {formatInventoryMoney(batchFinancials.purchase)}</strong>
          <small>优先按老系统账面进货金额汇总</small>
        </div>
        <div>
          <span>零售金额合计</span>
          <strong>¥ {formatInventoryMoney(batchFinancials.retail)}</strong>
          <small>优先按老系统账面零售金额汇总</small>
        </div>
        <div>
          <span>预计零售差额</span>
          <strong>
            ¥ {formatInventoryMoney(
              batchFinancials.retail - batchFinancials.purchase,
            )}
          </strong>
          <small>零售金额 − 进货金额，仅供核对</small>
        </div>
      </div>
      <div className="inventory-review-toolbar">
        <label className="inventory-review-search">
          <MagnifyingGlass size={17} />
          <input
            value={inventoryReviewSearch}
            onChange={(event) => setInventoryReviewSearch(event.target.value)}
            placeholder="搜索药品、厂家、批号或来源键"
          />
        </label>
        <SearchableSelect
          className="inventory-review-storage-filter"
          ariaLabel="按目标库房筛选库存明细"
          value={inventoryReviewStorage}
          onChange={setInventoryReviewStorage}
          options={storageOptions}
          placeholder="全部目标库房"
          searchPlaceholder="按目标库房名称过滤"
        />
        <div className="inventory-review-status-filter" aria-label="按校验状态筛选">
          {[
            ["ALL", "全部", rows.length],
            ["READY", inventoryBatchDetail.batch.status === "UNDONE" ? "已撤销" : "可迁移", readyCount],
            ["FAILED", "异常", failedCount],
          ].map(([value, label, count]) => (
            <button
              type="button"
              className={inventoryReviewStatus === value ? "is-active" : ""}
              onClick={() => setInventoryReviewStatus(value)}
              key={value}
            >
              {label} {count}
            </button>
          ))}
        </div>
        <span className="inventory-review-visible-count">
          显示 {filteredRows.length}/{rows.length} 组
        </span>
      </div>
      <div className="inventory-filtered-totals">
        <strong>当前显示合计</strong>
        <span>进货 ¥ {formatInventoryMoney(filteredFinancials.purchase)}</span>
        <span>零售 ¥ {formatInventoryMoney(filteredFinancials.retail)}</span>
        <span>
          差额 ¥ {formatInventoryMoney(
            filteredFinancials.retail - filteredFinancials.purchase,
          )}
        </span>
      </div>
      <div className="inventory-review-table-wrap">
        <table className="inventory-review-table">
          <thead>
            <tr>
              <th>来源与去向</th>
              <th>药品基本信息</th>
              <th>厂家与商品</th>
              <th>批号 / 效期</th>
              <th>数量 / 价格</th>
              <th>核对结果</th>
            </tr>
          </thead>
          <tbody>
            {filteredRows.map((row) => {
              const sourceIds = row.rawData?.sourceRecordIds || [];
              const invalid = ["INVALID", "FAILED"].includes(row.status);
              return (
                <tr className={invalid ? "is-invalid" : ""} key={row.rowId}>
                  <td>
                    <strong>{row.rawData?.sourceLocationName || "未命名来源库房"}</strong>
                    <span className="inventory-review-route">
                      <ArrowRight size={13} />
                      {row.normalizedData?.naSto || "未匹配目标库房"}
                    </span>
                    <code>{row.rawData?.sourceProductKey || row.sourceKey}</code>
                    <small>
                      {sourceIds.length > 1
                        ? `由 ${sourceIds.length} 条记录合并：${sourceIds.join("、")}`
                        : `来源记录 ${sourceIds[0] || row.sourceKey}`}
                    </small>
                  </td>
                  <td>
                    <strong>{row.rawData?.drugName || "未读取药品名称"}</strong>
                    <span>{row.rawData?.specification || "规格未提供"}</span>
                    <small>
                      剂型编码 {row.rawData?.dosageForm || "—"} · 最小单位 {row.rawData?.minimumUnit || "—"}
                    </small>
                    <span className="inventory-packaging-badge">
                      库房管理包装：{row.rawData?.saleSpecification || row.rawData?.specification || "规格未提供"} · {row.normalizedData?.unitSale || row.rawData?.saleUnit || "单位未提供"} × {row.normalizedData?.unitSaleFactor || row.rawData?.unitSaleFactor || "—"}
                    </span>
                    {(row.rawData?.productSaleUnit || row.rawData?.productUnitSaleFactor) && (
                      <small>
                        商品主档包装：{row.rawData?.productSaleUnit || "—"} × {row.rawData?.productUnitSaleFactor || "—"}（仅供对照）
                      </small>
                    )}
                  </td>
                  <td>
                    <strong>{row.rawData?.productName || row.rawData?.drugName || "—"}</strong>
                    <span>{row.rawData?.factoryName || "厂家未提供"}</span>
                  </td>
                  <td>
                    <strong>{row.normalizedData?.batchCode || "无批号"}</strong>
                    <span>有效期 {row.normalizedData?.effectiveDate || "未提供"}</span>
                  </td>
                  <td>
                    <strong>
                      {row.normalizedData?.amount || "0"} {row.normalizedData?.unitSale || row.rawData?.saleUnit || ""}
                    </strong>
                    <span>进价 {row.normalizedData?.pricePur || "0"} / {row.normalizedData?.unitSale || row.rawData?.saleUnit || "管理单位"}</span>
                    <small>进货金额 {row.normalizedData?.purchaseTotal || "—"}</small>
                    <span>零售价 {row.normalizedData?.priceSale || "0"} / {row.normalizedData?.unitSale || row.rawData?.saleUnit || "管理单位"}</span>
                    <small>零售金额 {row.normalizedData?.retailTotal || "—"}</small>
                  </td>
                  <td>
                    <span className={`inventory-review-status ${invalid ? "is-invalid" : "is-ready"}`}>
                      {statusLabel(row.status)}
                    </span>
                    {row.errorMessage && <p>{row.errorMessage}</p>}
                    {(row.rawData?.packagingNotes || []).map((note) => (
                      <small className="inventory-packaging-note" key={note}>{note}</small>
                    ))}
                  </td>
                </tr>
              );
            })}
            {!filteredRows.length && (
              <tr>
                <td className="inventory-review-empty" colSpan="6">
                  当前筛选条件下没有库存明细
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </section>
  );
}

function renderInventoryUndoPanel() {
  if (!inventoryUndoPreview) return null;
  return (
    <section
      className={`inventory-undo-panel ${inventoryUndoPreview.canUndo ? "is-ready" : "is-blocked"}`}
    >
      <div className="inventory-undo-panel__heading">
        {inventoryUndoPreview.canUndo ? (
          <ShieldCheck size={21} weight="fill" />
        ) : (
          <Warning size={21} weight="fill" />
        )}
        <div>
          <strong>
            {inventoryUndoPreview.canUndo
              ? "安全撤销预检已通过"
              : "发现后续业务，不能撤销"}
          </strong>
          <span>{inventoryUndoPreview.message}</span>
        </div>
        <button
          className="button button--secondary button--compact"
          type="button"
          disabled={busy === "inventory-undo-preview" || busy === "inventory-undo"}
          onClick={previewPhis27InventoryUndo}
        >
          <ListMagnifyingGlass size={16} />
          重新检查
        </button>
      </div>
      <div className="inventory-undo-scope">
        {inventoryUndoPreview.storages.map((storage) => (
          <article className={storage.canUndo ? "is-ready" : "is-blocked"} key={storage.idSto}>
            <div>
              <span>{storage.canUndo ? "可撤销" : "已阻止"}</span>
              <strong>{storage.name || storage.idSto}</strong>
              <small>
                盘点单 {storage.cdStoCheck || "—"} · {storage.inventoryCount} 组库存
              </small>
            </div>
            {storage.messages.length > 0 && (
              <ul>
                {storage.messages.map((message) => (
                  <li key={message}>{message}</li>
                ))}
              </ul>
            )}
          </article>
        ))}
      </div>
      {inventoryUndoPreview.canUndo && (
        <div className="inventory-undo-confirmation">
          <label>
            <input
              type="checkbox"
              checked={inventoryUndoConfirmed}
              disabled={busy === "inventory-undo"}
              onChange={(event) => setInventoryUndoConfirmed(event.target.checked)}
            />
            <span>
              我确认撤销 {inventoryUndoPreview.storageCount} 个库房的首次盘点、
              {inventoryUndoPreview.inventoryCount} 组初始库存及其初始账簿日志
            </span>
          </label>
          <div>
            <small>库房药品配置 hi_sto_med 将保留，方便重新迁移。</small>
            <button
              className="button button--danger"
              type="button"
              aria-busy={busy === "inventory-undo"}
              disabled={busy === "inventory-undo"}
              onClick={undoPhis27Inventory}
            >
              {busy === "inventory-undo" ? (
                <CircleNotch className="is-spinning" size={18} weight="bold" />
              ) : (
                <ArrowCounterClockwise size={18} weight="bold" />
              )}
              {busy === "inventory-undo"
                ? `正在安全撤销 · ${inventoryExecutionSeconds}秒`
                : "确认安全撤销首次盘点"}
            </button>
          </div>
          {busy === "inventory-undo" && (
            <div className="inventory-execution-progress" role="status" aria-live="polite">
              <CircleNotch className="is-spinning" size={18} weight="bold" />
              <div>
                <strong>正在重新校验并逆序撤销首次盘点</strong>
                <span>
                  已等待 {inventoryExecutionSeconds} 秒；事务完成前请勿关闭应用。
                </span>
              </div>
            </div>
          )}
        </div>
      )}
    </section>
  );
}
  return {
    renderInventoryMappingBoard,
    renderInventoryBatchScopeSummary,
    renderInventoryBatchReview,
    renderInventoryUndoPanel,
  };
}
