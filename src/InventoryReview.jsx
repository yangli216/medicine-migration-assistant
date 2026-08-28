import {
  ArrowCounterClockwise,
  ArrowRight,
  Check,
  CheckCircle,
  CircleNotch,
  FloppyDisk,
  LinkSimple,
  ListMagnifyingGlass,
  MagnifyingGlass,
  PencilSimple,
  ShieldCheck,
  Table,
  Warning,
  X,
} from "@phosphor-icons/react";
import { createPortal } from "react-dom";
import { SearchableSelect } from "./SearchableSelect";
import {
  completeInventoryOrganizationIds,
  inventoryLocationNeedsSourceResolution,
} from "./inventoryMapping";
import {
  formatInventoryMoney,
  inventoryFinancialTotals,
} from "./migrationPreview";
import { inventoryUnmatchedMedicines } from "./inventoryMedicineMatching";

export function createInventoryRenderers(context) {
  const {
    autoMatchInventoryMappings,
    busy,
    inventoryBatchDetail,
    inventoryActiveOrganizationId,
    inventoryExecutionSeconds,
    inventoryExceptionReason,
    inventoryExceptionRow,
    inventoryLocationSearch,
    inventoryLocationMappings,
    inventoryMedicineCatalog,
    inventoryMedicineConfirmations,
    inventoryMedicineMatchFilter,
    inventoryMedicineMatchOpen,
    inventoryMedicineMatchPage,
    inventoryMedicineMatchSearch,
    inventoryMedicineSelections,
    inventoryMedicineSelectedTargets,
    inventoryMedicineSuggestions,
    inventoryOrganizationMappings,
    inventoryReadiness,
    inventoryResolvedLocations,
    inventorySelectedLocationKeys,
    inventorySelectedOrganizationIds,
    inventoryReviewSearch,
    inventoryReviewStatus,
    inventoryReviewStorage,
    inventoryTrialRowId,
    inventoryTrialStatus,
    inventoryUndoConfirmed,
    inventoryUndoPreview,
    legacyInventoryCatalog,
    prepareInventoryBatch,
    confirmInventoryException,
    openInventoryMedicineMatching,
    searchInventoryTargetMedicines,
    previewInventoryUndo,
    setInventoryBatchDetail,
    setInventoryExceptionReason,
    setInventoryExceptionRow,
    setInventoryActiveOrganizationId,
    setInventoryLocationSearch,
    setInventoryLocationMappings,
    setInventoryMedicineConfirmations,
    setInventoryMedicineMatchFilter,
    setInventoryMedicineMatchOpen,
    setInventoryMedicineMatchPage,
    setInventoryMedicineMatchSearch,
    setInventoryMedicineSelections,
    setInventoryMedicineSelectedTargets,
    setInventoryMappingExpanded,
    setInventoryOrganizationMappings,
    setInventoryResolvedLocations,
    setInventorySelectedLocationKeys,
    setInventorySelectedOrganizationIds,
    setInventoryReviewSearch,
    setInventoryReviewStatus,
    setInventoryReviewStorage,
    setInventoryUndoConfirmed,
    targetOrganizationCatalog,
    targetStorageCatalog,
    saveInventoryMedicineMatches,
    trialInventoryRow,
    undoInventoryBatch,
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
  const activeOrganizationId = sourceOrganizationIds.includes(
    inventoryActiveOrganizationId,
  )
    ? inventoryActiveOrganizationId
    : sourceOrganizationIds[0] || "";
  const selectedLocationKeySet = new Set(inventorySelectedLocationKeys);
  const selectedLocations = inventoryReadiness.locations.filter((location) =>
    selectedLocationKeySet.has(location.sourceLocationKey),
  );
  const selectedSourceOrganizationIds = [
    ...new Set(
      selectedLocations
        .map((location) => location.organizationId)
        .filter(Boolean),
    ),
  ];
  const completedOrganizationIds = completeInventoryOrganizationIds(
    inventoryReadiness.locations,
    inventoryOrganizationMappings,
    inventoryLocationMappings,
    inventoryResolvedLocations,
    inventorySelectedLocationKeys,
  );
  const completedOrganizationCount = completedOrganizationIds.length;
  const completedOrganizationIdSet = new Set(completedOrganizationIds);
  const selectedOrganizationCount = inventorySelectedOrganizationIds.filter(
    (organizationId) => completedOrganizationIdSet.has(organizationId),
  ).length;
  const completedLocationCount = selectedLocations.filter(
    (location) =>
      inventoryLocationMappings[location.sourceLocationKey] &&
      (!inventoryLocationNeedsSourceResolution(location) ||
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
  const sourceOrganizationOptions = sourceOrganizationIds.map(
    (sourceOrganizationId) => {
      const organization = legacyInventoryCatalog.organizations.find(
        (item) => item.id === sourceOrganizationId,
      );
      const locations = inventoryReadiness.locations.filter(
        (location) => location.organizationId === sourceOrganizationId,
      );
      const selectedCount = locations.filter((location) =>
        selectedLocationKeySet.has(location.sourceLocationKey),
      ).length;
      return {
        value: sourceOrganizationId,
        label: organization?.name || `机构 ${sourceOrganizationId}`,
        description: `${sourceOrganizationId} · 药库/药房 ${locations.length}`,
        meta: selectedCount
          ? `本批已选 ${selectedCount} 个位置`
          : "本批尚未选择位置",
        keywords: `${organization?.name || ""} ${sourceOrganizationId}`,
      };
    },
  );
  return (
    <div className="inventory-mapping-board">
      <div className="inventory-mapping-board__heading">
        <div>
          <strong>机构与库房对应关系</strong>
          <span>先选老系统机构和本批库房，再设置对应关系；未选范围留待后续批次。</span>
        </div>
        <div className="inventory-mapping-progress">
          <span>
            可执行机构 {completedOrganizationCount}/{selectedSourceOrganizationIds.length}
          </span>
          <span>本批已选 {selectedOrganizationCount}</span>
          <span>
            库房 {completedLocationCount}/{selectedLocations.length}
          </span>
          <button
            className="button button--secondary button--compact"
            disabled={selectedLocations.length === 0}
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
      <div className="inventory-scope-navigator">
        <div>
          <span className="eyebrow">1. 选择要处理的老系统机构</span>
          <SearchableSelect
            ariaLabel="选择要处理的老系统机构"
            value={activeOrganizationId}
            onChange={(sourceOrganizationId) => {
              setInventoryActiveOrganizationId(sourceOrganizationId);
              setInventoryLocationSearch("");
            }}
            options={sourceOrganizationOptions}
            placeholder="按机构名称或编码查找"
            searchPlaceholder="搜索老系统机构"
          />
        </div>
        <span>
          共 {sourceOrganizationIds.length} 个机构 · 本批已选 {selectedSourceOrganizationIds.length} 个机构 / {selectedLocations.length} 个库房
        </span>
      </div>
      <div className="inventory-org-groups">
        {sourceOrganizationIds
          .filter((sourceOrganizationId) => sourceOrganizationId === activeOrganizationId)
          .map((sourceOrganizationId) => {
          const sourceOrganization =
            legacyInventoryCatalog.organizations.find(
              (item) => item.id === sourceOrganizationId,
            );
          const targetOrganizationId =
            inventoryOrganizationMappings[sourceOrganizationId] || "";
          const locations = inventoryReadiness.locations.filter(
            (location) => location.organizationId === sourceOrganizationId,
          );
          const scopedLocations = locations.filter((location) =>
            selectedLocationKeySet.has(location.sourceLocationKey),
          );
          const normalizedLocationSearch = inventoryLocationSearch
            .trim()
            .toLocaleLowerCase("zh-CN");
          const visibleScopeLocations = locations.filter((location) =>
            `${location.sourceLocationName} ${location.sourceLocationKey} ${location.sourceKind}`
              .toLocaleLowerCase("zh-CN")
              .includes(normalizedLocationSearch),
          );
          const selectedOrganizationStorages =
            targetStorageCatalog.storages.filter(
              (storage) =>
                storage.organizationId === targetOrganizationId &&
                ["1", "2"].includes(storage.storageType),
            );
          const organizationCompleted = scopedLocations.filter(
            (location) =>
              inventoryLocationMappings[location.sourceLocationKey] &&
              (!inventoryLocationNeedsSourceResolution(location) ||
                inventoryResolvedLocations[location.sourceLocationKey]),
          ).length;
          const organizationReady =
            scopedLocations.length > 0 &&
            organizationCompleted === scopedLocations.length &&
            Boolean(targetOrganizationId);
          const selectedForBatch =
            organizationReady &&
            inventorySelectedOrganizationIds.includes(sourceOrganizationId);
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
                    setInventorySelectedOrganizationIds((current) =>
                      current.filter((id) => id !== sourceOrganizationId),
                    );
                  }}
                  options={organizationOptions}
                  placeholder="选择新系统机构"
                  searchPlaceholder="按机构名称、编码或上级机构过滤"
                />
                <label
                  className={`inventory-org-group__batch-choice ${organizationReady ? "is-ready" : ""}`}
                  title={
                    organizationReady
                      ? "选择该机构纳入本次库存核对与迁移"
                      : "请先选择本批库房并完成这些库房的映射"
                  }
                >
                  <input
                    checked={selectedForBatch}
                    disabled={!organizationReady}
                    onChange={(event) =>
                      setInventorySelectedOrganizationIds((current) =>
                        event.target.checked
                          ? [...new Set([...current, sourceOrganizationId])]
                          : current.filter((id) => id !== sourceOrganizationId),
                      )
                    }
                    type="checkbox"
                  />
                  <span>
                    {organizationReady
                      ? selectedForBatch
                        ? "已纳入本批"
                        : "纳入本批"
                      : targetOrganizationId
                        ? scopedLocations.length
                          ? `还差 ${scopedLocations.length - organizationCompleted} 项`
                          : "先选本批库房"
                        : "待映射"}
                  </span>
                </label>
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
              <div className="inventory-location-scope-picker">
                <div className="inventory-location-scope-picker__heading">
                  <div>
                    <strong>2. 选择本批需要迁移的药库/药房</strong>
                    <span>
                      已选 {scopedLocations.length}/{locations.length}，只展开已选位置的映射项。
                    </span>
                  </div>
                  <div>
                    <button
                      className="button button--ghost button--compact"
                      type="button"
                      onClick={() => {
                        setInventorySelectedLocationKeys((current) => [
                          ...new Set([
                            ...current,
                            ...locations.map((location) => location.sourceLocationKey),
                          ]),
                        ]);
                        setInventorySelectedOrganizationIds((current) =>
                          current.filter((id) => id !== sourceOrganizationId),
                        );
                        setInventoryBatchDetail(null);
                      }}
                    >
                      全选本机构
                    </button>
                    <button
                      className="button button--ghost button--compact"
                      type="button"
                      disabled={scopedLocations.length === 0}
                      onClick={() => {
                        const organizationLocationKeys = new Set(
                          locations.map((location) => location.sourceLocationKey),
                        );
                        setInventorySelectedLocationKeys((current) =>
                          current.filter(
                            (locationKey) => !organizationLocationKeys.has(locationKey),
                          ),
                        );
                        setInventorySelectedOrganizationIds((current) =>
                          current.filter((id) => id !== sourceOrganizationId),
                        );
                        setInventoryBatchDetail(null);
                      }}
                    >
                      清空
                    </button>
                  </div>
                </div>
                <label className="inventory-location-scope-picker__search">
                  <MagnifyingGlass size={16} />
                  <input
                    aria-label="搜索本机构药库或药房"
                    type="search"
                    value={inventoryLocationSearch}
                    onChange={(event) =>
                      setInventoryLocationSearch(event.target.value)
                    }
                    placeholder="搜索药库/药房名称或识别码"
                  />
                </label>
                <div className="inventory-location-scope-picker__options">
                  {visibleScopeLocations.map((location) => {
                    const checked = selectedLocationKeySet.has(
                      location.sourceLocationKey,
                    );
                    return (
                      <label
                        className={`inventory-location-scope-option ${checked ? "is-selected" : ""}`}
                        key={location.sourceLocationKey}
                      >
                        <input
                          type="checkbox"
                          checked={checked}
                          onChange={(event) => {
                            setInventorySelectedLocationKeys((current) =>
                              event.target.checked
                                ? [...new Set([...current, location.sourceLocationKey])]
                                : current.filter(
                                    (locationKey) =>
                                      locationKey !== location.sourceLocationKey,
                                  ),
                            );
                            setInventorySelectedOrganizationIds((current) =>
                              current.filter((id) => id !== sourceOrganizationId),
                            );
                            setInventoryBatchDetail(null);
                          }}
                        />
                        <span className="inventory-location__kind">
                          {location.sourceKind === "WAREHOUSE" ? "药库" : "药房"}
                        </span>
                        <span>
                          <strong>{location.sourceLocationName}</strong>
                          <small>
                            {location.medicineCount} 种药品 · {location.stockRowCount} 条 · {location.sourceLocationKey}
                          </small>
                        </span>
                      </label>
                    );
                  })}
                  {visibleScopeLocations.length === 0 && (
                    <div className="inventory-location-scope-picker__empty">
                      没有匹配的药库或药房
                    </div>
                  )}
                </div>
              </div>
              <div className="inventory-org-locations">
                {scopedLocations.map((location) => {
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
                          <strong>{location.sourceLocationName}</strong>
                          <small>
                            {location.medicineCount} 种药品 · {location.stockRowCount} 条
                          </small>
                        </div>
                        {inventoryLocationNeedsSourceResolution(location) && (
                          <SearchableSelect
                            className="inventory-source-location-select"
                            ariaLabel="指定待确认药库库存所属的老系统药库"
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
                              setInventorySelectedOrganizationIds((current) =>
                                current.filter((id) => id !== sourceOrganizationId),
                              );
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
                          setInventorySelectedOrganizationIds((current) =>
                            current.filter((id) => id !== sourceOrganizationId),
                          );
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
                {scopedLocations.length === 0 && (
                  <div className="inventory-org-locations__empty">
                    请先在上方勾选本批需要迁移的药库或药房。
                  </div>
                )}
              </div>
            </section>
          );
        })}
      </div>
      <div className="inventory-mapping-actions inventory-mapping-actions--compact">
        <div>
          <ShieldCheck size={18} weight="fill" />
          <span>
            只会读取“已勾选库房 + 已纳入机构”的明细；其他机构和库房不进入本批校验或写入。
          </span>
        </div>
        <button
          className="button button--primary"
          disabled={busy === "inventory-prepare" || selectedOrganizationCount === 0}
          onClick={prepareInventoryBatch}
        >
          <ListMagnifyingGlass size={18} />
          {busy === "inventory-prepare"
            ? "正在检查…"
            : `读取并核对本批（${selectedOrganizationCount} 机构 / ${selectedLocations.length} 库房）`}
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
  const totalLocationCount = inventoryReadiness?.locations?.length || 0;
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
              未选的 {Math.max(totalOrganizationCount - sourceOrganizationIds.length, 0)} 个机构、{Math.max(totalLocationCount - locationPairs.length, 0)} 个库房留待后续批次。
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
  const unmatchedMedicines = inventoryUnmatchedMedicines(rows);
  const suggestionByKey = new Map(
    inventoryMedicineSuggestions.map((suggestion) => [suggestion.key, suggestion]),
  );
  const confirmedMedicineCount = unmatchedMedicines.filter(
    (medicine) => inventoryMedicineConfirmations[medicine.key],
  ).length;
  const normalizedMedicineMatchSearch = inventoryMedicineMatchSearch
    .trim()
    .toLocaleLowerCase("zh-CN");
  const visibleMatchingMedicines = unmatchedMedicines.filter((medicine) => {
    const confirmed = Boolean(inventoryMedicineConfirmations[medicine.key]);
    if (inventoryMedicineMatchFilter === "CONFIRMED" && !confirmed) return false;
    if (inventoryMedicineMatchFilter === "PENDING" && confirmed) return false;
    if (!normalizedMedicineMatchSearch) return true;
    return [
      medicine.drugName,
      medicine.productName,
      medicine.specification,
      medicine.factoryName,
      medicine.sourceProductKey,
    ]
      .filter(Boolean)
      .join(" ")
      .toLocaleLowerCase("zh-CN")
      .includes(normalizedMedicineMatchSearch);
  });
  const medicineMatchPageSize = 30;
  const medicineMatchPageCount = Math.max(
    1,
    Math.ceil(visibleMatchingMedicines.length / medicineMatchPageSize),
  );
  const currentMedicineMatchPage = Math.min(
    Math.max(1, inventoryMedicineMatchPage),
    medicineMatchPageCount,
  );
  const pagedMatchingMedicines = visibleMatchingMedicines.slice(
    (currentMedicineMatchPage - 1) * medicineMatchPageSize,
    currentMedicineMatchPage * medicineMatchPageSize,
  );
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
      {unmatchedMedicines.length > 0 && (
        <section className="inventory-medicine-match-panel">
          <div className="inventory-medicine-match-panel__heading">
            <div>
              <Warning size={20} weight="fill" />
              <div>
                <strong>{unmatchedMedicines.length} 种老系统药品尚未匹配新系统目录</strong>
                <span>
                  在独立匹配工作台中按名称、规格、厂家核对；原库存页面不再展开大量匹配内容。
                </span>
              </div>
            </div>
            <button
              className="button button--primary button--compact"
              type="button"
              disabled={busy === "inventory-medicine-catalog"}
              onClick={openInventoryMedicineMatching}
            >
              {busy === "inventory-medicine-catalog" ? (
                <CircleNotch className="is-spinning" size={16} weight="bold" />
              ) : (
                <ListMagnifyingGlass size={16} />
              )}
              {busy === "inventory-medicine-catalog"
                ? "正在分析…"
                : inventoryMedicineCatalog
                  ? `继续匹配（已确认 ${confirmedMedicineCount}）`
                  : "打开智能匹配工作台"}
            </button>
          </div>
        </section>
      )}
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
      <div className="inventory-review-scroll-guide" aria-hidden="true">
        <span>← 横向滚动查看完整字段 →</span>
      </div>
      <div
        className="inventory-review-table-wrap"
        tabIndex="0"
        role="region"
        aria-label="库存核对明细，可横向和纵向滚动查看完整内容"
      >
        <table className="inventory-review-table">
          <thead>
            <tr>
              <th>来源与去向</th>
              <th>药品基本信息</th>
              <th>厂家与商品</th>
              <th>批号 / 效期</th>
              <th>数量 / 价格</th>
              <th>核对结果</th>
              <th>库存试迁移</th>
            </tr>
          </thead>
          <tbody>
            {filteredRows.map((row) => {
              const sourceIds = row.rawData?.sourceRecordIds || [];
              const invalid = ["INVALID", "FAILED"].includes(row.status);
              const exceptionConfirmed =
                row.errorCode === "INVENTORY_EXCEPTION_CONFIRMED";
              const validationIssues = row.rawData?.inventoryValidationIssues || [];
              const canConfirmException =
                row.status === "INVALID" &&
                validationIssues.length > 0 &&
                validationIssues.every((issue) => issue.reviewable === true);
              const idSto = `${row.normalizedData?.idSto || ""}`.trim();
              const storageTrial = inventoryTrialStatus.latest.get(idSto);
              const storageTrialPassed =
                storageTrial?.result === "SUCCESS" &&
                storageTrial.afterData?.rolledBack === true;
              const trialRunning = inventoryTrialRowId === row.rowId;
              const trialEligible = ["VALIDATED", "FAILED"].includes(row.status);
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
                    <span className={`inventory-review-status ${invalid ? "is-invalid" : exceptionConfirmed ? "is-exception" : "is-ready"}`}>
                      {exceptionConfirmed ? "人工确认" : statusLabel(row.status)}
                    </span>
                    {row.errorMessage && <p>{row.errorMessage}</p>}
                    {canConfirmException && (
                      <button
                        className="button button--secondary button--compact inventory-exception-trigger"
                        type="button"
                        onClick={() => {
                          setInventoryExceptionRow(row);
                          setInventoryExceptionReason("");
                        }}
                      >
                        <ShieldCheck size={15} />
                        确认例外
                      </button>
                    )}
                    {(row.rawData?.packagingNotes || []).map((note) => (
                      <small className="inventory-packaging-note" key={note}>{note}</small>
                    ))}
                  </td>
                  <td className="inventory-trial-cell">
                    {storageTrial && (
                      <span
                        className={`inventory-trial-result ${storageTrialPassed ? "is-success" : "is-danger"}`}
                        title={storageTrial.message}
                      >
                        {storageTrialPassed
                          ? storageTrial.rowId === row.rowId
                            ? "样例通过 · 已回滚"
                            : "本库房已通过"
                          : "未通过 · 已回滚"}
                      </span>
                    )}
                    {trialEligible && (
                      <button
                        className="button button--secondary button--compact inventory-trial-button"
                        type="button"
                        disabled={busy === "inventory-trial"}
                        onClick={() => trialInventoryRow(row)}
                      >
                        {trialRunning ? (
                          <CircleNotch className="is-spinning" size={15} weight="bold" />
                        ) : (
                          <ShieldCheck size={15} />
                        )}
                        {trialRunning
                          ? "验证中…"
                          : storageTrialPassed
                            ? "换此行重验"
                            : "试迁移此行"}
                      </button>
                    )}
                  </td>
                </tr>
              );
            })}
            {!filteredRows.length && (
              <tr>
                <td className="inventory-review-empty" colSpan="7">
                  当前筛选条件下没有库存明细
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
      {inventoryExceptionRow && createPortal(
        <div className="inventory-exception-overlay">
          <section
            className="inventory-exception-dialog"
            role="dialog"
            aria-modal="true"
            aria-label="人工确认库存校验例外"
          >
            <header>
              <div>
                <span className="eyebrow">机构库存 · 人工复核</span>
                <h2>确认本组库存例外</h2>
                <p>该库存仍会完整迁移，并保留原校验结果、确认原因和操作人审计记录。</p>
              </div>
              <button
                type="button"
                aria-label="关闭人工确认窗口"
                disabled={busy === "inventory-exception"}
                onClick={() => setInventoryExceptionRow(null)}
              >
                <X size={20} />
              </button>
            </header>
            <div className="inventory-exception-dialog__body">
              <div className="inventory-exception-medicine">
                <strong>{inventoryExceptionRow.rawData?.drugName || "未读取药品名称"}</strong>
                <span>{inventoryExceptionRow.rawData?.specification || "规格未提供"}</span>
                <code>{inventoryExceptionRow.rawData?.sourceProductKey || inventoryExceptionRow.sourceKey}</code>
              </div>
              <div className="inventory-exception-original">
                <Warning size={18} weight="fill" />
                <div>
                  <strong>原校验原因</strong>
                  <p>{inventoryExceptionRow.errorMessage}</p>
                </div>
              </div>
              <label>
                <span>人工确认原因 <b>必填</b></span>
                <textarea
                  autoFocus
                  maxLength="300"
                  value={inventoryExceptionReason}
                  onChange={(event) => setInventoryExceptionReason(event.target.value)}
                  placeholder="例如：已与药房负责人核对，现场确认为每盒 1 支，按盒管理库存。"
                />
                <small>{inventoryExceptionReason.trim().length}/300 字</small>
              </label>
              <p className="inventory-exception-warning">
                确认后该目标库房原有试迁移结果会失效，必须重新试迁移后才能正式写入。
              </p>
            </div>
            <footer>
              <button
                className="button button--secondary"
                type="button"
                disabled={busy === "inventory-exception"}
                onClick={() => setInventoryExceptionRow(null)}
              >
                取消
              </button>
              <button
                className="button button--primary"
                type="button"
                disabled={
                  busy === "inventory-exception" ||
                  inventoryExceptionReason.trim().length < 2
                }
                onClick={confirmInventoryException}
              >
                {busy === "inventory-exception" ? (
                  <CircleNotch className="is-spinning" size={16} weight="bold" />
                ) : (
                  <ShieldCheck size={16} />
                )}
                {busy === "inventory-exception" ? "正在记录…" : "确认例外并继续"}
              </button>
            </footer>
          </section>
        </div>,
        document.body,
      )}
      {inventoryMedicineMatchOpen && inventoryMedicineCatalog && createPortal(
        <div className="inventory-medicine-match-overlay">
          <section
            className="inventory-medicine-match-dialog"
            role="dialog"
            aria-modal="true"
            aria-label="未匹配药品智能匹配工作台"
          >
            <header className="inventory-medicine-match-dialog__header">
              <div>
                <span className="eyebrow">机构库存 · 药品目录核对</span>
                <h2>未匹配药品智能匹配</h2>
                <p>
                  已按名称、规格、厂家综合相似度预选最匹配商品。预选不等于保存，请核对完整信息后逐项确认。
                </p>
              </div>
              <div className="inventory-medicine-match-dialog__summary">
                <span>待处理 {unmatchedMedicines.length}</span>
                <strong>已确认 {confirmedMedicineCount}</strong>
                <span>目标目录 {inventoryMedicineCatalog.catalogCount || 0}</span>
                <button
                  type="button"
                  aria-label="关闭药品匹配工作台"
                  onClick={() => setInventoryMedicineMatchOpen(false)}
                >
                  <X size={20} />
                </button>
              </div>
            </header>
            <div className="inventory-medicine-match-dialog__toolbar">
              <label>
                <MagnifyingGlass size={17} />
                <input
                  value={inventoryMedicineMatchSearch}
                  onChange={(event) => {
                    setInventoryMedicineMatchSearch(event.target.value);
                    setInventoryMedicineMatchPage(1);
                  }}
                  placeholder="搜索老系统药品、规格、厂家或来源键"
                />
              </label>
              <div aria-label="筛选匹配确认状态">
                {[
                  ["ALL", "全部", unmatchedMedicines.length],
                  ["PENDING", "待确认", unmatchedMedicines.length - confirmedMedicineCount],
                  ["CONFIRMED", "已确认", confirmedMedicineCount],
                ].map(([value, label, count]) => (
                  <button
                    type="button"
                    className={inventoryMedicineMatchFilter === value ? "is-active" : ""}
                    onClick={() => {
                      setInventoryMedicineMatchFilter(value);
                      setInventoryMedicineMatchPage(1);
                    }}
                    key={value}
                  >
                    {label} {count}
                  </button>
                ))}
              </div>
              <span>
                显示 {visibleMatchingMedicines.length}/{unmatchedMedicines.length} 种 · 第 {currentMedicineMatchPage}/{medicineMatchPageCount} 页
              </span>
            </div>
            <div className="inventory-medicine-match-dialog__body">
              {pagedMatchingMedicines.map((source) => {
                const suggestion = suggestionByKey.get(source.key);
                const ranked = suggestion?.candidates || [];
                const rankedById = new Map(
                  ranked.map((candidate) => [candidate.target.idMedPro, candidate]),
                );
                const selectedId = inventoryMedicineSelections[source.key] || "";
                const selectedTarget = inventoryMedicineSelectedTargets[source.key];
                const selectedCandidate = rankedById.get(selectedId);
                const confirmed = Boolean(inventoryMedicineConfirmations[source.key]);
                const recommendedOptions = ranked.map((candidate) => ({
                  value: candidate.target.idMedPro,
                  label: candidate.target.drugName || candidate.target.productName,
                  description: `${candidate.target.specification || candidate.target.saleSpecification || "规格未提供"} · ${candidate.target.factoryName || "厂家未提供"}`,
                  meta: `${candidate.exact ? "完全一致" : `相似度 ${candidate.scorePercent}%`} · ${candidate.reason}`,
                  keywords: `${candidate.target.productName || ""} ${candidate.target.externalCode || ""} ${candidate.target.approvalCode || ""}`,
                  group: "智能推荐",
                  data: candidate.target,
                }));
                const targetOption = (medicine) => ({
                  value: medicine.idMedPro,
                  label: medicine.drugName || medicine.productName,
                  description: `${medicine.specification || medicine.saleSpecification || "规格未提供"} · ${medicine.factoryName || "厂家未提供"}`,
                  meta: `${medicine.private ? "机构私有商品" : "租户通用商品"} · ${medicine.externalCode || medicine.approvalCode || medicine.idMedPro}`,
                  keywords: `${medicine.productName || ""} ${medicine.externalCode || ""} ${medicine.approvalCode || ""}`,
                  group: "完整目录搜索结果",
                  data: medicine,
                });
                return (
                  <article
                    className={`inventory-medicine-match-card ${confirmed ? "is-confirmed" : ""}`}
                    key={source.key}
                  >
                    <div className="inventory-medicine-match-card__source">
                      <span>老系统药品</span>
                      <strong title={source.drugName}>{source.drugName || "未读取药品名称"}</strong>
                      <dl>
                        <div><dt>规格</dt><dd>{source.specification || "未提供"}</dd></div>
                        <div><dt>厂家</dt><dd>{source.factoryName || "未提供"}</dd></div>
                        <div><dt>商品名</dt><dd>{source.productName || "未提供"}</dd></div>
                        <div><dt>来源</dt><dd>{source.sourceProductKey} · {source.rowCount} 组库存</dd></div>
                      </dl>
                    </div>
                    <ArrowRight className="inventory-medicine-match-card__arrow" size={19} />
                    <div className="inventory-medicine-match-card__target">
                      <div className="inventory-medicine-match-card__select-row">
                        <SearchableSelect
                          ariaLabel={`为${source.drugName || source.sourceProductKey}选择新系统药品`}
                          value={selectedId}
                          onChange={(idMedPro, option) => {
                            const target =
                              option?.data || rankedById.get(idMedPro)?.target;
                            setInventoryMedicineSelections((current) => ({
                              ...current,
                              [source.key]: idMedPro,
                            }));
                            setInventoryMedicineSelectedTargets((current) => ({
                              ...current,
                              [source.key]: target,
                            }));
                            setInventoryMedicineConfirmations((current) => ({
                              ...current,
                              [source.key]: false,
                            }));
                          }}
                          options={recommendedOptions}
                          loadOptions={async (query) =>
                            (
                              await searchInventoryTargetMedicines(
                                source.targetOrganizationId,
                                query,
                              )
                            ).map(targetOption)
                          }
                          placeholder="选择新系统药品商品"
                          searchPlaceholder="输入关键词搜索完整目标目录"
                          emptyText="请输入名称、规格、厂家、批准文号或商品编码"
                          maxVisibleOptions={60}
                        />
                        {selectedCandidate ? (
                          <span className={`inventory-medicine-match-score ${selectedCandidate.exact ? "is-exact" : ""}`}>
                            {selectedCandidate.exact
                              ? "完全一致"
                              : `相似度 ${selectedCandidate.scorePercent}%`}
                          </span>
                        ) : (
                          <span className="inventory-medicine-match-score">人工选择</span>
                        )}
                      </div>
                      {selectedTarget ? (
                        <div className="inventory-medicine-match-card__selected">
                          <div className="inventory-medicine-match-card__selected-heading">
                            <div>
                              <span>已选新系统药品商品</span>
                              <strong>{selectedTarget.drugName || selectedTarget.productName}</strong>
                            </div>
                            <button
                              type="button"
                              className={confirmed ? "is-confirmed" : ""}
                              onClick={() =>
                                setInventoryMedicineConfirmations((current) => ({
                                  ...current,
                                  [source.key]: !current[source.key],
                                }))
                              }
                            >
                              <Check size={16} weight="bold" />
                              {confirmed ? "已确认" : "确认匹配"}
                            </button>
                          </div>
                          <dl>
                            <div><dt>基础规格</dt><dd>{selectedTarget.specification || "未提供"}</dd></div>
                            <div><dt>商品名称</dt><dd>{selectedTarget.productName || "未提供"}</dd></div>
                            <div><dt>销售规格</dt><dd>{selectedTarget.saleSpecification || "未提供"}</dd></div>
                            <div><dt>生产厂家</dt><dd>{selectedTarget.factoryName || "未提供"}</dd></div>
                            <div><dt>最小单位</dt><dd>{selectedTarget.minimumUnit || "未提供"}</dd></div>
                            <div><dt>批准文号</dt><dd>{selectedTarget.approvalCode || "未提供"}</dd></div>
                            <div><dt>商品编码</dt><dd>{selectedTarget.externalCode || selectedTarget.idMedPro}</dd></div>
                            <div><dt>使用范围</dt><dd>{selectedTarget.private ? "当前目标机构私有" : "当前租户通用"}</dd></div>
                          </dl>
                          <p>
                            {selectedCandidate
                              ? `匹配依据：${selectedCandidate.reason}`
                              : "该商品由人工搜索选择，请重点核对名称、规格和厂家。"}
                          </p>
                        </div>
                      ) : (
                        <div className="inventory-medicine-match-card__empty">
                          未找到可用候选，请搜索新系统完整药品目录。
                        </div>
                      )}
                    </div>
                  </article>
                );
              })}
              {!visibleMatchingMedicines.length && (
                <div className="inventory-medicine-match-dialog__empty">
                  当前搜索和筛选条件下没有药品
                </div>
              )}
            </div>
            <footer className="inventory-medicine-match-dialog__footer">
              <span>
                已确认 {confirmedMedicineCount}/{unmatchedMedicines.length} 种；未确认项仍保留异常，不会写入库存。
              </span>
              <div>
                {medicineMatchPageCount > 1 && (
                  <div className="inventory-medicine-match-pagination" aria-label="药品匹配分页">
                    <button
                      className="button button--secondary button--compact"
                      type="button"
                      disabled={currentMedicineMatchPage <= 1}
                      onClick={() => setInventoryMedicineMatchPage(currentMedicineMatchPage - 1)}
                    >
                      上一页
                    </button>
                    <span>{currentMedicineMatchPage}/{medicineMatchPageCount}</span>
                    <button
                      className="button button--secondary button--compact"
                      type="button"
                      disabled={currentMedicineMatchPage >= medicineMatchPageCount}
                      onClick={() => setInventoryMedicineMatchPage(currentMedicineMatchPage + 1)}
                    >
                      下一页
                    </button>
                  </div>
                )}
                <button
                  className="button button--secondary"
                  type="button"
                  onClick={() => setInventoryMedicineMatchOpen(false)}
                >
                  暂时关闭
                </button>
                <button
                  className="button button--primary"
                  type="button"
                  disabled={!confirmedMedicineCount || busy === "inventory-medicine-save"}
                  onClick={saveInventoryMedicineMatches}
                >
                  {busy === "inventory-medicine-save" ? (
                    <CircleNotch className="is-spinning" size={17} weight="bold" />
                  ) : (
                    <FloppyDisk size={17} />
                  )}
                  {busy === "inventory-medicine-save"
                    ? "正在保存并复核…"
                    : `保存 ${confirmedMedicineCount} 项并重新核对`}
                </button>
              </div>
            </footer>
          </section>
        </div>,
        document.body,
      )}
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
          onClick={previewInventoryUndo}
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
              onClick={undoInventoryBatch}
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
