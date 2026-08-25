import {
  ArrowCounterClockwise,
  ArrowLeft,
  ArrowRight,
  CaretDown,
  CheckCircle,
  CircleNotch,
  Database,
  Info,
  ListMagnifyingGlass,
  PencilSimple,
  Plugs,
  ShieldCheck,
  Warning,
} from "@phosphor-icons/react";
import {
  ConnectionForm,
  ConnectionPicker,
} from "./DatabaseConnections";
import { SearchableSelect } from "./SearchableSelect";
import { inventoryLocationNeedsSourceResolution } from "./inventoryMapping";

export function InventoryMigrationScreen({ context }) {
  const {
    busy,
    databaseConnections,
    databaseDrivers,
    driverPacks,
    executePhis27Inventory,
    forgetSourceConnection,
    forgetTargetDatabaseConnection,
    hasSavedSourceConnection,
    hasSavedTargetDatabase,
    inspectPhis27Inventory,
    inventoryBatchDetail,
    inventoryExecutionSeconds,
    inventoryLocationMappings,
    inventoryMappingExpanded,
    inventoryOrganizationMappings,
    inventoryPreflightExpanded,
    inventoryReadiness,
    inventoryResolvedLocations,
    inventoryReviewConfirmed,
    inventoryTrialStatus,
    inventorySourceExpanded,
    inventoryTargetExpanded,
    legacyInventoryCatalog,
    loadInventoryTargetStorages,
    migrationType,
    preparePhis27Inventory,
    previewPhis27InventoryUndo,
    rememberSourcePassword,
    rememberTargetDatabasePassword,
    renderInventoryBatchReview,
    renderInventoryBatchScopeSummary,
    renderInventoryMappingBoard,
    renderInventoryUndoPanel,
    selectDatabaseConnection,
    selectedSourceConnectionId,
    selectedTargetConnectionId,
    setConnectionManagerOpen,
    setInventoryBatchDetail,
    setInventoryLocationMappings,
    setInventoryOrganizationMappings,
    setInventoryPreflightExpanded,
    setInventoryReadiness,
    setInventoryResolvedLocations,
    setInventorySelectedOrganizationIds,
    setInventoryReviewConfirmed,
    setInventorySourceExpanded,
    setInventoryTargetExpanded,
    setLegacyInventoryCatalog,
    setRememberSourcePassword,
    setRememberTargetDatabasePassword,
    setSelectedSourceConnectionId,
    setSelectedTargetConnectionId,
    setSourceConnectionEditing,
    setSourceProfile,
    setStep,
    setTargetConnectionEditing,
    setTargetOrganizationCatalog,
    setTargetProfile,
    setTargetStorageCatalog,
    sourceConnectionEditing,
    sourceProfile,
    step,
    targetAuth,
    targetConnectionEditing,
    targetOrganizationCatalog,
    targetProfile,
    targetStorageCatalog,
    tenantId,
  } = context;

  return (
    <>
{step === 2 && migrationType === "INVENTORY" && (
  <section className="screen inventory-screen">
    <div className="screen-heading screen-heading--row">
      <div>
        <span className="eyebrow">第 3 步 · 选择本批迁移范围</span>
        <h1>先选择需要迁移的机构和库房</h1>
        <p>
          先轻量读取机构和非零库存范围；完成本批机构、库房映射后，才读取所选范围的
          YK_KCMX / YF_KCMX 明细并核对药品台账。
        </p>
      </div>
      <div className="source-summary">
        <ShieldCheck size={24} />
        <div>
          <small>基础数据前置条件</small>
          <strong>药品基础迁移已完成并核实</strong>
        </div>
      </div>
    </div>
    <div className="source-db-layout inventory-source-layout">
      {inventorySourceExpanded || !inventoryReadiness ? (
        <>
      <ConnectionPicker
        purpose="SOURCE"
        entries={databaseConnections}
        selectedId={selectedSourceConnectionId}
        onSelect={(connectionId) => {
          selectDatabaseConnection(connectionId, "SOURCE");
          setInventoryReadiness(null);
          setInventorySourceExpanded(true);
          setLegacyInventoryCatalog(null);
          setTargetOrganizationCatalog(null);
          setInventoryOrganizationMappings({});
          setTargetStorageCatalog(null);
          setInventoryLocationMappings({});
          setInventoryResolvedLocations({});
          setInventorySelectedOrganizationIds([]);
          setInventoryBatchDetail(null);
        }}
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
            setInventoryReadiness(null);
            setInventorySourceExpanded(true);
            setLegacyInventoryCatalog(null);
            setTargetOrganizationCatalog(null);
            setInventoryOrganizationMappings({});
            setTargetStorageCatalog(null);
            setInventoryLocationMappings({});
            setInventoryResolvedLocations({});
            setInventorySelectedOrganizationIds([]);
            setInventoryBatchDetail(null);
          }}
          title="二系列phis老系统只读连接"
          drivers={databaseDrivers}
          driverPacks={driverPacks}
          rememberPassword={rememberSourcePassword}
          onRememberPasswordChange={setRememberSourcePassword}
          hasSavedConnection={hasSavedSourceConnection}
          onForgetSaved={forgetSourceConnection}
          showCredentialPreference={false}
        />
      )}
      <div className="inventory-preflight-card">
        <div className="inventory-preflight-card__copy">
          <Database size={22} weight="duotone" />
          <div>
            <strong>读取机构及非零库存范围</strong>
            <span>
              本次只汇总机构、药库和药房，不读取全部药品明细；选定本批范围后再核对主键、批号、效期和价格。
            </span>
          </div>
        </div>
        <button
          className="button button--primary"
          disabled={busy === "inventory-inspect"}
          onClick={inspectPhis27Inventory}
        >
          <ListMagnifyingGlass size={19} />
          {busy === "inventory-inspect"
            ? "正在读取范围…"
            : "读取机构与库存范围"}
        </button>
      </div>
        </>
      ) : (
        <div className="inventory-connection-summary">
          <div>
            <CheckCircle size={20} weight="fill" />
            <div>
              <strong>老系统机构与库存范围已读取</strong>
              <span>
                {inventoryReadiness.schema} · {sourceProfile.host}:{sourceProfile.port}
              </span>
            </div>
          </div>
          <button
            className="button button--secondary button--compact"
            onClick={() => setInventorySourceExpanded(true)}
          >
            <PencilSimple size={16} />
            修改数据源
          </button>
        </div>
      )}
    </div>
    {inventoryReadiness && (
      <div className="inventory-readiness">
        <div
          className={`access-result ${inventoryReadiness.readyForLocationMapping ? "access-result--success" : "access-result--warning"}`}
        >
          {inventoryReadiness.readyForLocationMapping ? (
            <CheckCircle size={20} weight="fill" />
          ) : (
            <Warning size={20} weight="fill" />
          )}
          <div>
            <strong>{inventoryReadiness.message}</strong>
            <span>
              {inventoryReadiness.schema} · {inventoryReadiness.sourceName}
            </span>
          </div>
        </div>
        <div className="inventory-readiness-summary">
          <div>
            <span>
              预计 {inventoryReadiness.stockRowCount} 条明细
            </span>
            <span>
              {inventoryReadiness.medicineCount} 个药品-库房组合
            </span>
            <span>
              {inventoryReadiness.locations.length} 个待映射位置
            </span>
          </div>
          <button
            className="button button--ghost button--compact"
            onClick={() =>
              setInventoryPreflightExpanded((current) => !current)
            }
          >
            {inventoryPreflightExpanded ? "收起范围详情" : "查看范围详情"}
            <CaretDown
              size={15}
              className={inventoryPreflightExpanded ? "is-rotated" : ""}
            />
          </button>
        </div>
        {inventoryPreflightExpanded && (
          <>
          <div className="summary-cards inventory-summary-cards">
          {[
            ["预计库存明细", inventoryReadiness.stockRowCount],
            ["药品-库房组合", inventoryReadiness.stockGroupCount],
            ["待选库房", inventoryReadiness.locations.length],
          ].map(([label, value], index) => (
            <div
              className={`summary-card ${index === 3 ? "summary-card--ready" : index === 4 && value ? "summary-card--danger" : ""}`}
              key={label}
            >
              <span>{label}</span>
              <strong>{value}</strong>
            </div>
          ))}
        </div>
        <div className="inventory-location-list">
          <div className="inventory-location-list__heading">
            <div>
              <strong>本次可选择的库存位置</strong>
              <span>先选定一个或多个完整机构，再读取这些机构的库存明细进行核对。</span>
            </div>
            <em>{inventoryReadiness.locations.length} 个位置</em>
          </div>
          {inventoryReadiness.locations.map((location) => (
            <article key={location.sourceLocationKey}>
              <span className="inventory-location__kind">
                {location.sourceKind === "WAREHOUSE" ? "药库" : "药房"}
              </span>
              <div className="inventory-location__name">
                <strong>{location.sourceLocationName}</strong>
                <small>
                  {location.sourceLocationKey} · 机构 {location.organizationId || "未提供"}
                </small>
              </div>
              <div className="inventory-location__counts">
                <span>{location.medicineCount} 种药品</span>
                <span>{location.stockRowCount} 条明细</span>
              </div>
              <div className={`inventory-location__status inventory-location__status--${location.mappingStatus.toLowerCase()}`}>
                <strong>
                  {location.mappingStatus === "PENDING_TARGET_MAPPING"
                    ? "待映射新库房"
                    : "需人工确认"}
                </strong>
                <small>{location.mappingMessage}</small>
              </div>
            </article>
          ))}
        </div>
        <div className="prerequisite-note">
          <Info size={17} />
          <span>
            药库库存优先按 YK_YPXX 中的 YKSB + YPXH 唯一关系拆分到具体药库；无法唯一判断时，只将该药品列为待确认，不会复制库存。药库数量取 YK_KCMX.KCSL，药房数量取 YF_KCMX.YPSL。
          </span>
        </div>
        {inventoryReadiness.unresolvedSourceKeys.length > 0 && (
          <div className="inventory-unresolved">
            <Warning size={18} weight="fill" />
            <div>
              <strong>未关联药品来源键（最多展示 20 个）</strong>
              <span>{inventoryReadiness.unresolvedSourceKeys.join("、")}</span>
            </div>
          </div>
        )}
        {inventoryReadiness.warnings.map((warning) => (
          <div className="prerequisite-note" key={warning}>
            <Info size={17} />
            <span>{warning}</span>
          </div>
        ))}
          </>
        )}
        {inventoryReadiness.readyForLocationMapping && (
          <div className="inventory-target-panel">
            <div className="inventory-target-panel__heading">
              <div>
                <span className="eyebrow">目标机构与库房</span>
                <strong>选择本批机构并设置库房对应关系</strong>
                <p>
                  每个已完整映射的机构都可以独立进入本批，其余机构留待后续处理。
                </p>
              </div>
              {targetStorageCatalog && (
                <span className="status-pill status-pill--ready">
                  {
                    targetStorageCatalog.storages.filter((storage) =>
                      ["1", "2"].includes(storage.storageType),
                    ).length
                  } 个有效药库/药房
                </span>
              )}
            </div>
            {inventoryTargetExpanded || !targetStorageCatalog ? (
              <>
            <ConnectionPicker
              purpose="TARGET"
              entries={databaseConnections}
              selectedId={selectedTargetConnectionId}
              onSelect={(connectionId) => {
                selectDatabaseConnection(connectionId, "TARGET");
                setTargetOrganizationCatalog(null);
                setInventoryTargetExpanded(true);
                setInventoryOrganizationMappings({});
                setTargetStorageCatalog(null);
                setInventoryLocationMappings({});
                setInventoryResolvedLocations({});
                setInventorySelectedOrganizationIds([]);
                setInventoryBatchDetail(null);
              }}
              onManage={() => setConnectionManagerOpen(true)}
              editing={targetConnectionEditing}
              onToggleEditing={() =>
                setTargetConnectionEditing((current) => !current)
              }
            />
            {(!selectedTargetConnectionId || targetConnectionEditing) && (
              <ConnectionForm
                value={targetProfile}
                onChange={(profile) => {
                  setTargetProfile(profile);
                  setSelectedTargetConnectionId("");
                  setTargetConnectionEditing(true);
                  setTargetOrganizationCatalog(null);
                  setInventoryTargetExpanded(true);
                  setInventoryOrganizationMappings({});
                  setTargetStorageCatalog(null);
                  setInventoryLocationMappings({});
                  setInventoryResolvedLocations({});
                  setInventorySelectedOrganizationIds([]);
                  setInventoryBatchDetail(null);
                }}
                title="新系统目标数据库"
                drivers={databaseDrivers}
                driverPacks={driverPacks}
                rememberPassword={rememberTargetDatabasePassword}
                onRememberPasswordChange={setRememberTargetDatabasePassword}
                hasSavedConnection={hasSavedTargetDatabase}
                onForgetSaved={forgetTargetDatabaseConnection}
                showCredentialPreference={false}
              />
            )}
            <div className="inventory-target-actions">
              <div>
                <strong>当前租户：{tenantId || targetAuth?.tenantId}</strong>
                <span>仓储和库存重复检查均限定在当前认证租户内。</span>
              </div>
              <button
                className="button button--secondary"
                disabled={busy === "inventory-target"}
                onClick={loadInventoryTargetStorages}
              >
                <Plugs size={18} />
                {busy === "inventory-target"
                  ? "正在读取…"
                  : "读取机构与库房清单"}
              </button>
            </div>
              </>
            ) : (
              <div className="inventory-connection-summary">
                <div>
                  <CheckCircle size={20} weight="fill" />
                  <div>
                    <strong>新系统机构与库房已读取</strong>
                    <span>
                      {targetOrganizationCatalog?.organizations.length || 0} 个机构 · {
                        new Set(
                          targetStorageCatalog.storages
                            .filter((storage) =>
                              ["1", "2"].includes(storage.storageType),
                            )
                            .map((storage) => storage.organizationId),
                        ).size
                      } 个机构已配置药库/药房
                    </span>
                  </div>
                </div>
                <button
                  className="button button--secondary button--compact"
                  onClick={() => setInventoryTargetExpanded(true)}
                >
                  <PencilSimple size={16} />
                  修改目标库
                </button>
              </div>
            )}
            {inventoryBatchDetail && !inventoryMappingExpanded
              ? renderInventoryBatchScopeSummary()
              : renderInventoryMappingBoard()}
            {false && targetStorageCatalog && targetOrganizationCatalog && legacyInventoryCatalog && (
              <div className="inventory-mapping-list">
                <div className="inventory-mapping-list__heading">
                  <strong>第一步：老系统机构 → 新系统机构</strong>
                  <span>新系统机构来源：api/bbp.organization/findByTenantId</span>
                </div>
                {[
                  ...new Set(
                    inventoryReadiness.locations
                      .map((location) => location.organizationId)
                      .filter(Boolean),
                  ),
                ].map((sourceOrganizationId) => {
                  const sourceOrganization =
                    legacyInventoryCatalog.organizations.find(
                      (item) => item.id === sourceOrganizationId,
                    );
                  const organizationOptions =
                    targetOrganizationCatalog.organizations.map(
                      (organization) => ({
                        value: organization.id,
                        label: organization.name || organization.fullName,
                        description: `${organization.cd || "无编码"} · ${organization.orgTypeText || "未标注类型"}`,
                        meta: organization.parentText
                          ? `上级：${organization.parentText}`
                          : organization.fullName || "当前租户机构",
                        keywords: `${organization.name} ${organization.fullName} ${organization.cd} ${organization.parentText}`,
                      }),
                    );
                  return (
                    <div
                      className="inventory-mapping-row inventory-organization-row"
                      key={sourceOrganizationId}
                    >
                      <div>
                        <span className="inventory-location__kind">机构</span>
                        <strong>
                          {sourceOrganization?.name || "未命名老系统机构"}
                        </strong>
                        <small>
                          SYS_ORGANIZATION · {sourceOrganizationId}
                        </small>
                      </div>
                      <ArrowRight size={18} />
                      <SearchableSelect
                        ariaLabel={`选择${sourceOrganization?.name || sourceOrganizationId}对应的新系统机构`}
                        value={
                          inventoryOrganizationMappings[
                            sourceOrganizationId
                          ] || ""
                        }
                        onChange={(targetOrganizationId) => {
                          setInventoryOrganizationMappings((current) => ({
                            ...current,
                            [sourceOrganizationId]: targetOrganizationId,
                          }));
                          setInventoryLocationMappings((current) =>
                            Object.fromEntries(
                              Object.entries(current).filter(
                                ([sourceLocationKey, targetStorageId]) => {
                                  const sourceLocation =
                                    inventoryReadiness.locations.find(
                                      (item) =>
                                        item.sourceLocationKey ===
                                        sourceLocationKey,
                                    );
                                  const targetStorage =
                                    targetStorageCatalog.storages.find(
                                      (item) =>
                                        item.idSto === targetStorageId,
                                    );
                                  return (
                                    sourceLocation?.organizationId !==
                                      sourceOrganizationId ||
                                    targetStorage?.organizationId ===
                                      targetOrganizationId
                                  );
                                },
                              ),
                            ),
                          );
                          setInventoryBatchDetail(null);
                        }}
                        options={organizationOptions}
                        placeholder="选择对应的新系统机构"
                        searchPlaceholder="按机构名称、编码或上级机构过滤"
                      />
                    </div>
                  );
                })}
                <div className="inventory-mapping-list__heading">
                  <strong>第二步：老系统药库/药房 → 新系统库房</strong>
                  <span>仅显示已选新机构下、类型相符的有效仓储</span>
                </div>
                {inventoryReadiness.locations.map((location) => {
                  const expectedType =
                    location.sourceKind === "WAREHOUSE" ? "1" : "2";
                  const targetOrganizationId =
                    inventoryOrganizationMappings[
                      location.organizationId
                    ];
                  const sourceLocationOptions =
                    legacyInventoryCatalog.locations
                      .filter(
                        (item) =>
                          item.organizationId ===
                            location.organizationId &&
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
                  const options = targetStorageCatalog.storages
                    .filter(
                      (storage) =>
                        storage.storageType === expectedType &&
                        storage.organizationId === targetOrganizationId,
                    )
                    .map((storage) => ({
                      value: storage.idSto,
                      label: storage.name,
                      description: `${storage.storageTypeName} · 机构 ${storage.organizationId || "未提供"}`,
                      meta: storage.productTypes
                        ? `适用物品类型：${storage.productTypes}`
                        : "未限制物品类型",
                      keywords: `${storage.name} ${storage.idSto} ${storage.organizationId}`,
                    }));
                  return (
                    <div
                      className="inventory-mapping-row"
                      key={location.sourceLocationKey}
                    >
                      <div>
                        <span className="inventory-location__kind">
                          {location.sourceKind === "WAREHOUSE"
                            ? "药库"
                            : "药房"}
                        </span>
                        <strong>{location.sourceLocationName}</strong>
                        <small>
                          {location.sourceLocationKey} · {location.medicineCount} 种药品
                        </small>
                        {inventoryLocationNeedsSourceResolution(location) && (
                          <SearchableSelect
                            ariaLabel="指定待确认药库库存所属的老系统药库"
                            value={
                              inventoryResolvedLocations[
                                location.sourceLocationKey
                              ] || ""
                            }
                            onChange={(sourceLocationKey) => {
                              setInventoryResolvedLocations((current) => ({
                                ...current,
                                [location.sourceLocationKey]:
                                  sourceLocationKey,
                              }));
                              setInventoryBatchDetail(null);
                            }}
                            options={sourceLocationOptions}
                            placeholder="先指定实际老系统药库"
                            searchPlaceholder="按老系统药库名称或识别码过滤"
                          />
                        )}
                      </div>
                      <ArrowRight size={18} />
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
                        options={options}
                        placeholder={
                          !targetOrganizationId
                            ? "请先选择对应的新系统机构"
                            : options.length
                            ? "选择对应的新系统库房"
                            : `没有可用的新系统${location.sourceKind === "WAREHOUSE" ? "药库" : "药房"}`
                        }
                        searchPlaceholder="按库房名称、主键或机构过滤"
                      />
                    </div>
                  );
                })}
                <div className="inventory-mapping-actions">
                  <div>
                    <ShieldCheck size={19} weight="fill" />
                    <span>
                      将按“库房 + 药品商品 + 进销价格 + 批号 + 效期”检查目标库存，发现重复立即阻止。
                    </span>
                  </div>
                  <button
                    className="button button--primary"
                    disabled={busy === "inventory-prepare"}
                    onClick={preparePhis27Inventory}
                  >
                    <ListMagnifyingGlass size={18} />
                    {busy === "inventory-prepare"
                      ? "正在检查…"
                      : "保存映射并检查重复"}
                  </button>
                </div>
              </div>
            )}
            {inventoryBatchDetail && (
              <div className="inventory-preflight-result">
                <div className="inventory-preflight-result__heading">
                  <div>
                    {inventoryBatchDetail.batch.failCount ? (
                      <Warning size={21} weight="fill" />
                    ) : (
                      <CheckCircle size={21} weight="fill" />
                    )}
                    <div>
                      <strong>
                        {inventoryBatchDetail.batch.status === "UNDONE"
                          ? "首次盘点已安全撤销"
                          : inventoryBatchDetail.batch.status === "SUCCESS"
                          ? "首次盘点和初始账簿已建立"
                          : inventoryBatchDetail.batch.failCount
                            ? "发现需要处理的库存组"
                            : "库房映射和防重检查已通过"}
                      </strong>
                      <span>
                        {inventoryBatchDetail.batch.status === "UNDONE"
                          ? "本批首次盘点、初始库存和初始账簿日志已移除，来源台账已释放。"
                          : inventoryBatchDetail.batch.status === "SUCCESS"
                          ? "盘点主从表、库存和变动日志已在库房事务中一并完成。"
                          : "尚未写入目标库存；正式执行前还会再次检查库房是否为空。"}
                      </span>
                    </div>
                  </div>
                  <code>{inventoryBatchDetail.batch.batchId}</code>
                </div>
                {renderInventoryBatchReview()}
                {inventoryBatchDetail.batch.status === "UNDONE" ? (
                  <div className="inventory-write-gate inventory-write-gate--undone">
                    <ArrowCounterClockwise size={20} weight="fill" />
                    <div>
                      <strong>本批首次盘点已安全撤销</strong>
                      <span>
                        库房药品配置已保留；如需重新迁移，请重新读取库存并建立新批次。
                      </span>
                    </div>
                  </div>
                ) : inventoryBatchDetail.batch.status === "SUCCESS" ? (
                  <div className="inventory-write-gate">
                    <CheckCircle size={20} weight="fill" />
                    <div>
                      <strong>
                        已成功建立 {inventoryBatchDetail.batch.successCount} 组初始库存
                      </strong>
                      <span>
                        盘点单号及目标库存主键已写入本地迁移日志，可从历史迁移日志中追溯。
                      </span>
                    </div>
                    <button
                      className="button button--secondary button--compact inventory-undo-trigger"
                      type="button"
                      disabled={busy === "inventory-undo-preview" || busy === "inventory-undo"}
                      onClick={previewPhis27InventoryUndo}
                    >
                      <ArrowCounterClockwise size={17} />
                      {busy === "inventory-undo-preview"
                        ? "正在检查能否撤销…"
                        : "安全撤销首次盘点"}
                    </button>
                  </div>
                ) : inventoryBatchDetail.batch.failCount > 0 ? (
                  <div className="inventory-write-gate inventory-write-gate--blocked">
                    <Warning size={20} weight="fill" />
                    <div>
                      <strong>
                        还有 {inventoryBatchDetail.batch.failCount} 组库存不能迁移
                      </strong>
                      <span>
                        请在上方选择“异常”查看完整原因；本批不会写入任何目标库存。
                      </span>
                    </div>
                    {inventoryBatchDetail.batch.successCount > 0 && (
                      <button
                        className="button button--secondary button--compact inventory-undo-trigger"
                        type="button"
                        disabled={busy === "inventory-undo-preview" || busy === "inventory-undo"}
                        onClick={previewPhis27InventoryUndo}
                      >
                        <ArrowCounterClockwise size={17} />
                        检查已成功部分能否撤销
                      </button>
                    )}
                  </div>
                ) : (
                  <div className="inventory-write-gate inventory-write-gate--confirm">
                    <ShieldCheck size={20} weight="fill" />
                    <div>
                      <strong>
                        {inventoryTrialStatus.complete
                          ? `${inventoryBatchDetail.batch.validCount} 组库存可进入正式写入`
                          : `先完成目标库房试迁移（${inventoryTrialStatus.passedStorageIds.length}/${inventoryTrialStatus.requiredStorageIds.length}）`}
                      </strong>
                      <span>
                        {inventoryTrialStatus.complete
                          ? "每个库房均已用一条明细完整验证写入，测试事务已回滚；正式盘点单号按当天日期 + 3 位流水生成。"
                          : "请在上方明细中为每个目标库房任选一条数据执行试迁移；成功或失败都会自动回滚。目标库房已有盘点或库存，整库就会被阻止。"}
                      </span>
                      <label className="inventory-review-confirmation">
                        <input
                          type="checkbox"
                          checked={inventoryReviewConfirmed}
                          disabled={busy === "inventory-execute"}
                          onChange={(event) =>
                            setInventoryReviewConfirmed(event.target.checked)
                          }
                        />
                        <span>我已核对本批机构、库房、药品、批号、效期、数量和价格</span>
                      </label>
                      {busy === "inventory-execute" && (
                        <div
                          className="inventory-execution-progress"
                          aria-live="polite"
                          role="status"
                        >
                          <CircleNotch
                            className="is-spinning"
                            size={18}
                            weight="bold"
                          />
                          <div>
                            <strong>正在按库房建立首次盘点和初始账簿</strong>
                            <span>
                              已等待 {inventoryExecutionSeconds} 秒；执行期间请勿关闭应用或重复操作。
                            </span>
                          </div>
                        </div>
                      )}
                    </div>
                    <button
                      className="button button--danger"
                      aria-busy={busy === "inventory-execute"}
                      disabled={
                        busy === "inventory-execute" ||
                        busy === "inventory-trial" ||
                        !inventoryTrialStatus.complete
                      }
                      onClick={executePhis27Inventory}
                    >
                      {busy === "inventory-execute" ? (
                        <CircleNotch
                          className="is-spinning"
                          size={18}
                          weight="bold"
                        />
                      ) : (
                        <ShieldCheck size={18} weight="fill" />
                      )}
                      {busy === "inventory-execute"
                        ? `正在执行 · ${inventoryExecutionSeconds}秒`
                        : inventoryTrialStatus.complete
                          ? "正式执行首次盘点"
                          : `请先试迁移库房 ${inventoryTrialStatus.passedStorageIds.length}/${inventoryTrialStatus.requiredStorageIds.length}`}
                    </button>
                  </div>
                )}
                {renderInventoryUndoPanel()}
              </div>
            )}
          </div>
        )}
      </div>
    )}
    <div className="screen-actions">
      <button
        className="button button--secondary"
        onClick={() => setStep(1)}
      >
        <ArrowLeft />
        返回选择任务
      </button>
      {inventoryReadiness?.readyForLocationMapping && !targetStorageCatalog && (
        <div className="inventory-next-note">
          <CheckCircle size={18} weight="fill" />
          库存范围已读取，请继续连接目标库并选择本批机构；药品主键将在读取所选机构明细时核对。
        </div>
      )}
    </div>
  </section>
)}
    </>
  );
}
