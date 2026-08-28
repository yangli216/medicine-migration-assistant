import {
  lazy,
  Suspense,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  ArrowLeft,
  ArrowRight,
  ArrowCounterClockwise,
  Check,
  CheckCircle,
  CircleNotch,
  CaretDown,
  Clock,
  ClockCounterClockwise,
  Code,
  Database,
  FileCsv,
  FirstAidKit,
  FloppyDisk,
  Gear,
  HardDrives,
  Info,
  LinkSimple,
  ListMagnifyingGlass,
  LockKey,
  MagnifyingGlass,
  PencilSimple,
  Play,
  Plugs,
  Plus,
  Rows,
  ShieldCheck,
  Table,
  Trash,
  UploadSimple,
  UserCircle,
  Warning,
  X,
} from "@phosphor-icons/react";
import { command, demoRows, isDesktop, objectId } from "./api";
import { Header, Stepper } from "./AppChrome";
import {
  ConnectionForm,
  ConnectionManager,
  ConnectionPicker,
  Field,
  connectionEndpoint,
  connectionSupports,
  databaseKinds,
  initialProfile,
  newConnectionDraft,
  profilesMatch,
} from "./DatabaseConnections";
import { SearchableSelect } from "./SearchableSelect";
import {
  applyFieldMapping,
  EMPTY_VALUE_MAPPING_SOURCE,
  parseValueMappings,
} from "./transforms";
import { FieldAdvancedRuleEditor } from "./FieldAdvancedRuleEditor";
import {
  buildDictionaryValueMappings,
  buildPhis27PresetRules,
  dictionaryItemValue,
  findDictionaryItem,
  mergeValueMappingText,
  replaceValueMappingText,
  restoreCostMergeMappings,
} from "./dictionary";
import {
  defaultTransformForField,
  fieldsMentionedInValidationError,
  targetFields,
  targetLocations,
  targetPhysicalLocationText,
} from "./migrationFields";
import { SummaryCards, statusMeta } from "./MigrationStatus";
import { DataTable, ValidationResults } from "./MigrationResults";
import {
  completeInventoryOrganizationIds,
  inventoryLocationNeedsSourceResolution,
} from "./inventoryMapping";
import { inventoryUnmatchedMedicines } from "./inventoryMedicineMatching";
import {
  MedicineSourceScreen,
  SourceRecognitionScreen,
  SystemConnectionScreen,
  TaskSelectionScreen,
} from "./PrimaryFlowScreens";
import {
  firstPresentValue,
  formatInventoryMoney,
  inventoryFinancialTotals,
  medicinePreviewColumns,
  medicinePreviewLabels,
  medicineSampleContext,
  medicineSampleLabel,
  parseCsv,
  randomRowIndex,
  sourceDictionaryItem,
  sourceDictionaryPropertySummary,
  sourceFieldDisplayName,
  sourceFieldPhysicalOrigin,
  sourceValueLabel,
} from "./migrationPreview";
import {
  DictionaryMappingEditor,
  FieldMappingNavigator,
} from "./FieldMappingWorkspace";
import {
  buildFieldMappingStatuses,
  dictionaryRowsForField,
} from "./fieldMappingStatus";
import {
  applySourceDictionaryOverrides,
  sourceDictionaryScopeKey,
  updateScopedDictionaryOverride,
} from "./sourceDictionaryOverrides";
import {
  databaseSourceIdentity,
  legacySourceMappingPromotionRequest,
  sourceAdapterId,
  sourceMappingProfileForSelection,
  sourceMappingProfileScope,
} from "./sourceMappingProfiles";
import { sourceObjectFromSafeQuery } from "./sourceDatabaseObjects";
import { buildSourceObjectDiagnostic } from "./sourceObjectDiagnostics";
import { sourceFieldMatchScore } from "./sourceFieldMatching";
import {
  assessSourceKeyColumns,
  assessSourceKeyFields,
  chooseInitialSourceKeyFields,
  encodeSourceKey,
} from "./sourceKeyAssessment";
import {
  adapterProvidesSourceKey,
  medicineWorkflowFor,
  sourceKeyDisplayLabel,
  usesMappingPreset,
} from "./sourceAdapterWorkflow";
import {
  failedSourceObjectSurveyResult,
  rankSourceObjectSurveyResults,
  sourceObjectSurveyCandidates,
  sourceObjectSurveyResult,
} from "./sourceObjectSurvey";
import {
  hasSuccessfulTargetTrial,
  latestTargetTrialByRow,
} from "./trialMigration";
import { inventoryTrialCoverage } from "./inventoryTrialMigration";

const LazyInventoryMigrationScreen = lazy(() =>
  import("./InventoryMigrationScreen").then((module) => ({
    default: module.InventoryMigrationScreen,
  })),
);
const LazyMigrationHistory = lazy(() =>
  import("./MigrationHistory").then((module) => ({
    default: module.MigrationHistory,
  })),
);

function DeferredModuleLoading({ label, dialog = false }) {
  const content = (
    <div className="deferred-module-loading" role="status" aria-live="polite">
      <CircleNotch className="is-spinning" size={20} weight="bold" />
      <span>{label}</span>
    </div>
  );
  return dialog ? (
    <div className="connection-manager-backdrop deferred-module-backdrop">
      {content}
    </div>
  ) : (
    <section className="screen deferred-module-screen">{content}</section>
  );
}

function sourceAdapterIdentity(adapterName, profile, schema) {
  const service = profile.serviceName || profile.database || "Database";
  return `${adapterName || "三方 HIS"} · ${profile.host}:${profile.port}/${service} · ${schema}`;
}

const initialTargetSystemUrl = "http://10.17.18.88:8000/rbmh-phis/";

export function App() {
  const fileInput = useRef(null);
  const workspaceRef = useRef(null);
  const prepareBatchLock = useRef(false);
  const migrationExecutionLock = useRef(false);
  const trialMigrationLock = useRef(false);
  const inventoryExecutionLock = useRef(false);
  const inventoryTrialLock = useRef(false);
  const inventoryUndoLock = useRef(false);
  const noticeTimerRef = useRef(null);
  const costMergeMappingsRef = useRef({});
  const costMergeSaveQueueRef = useRef(Promise.resolve());
  const [runtime, setRuntime] = useState(
    isDesktop ? "tauri-rust" : "browser-preview",
  );
  const [step, setStep] = useState(0);
  const [targetSystemUrl, setTargetSystemUrl] = useState(
    initialTargetSystemUrl,
  );
  const [targetSystemProbe, setTargetSystemProbe] = useState(null);
  const [loginTenantId, setLoginTenantId] = useState("");
  const [loginPassword, setLoginPassword] = useState("");
  const [rememberTargetSystemPassword, setRememberTargetSystemPassword] =
    useState(true);
  const [hasSavedTargetSystem, setHasSavedTargetSystem] = useState(false);
  const [targetAuth, setTargetAuth] = useState(null);
  const [editingTargetConnection, setEditingTargetConnection] = useState(false);
  const [dictionaryCatalog, setDictionaryCatalog] = useState(null);
  const [costMergeCatalog, setCostMergeCatalog] = useState(null);
  const [costMergeMappings, setCostMergeMappings] = useState({});
  const [migrationType, setMigrationType] = useState("MEDICINE_BASE");
  const [sourceMode, setSourceMode] = useState("file");
  const [sourceProfile, setSourceProfile] = useState(initialProfile);
  const [rememberSourcePassword, setRememberSourcePassword] = useState(true);
  const [hasSavedSourceConnection, setHasSavedSourceConnection] =
    useState(false);
  const [sourceAdapterInspection, setSourceAdapterInspection] = useState(null);
  const [sourceAdapterDiagnostics, setSourceAdapterDiagnostics] = useState([]);
  const [selectedMedicineAdapterId, setSelectedMedicineAdapterId] =
    useState("PHIS27");
  const [activeSourceAdapterId, setActiveSourceAdapterId] =
    useState("GENERIC_FILE");
  const [activeSourceSelection, setActiveSourceSelection] = useState(null);
  const [legacyScope, setLegacyScope] = useState("USED_ACTIVE");
  const [inventoryReadiness, setInventoryReadiness] = useState(null);
  const [inventorySourceDiagnostics, setInventorySourceDiagnostics] =
    useState([]);
  const [inventorySourceExpanded, setInventorySourceExpanded] = useState(true);
  const [inventoryTargetExpanded, setInventoryTargetExpanded] = useState(true);
  const [inventoryPreflightExpanded, setInventoryPreflightExpanded] =
    useState(false);
  const [inventorySourceCatalog, setInventorySourceCatalog] = useState(null);
  const [selectedInventoryAdapterId, setSelectedInventoryAdapterId] =
    useState("PHIS27");
  const [targetOrganizationCatalog, setTargetOrganizationCatalog] =
    useState(null);
  const [inventoryOrganizationMappings, setInventoryOrganizationMappings] =
    useState({});
  const [
    inventorySelectedOrganizationIds,
    setInventorySelectedOrganizationIds,
  ] = useState([]);
  const [inventorySelectedLocationKeys, setInventorySelectedLocationKeys] =
    useState([]);
  const [inventoryActiveOrganizationId, setInventoryActiveOrganizationId] =
    useState("");
  const [inventoryLocationSearch, setInventoryLocationSearch] = useState("");
  const [targetStorageCatalog, setTargetStorageCatalog] = useState(null);
  const [inventoryLocationMappings, setInventoryLocationMappings] = useState(
    {},
  );
  const [inventoryResolvedLocations, setInventoryResolvedLocations] = useState(
    {},
  );
  const [inventoryBatchDetail, setInventoryBatchDetail] = useState(null);
  const [inventoryMedicineCatalog, setInventoryMedicineCatalog] =
    useState(null);
  const [inventoryMedicineSuggestions, setInventoryMedicineSuggestions] =
    useState([]);
  const [inventoryMedicineSelections, setInventoryMedicineSelections] =
    useState({});
  const [
    inventoryMedicineSelectedTargets,
    setInventoryMedicineSelectedTargets,
  ] = useState({});
  const [inventoryMedicineConfirmations, setInventoryMedicineConfirmations] =
    useState({});
  const [inventoryMedicineMatchOpen, setInventoryMedicineMatchOpen] =
    useState(false);
  const [inventoryMedicineMatchSearch, setInventoryMedicineMatchSearch] =
    useState("");
  const [inventoryMedicineMatchFilter, setInventoryMedicineMatchFilter] =
    useState("ALL");
  const [inventoryMedicineMatchPage, setInventoryMedicineMatchPage] =
    useState(1);
  const [inventoryTrialRowId, setInventoryTrialRowId] = useState("");
  const [inventoryMappingExpanded, setInventoryMappingExpanded] =
    useState(true);
  const [inventoryReviewSearch, setInventoryReviewSearch] = useState("");
  const [inventoryReviewStorage, setInventoryReviewStorage] = useState("");
  const [inventoryReviewStatus, setInventoryReviewStatus] = useState("ALL");
  const [inventoryReviewConfirmed, setInventoryReviewConfirmed] =
    useState(false);
  const [inventoryExceptionRow, setInventoryExceptionRow] = useState(null);
  const [inventoryExceptionReason, setInventoryExceptionReason] = useState("");
  const [inventoryExecutionSeconds, setInventoryExecutionSeconds] = useState(0);
  const [inventoryUndoPreview, setInventoryUndoPreview] = useState(null);
  const [inventoryUndoConfirmed, setInventoryUndoConfirmed] = useState(false);
  const [targetProfile, setTargetProfile] = useState(initialProfile);
  const [rememberTargetDatabasePassword, setRememberTargetDatabasePassword] =
    useState(true);
  const [hasSavedTargetDatabase, setHasSavedTargetDatabase] = useState(false);
  const [databaseConnections, setDatabaseConnections] = useState([]);
  const [connectionManagerOpen, setConnectionManagerOpen] = useState(false);
  const [migrationHistoryOpen, setMigrationHistoryOpen] = useState(false);
  const [historyBatches, setHistoryBatches] = useState([]);
  const [historyBatchDetail, setHistoryBatchDetail] = useState(null);
  const [selectedSourceConnectionId, setSelectedSourceConnectionId] =
    useState("");
  const [selectedTargetConnectionId, setSelectedTargetConnectionId] =
    useState("");
  const [sourceConnectionEditing, setSourceConnectionEditing] = useState(true);
  const [targetConnectionEditing, setTargetConnectionEditing] = useState(true);
  const [query, setQuery] = useState("");
  const [showCustomQuery, setShowCustomQuery] = useState(false);
  const [sourceObjects, setSourceObjects] = useState([]);
  const [selectedSourceObject, setSelectedSourceObject] = useState("");
  const [sourceObjectPreview, setSourceObjectPreview] = useState(null);
  const [sourceObjectCount, setSourceObjectCount] = useState(null);
  const [sourceObjectCountBusy, setSourceObjectCountBusy] = useState(false);
  const sourceObjectPreviewRequestRef = useRef(0);
  const [sourceObjectSurvey, setSourceObjectSurvey] = useState(null);
  const sourceObjectSurveyRequestRef = useRef(0);
  const [sourceName, setSourceName] = useState("尚未选择数据源");
  const [sourceDescription, setSourceDescription] = useState("");
  const [rows, setRows] = useState([]);
  const [columns, setColumns] = useState([]);
  const [columnMetadata, setColumnMetadata] = useState({});
  const [sourceDictionaryOverrides, setSourceDictionaryOverrides] = useState(
    {},
  );
  const [sourceKeyFields, setSourceKeyFields] = useState([]);
  const [sourceKeyConfirmed, setSourceKeyConfirmed] = useState(false);
  const [mapping, setMapping] = useState({});
  const [rules, setRules] = useState({});
  const [fieldIndex, setFieldIndex] = useState(0);
  const [mappingSampleIndex, setMappingSampleIndex] = useState(0);
  const [expert, setExpert] = useState(false);
  const [expertDictionaryFieldKey, setExpertDictionaryFieldKey] = useState("");
  const [allowCreateFactory, setAllowCreateFactory] = useState(false);
  const [conflictStrategy, setConflictStrategy] = useState("INCREMENTAL");
  const [batchDetail, setBatchDetail] = useState(null);
  const [overwritePreview, setOverwritePreview] = useState(null);
  const [selectedOverwriteRowIds, setSelectedOverwriteRowIds] = useState([]);
  const [tenantId, setTenantId] = useState("");
  const [operatorId, setOperatorId] = useState("");
  const [activeResultTab, setActiveResultTab] = useState("rows");
  const [trialRowId, setTrialRowId] = useState("");
  const [validationResultFilter, setValidationResultFilter] =
    useState("INVALID");
  const [skipInvalidRowsOnExecute, setSkipInvalidRowsOnExecute] =
    useState(true);
  const [validationFieldContext, setValidationFieldContext] = useState(null);
  const [busy, setBusy] = useState("");
  const [prepareElapsedSeconds, setPrepareElapsedSeconds] = useState(0);
  const [notice, setNotice] = useState(null);
  const [databaseDrivers, setDatabaseDrivers] = useState([]);
  const [driverPacks, setDriverPacks] = useState([]);
  const [sourceAdapters, setSourceAdapters] = useState([]);
  const activeSourceAdapter = sourceAdapters.find(
    (adapter) => adapter.id === activeSourceAdapterId,
  );
  const activeMedicineWorkflow = medicineWorkflowFor(activeSourceAdapter);
  const activeAdapterProvidesSourceKey =
    adapterProvidesSourceKey(activeSourceAdapter);
  const activeUsesPhis27Preset = usesMappingPreset(
    activeSourceAdapter,
    "PHIS27",
  );
  const sourceKey = sourceKeyDisplayLabel(activeSourceAdapter, sourceKeyFields);
  const [mappingProfileStatus, setMappingProfileStatus] = useState(null);
  const [sourceTemplateImport, setSourceTemplateImport] = useState(null);

  useLayoutEffect(() => {
    window.scrollTo({ top: 0, left: 0, behavior: "auto" });
    workspaceRef.current?.scrollTo?.({ top: 0, left: 0, behavior: "auto" });
  }, [step]);

  useEffect(
    () => () => {
      if (noticeTimerRef.current) {
        window.clearTimeout(noticeTimerRef.current);
      }
    },
    [],
  );

  useEffect(() => {
    if (busy !== "prepare") {
      setPrepareElapsedSeconds(0);
      return undefined;
    }
    const startedAt = Date.now();
    const updateElapsed = () =>
      setPrepareElapsedSeconds(Math.floor((Date.now() - startedAt) / 1000));
    updateElapsed();
    const timer = window.setInterval(updateElapsed, 1000);
    return () => window.clearInterval(timer);
  }, [busy]);

  useEffect(() => {
    const compatible = sourceAdapters.filter(
      (adapter) =>
        adapter.automaticDetection &&
        adapter.migrationTasks.includes("MEDICINE_BASE") &&
        adapter.databaseFamilies.includes(sourceProfile.kind),
    );
    if (compatible.some((adapter) => adapter.id === selectedMedicineAdapterId))
      return;
    setSelectedMedicineAdapterId(compatible[0]?.id || "");
    setSourceAdapterInspection(null);
  }, [selectedMedicineAdapterId, sourceAdapters, sourceProfile.kind]);

  useEffect(() => {
    command("app_health")
      .then((health) => setRuntime(health.runtime))
      .catch(() => {});
    command("list_database_drivers")
      .then(setDatabaseDrivers)
      .catch(() => setDatabaseDrivers([]));
    command("list_driver_packs")
      .then(setDriverPacks)
      .catch(() => setDriverPacks([]));
    command("list_source_adapters")
      .then(setSourceAdapters)
      .catch(() => setSourceAdapters([]));
    Promise.all([
      command("load_saved_connections"),
      command("list_database_connections"),
    ])
      .then(([saved, connections]) => {
        setDatabaseConnections(connections || []);
        if (saved?.source?.profile) {
          setSourceProfile(saved.source.profile);
          setRememberSourcePassword(saved.source.rememberPassword);
          setHasSavedSourceConnection(true);
          setSourceMode("database");
          const selected = (connections || []).find(
            (entry) =>
              connectionSupports(entry, "SOURCE") &&
              profilesMatch(entry.profile, saved.source.profile),
          );
          if (selected) {
            setSelectedSourceConnectionId(selected.connectionId);
            setSourceConnectionEditing(false);
          }
        }
        if (saved?.targetDatabase?.profile) {
          setTargetProfile(saved.targetDatabase.profile);
          setRememberTargetDatabasePassword(
            saved.targetDatabase.rememberPassword,
          );
          setHasSavedTargetDatabase(true);
          const selected = (connections || []).find(
            (entry) =>
              connectionSupports(entry, "TARGET") &&
              profilesMatch(entry.profile, saved.targetDatabase.profile),
          );
          if (selected) {
            setSelectedTargetConnectionId(selected.connectionId);
            setTargetConnectionEditing(false);
          }
        }
        if (saved?.targetSystem) {
          setTargetSystemUrl(
            saved.targetSystem.baseUrl || initialTargetSystemUrl,
          );
          setLoginTenantId(saved.targetSystem.tenantId || "");
          setLoginPassword(saved.targetSystem.password || "");
          setRememberTargetSystemPassword(saved.targetSystem.rememberPassword);
          setHasSavedTargetSystem(true);
        }
      })
      .catch(() => {});
  }, []);

  useEffect(() => {
    if (!["inventory-execute", "inventory-undo"].includes(busy)) {
      setInventoryExecutionSeconds(0);
      return undefined;
    }
    const startedAt = Date.now();
    setInventoryExecutionSeconds(0);
    const timer = window.setInterval(() => {
      setInventoryExecutionSeconds(Math.floor((Date.now() - startedAt) / 1000));
    }, 1000);
    return () => window.clearInterval(timer);
  }, [busy]);

  const currentField = targetFields[fieldIndex];
  const dictionariesById = useMemo(
    () =>
      Object.fromEntries(
        (dictionaryCatalog?.dictionaries || []).map((dictionary) => [
          dictionary.dicId,
          dictionary,
        ]),
      ),
    [dictionaryCatalog],
  );
  const currentDictionary = currentField?.dictionaryId
    ? dictionariesById[currentField.dictionaryId]
    : null;
  const currentSourceField = currentField ? mapping[currentField.key] : "";
  const currentSourceDictionary = currentSourceField
    ? columnMetadata[currentSourceField]?.sourceDictionary || null
    : null;
  const currentDictionaryScopeKey = useMemo(
    () =>
      sourceDictionaryScopeKey(
        sourceProfile,
        sourceAdapterInspection?.schema || sourceProfile.schema,
      ),
    [sourceAdapterInspection?.schema, sourceProfile],
  );
  const currentDictionaryScopeLabel = sourceAdapterIdentity(
    sourceAdapters.find((adapter) => adapter.id === activeSourceAdapterId)
      ?.name,
    sourceProfile,
    sourceAdapterInspection?.schema || sourceProfile.schema || "当前 Schema",
  );
  const articleTypeDictionary =
    dictionariesById["rbmh.base.med.articleType"] || null;
  const mappedCount = targetFields.filter(
    (field) => mapping[field.key] || rules[field.key]?.defaultValue,
  ).length;
  const eligibleColumns = useMemo(
    () =>
      columns.filter(
        (column) => columnMetadata[column]?.mappingEligible !== false,
      ),
    [columns, columnMetadata],
  );
  const mappingSample = rows[mappingSampleIndex] || rows[0] || {};
  const sampleContext = useMemo(
    () => medicineSampleContext(mappingSample, columnMetadata),
    [mappingSample, columnMetadata],
  );
  const sampleRowOptions = useMemo(
    () =>
      rows.map((row, index) => ({
        value: `${index}`,
        label: medicineSampleLabel(row, index),
        keywords: Object.values(row)
          .filter((value) => value !== null && value !== undefined)
          .slice(0, 12)
          .join(" "),
      })),
    [rows],
  );
  const sourceFieldOptions = useMemo(
    () =>
      eligibleColumns.map((column) => {
        const metadata = columnMetadata[column] || {};
        const origin = sourceFieldPhysicalOrigin(metadata);
        const sampleValue = mappingSample[column];
        const sourceDictionary = metadata.sourceDictionary;
        return {
          value: column,
          label: sourceFieldDisplayName(column, metadata),
          description: [
            origin ? `来源：${origin}` : "",
            sourceDictionary?.name
              ? `二系列字典：${sourceDictionary.name}`
              : "",
          ]
            .filter(Boolean)
            .join(" · "),
          meta: `当前样例：${sourceValueLabel(sampleValue, metadata)}`,
          keywords: `${metadata.comment || ""} ${origin} ${sourceDictionary?.name || ""} ${sourceValueLabel(sampleValue, metadata)}`,
        };
      }),
    [eligibleColumns, columnMetadata, mappingSample],
  );
  const suggestions = useMemo(() => {
    if (!currentField) return [];
    return [...eligibleColumns]
      .map((column) => ({
        column,
        score: sourceFieldMatchScore(
          column,
          currentField,
          columnMetadata[column],
        ),
      }))
      .filter((candidate) => candidate.score >= 70)
      .sort((a, b) => b.score - a.score)
      .slice(0, 3);
  }, [columnMetadata, eligibleColumns, currentField]);
  const currentMappingPreview = useMemo(() => {
    if (!currentField) return null;
    const sourceField = mapping[currentField.key];
    if (!sourceField) return null;
    const rule = rules[currentField.key] || {};
    const additionalSourceFields = rule.additionalSourceFields || [];
    const sourceValues = [sourceField, ...additionalSourceFields]
      .map((field) => mappingSample[field])
      .filter(
        (value) =>
          value !== null && value !== undefined && `${value}`.trim() !== "",
      );
    const original = additionalSourceFields.length
      ? sourceValues.join(`${rule.joinSeparator ?? ""}`)
      : mappingSample[sourceField];
    const parsedMappings = parseValueMappings(rule.valueMappingsText).mappings;
    const mappingLookup =
      original === null || original === undefined || `${original}`.trim() === ""
        ? EMPTY_VALUE_MAPPING_SOURCE
        : `${original}`.trim();
    const ignored =
      Object.prototype.hasOwnProperty.call(parsedMappings, mappingLookup) &&
      parsedMappings[mappingLookup] === null;
    const converted = applyFieldMapping(mappingSample, {
      ...rule,
      sourceField,
      transform: rule.transform || defaultTransformForField(currentField),
      valueMappings: parsedMappings,
    });
    const dictionaryItem = currentDictionary?.items.find(
      (item) => `${dictionaryItemValue(item)}` === `${converted ?? ""}`,
    );
    return {
      original,
      sourceDictionaryText:
        sourceDictionaryItem(columnMetadata[sourceField], original)?.text || "",
      sourceDictionaryName:
        columnMetadata[sourceField]?.sourceDictionary?.name || "",
      ignored,
      converted,
      dictionaryCode: dictionaryItem ? dictionaryItemValue(dictionaryItem) : "",
      dictionaryText: dictionaryItem?.text || dictionaryItem?.na || "",
    };
  }, [
    columnMetadata,
    currentField,
    currentDictionary,
    mapping,
    mappingSample,
    rules,
  ]);
  const currentDictionaryRows = useMemo(() => {
    if (!currentField) return [];
    return dictionaryRowsForField({
      columnMetadata,
      dictionary: currentDictionary,
      field: currentField,
      findDictionaryItem,
      mapping,
      rows,
      rules,
      sourceDictionaryItem,
      sourceDictionaryPropertySummary,
    });
  }, [columnMetadata, currentDictionary, currentField, mapping, rows, rules]);
  const mappingStatuses = useMemo(
    () =>
      buildFieldMappingStatuses({
        columnMetadata,
        dictionariesById,
        fields: targetFields,
        findDictionaryItem,
        mapping,
        rows,
        rules,
        sourceDictionaryItem,
        sourceDictionaryPropertySummary,
      }),
    [columnMetadata, dictionariesById, mapping, rows, rules],
  );
  const unresolvedDictionaryStatuses = mappingStatuses.filter(
    (status) => status.configured && status.dictionaryPending > 0,
  );
  const unresolvedDictionaryValueCount = unresolvedDictionaryStatuses.reduce(
    (total, status) => total + status.dictionaryPending,
    0,
  );
  const validationFailureCount =
    batchDetail?.rows?.filter((row) => row.status === "INVALID").length || 0;
  const trialResultByRow = useMemo(
    () => latestTargetTrialByRow(batchDetail?.audits || [], targetProfile),
    [batchDetail?.audits, targetProfile],
  );
  const targetTrialPassed = useMemo(
    () => hasSuccessfulTargetTrial(batchDetail?.audits || [], targetProfile),
    [batchDetail?.audits, targetProfile],
  );
  const formalMigrationNeedsTrial =
    Boolean(batchDetail) &&
    batchDetail.batch.successCount === 0 &&
    !targetTrialPassed;
  const inventoryTrialStatus = useMemo(
    () => inventoryTrialCoverage(inventoryBatchDetail, targetProfile),
    [inventoryBatchDetail, targetProfile],
  );

  const dismissNotice = () => {
    if (noticeTimerRef.current) {
      window.clearTimeout(noticeTimerRef.current);
      noticeTimerRef.current = null;
    }
    setNotice(null);
  };

  const notify = (message, tone = "success") => {
    if (noticeTimerRef.current) {
      window.clearTimeout(noticeTimerRef.current);
    }
    setNotice({ message, tone });
    const duration = tone === "danger" ? 15_000 : 4_000;
    noticeTimerRef.current = window.setTimeout(() => {
      noticeTimerRef.current = null;
      setNotice(null);
    }, duration);
  };
  const fail = (error) =>
    notify(
      typeof error === "string" ? error : error?.message || `${error}`,
      "danger",
    );

  function invalidateSourceObjectPreview() {
    sourceObjectPreviewRequestRef.current += 1;
    setSourceObjectPreview(null);
    setSourceObjectCount(null);
    setSourceObjectCountBusy(false);
  }

  function invalidateSourceObjectSurvey() {
    sourceObjectSurveyRequestRef.current += 1;
    setSourceObjectSurvey(null);
  }

  function updateDatabaseConnectionList(saved) {
    setDatabaseConnections((current) => [
      saved,
      ...current.filter((entry) => entry.connectionId !== saved.connectionId),
    ]);
  }

  async function saveManagedDatabaseConnection(draft) {
    setBusy("connection-manager-save");
    try {
      const saved = await command("save_database_connection", {
        request: {
          connectionId: draft.connectionId || "",
          name: draft.name,
          purpose: draft.purpose,
          profile: draft.profile,
          rememberPassword: draft.rememberPassword,
        },
      });
      updateDatabaseConnectionList(saved);
      if (selectedSourceConnectionId === saved.connectionId) {
        setSourceProfile({ ...saved.profile });
        setRememberSourcePassword(saved.rememberPassword);
        setSourceObjects([]);
        setSelectedSourceObject("");
        invalidateSourceObjectPreview();
        invalidateSourceObjectSurvey();
        setQuery("");
        restoreGenericDatabaseQuery(saved.profile).catch(() => {});
      }
      if (selectedTargetConnectionId === saved.connectionId) {
        setTargetProfile({ ...saved.profile });
        setRememberTargetDatabasePassword(saved.rememberPassword);
      }
      notify(`已保存连接“${saved.name}”`);
      return saved;
    } catch (error) {
      fail(error);
      return null;
    } finally {
      setBusy("");
    }
  }

  async function deleteManagedDatabaseConnection(connectionId) {
    setBusy("connection-manager-delete");
    try {
      await command("delete_database_connection", { connectionId });
      setDatabaseConnections((current) =>
        current.filter((entry) => entry.connectionId !== connectionId),
      );
      if (selectedSourceConnectionId === connectionId) {
        setSelectedSourceConnectionId("");
        setSourceConnectionEditing(true);
      }
      if (selectedTargetConnectionId === connectionId) {
        setSelectedTargetConnectionId("");
        setTargetConnectionEditing(true);
      }
      notify("连接已从本地连接库删除");
      return true;
    } catch (error) {
      fail(error);
      return false;
    } finally {
      setBusy("");
    }
  }

  async function testManagedDatabaseConnection(profile) {
    setBusy("connection-manager-test");
    try {
      const result = await command("test_database_connection", { profile });
      notify(result.message);
      return result;
    } catch (error) {
      fail(error);
      return null;
    } finally {
      setBusy("");
    }
  }

  function useDatabaseConnection(entry, purpose, closeManager = true) {
    if (!entry) return;
    if (purpose === "SOURCE") {
      setSourceProfile({ ...entry.profile });
      setRememberSourcePassword(entry.rememberPassword);
      setHasSavedSourceConnection(true);
      setSelectedSourceConnectionId(entry.connectionId);
      setSourceConnectionEditing(false);
      setSourceMode("database");
      setSourceAdapterInspection(null);
      setSourceObjects([]);
      setSelectedSourceObject("");
      invalidateSourceObjectPreview();
      invalidateSourceObjectSurvey();
      setQuery("");
      command("save_source_connection", {
        request: {
          profile: entry.profile,
          rememberPassword: entry.rememberPassword,
        },
      }).catch(fail);
      notify(`老库读取已切换为“${entry.name}”`);
      restoreGenericDatabaseQuery(entry.profile).catch(() => {});
    } else {
      setTargetProfile({ ...entry.profile });
      setRememberTargetDatabasePassword(entry.rememberPassword);
      setHasSavedTargetDatabase(true);
      setSelectedTargetConnectionId(entry.connectionId);
      setTargetConnectionEditing(false);
      setOverwritePreview(null);
      setSelectedOverwriteRowIds([]);
      command("save_target_database_connection", {
        request: {
          profile: entry.profile,
          rememberPassword: entry.rememberPassword,
        },
      }).catch(fail);
      notify(`目标库写入已切换为“${entry.name}”`);
    }
    if (closeManager) setConnectionManagerOpen(false);
  }

  async function restoreGenericDatabaseQuery(
    profile,
    targetTenantId = targetAuth?.tenantId || tenantId || loginTenantId,
  ) {
    const scope = sourceMappingProfileScope({
      adapterId: sourceAdapterId({ sourceMode: "database" }),
      profile,
      sourceMode: "database",
      targetTenantId,
    });
    if (!scope) return null;
    const saved = await command("load_source_mapping_profile", {
      request: {
        scope,
        adapterVersion: sourceAdapterVersionFor(scope.adapterId),
        templateCompatibleFromVersion: sourceAdapterCompatibleFromFor(
          scope.adapterId,
        ),
      },
    });
    if (saved?.sourceQuery) {
      setQuery(saved.sourceQuery);
      setSelectedSourceObject(sourceObjectFromSafeQuery(saved.sourceQuery));
    }
    return saved;
  }

  function selectDatabaseConnection(connectionId, purpose) {
    const entry = databaseConnections.find(
      (item) => item.connectionId === connectionId,
    );
    useDatabaseConnection(entry, purpose, false);
  }

  async function rememberSourceConnection() {
    const saved = await command("save_source_connection", {
      request: {
        profile: sourceProfile,
        rememberPassword: rememberSourcePassword,
      },
    });
    setHasSavedSourceConnection(true);
    setRememberSourcePassword(saved.rememberPassword);
    return saved;
  }

  async function rememberTargetDatabaseConnection() {
    const saved = await command("save_target_database_connection", {
      request: {
        profile: targetProfile,
        rememberPassword: rememberTargetDatabasePassword,
      },
    });
    setHasSavedTargetDatabase(true);
    setRememberTargetDatabasePassword(saved.rememberPassword);
    return saved;
  }

  async function loadHistoryBatch(batchId) {
    setBusy("history-detail");
    try {
      const detail = await command("load_migration_batch", { batchId });
      setHistoryBatchDetail(detail);
      return detail;
    } catch (error) {
      fail(error);
      return null;
    } finally {
      setBusy("");
    }
  }

  async function openMigrationHistory() {
    setMigrationHistoryOpen(true);
    setBusy("history-list");
    try {
      const batches = await command("list_recent_batches", { limit: 100 });
      setHistoryBatches(batches);
      if (!batches.length) {
        setHistoryBatchDetail(null);
        return;
      }
      const selectedId = historyBatchDetail?.batch?.batchId;
      const nextBatch =
        batches.find((batch) => batch.batchId === selectedId) || batches[0];
      const detail = await command("load_migration_batch", {
        batchId: nextBatch.batchId,
      });
      setHistoryBatchDetail(detail);
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  async function forgetSourceConnection() {
    await command("forget_source_connection");
    setSourceProfile({ ...initialProfile });
    setRememberSourcePassword(true);
    setHasSavedSourceConnection(false);
    setSourceAdapterInspection(null);
    setSourceObjects([]);
    setSelectedSourceObject("");
    invalidateSourceObjectPreview();
    invalidateSourceObjectSurvey();
    setQuery("");
    notify("已删除保存的老库连接和本地加密密码");
  }

  async function forgetTargetDatabaseConnection() {
    await command("forget_target_database_connection");
    setTargetProfile({ ...initialProfile });
    setRememberTargetDatabasePassword(true);
    setHasSavedTargetDatabase(false);
    notify("已删除保存的新系统数据库连接和本地加密密码");
  }

  async function forgetTargetSystemConnection() {
    await command("forget_target_system_connection");
    setTargetSystemUrl(initialTargetSystemUrl);
    setLoginTenantId("");
    setLoginPassword("");
    setRememberTargetSystemPassword(true);
    setHasSavedTargetSystem(false);
    setTargetSystemProbe(null);
    setTargetAuth(null);
    setEditingTargetConnection(false);
    notify("已删除保存的新系统登录信息和本地加密密码");
  }

  async function probeTargetSystem() {
    setBusy("target-system-probe");
    setTargetSystemProbe(null);
    try {
      const result = await command("probe_target_system", {
        request: { baseUrl: targetSystemUrl },
      });
      setTargetSystemUrl(result.baseUrl);
      setTargetSystemProbe(result);
      notify(`${result.message} · ${result.latencyMs}ms`);
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  async function loginTargetSystem(event) {
    event?.preventDefault();
    const loginBaseUrl = targetSystemProbe?.baseUrl || targetSystemUrl;
    setBusy("target-system-login");
    try {
      const result = await command("login_target_system", {
        request: {
          baseUrl: loginBaseUrl,
          tenantId: loginTenantId,
          loginName: "system",
          password: loginPassword,
        },
      });
      const [catalog, costMerges, persistedCostMerges] = await Promise.all([
        command("load_target_dictionaries"),
        command("load_medicine_cost_merges"),
        command("load_cost_merge_mapping_profile", {
          request: {
            baseUrl: result.baseUrl,
            tenantId: result.tenantId,
          },
        })
          .then((profile) => ({ profile, error: "" }))
          .catch((error) => ({
            profile: null,
            error: error?.message || `${error}`,
          })),
      ]);
      const articleTypes = catalog.dictionaries.find(
        (dictionary) => dictionary.dicId === "rbmh.base.med.articleType",
      );
      const restoredCostMerges = restoreCostMergeMappings(
        articleTypes?.items || [],
        costMerges.items,
        persistedCostMerges.profile?.mappings,
      );
      setDictionaryCatalog(catalog);
      setCostMergeCatalog(costMerges);
      applyCostMergeMappings(restoredCostMerges.mappings);
      setTargetAuth(result);
      setEditingTargetConnection(false);
      setTargetSystemUrl(result.baseUrl);
      setTenantId(result.tenantId);
      setOperatorId(result.userId);
      if (sourceMode === "database") {
        await restoreGenericDatabaseQuery(sourceProfile, result.tenantId).catch(
          () => null,
        );
      }
      const saved = await command("save_target_system_connection", {
        request: {
          baseUrl: result.baseUrl,
          tenantId: loginTenantId,
          password: loginPassword,
          rememberPassword: rememberTargetSystemPassword,
        },
      });
      setHasSavedTargetSystem(true);
      setRememberTargetSystemPassword(saved.rememberPassword);
      setLoginPassword("");
      const restoredMessage = restoredCostMerges.restoredCount
        ? `；已恢复 ${restoredCostMerges.restoredCount - restoredCostMerges.staleCount} 项费用归并设置`
        : "";
      const staleMessage = restoredCostMerges.staleCount
        ? `；${restoredCostMerges.staleCount} 项原费用归并已失效，已标记为未设置，请重新确认`
        : "";
      const persistenceWarning = persistedCostMerges.error
        ? `；费用归并历史设置读取失败：${persistedCostMerges.error}`
        : "";
      notify(
        `${result.message}；${catalog.message}；${costMerges.message}${restoredMessage}${staleMessage}${persistenceWarning}${catalog.warnings?.length ? `（${catalog.warnings.length} 个可选字典暂不可用）` : ""}`,
        restoredCostMerges.staleCount || persistedCostMerges.error
          ? "danger"
          : "success",
      );
    } catch (error) {
      if (!targetAuth) {
        setDictionaryCatalog(null);
        setCostMergeCatalog(null);
        applyCostMergeMappings({});
      }
      fail(error);
    } finally {
      setBusy("");
    }
  }

  function applyCostMergeMappings(nextMappings) {
    costMergeMappingsRef.current = nextMappings;
    setCostMergeMappings(nextMappings);
  }

  function updateCostMergeMapping(articleKey, costMergeId) {
    const nextMappings = {
      ...costMergeMappingsRef.current,
      [articleKey]: costMergeId,
    };
    applyCostMergeMappings(nextMappings);
    if (!targetAuth) return;
    const request = {
      baseUrl: targetAuth.baseUrl,
      tenantId: targetAuth.tenantId,
      mappings: nextMappings,
    };
    costMergeSaveQueueRef.current = costMergeSaveQueueRef.current
      .catch(() => {})
      .then(() => command("save_cost_merge_mapping_profile", { request }))
      .then(
        () => notify("费用归并映射已按当前新系统租户自动保存到本机"),
        (error) => fail(`费用归并映射保存失败：${error?.message || error}`),
      );
  }

  function acceptData(data, name, options = {}) {
    if (!data.length) return fail("文件中没有可识别的数据行");
    const acceptedAdapterId =
      options.adapterId ||
      sourceAdapterId({
        phis27: Boolean(options.phis27Preset),
        sourceMode,
      });
    const acceptedAdapter = sourceAdapters.find(
      (adapter) => adapter.id === acceptedAdapterId,
    );
    const acceptedWorkflow = medicineWorkflowFor(acceptedAdapter);
    const acceptedMappingPreset = `${
      options.mappingPreset ||
      (options.phis27Preset ? "PHIS27" : acceptedWorkflow.mappingPreset)
    }`.toUpperCase();
    const acceptedAdapterProvidesSourceKey =
      adapterProvidesSourceKey(acceptedAdapter);
    setSourceTemplateImport(null);
    const detectedColumns = Object.keys(data[0]);
    const metadataByName = Object.fromEntries(
      (options.columnMetadata || []).map((item) => [item.name, item]),
    );
    setRows(data);
    setColumns(detectedColumns);
    setColumnMetadata(metadataByName);
    setMappingSampleIndex(randomRowIndex(data.length));
    setSourceName(name);
    setSourceDescription(options.description || name);
    setActiveSourceAdapterId(acceptedAdapterId);
    setActiveSourceSelection(options.sourceSelection || null);
    const sourceKeyReview = assessSourceKeyColumns({
      rows: data,
      columns: detectedColumns,
      columnMetadata: metadataByName,
    });
    setSourceKeyFields(
      chooseInitialSourceKeyFields({
        rows: data,
        review: sourceKeyReview,
        preferredFields:
          options.sourceKeyFields ||
          options.mappingProfile?.sourceKeyFields ||
          (acceptedAdapterProvidesSourceKey
            ? [acceptedWorkflow.sourceKeyField]
            : []),
        legacySourceKey:
          options.sourceKey || options.mappingProfile?.sourceKey || "",
      }),
    );
    setSourceKeyConfirmed(acceptedAdapterProvidesSourceKey);
    const mappingColumns = detectedColumns.filter(
      (column) => metadataByName[column]?.mappingEligible !== false,
    );
    const restoredMapping = options.mappingProfile?.mapping || null;
    const auto = {};
    let restoredCount = 0;
    let missingCount = 0;
    targetFields.forEach((field) => {
      if (
        restoredMapping &&
        Object.prototype.hasOwnProperty.call(restoredMapping, field.key)
      ) {
        const savedSource = restoredMapping[field.key] || "";
        if (!savedSource || mappingColumns.includes(savedSource)) {
          auto[field.key] = savedSource;
          if (savedSource) restoredCount += 1;
          return;
        }
        missingCount += 1;
      }
      const best = mappingColumns
        .map((column) => ({
          column,
          score: sourceFieldMatchScore(column, field, metadataByName[column]),
        }))
        .sort((a, b) => b.score - a.score)[0];
      if (best?.score >= 55) auto[field.key] = best.column;
    });
    if (
      acceptedMappingPreset === "PHIS27" &&
      mappingColumns.includes("PRE_UNIT") &&
      metadataByName.PRE_UNIT?.sourceColumn === "ZXDW"
    ) {
      auto.unitPre = "PRE_UNIT";
    }
    const targetKeys = new Set(targetFields.map((field) => field.key));
    const restoredRules = Object.fromEntries(
      Object.entries(options.mappingProfile?.rules || {})
        .filter(
          ([target, rule]) =>
            targetKeys.has(target) && rule && typeof rule === "object",
        )
        .map(([target, rule]) => {
          const conditionField = mappingColumns.includes(rule.conditionField)
            ? rule.conditionField
            : "";
          return [
            target,
            {
              ...rule,
              additionalSourceFields: (
                rule.additionalSourceFields || []
              ).filter((field) => mappingColumns.includes(field)),
              conditionField,
              conditionOperator: conditionField
                ? rule.conditionOperator || "ALWAYS"
                : "ALWAYS",
            },
          ];
        }),
    );
    const effectiveRules =
      acceptedMappingPreset === "PHIS27"
        ? buildPhis27PresetRules({
            rows: data,
            mapping: auto,
            columnMetadata: metadataByName,
            targetFields,
            dictionariesById,
            rules: restoredRules,
          })
        : restoredRules;
    setMapping(auto);
    setRules(effectiveRules);
    setMappingProfileStatus(
      options.mappingProfile
        ? {
            restored: true,
            restoredCount,
            missingCount,
            savedAt: options.mappingProfile.savedAt,
            compatibility: options.mappingProfile.compatibility || "CURRENT",
            compatibilityMessage:
              options.mappingProfile.compatibilityMessage || "",
            requiresReview: Boolean(options.mappingProfile.requiresReview),
            storageWarning: options.mappingProfile.storageWarning || "",
          }
        : null,
    );
    setStep(3);
    if (!options.skipNotice)
      notify(`已读取 ${data.length} 行、${detectedColumns.length} 个字段`);
    return { restoredCount, missingCount };
  }

  async function loadFile(file) {
    try {
      const text = await file.text();
      const data = file.name.toLowerCase().endsWith(".json")
        ? JSON.parse(text)
        : parseCsv(text);
      const scope = sourceMappingProfileScope({
        adapterId: sourceAdapterId({ sourceMode: "file" }),
        fileName: file.name,
        sourceMode: "file",
        targetTenantId: targetAuth?.tenantId || tenantId || loginTenantId,
      });
      const savedMappingProfile = scope
        ? await command("load_source_mapping_profile", {
            request: {
              scope,
              adapterVersion: sourceAdapterVersionFor(scope.adapterId),
              templateCompatibleFromVersion: sourceAdapterCompatibleFromFor(
                scope.adapterId,
              ),
            },
          })
        : null;
      const accepted = acceptData(
        Array.isArray(data) ? data : data.rows || [],
        file.name,
        { mappingProfile: savedMappingProfile },
      );
      if (savedMappingProfile) {
        notify(
          `已读取文件并恢复 ${accepted.restoredCount} 项来源模板配置${accepted.missingCount ? `；${accepted.missingCount} 项旧字段已失效并重新建议` : ""}`,
        );
      }
    } catch (error) {
      fail(`文件解析失败：${error.message}`);
    }
  }

  async function loadSourceObjects() {
    invalidateSourceObjectPreview();
    invalidateSourceObjectSurvey();
    setBusy("source-objects");
    try {
      const [checked, objects] = await Promise.all([
        command("test_database_connection", { profile: sourceProfile }),
        command("list_source_tables", { profile: sourceProfile }),
      ]);
      const available = [...new Set(objects || [])];
      setSourceObjects(available);
      setSelectedSourceObject((current) =>
        available.includes(current)
          ? current
          : available.length === 1
            ? available[0]
            : "",
      );
      await rememberSourceConnection();
      if (!available.length)
        return fail("当前账号没有可见的业务表或视图，请检查 Schema 和只读权限");
      notify(`${checked.message}；已读取 ${available.length} 个可见表/视图`);
      return available;
    } catch (error) {
      setSourceObjects([]);
      setSelectedSourceObject("");
      invalidateSourceObjectPreview();
      invalidateSourceObjectSurvey();
      fail(error);
      return [];
    } finally {
      setBusy("");
    }
  }

  async function loadGenericDatabase({ objectName = "", customQuery = "" }) {
    const usesObject = Boolean(objectName);
    if (!usesObject && !customQuery.trim())
      return fail("请先填写要执行的自定义只读 SQL");
    setBusy("source");
    try {
      const checked = await command("test_database_connection", {
        profile: sourceProfile,
      });
      const baseScope = sourceMappingProfileScope({
        adapterId: sourceAdapterId({ sourceMode: "database" }),
        profile: sourceProfile,
        sourceMode: "database",
        targetTenantId: targetAuth?.tenantId || tenantId || loginTenantId,
      });
      const scopedScope = sourceMappingProfileScope({
        adapterId: sourceAdapterId({ sourceMode: "database" }),
        profile: sourceProfile,
        sourceObject: usesObject ? objectName : "",
        sourceQuery: usesObject ? "" : customQuery,
        sourceMode: "database",
        targetTenantId: targetAuth?.tenantId || tenantId || loginTenantId,
      });
      const loadMappingProfile = (scope) =>
        scope
          ? command("load_source_mapping_profile", {
              request: {
                scope,
                adapterVersion: sourceAdapterVersionFor(scope.adapterId),
                templateCompatibleFromVersion: sourceAdapterCompatibleFromFor(
                  scope.adapterId,
                ),
              },
            })
          : Promise.resolve(null);
      const [sourceResult, scopedMappingProfile, legacyMappingProfile] =
        await Promise.all([
          usesObject
            ? command("preview_source_object", {
                request: {
                  connection: sourceProfile,
                  objectName,
                  limit: 10000,
                },
              })
            : command("preview_source", {
                request: {
                  connection: sourceProfile,
                  query: customQuery,
                  limit: 10000,
                },
              }),
          loadMappingProfile(scopedScope),
          scopedScope?.sourceIdentity !== baseScope?.sourceIdentity
            ? loadMappingProfile(baseScope)
            : Promise.resolve(null),
        ]);
      const preview = usesObject ? sourceResult.preview : sourceResult;
      const resolvedQuery = usesObject
        ? sourceResult.query
        : customQuery.trim();
      const savedMappingProfile = sourceMappingProfileForSelection({
        scopedProfile: scopedMappingProfile,
        legacyProfile: legacyMappingProfile,
        resolvedQuery,
      });
      if (preview.truncated)
        return fail(
          usesObject
            ? `表/视图 ${objectName} 超过单批10,000行，请改用高级 SQL 添加稳定筛选条件`
            : "自定义查询结果超过单批10,000行，请按稳定条件拆分批次后读取",
        );
      let similarMappingCandidate = null;
      if (!savedMappingProfile && scopedScope) {
        try {
          similarMappingCandidate = await command(
            "recommend_source_mapping_profile",
            {
              request: {
                scope: scopedScope,
                columns: preview.columns?.length
                  ? preview.columns
                  : Object.keys(preview.rows?.[0] || {}),
                adapterVersion: sourceAdapterVersionFor(scopedScope.adapterId),
                templateCompatibleFromVersion: sourceAdapterCompatibleFromFor(
                  scopedScope.adapterId,
                ),
              },
            },
          );
        } catch {
          // Similar-source reuse is an optional convenience. Never block a fresh mapping flow
          // because local recommendation history is missing or unreadable.
        }
      }
      let mappingProfileForCurrentRead =
        savedMappingProfile || similarMappingCandidate?.profile || null;
      const promotionRequest = legacySourceMappingPromotionRequest({
        scopedScope,
        scopedProfile: scopedMappingProfile,
        legacyProfile: legacyMappingProfile,
        resolvedQuery,
      });
      if (promotionRequest) {
        try {
          await command("save_source_mapping_profile", {
            request: promotionRequest,
          });
        } catch {
          mappingProfileForCurrentRead = {
            ...savedMappingProfile,
            storageWarning: "旧模板已恢复，但未能按当前表/视图或查询单独固化",
          };
        }
      }
      setQuery(resolvedQuery);
      const sourceEndpointName =
        sourceProfile.database ||
        sourceProfile.serviceName ||
        sourceProfile.host ||
        "当前数据库";
      const accepted = acceptData(
        preview.rows,
        `${databaseKinds[sourceProfile.kind]?.label || sourceProfile.kind} · ${sourceEndpointName} · ${usesObject ? objectName : "自定义只读查询"}`,
        {
          description: usesObject
            ? `安全读取表/视图：${objectName}`
            : resolvedQuery,
          columnMetadata: preview.columnMetadata,
          mappingProfile: mappingProfileForCurrentRead,
          sourceSelection: usesObject
            ? { objectName }
            : { sourceQuery: resolvedQuery },
        },
      );
      await rememberSourceConnection();
      notify(
        mappingProfileForCurrentRead
          ? similarMappingCandidate
            ? `${checked.message}，已读取 ${preview.rows.length} 行；发现同结构本地模板并复用 ${accepted.restoredCount} 项字段建议，请在进入校验前核对`
            : `${checked.message}，已读取 ${preview.rows.length} 行并恢复 ${accepted.restoredCount} 项来源模板配置${accepted.missingCount ? `；${accepted.missingCount} 项旧字段已失效并重新建议` : ""}${promotionRequest && !mappingProfileForCurrentRead.storageWarning ? "；旧模板已自动按当前来源对象固化" : ""}${mappingProfileForCurrentRead.storageWarning ? `；${mappingProfileForCurrentRead.storageWarning}` : ""}`
          : `${checked.message}，已读取 ${preview.rows.length} 行`,
        mappingProfileForCurrentRead?.storageWarning ? "danger" : "success",
      );
      return preview;
    } catch (error) {
      fail(error);
      return null;
    } finally {
      setBusy("");
    }
  }

  async function loadDatabase() {
    return loadGenericDatabase({ customQuery: query });
  }

  async function previewDatabaseObject(requestedObjectName = "") {
    const objectName =
      typeof requestedObjectName === "string" && requestedObjectName
        ? requestedObjectName
        : selectedSourceObject;
    if (!objectName) return fail("请先选择一个来源表或视图");
    if (!sourceObjects.includes(objectName))
      return fail("来源表或视图已不在当前清单中，请刷新后重试");
    const requestId = sourceObjectPreviewRequestRef.current + 1;
    sourceObjectPreviewRequestRef.current = requestId;
    if (objectName !== selectedSourceObject)
      setSelectedSourceObject(objectName);
    setSourceObjectPreview(null);
    setSourceObjectCount(null);
    setSourceObjectCountBusy(false);
    setBusy("source-object-preview");
    try {
      const sourceResult = await command("preview_source_object", {
        request: {
          connection: sourceProfile,
          objectName,
          limit: 6,
        },
      });
      if (sourceObjectPreviewRequestRef.current !== requestId) return null;
      const preview = sourceResult.preview;
      setSourceObjectPreview(sourceResult);
      await rememberSourceConnection();
      void countDatabaseObjectRows(sourceResult.objectName, requestId);
      notify(
        preview.rows.length
          ? `已读取 ${preview.columns.length} 个字段和 ${preview.rows.length} 行样例，请核对后再读取本批`
          : `表/视图 ${objectName} 当前没有可读取的数据`,
        preview.rows.length ? "success" : "danger",
      );
      return sourceResult;
    } catch (error) {
      fail(error);
      return null;
    } finally {
      setBusy("");
    }
  }

  async function surveyLikelySourceObjects() {
    const candidates = sourceObjectSurveyCandidates(sourceObjects, 5);
    if (!candidates.length)
      return fail(
        "当前清单没有明确的药品名称线索，请直接搜索并预览已知业务表或视图",
      );
    const requestId = sourceObjectSurveyRequestRef.current + 1;
    sourceObjectSurveyRequestRef.current = requestId;
    let completed = [];
    setSourceObjectSurvey({
      status: "RUNNING",
      completedCount: 0,
      totalCount: candidates.length,
      results: [],
    });
    setBusy("source-object-survey");
    try {
      const surveyed = await command("survey_source_objects", {
        request: {
          connection: sourceProfile,
          objectNames: candidates.map((option) => option.value),
        },
      });
      if (sourceObjectSurveyRequestRef.current !== requestId) return null;
      const surveyedByName = Object.fromEntries(
        (surveyed || []).map((item) => [item.objectName, item]),
      );
      completed = rankSourceObjectSurveyResults(
        candidates.map((option) => {
          const item = surveyedByName[option.value];
          return item?.status === "READY" && item.sourceResult
            ? sourceObjectSurveyResult({
                option,
                sourceResult: item.sourceResult,
                targetFields,
              })
            : failedSourceObjectSurveyResult(option);
        }),
      );
      const successful = completed.filter(
        (result) => result.status === "READY",
      );
      setSourceObjectSurvey({
        status: "COMPLETE",
        completedCount: completed.length,
        totalCount: candidates.length,
        results: completed,
      });
      await rememberSourceConnection();
      notify(
        `已轻量核对 ${candidates.length} 个候选，其中 ${successful.length} 个返回有效字段；请人工选择后完成正式预览`,
        successful.length ? "success" : "danger",
      );
      return completed;
    } catch (error) {
      if (sourceObjectSurveyRequestRef.current === requestId) fail(error);
      return null;
    } finally {
      setBusy((current) => (current === "source-object-survey" ? "" : current));
    }
  }

  async function countDatabaseObjectRows(objectName, requestId) {
    setSourceObjectCountBusy(true);
    try {
      const result = await command("count_source_object_rows", {
        request: {
          connection: sourceProfile,
          objectName,
        },
      });
      if (sourceObjectPreviewRequestRef.current !== requestId) return;
      setSourceObjectCount(result);
    } catch {
      if (sourceObjectPreviewRequestRef.current !== requestId) return;
      setSourceObjectCount({
        objectName,
        error: "短时行数探测未完成；正式读取时仍会执行单批上限检查",
      });
    } finally {
      if (sourceObjectPreviewRequestRef.current === requestId)
        setSourceObjectCountBusy(false);
    }
  }

  async function loadDatabaseObject() {
    if (!selectedSourceObject) return fail("请先选择一个来源表或视图");
    if (sourceObjectPreview?.objectName !== selectedSourceObject)
      return fail("请先预览所选表或视图的字段和样例，再确认读取本批");
    if (!sourceObjectPreview.preview?.rows?.length)
      return fail("所选表或视图当前没有可迁移数据，请选择其他对象");
    if (
      sourceObjectCount?.objectName === selectedSourceObject &&
      Number(sourceObjectCount.rowCount || 0) > 10_000
    )
      return fail(
        `表/视图 ${selectedSourceObject} ${sourceObjectCount.isExact === false ? "至少" : "共"} ${Number(sourceObjectCount.rowCount).toLocaleString()} 行，超过单批10,000行；请使用高级 SQL 按稳定条件拆分，或让数据库人员提供已筛选业务视图`,
      );
    return loadGenericDatabase({ objectName: selectedSourceObject });
  }

  async function testSourceConnection() {
    setBusy("source-test");
    try {
      const checked = await command("test_database_connection", {
        profile: sourceProfile,
      });
      await rememberSourceConnection();
      notify(`${checked.message} · ${checked.latencyMs}ms`);
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  async function inspectMedicineSourceAdapter() {
    const adapter = sourceAdapters.find(
      (item) =>
        item.id === selectedMedicineAdapterId &&
        item.automaticDetection &&
        item.migrationTasks.includes("MEDICINE_BASE") &&
        item.databaseFamilies.includes(sourceProfile.kind),
    );
    if (!adapter) return fail("请选择支持当前数据库的自动药品来源适配器");
    setBusy("medicine-adapter-inspect");
    setSourceAdapterInspection(null);
    setSourceAdapterDiagnostics([]);
    try {
      const checked = await command("test_database_connection", {
        profile: sourceProfile,
      });
      const inspection = await command("inspect_medicine_source_adapter", {
        request: {
          adapterId: adapter.id,
          connection: sourceProfile,
        },
      });
      await rememberSourceConnection();
      setSourceAdapterInspection(inspection);
      await archiveSourceAdapterDiagnostic(inspection, {
        migrationTask: "MEDICINE_BASE",
        profile: sourceProfile,
        setDiagnostics: setSourceAdapterDiagnostics,
      }).catch(() => {
        setSourceAdapterDiagnostics([]);
      });
      const recommended = inspection.scopes?.find((scope) => scope.recommended);
      if (recommended) setLegacyScope(recommended.id);
      if (!inspection.detected) return fail(inspection.message);
      notify(`${checked.message}；${inspection.message}`);
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  async function archiveSourceAdapterDiagnostic(
    inspection,
    {
      migrationTask = "MEDICINE_BASE",
      profile = sourceProfile,
      setDiagnostics = setSourceAdapterDiagnostics,
    } = {},
  ) {
    const scope = sourceMappingProfileScope({
      adapterId: inspection.adapterId,
      profile,
      sourceMode: "database",
      targetTenantId: targetAuth?.tenantId || tenantId || loginTenantId,
    });
    if (!scope) return [];
    const history = await command("save_source_adapter_diagnostic", {
      request: {
        scope,
        migrationTask,
        adapterVersion: inspection.adapterVersion,
        databaseFamily: profile.kind,
        schema: inspection.schema,
        detected: inspection.detected,
        checkedObjects: inspection.checkedObjects || [],
        missingObjects: inspection.missingObjects || [],
        objectStructures: inspection.objectStructures || [],
        metrics: inspection.metrics || [],
        warnings: inspection.warnings || [],
        message: inspection.message,
      },
    });
    setDiagnostics(history || []);
    return history;
  }

  async function loadMedicineSourceAdapter() {
    if (!sourceAdapterInspection?.detected)
      return fail("请先识别当前来源适配器的数据结构");
    const adapter = sourceAdapters.find(
      (item) => item.id === sourceAdapterInspection.adapterId,
    );
    const workflow = medicineWorkflowFor(adapter);
    const selected = sourceAdapterInspection.scopes.find(
      (scope) => scope.id === legacyScope,
    );
    setBusy("medicine-adapter-load");
    try {
      const mappingScope = sourceMappingProfileScope({
        adapterId: sourceAdapterInspection.adapterId,
        profile: sourceProfile,
        sourceMode: "database",
        targetTenantId: targetAuth?.tenantId || tenantId || loginTenantId,
      });
      const [preview, scopedMappingProfile, legacyMappingProfile] =
        await Promise.all([
          command("load_medicine_source_adapter", {
            request: {
              adapterId: sourceAdapterInspection.adapterId,
              connection: sourceProfile,
              scope: legacyScope,
              limit: 10000,
            },
          }),
          mappingScope
            ? command("load_source_mapping_profile", {
                request: {
                  scope: mappingScope,
                  adapterVersion: sourceAdapterInspection.adapterVersion,
                  templateCompatibleFromVersion: sourceAdapterCompatibleFromFor(
                    sourceAdapterInspection.adapterId,
                  ),
                },
              })
            : Promise.resolve(null),
          workflow.legacyProfileKind === "PHIS27_V2"
            ? command("load_phis27_mapping_profile")
            : Promise.resolve(null),
        ]);
      const savedMappingProfile = scopedMappingProfile || legacyMappingProfile;
      if (preview.truncated)
        return fail("当前范围超过单批10,000行，请缩小范围后再读取");
      setAllowCreateFactory(true);
      const savedDictionaryOverrides =
        savedMappingProfile?.dictionaryOverrides || {};
      const dictionaryScopeKey = sourceDictionaryScopeKey(
        sourceProfile,
        sourceAdapterInspection.schema,
      );
      const customizedMetadata = applySourceDictionaryOverrides(
        preview.columnMetadata,
        savedDictionaryOverrides[dictionaryScopeKey] || {},
      );
      setSourceDictionaryOverrides(savedDictionaryOverrides);
      const restored = acceptData(
        preview.rows,
        `${sourceAdapterInspection.adapterName} · ${sourceAdapterInspection.schema} · ${selected?.label || legacyScope}`,
        {
          sourceKey: workflow.sourceKeyField,
          adapterId: sourceAdapterInspection.adapterId,
          description: `来源适配器:${sourceAdapterInspection.adapterId}; scope=${legacyScope}; schema=${sourceAdapterInspection.schema}`,
          columnMetadata: customizedMetadata,
          mappingProfile: savedMappingProfile,
          mappingPreset: workflow.mappingPreset,
          skipNotice: true,
        },
      );
      notify(
        savedMappingProfile
          ? `已读取 ${preview.rows.length} 行并恢复 ${restored.restoredCount} 项固化映射${restored.missingCount ? `；${restored.missingCount} 项来源字段已变化，已重新建议` : ""}`
          : `已按安全模板读取 ${preview.rows.length} 行，首次使用请核对并固化映射`,
      );
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  function inventoryDiagnosticInspection(readiness) {
    const structureBlocked = readiness?.compatibility?.status === "BLOCKED";
    return {
      ...readiness,
      detected:
        !structureBlocked && Boolean(readiness?.readyForLocationMapping),
      metrics: [
        {
          id: "STOCK_ROWS",
          label: "库存明细",
          value: Number(readiness?.stockRowCount || 0),
        },
        {
          id: "STOCK_GROUPS",
          label: "药品-库房组合",
          value: Number(readiness?.stockGroupCount || 0),
        },
        {
          id: "MEDICINES",
          label: "库存药品",
          value: Number(readiness?.medicineCount || 0),
        },
        {
          id: "LOCATIONS",
          label: "库存位置",
          value: Number(readiness?.locations?.length || 0),
        },
      ],
    };
  }

  async function inspectInventorySourceAdapter() {
    const adapter = sourceAdapters.find(
      (item) =>
        item.id === selectedInventoryAdapterId &&
        item.migrationTasks.includes("INVENTORY") &&
        item.databaseFamilies.includes(sourceProfile.kind),
    );
    if (!adapter) return fail("请选择支持当前数据库的库存来源适配器");
    setBusy("inventory-inspect");
    setInventoryReadiness(null);
    setInventorySourceDiagnostics([]);
    setInventorySourceCatalog(null);
    setTargetOrganizationCatalog(null);
    setInventoryOrganizationMappings({});
    setInventorySelectedOrganizationIds([]);
    setInventorySelectedLocationKeys([]);
    setInventoryActiveOrganizationId("");
    setInventoryLocationSearch("");
    setTargetStorageCatalog(null);
    setInventoryLocationMappings({});
    setInventoryResolvedLocations({});
    setInventoryBatchDetail(null);
    setInventoryUndoPreview(null);
    setInventoryUndoConfirmed(false);
    try {
      const schema =
        `${sourceProfile.schema || sourceProfile.username || "SOURCE"}`
          .trim()
          .toUpperCase();
      const sourceIdentity = sourceAdapterIdentity(
        adapter.name,
        sourceProfile,
        schema,
      );
      const readiness = await command("inspect_inventory_source_adapter", {
        request: {
          adapterId: adapter.id,
          connection: sourceProfile,
          sourceName: sourceIdentity,
        },
      });
      await rememberSourceConnection();
      setInventoryReadiness(readiness);
      setInventoryActiveOrganizationId(
        readiness.locations.find((location) => location.organizationId)
          ?.organizationId || "",
      );
      const structureBlocked =
        readiness.compatibility?.status === "BLOCKED";
      await archiveSourceAdapterDiagnostic(
        inventoryDiagnosticInspection(readiness),
        {
          migrationTask: "INVENTORY",
          profile: sourceProfile,
          setDiagnostics: setInventorySourceDiagnostics,
        },
      ).catch(() => setInventorySourceDiagnostics([]));
      setInventorySourceExpanded(structureBlocked);
      setInventoryPreflightExpanded(false);
      if (structureBlocked) fail(readiness.message);
      else notify(readiness.message);
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  function inventorySourceIdentity() {
    if (inventoryReadiness?.sourceName) return inventoryReadiness.sourceName;
    const schema =
      `${sourceProfile.schema || sourceProfile.username || "SOURCE"}`
        .trim()
        .toUpperCase();
    const adapter = sourceAdapters.find(
      (item) => item.id === selectedInventoryAdapterId,
    );
    return adapter
      ? sourceAdapterIdentity(adapter.name, sourceProfile, schema)
      : `${selectedInventoryAdapterId} · ${databaseSourceIdentity(sourceProfile)}`;
  }

  async function loadInventoryTargetStorages() {
    setBusy("inventory-target");
    setTargetStorageCatalog(null);
    setInventoryBatchDetail(null);
    try {
      const sourceName = inventorySourceIdentity();
      const [
        legacyCatalog,
        organizationCatalog,
        catalog,
        savedMappings,
        savedOrganizations,
      ] = await Promise.all([
        command("load_inventory_source_catalog", {
          request: {
            adapterId:
              inventoryReadiness?.adapterId || selectedInventoryAdapterId,
            connection: sourceProfile,
          },
        }),
        command("load_inventory_target_organizations"),
        command("load_inventory_target_storages", { target: targetProfile }),
        command("load_inventory_location_mappings", {
          sourceName,
          target: targetProfile,
        }),
        command("load_inventory_organization_mappings", { sourceName }),
      ]);
      await rememberTargetDatabaseConnection();
      const availableIds = new Set(
        catalog.storages.map((storage) => storage.idSto),
      );
      const availableOrganizationIds = new Set(
        organizationCatalog.organizations.map(
          (organization) => organization.id,
        ),
      );
      const availableStorageOrganizationIds = new Set(
        catalog.storages
          .filter((storage) => ["1", "2"].includes(storage.storageType))
          .map((storage) => storage.organizationId),
      );
      setInventorySourceCatalog(legacyCatalog);
      setTargetOrganizationCatalog(organizationCatalog);
      setTargetStorageCatalog(catalog);
      setInventoryTargetExpanded(false);
      setInventoryOrganizationMappings(
        Object.fromEntries(
          savedOrganizations
            .filter(
              (mapping) =>
                availableOrganizationIds.has(mapping.targetOrganizationId) &&
                availableStorageOrganizationIds.has(
                  mapping.targetOrganizationId,
                ),
            )
            .map((mapping) => [
              mapping.sourceOrganizationId,
              mapping.targetOrganizationId,
            ]),
        ),
      );
      setInventorySelectedOrganizationIds([]);
      setInventorySelectedLocationKeys([]);
      setInventoryLocationSearch("");
      setInventoryLocationMappings(
        Object.fromEntries(
          savedMappings
            .filter((mapping) => availableIds.has(mapping.targetIdSto))
            .map((mapping) => [mapping.sourceLocationKey, mapping.targetIdSto]),
        ),
      );
      setInventoryResolvedLocations(
        Object.fromEntries(
          savedMappings
            .filter((mapping) => mapping.resolvedSourceLocationKey)
            .map((mapping) => [
              mapping.sourceLocationKey,
              mapping.resolvedSourceLocationKey,
            ]),
        ),
      );
      notify(
        `${legacyCatalog.message}；${organizationCatalog.message}；${catalog.message}${savedMappings.length || savedOrganizations.length ? `，已恢复 ${savedOrganizations.length} 项机构、${savedMappings.length} 项库房映射` : ""}`,
      );
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  async function prepareInventoryBatch() {
    const locations = inventoryReadiness?.locations || [];
    const storages = targetStorageCatalog?.storages || [];
    const targetOrganizations = targetOrganizationCatalog?.organizations || [];
    const legacyOrganizations = inventorySourceCatalog?.organizations || [];
    const completedOrganizationIds = completeInventoryOrganizationIds(
      locations,
      inventoryOrganizationMappings,
      inventoryLocationMappings,
      inventoryResolvedLocations,
      inventorySelectedLocationKeys,
    );
    const completedOrganizationIdSet = new Set(completedOrganizationIds);
    const selectedBatchOrganizationIds =
      inventorySelectedOrganizationIds.filter((organizationId) =>
        completedOrganizationIdSet.has(organizationId),
      );
    if (!selectedBatchOrganizationIds.length) {
      return fail("请先选择并完成本批库房映射，再勾选至少一个机构纳入本批");
    }
    const selectedOrganizationIds = new Set(selectedBatchOrganizationIds);
    const selectedLocationKeySet = new Set(inventorySelectedLocationKeys);
    const selectedLocations = locations.filter(
      (location) =>
        selectedOrganizationIds.has(location.organizationId) &&
        selectedLocationKeySet.has(location.sourceLocationKey),
    );
    if (!selectedLocations.length) {
      return fail("请先选择本批需要迁移的药库或药房");
    }
    const storageById = new Map(
      storages.map((storage) => [storage.idSto, storage]),
    );
    const missing = selectedLocations.filter(
      (location) => !inventoryLocationMappings[location.sourceLocationKey],
    );
    if (missing.length) {
      return fail(
        `请先为 ${missing.map((item) => item.sourceLocationName).join("、")} 选择新系统库房`,
      );
    }
    const unresolvedWarehouses = selectedLocations.filter(
      (location) =>
        inventoryLocationNeedsSourceResolution(location) &&
        !inventoryResolvedLocations[location.sourceLocationKey],
    );
    if (unresolvedWarehouses.length) {
      return fail("请先为待确认的药库库存指定实际老系统药库");
    }
    const mappings = selectedLocations.map((location) => {
      const storage = storageById.get(
        inventoryLocationMappings[location.sourceLocationKey],
      );
      const resolvedSourceLocationKey =
        inventoryResolvedLocations[location.sourceLocationKey] ||
        (inventoryLocationNeedsSourceResolution(location)
          ? ""
          : location.sourceLocationKey);
      const resolvedLocation = inventorySourceCatalog?.locations?.find(
        (item) => item.sourceLocationKey === resolvedSourceLocationKey,
      );
      return {
        sourceLocationKey: location.sourceLocationKey,
        sourceKind: location.sourceKind,
        sourceLocationName:
          resolvedLocation?.name || location.sourceLocationName,
        sourceOrganizationId: location.organizationId,
        resolvedSourceLocationKey,
        sourceResolutionRequired:
          inventoryLocationNeedsSourceResolution(location),
        targetIdSto: storage.idSto,
        targetName: storage.name,
        targetIdOrg: storage.organizationId,
      };
    });
    const organizationMappings = selectedBatchOrganizationIds.map(
      (sourceId) => {
        const source = legacyOrganizations.find((item) => item.id === sourceId);
        const targetId = inventoryOrganizationMappings[sourceId];
        const target = targetOrganizations.find((item) => item.id === targetId);
        return {
          sourceOrganizationId: sourceId,
          sourceOrganizationName: source?.name || sourceId,
          targetOrganizationId: targetId,
          targetOrganizationName: target?.name || targetId,
        };
      },
    );
    setBusy("inventory-prepare");
    setInventoryBatchDetail(null);
    setInventoryMedicineCatalog(null);
    setInventoryMedicineSuggestions([]);
    setInventoryMedicineSelections({});
    setInventoryMedicineSelectedTargets({});
    setInventoryMedicineConfirmations({});
    setInventoryMedicineMatchOpen(false);
    setInventoryMedicineMatchSearch("");
    setInventoryMedicineMatchFilter("ALL");
    setInventoryMedicineMatchPage(1);
    setInventoryUndoPreview(null);
    setInventoryUndoConfirmed(false);
    setInventoryReviewConfirmed(false);
    setInventoryReviewSearch("");
    setInventoryReviewStorage("");
    setInventoryReviewStatus("ALL");
    setInventoryExceptionRow(null);
    setInventoryExceptionReason("");
    try {
      const detail = await command("prepare_inventory_batch", {
        request: {
          adapterId:
            inventoryReadiness?.adapterId || selectedInventoryAdapterId,
          source: sourceProfile,
          target: targetProfile,
          sourceName: inventorySourceIdentity(),
          organizationMappings,
          mappings,
        },
      });
      setInventoryBatchDetail(detail);
      setInventoryMappingExpanded(false);
      notify(
        detail.batch.failCount
          ? `${selectedBatchOrganizationIds.length} 个机构、${selectedLocations.length} 个库房防重预检完成：${detail.batch.validCount} 组可写入，${detail.batch.failCount} 组需处理`
          : `${selectedBatchOrganizationIds.length} 个机构、${selectedLocations.length} 个库房防重预检通过：${detail.batch.validCount} 组可进入正式写入`,
      );
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  async function openInventoryMedicineMatching() {
    if (!inventoryBatchDetail) return;
    const unmatched = inventoryUnmatchedMedicines(inventoryBatchDetail.rows);
    if (!unmatched.length) {
      return notify("本批没有需要补充目录匹配的药品");
    }
    if (inventoryMedicineCatalog && inventoryMedicineSuggestions.length) {
      setInventoryMedicineMatchOpen(true);
      return;
    }
    setBusy("inventory-medicine-catalog");
    try {
      const result = await command("recommend_inventory_medicine_matches", {
        request: {
          target: targetProfile,
          sources: unmatched,
          limit: 8,
        },
      });
      const suggestions = result.suggestions || [];
      const selections = Object.fromEntries(
        suggestions
          .filter((item) => item.candidates[0]?.target)
          .map((item) => [item.key, item.candidates[0].target.idMedPro]),
      );
      const selectedTargets = Object.fromEntries(
        suggestions
          .filter((item) => item.candidates[0]?.target)
          .map((item) => [item.key, item.candidates[0].target]),
      );
      setInventoryMedicineCatalog(result);
      setInventoryMedicineSuggestions(suggestions);
      setInventoryMedicineSelections(selections);
      setInventoryMedicineSelectedTargets(selectedTargets);
      setInventoryMedicineConfirmations({});
      setInventoryMedicineMatchSearch("");
      setInventoryMedicineMatchFilter("ALL");
      setInventoryMedicineMatchPage(1);
      setInventoryMedicineMatchOpen(true);
      const selectedCount = Object.keys(selections).length;
      notify(
        `${result.message}，耗时 ${Math.max(0, Number(result.elapsedMs || 0))} 毫秒；已为 ${selectedCount} 项预选最优候选`,
      );
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  async function searchInventoryTargetMedicines(targetOrganizationId, query) {
    const result = await command("search_inventory_target_medicines", {
      request: {
        target: targetProfile,
        targetOrganizationId,
        query,
        limit: 60,
      },
    });
    return result.medicines || [];
  }

  async function saveInventoryMedicineMatches() {
    if (!inventoryBatchDetail) return;
    const matches = inventoryMedicineSuggestions.flatMap((suggestion) => {
      if (!inventoryMedicineConfirmations[suggestion.key]) return [];
      const target = inventoryMedicineSelectedTargets[suggestion.key];
      if (!target) return [];
      return [
        {
          sourceProductKey: suggestion.sourceProductKey,
          targetOrganizationId: suggestion.targetOrganizationId,
          idMed: target.idMed,
          idMedPro: target.idMedPro,
          matchMethod:
            suggestion.autoCandidate?.idMedPro === target.idMedPro
              ? "EXACT_AUTO"
              : "MANUAL",
        },
      ];
    });
    if (!matches.length) {
      return fail("请至少确认一个匹配结果后再保存");
    }
    setBusy("inventory-medicine-save");
    try {
      await command("save_inventory_medicine_matches", {
        request: {
          batchId: inventoryBatchDetail.batch.batchId,
          target: targetProfile,
          matches,
        },
      });
      await prepareInventoryBatch();
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  async function executeInventoryBatch() {
    if (inventoryExecutionLock.current || busy === "inventory-execute") return;
    if (!inventoryBatchDetail || inventoryBatchDetail.batch.failCount > 0) {
      return fail("请先完成库房映射并处理全部库存预检失败项");
    }
    if (!inventoryReviewConfirmed) {
      return fail("请先核对本批机构、库房、药品、批号、效期、数量和价格");
    }
    if (!inventoryTrialStatus.complete) {
      return fail(
        `正式执行前还需完成 ${inventoryTrialStatus.pendingStorageIds.length} 个目标库房的库存试迁移`,
      );
    }
    const storageCount = new Set(
      inventoryBatchDetail.rows
        .filter((row) => row.status === "VALIDATED")
        .map((row) => row.normalizedData?.idSto)
        .filter(Boolean),
    ).size;
    if (!storageCount) return fail("当前批次没有可执行的目标库房");
    inventoryExecutionLock.current = true;
    setBusy("inventory-execute");
    try {
      await new Promise((resolve) =>
        window.requestAnimationFrame(() =>
          window.requestAnimationFrame(resolve),
        ),
      );
      const detail = await command("execute_inventory_batch", {
        request: {
          batchId: inventoryBatchDetail.batch.batchId,
          target: targetProfile,
        },
      });
      setInventoryBatchDetail(detail);
      setInventoryUndoPreview(null);
      setInventoryUndoConfirmed(false);
      notify(
        detail.batch.failCount
          ? `首次盘点执行完成：${detail.batch.successCount} 组成功，${detail.batch.failCount} 组失败`
          : `首次盘点执行成功：${detail.batch.successCount} 组库存已建立初始账簿`,
      );
    } catch (error) {
      fail(error);
    } finally {
      inventoryExecutionLock.current = false;
      setBusy("");
    }
  }

  async function trialInventoryRow(row) {
    if (!inventoryBatchDetail || inventoryTrialLock.current) return;
    inventoryTrialLock.current = true;
    setInventoryTrialRowId(row.rowId);
    setBusy("inventory-trial");
    try {
      const response = await command("trial_inventory_row", {
        request: {
          batchId: inventoryBatchDetail.batch.batchId,
          rowId: row.rowId,
          target: targetProfile,
        },
      });
      setInventoryBatchDetail(response.detail);
      notify(
        response.result.message,
        response.result.ok ? "success" : "danger",
      );
    } catch (error) {
      fail(error);
    } finally {
      inventoryTrialLock.current = false;
      setInventoryTrialRowId("");
      setBusy("");
    }
  }

  async function confirmInventoryException() {
    if (!inventoryBatchDetail || !inventoryExceptionRow) return;
    const reason = inventoryExceptionReason.trim();
    if (reason.length < 2) return fail("请填写至少 2 个字的人工确认原因");
    setBusy("inventory-exception");
    try {
      const detail = await command("confirm_inventory_exception", {
        request: {
          batchId: inventoryBatchDetail.batch.batchId,
          rowId: inventoryExceptionRow.rowId,
          target: targetProfile,
          reason,
        },
      });
      setInventoryBatchDetail(detail);
      setInventoryExceptionRow(null);
      setInventoryExceptionReason("");
      setInventoryReviewConfirmed(false);
      notify("已记录人工确认例外；该目标库房需要重新完成试迁移");
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  async function previewInventoryUndo() {
    if (!inventoryBatchDetail) return;
    setBusy("inventory-undo-preview");
    setInventoryUndoPreview(null);
    setInventoryUndoConfirmed(false);
    try {
      const preview = await command("preview_inventory_undo", {
        request: {
          batchId: inventoryBatchDetail.batch.batchId,
          target: targetProfile,
        },
      });
      setInventoryUndoPreview(preview);
      notify(preview.message);
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  async function undoInventoryBatch() {
    if (inventoryUndoLock.current || busy === "inventory-undo") return;
    if (!inventoryUndoPreview?.canUndo) {
      return fail("请先完成撤销预检，并处理所有后续业务阻断项");
    }
    if (!inventoryUndoConfirmed) {
      return fail("请先确认撤销范围和影响");
    }
    inventoryUndoLock.current = true;
    setBusy("inventory-undo");
    try {
      await new Promise((resolve) =>
        window.requestAnimationFrame(() =>
          window.requestAnimationFrame(resolve),
        ),
      );
      const detail = await command("undo_inventory_batch", {
        request: {
          batchId: inventoryBatchDetail.batch.batchId,
          target: targetProfile,
        },
      });
      setInventoryBatchDetail(detail);
      setInventoryUndoPreview(null);
      setInventoryUndoConfirmed(false);
      notify("首次盘点、初始库存和初始账簿已安全撤销");
    } catch (error) {
      fail(error);
    } finally {
      inventoryUndoLock.current = false;
      setBusy("");
    }
  }

  function autoMatchInventoryMappings() {
    if (
      !inventoryReadiness ||
      !inventorySourceCatalog ||
      !targetOrganizationCatalog ||
      !targetStorageCatalog
    )
      return;
    const normalizeName = (value) =>
      `${value || ""}`.trim().replace(/\s+/g, "").toLocaleLowerCase("zh-CN");
    const nextOrganizations = { ...inventoryOrganizationMappings };
    const nextLocations = { ...inventoryLocationMappings };
    const nextResolved = { ...inventoryResolvedLocations };
    const selectedLocationKeySet = new Set(inventorySelectedLocationKeys);
    const sourceOrganizationIds = [
      ...new Set(
        inventoryReadiness.locations
          .filter((location) =>
            selectedLocationKeySet.has(location.sourceLocationKey),
          )
          .map((location) => location.organizationId)
          .filter(Boolean),
      ),
    ];
    let matchedOrganizations = 0;
    let matchedLocations = 0;
    for (const sourceOrganizationId of sourceOrganizationIds) {
      const sourceOrganization = inventorySourceCatalog.organizations.find(
        (item) => item.id === sourceOrganizationId,
      );
      const matches = targetOrganizationCatalog.organizations.filter(
        (item) =>
          normalizeName(item.name || item.fullName) ===
          normalizeName(sourceOrganization?.name),
      );
      if (!nextOrganizations[sourceOrganizationId] && matches.length === 1) {
        nextOrganizations[sourceOrganizationId] = matches[0].id;
        matchedOrganizations += 1;
      }
      const targetOrganizationId = nextOrganizations[sourceOrganizationId];
      if (!targetOrganizationId) continue;
      for (const location of inventoryReadiness.locations.filter(
        (item) =>
          item.organizationId === sourceOrganizationId &&
          selectedLocationKeySet.has(item.sourceLocationKey),
      )) {
        const sourceCandidates = inventorySourceCatalog.locations.filter(
          (item) =>
            item.organizationId === sourceOrganizationId &&
            item.sourceKind === location.sourceKind &&
            item.active,
        );
        const needsSourceResolution =
          inventoryLocationNeedsSourceResolution(location);
        const sourceLocation = needsSourceResolution
          ? sourceCandidates.find(
              (item) =>
                normalizeName(item.name) ===
                normalizeName(location.sourceLocationName),
            )
          : sourceCandidates.find(
              (item) => item.sourceLocationKey === location.sourceLocationKey,
            );
        if (
          needsSourceResolution &&
          sourceLocation &&
          !nextResolved[location.sourceLocationKey]
        ) {
          nextResolved[location.sourceLocationKey] =
            sourceLocation.sourceLocationKey;
        }
        const sourceName = sourceLocation?.name || location.sourceLocationName;
        const expectedType = location.sourceKind === "WAREHOUSE" ? "1" : "2";
        const storageMatches = targetStorageCatalog.storages.filter(
          (storage) =>
            storage.organizationId === targetOrganizationId &&
            storage.storageType === expectedType &&
            normalizeName(storage.name) === normalizeName(sourceName),
        );
        if (
          !nextLocations[location.sourceLocationKey] &&
          storageMatches.length === 1
        ) {
          nextLocations[location.sourceLocationKey] = storageMatches[0].idSto;
          matchedLocations += 1;
        }
      }
    }
    setInventoryOrganizationMappings(nextOrganizations);
    setInventoryLocationMappings(nextLocations);
    setInventoryResolvedLocations(nextResolved);
    setInventoryBatchDetail(null);
    notify(
      matchedOrganizations || matchedLocations
        ? `已安全匹配 ${matchedOrganizations} 个同名机构、${matchedLocations} 个同名库房/药房`
        : "没有发现唯一且完全同名的可自动匹配项，请手工选择",
    );
  }

  const inventoryRenderContext = {
    autoMatchInventoryMappings,
    busy,
    inventoryBatchDetail,
    inventoryExecutionSeconds,
    inventoryExceptionReason,
    inventoryExceptionRow,
    inventoryLocationMappings,
    inventoryMedicineCatalog,
    inventoryMedicineConfirmations,
    inventoryMedicineMatchFilter,
    inventoryMedicineMatchOpen,
    inventoryMedicineMatchPage,
    inventoryMedicineMatchSearch,
    inventoryMedicineSelectedTargets,
    inventoryMedicineSelections,
    inventoryMedicineSuggestions,
    inventoryOrganizationMappings,
    inventoryReadiness,
    inventoryActiveOrganizationId,
    inventoryLocationSearch,
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
    inventorySourceCatalog,
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
  };

  function beginMapping() {
    const selectedAssessment = assessSourceKeyFields({
      rows,
      fields: sourceKeyFields,
    });
    if (!selectedAssessment.completeUnique) {
      return fail(
        sourceKeyFields.length
          ? "当前来源字段组合在整批数据中存在空值或重复，请调整组合"
          : "请选择一至三个字段组成当前批次非空且唯一的稳定业务键",
      );
    }
    if (!activeAdapterProvidesSourceKey && !sourceKeyConfirmed)
      return fail("请确认所选字段是老系统稳定业务主键，不会被复用或随业务修改");
    setFieldIndex(0);
    setStep(4);
  }

  function currentSourceMappingProfileScope() {
    return sourceMappingProfileScope({
      adapterId: activeSourceAdapterId || sourceAdapterId({ sourceMode }),
      fileName: sourceName,
      profile: sourceProfile,
      sourceObject: activeSourceSelection?.objectName || "",
      sourceQuery: activeSourceSelection?.sourceQuery || "",
      sourceMode,
      targetTenantId: targetAuth?.tenantId || tenantId || loginTenantId,
    });
  }

  function sourceAdapterVersionFor(adapterId) {
    return Number(
      sourceAdapters.find((adapter) => adapter.id === adapterId)?.version || 0,
    );
  }

  function sourceAdapterCompatibleFromFor(adapterId) {
    const adapter = sourceAdapters.find((item) => item.id === adapterId);
    return Number(
      adapter?.templateCompatibleFromVersion || adapter?.version || 0,
    );
  }

  function currentSourceUsesScopedDictionaries() {
    return Boolean(
      sourceAdapters.find((adapter) => adapter.id === activeSourceAdapterId)
        ?.automaticDetection,
    );
  }

  async function persistSourceMappingProfile(
    dictionaryOverrides = sourceDictionaryOverrides,
  ) {
    const scope = currentSourceMappingProfileScope();
    if (!scope) return null;
    const completeMapping = Object.fromEntries(
      targetFields.map((field) => [field.key, mapping[field.key] || ""]),
    );
    const saveRequest = {
      adapterVersion: sourceAdapterVersionFor(scope.adapterId),
      sourceKey: sourceKeyFields.length === 1 ? sourceKeyFields[0] : "",
      sourceKeyFields,
      sourceQuery: activeSourceAdapter?.automaticDetection ? "" : query.trim(),
      mapping: completeMapping,
      rules,
      dictionaryOverrides,
    };
    const saved = await command("save_source_mapping_profile", {
      request: { scope, ...saveRequest },
    });
    if (
      scope.adapterId === "GENERIC_DATABASE" &&
      sourceMode === "database" &&
      activeSourceSelection
    ) {
      const baseScope = sourceMappingProfileScope({
        adapterId: scope.adapterId,
        profile: sourceProfile,
        sourceMode,
        targetTenantId: targetAuth?.tenantId || tenantId || loginTenantId,
      });
      if (baseScope && baseScope.sourceIdentity !== scope.sourceIdentity) {
        await command("save_source_mapping_profile", {
          request: { scope: baseScope, ...saveRequest },
        });
      }
    }
    setMappingProfileStatus({
      restored: true,
      restoredCount: Object.values(saved.mapping).filter(Boolean).length,
      missingCount: 0,
      savedAt: saved.savedAt,
      compatibility: saved.compatibility || "CURRENT",
      compatibilityMessage: saved.compatibilityMessage || "",
      requiresReview: Boolean(saved.requiresReview),
    });
    return saved;
  }

  async function exportSourceMappingTemplate() {
    const scope = currentSourceMappingProfileScope();
    if (!scope) return fail("请先连接当前来源和目标租户，再导出来源模板");
    const adapter = sourceAdapters.find((item) => item.id === scope.adapterId);
    try {
      const exported = await command("export_source_mapping_template", {
        request: {
          scope,
          adapterVersion: `${adapter?.version || ""}`,
          templateCompatibleFromVersion: Number(
            adapter?.templateCompatibleFromVersion || adapter?.version || 0,
          ),
          dictionaryScopeKey: currentSourceUsesScopedDictionaries()
            ? currentDictionaryScopeKey
            : "",
        },
      });
      const blob = new Blob([exported.content], {
        type: "application/json;charset=utf-8",
      });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = exported.fileName;
      document.body.appendChild(anchor);
      anchor.click();
      anchor.remove();
      URL.revokeObjectURL(url);
      notify("来源模板已导出；文件不含数据库地址、账号密码和目标租户");
      return exported;
    } catch (error) {
      fail(error);
      return null;
    }
  }

  async function exportSourceObjectDiagnostic(suitability) {
    if (
      !selectedSourceObject ||
      sourceObjectPreview?.objectName !== selectedSourceObject ||
      !sourceObjectPreview.preview?.columns?.length
    )
      return fail("请先预览来源表或视图，再导出结构诊断");
    try {
      const exported = await buildSourceObjectDiagnostic({
        databaseFamily: sourceProfile.kind,
        schema: sourceProfile.schema,
        objectName: selectedSourceObject,
        preview: sourceObjectPreview.preview,
        metadataMessage: sourceObjectPreview.metadataMessage,
        rowCount:
          sourceObjectCount?.objectName === selectedSourceObject
            ? sourceObjectCount
            : null,
        suitability,
      });
      const blob = new Blob([exported.content], {
        type: "application/json;charset=utf-8",
      });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = exported.fileName;
      document.body.appendChild(anchor);
      anchor.click();
      anchor.remove();
      URL.revokeObjectURL(url);
      notify("脱敏结构诊断已导出；不含连接信息、目标租户或任何原始样例值");
      return exported;
    } catch (error) {
      fail(error);
      return null;
    }
  }

  function exportSourceAdapterDiagnosticReport({
    inspection,
    migrationTask,
    profile,
  }) {
    if (!inspection) return fail("请先识别当前三方 HIS 数据结构");
    const adapter = sourceAdapters.find(
      (item) => item.id === inspection.adapterId,
    );
    const compatibility = inspection.compatibility;
    const taskLabel =
      migrationTask === "INVENTORY" ? "机构库存" : "药品基础数据";
    const compatibilityLines = (label, items = []) => [
      `${label}（${items.length}）：`,
      ...(items.length
        ? items.map(
            (item) =>
              `- ${item.path}：${item.message}${item.usedBy?.length ? `；影响 ${item.usedBy.join("、")}` : ""}${item.fallback ? `；处理方式 ${item.fallback}` : ""}`,
          )
        : ["- 无"]),
    ];
    const lines = [
      "三方 HIS 来源适配器项目差异报告",
      `生成时间：${new Date().toLocaleString("zh-CN", { hour12: false })}`,
      `迁移任务：${taskLabel}（${migrationTask}）`,
      `适配器：${inspection.adapterName} v${inspection.adapterVersion}`,
      `适配器 ID：${inspection.adapterId}`,
      `数据库类型：${profile.kind}`,
      `来源 Schema：${inspection.schema || "未识别"}`,
      `识别结论：${inspection.detected ? "通过" : "未通过"}`,
      `说明：${inspection.message}`,
      `能力：${adapter?.migrationTasks?.join("、") || "未声明"}`,
      `已核对对象：${inspection.checkedObjects?.join("、") || "无"}`,
      `缺失对象：${inspection.missingObjects?.join("、") || "无"}`,
      ...(compatibility
        ? [
            `契约影响：${compatibility.status} · ${compatibility.message}`,
            "",
            ...compatibilityLines("阻断项", compatibility.blockers),
            "",
            ...compatibilityLines("需核对", compatibility.reviews),
            "",
            ...compatibilityLines(
              "可兼容降级",
              compatibility.compatibleFallbacks,
            ),
          ]
        : []),
      "",
      "指标：",
      ...(inspection.metrics || []).map(
        (metric) => `- ${metric.label}：${metric.value}`,
      ),
      ...((inspection.scopes || []).length
        ? [
            "",
            "可选范围：",
            ...inspection.scopes.map(
              (scope) =>
                `- ${scope.label}（${scope.id}）：${scope.description}；${scope.medicineCount} 种 / 约 ${scope.estimatedRows} 行`,
            ),
          ]
        : []),
      "",
      "提醒：",
      ...((inspection.warnings || []).length
        ? inspection.warnings.map((warning) => `- ${warning}`)
        : ["- 无"]),
      "",
      "安全说明：本报告不包含主机地址、端口、数据库账号密码或目标租户。",
    ];
    const blob = new Blob([lines.join("\n")], {
      type: "text/plain;charset=utf-8",
    });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `${inspection.adapterId.toLowerCase()}-${migrationTask
      .toLowerCase()
      .replaceAll("_", "-")}-project-diagnostic.txt`;
    document.body.appendChild(anchor);
    anchor.click();
    anchor.remove();
    URL.revokeObjectURL(url);
    notify("适配器项目差异报告已导出，未包含连接地址或账号密码");
  }

  function exportSourceAdapterDiagnostic() {
    return exportSourceAdapterDiagnosticReport({
      inspection: sourceAdapterInspection,
      migrationTask: "MEDICINE_BASE",
      profile: sourceProfile,
    });
  }

  function exportInventorySourceAdapterDiagnostic() {
    return exportSourceAdapterDiagnosticReport({
      inspection: inventoryReadiness
        ? inventoryDiagnosticInspection(inventoryReadiness)
        : null,
      migrationTask: "INVENTORY",
      profile: sourceProfile,
    });
  }

  async function exportSourceAdapterSupportPackageFor({
    inspection,
    migrationTask,
    profile,
  }) {
    if (!inspection) return fail("请先识别当前三方 HIS 数据结构");
    const scope = sourceMappingProfileScope({
      adapterId: inspection.adapterId,
      profile,
      sourceMode: "database",
      targetTenantId: targetAuth?.tenantId || tenantId || loginTenantId,
    });
    if (!scope) return fail("当前来源身份尚未建立，请重新识别数据结构");
    try {
      const exported = await command("export_source_adapter_support_package", {
        request: { scope, migrationTask },
      });
      const blob = new Blob([exported.content], {
        type: "application/json;charset=utf-8",
      });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = exported.fileName;
      document.body.appendChild(anchor);
      anchor.click();
      anchor.remove();
      URL.revokeObjectURL(url);
      notify(
        `适配器支持包已导出，共 ${exported.diagnosticCount} 次结构变化；包含脱敏表字段结构，不含连接信息、账号密码、目标租户或业务数据`,
      );
      return exported;
    } catch (error) {
      fail(error);
      return null;
    }
  }

  function exportSourceAdapterSupportPackage() {
    return exportSourceAdapterSupportPackageFor({
      inspection: sourceAdapterInspection,
      migrationTask: "MEDICINE_BASE",
      profile: sourceProfile,
    });
  }

  function exportInventorySourceAdapterSupportPackage() {
    return exportSourceAdapterSupportPackageFor({
      inspection: inventoryReadiness,
      migrationTask: "INVENTORY",
      profile: sourceProfile,
    });
  }

  async function previewSourceMappingTemplateFile(file) {
    if (!file) return;
    const scope = currentSourceMappingProfileScope();
    if (!scope) return fail("请先连接当前来源和目标租户，再导入来源模板");
    try {
      const content = await file.text();
      const preview = await command("preview_source_mapping_template", {
        request: {
          scope,
          content,
          adapterVersion: sourceAdapterVersionFor(scope.adapterId),
          templateCompatibleFromVersion: sourceAdapterCompatibleFromFor(
            scope.adapterId,
          ),
          dictionaryScopeKey: currentSourceUsesScopedDictionaries()
            ? currentDictionaryScopeKey
            : "",
        },
      });
      setSourceTemplateImport({ fileName: file.name, content, preview });
    } catch (error) {
      setSourceTemplateImport(null);
      fail(error);
    }
  }

  async function confirmSourceMappingTemplateImport() {
    if (!sourceTemplateImport) return;
    const scope = currentSourceMappingProfileScope();
    if (!scope) return fail("当前来源身份已变化，请重新选择模板文件");
    setBusy("source-template-import");
    try {
      const imported = await command("import_source_mapping_template", {
        request: {
          scope,
          content: sourceTemplateImport.content,
          adapterVersion: sourceAdapterVersionFor(scope.adapterId),
          templateCompatibleFromVersion: sourceAdapterCompatibleFromFor(
            scope.adapterId,
          ),
          dictionaryScopeKey: currentSourceUsesScopedDictionaries()
            ? currentDictionaryScopeKey
            : "",
        },
      });
      let effectiveImported = imported;
      if (
        activeSourceAdapterId === "GENERIC_DATABASE" &&
        sourceMode === "database"
      ) {
        const preservedRequest = {
          adapterVersion: imported.adapterVersion,
          sourceKey: imported.sourceKey,
          sourceKeyFields: imported.sourceKeyFields || [],
          sourceQuery: query.trim(),
          mapping: imported.mapping,
          rules: imported.rules,
          dictionaryOverrides: imported.dictionaryOverrides || {},
        };
        effectiveImported = await command("save_source_mapping_profile", {
          request: { scope, ...preservedRequest },
        });
        const baseScope = sourceMappingProfileScope({
          adapterId: scope.adapterId,
          profile: sourceProfile,
          sourceMode,
          targetTenantId: targetAuth?.tenantId || tenantId || loginTenantId,
        });
        if (baseScope && baseScope.sourceIdentity !== scope.sourceIdentity) {
          await command("save_source_mapping_profile", {
            request: { scope: baseScope, ...preservedRequest },
          });
        }
      } else if (imported.sourceQuery) {
        setQuery(imported.sourceQuery);
      }
      setSourceDictionaryOverrides(effectiveImported.dictionaryOverrides || {});
      const restored = acceptData(rows, sourceName, {
        adapterId: activeSourceAdapterId,
        description: sourceDescription,
        columnMetadata: Object.values(columnMetadata),
        mappingProfile: effectiveImported,
        mappingPreset: activeMedicineWorkflow.mappingPreset,
        sourceSelection: activeSourceSelection,
        skipNotice: true,
      });
      notify(
        `来源模板已导入并恢复 ${restored.restoredCount} 项字段配置${restored.missingCount ? `；${restored.missingCount} 项字段与当前来源不一致，已重新建议` : ""}${effectiveImported.requiresReview ? "；适配器版本已变化，请核对后再进入校验" : ""}`,
      );
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  async function saveSourceDictionaryOverride(dictionary, items) {
    if (!dictionary?.id || !currentSourceUsesScopedDictionaries()) return;
    const nextOverrides = updateScopedDictionaryOverride(
      sourceDictionaryOverrides,
      currentDictionaryScopeKey,
      dictionary.id,
      items,
    );
    setBusy("source-dictionary-save");
    try {
      const saved = await persistSourceMappingProfile(nextOverrides);
      const savedOverrides = saved.dictionaryOverrides || nextOverrides;
      const effectiveItems =
        items === null
          ? dictionary.presetItems || []
          : savedOverrides[currentDictionaryScopeKey]?.[dictionary.id] || items;
      setSourceDictionaryOverrides(savedOverrides);
      setColumnMetadata((current) =>
        Object.fromEntries(
          Object.entries(current).map(([column, metadata]) => {
            if (metadata.sourceDictionary?.id !== dictionary.id) {
              return [column, metadata];
            }
            return [
              column,
              {
                ...metadata,
                sourceDictionary: {
                  ...metadata.sourceDictionary,
                  items: effectiveItems,
                  customized: items !== null,
                  loadStatus:
                    items === null
                      ? metadata.sourceDictionary.originalLoadStatus ||
                        "bundled"
                      : "customized",
                  loadMessage:
                    items === null
                      ? metadata.sourceDictionary.originalLoadMessage ||
                        "已恢复当前连接读取或系统预设的来源字典"
                      : `当前项目已人工调整 ${effectiveItems.length} 个来源字典项`,
                },
              },
            ];
          }),
        ),
      );
      setMappingProfileStatus({
        restored: true,
        restoredCount: Object.values(saved.mapping || completeMapping).filter(
          Boolean,
        ).length,
        missingCount: 0,
        savedAt: saved.savedAt,
      });
      notify(
        items === null
          ? `已恢复“${dictionary.name}”默认定义`
          : `已保存“${dictionary.name}”的项目级调整，推荐与校验已更新`,
      );
    } catch (error) {
      fail(error);
      throw error;
    } finally {
      setBusy("");
    }
  }

  async function continueFieldMapping() {
    if (fieldIndex < targetFields.length - 1) {
      setFieldIndex(fieldIndex + 1);
      return;
    }
    const unresolved = unresolvedDictionaryStatuses[0];
    if (unresolved) {
      setFieldIndex(unresolved.index);
      return fail(
        `映射阶段还有 ${unresolvedDictionaryValueCount} 个字典值待确认；已定位到“${unresolved.field.label}”，请完成匹配或明确忽略`,
      );
    }
    setBusy("mapping-save");
    try {
      const saved = await persistSourceMappingProfile();
      setStep(5);
      if (saved) notify("当前来源的字段映射和转换规则已保存到本机");
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  async function saveExpertMappingAndContinue() {
    const unresolved = unresolvedDictionaryStatuses[0];
    if (unresolved) {
      setExpertDictionaryFieldKey(unresolved.field.key);
      return fail(
        `映射阶段还有 ${unresolvedDictionaryValueCount} 个字典值待确认；已展开“${unresolved.field.label}”的字典配置`,
      );
    }
    setBusy("mapping-save");
    try {
      const saved = await persistSourceMappingProfile();
      setExpert(false);
      setExpertDictionaryFieldKey("");
      setStep(5);
      if (saved) notify("当前来源的字段映射和转换规则已更新");
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  function fieldRulePreview(field) {
    const sourceField = mapping[field.key];
    if (!sourceField || !rows.length) return "";
    const sampleRow = rows.find(
      (row) => `${row[sourceField] ?? ""}`.trim() !== "",
    );
    const rule = rules[field.key] || {};
    const sourceValues = [sourceField, ...(rule.additionalSourceFields || [])]
      .map((item) => sampleRow?.[item])
      .filter(
        (value) =>
          value !== null && value !== undefined && `${value}`.trim() !== "",
      );
    const original = (rule.additionalSourceFields || []).length
      ? sourceValues.join(`${rule.joinSeparator ?? ""}`)
      : sampleRow?.[sourceField];
    if (original === undefined) return "";
    const converted = applyFieldMapping(sampleRow, {
      ...rule,
      sourceField,
      transform: rule.transform || defaultTransformForField(field),
      valueMappings: parseValueMappings(rule.valueMappingsText).mappings,
    });
    return `样例：${original} → ${converted ?? "空值"}`;
  }

  function dictionaryForField(field) {
    return field?.dictionaryId ? dictionariesById[field.dictionaryId] : null;
  }

  function autoMapDictionary(field) {
    const dictionary = dictionaryForField(field);
    const sourceField = mapping[field.key];
    if (!dictionary || !sourceField) {
      return fail("请先为该目标字段选择老系统来源字段");
    }
    const additions = buildDictionaryValueMappings(
      rows.map((row) => row[sourceField]),
      dictionary.items,
      columnMetadata[sourceField]?.sourceDictionary?.items || [],
    );
    const count = Object.keys(additions).length;
    if (!count) {
      return fail(
        "没有找到含义一致的字典项；相同编码不会自动视为相同含义，请人工确认未匹配项",
      );
    }
    setRules((current) => ({
      ...current,
      [field.key]: {
        ...current[field.key],
        valueMappingsText: mergeValueMappingText(
          current[field.key]?.valueMappingsText,
          additions,
        ),
      },
    }));
    notify(
      `已为“${field.label}”生成 ${count} 条高置信语义映射；歧义项仍保留人工确认`,
    );
  }

  function setDictionaryMapping(field, sourceValue, targetValue) {
    if (!field) return;
    setRules((current) => ({
      ...current,
      [field.key]: {
        ...current[field.key],
        valueMappingsText: replaceValueMappingText(
          current[field.key]?.valueMappingsText,
          sourceValue,
          targetValue,
        ),
      },
    }));
  }

  function clearDictionaryMappings(field) {
    if (!field) return;
    setRules((current) => ({
      ...current,
      [field.key]: {
        ...current[field.key],
        valueMappingsText: "",
      },
    }));
    notify(`已清空“${field.label}”的字典转换`, "success");
  }

  function openMappingField(index, closeExpert = false) {
    setFieldIndex(index);
    if (closeExpert) {
      setExpert(false);
      setExpertDictionaryFieldKey("");
    }
  }

  async function prepareBatch() {
    if (prepareBatchLock.current) return;
    const unresolvedDictionary = mappingStatuses.find(
      (status) => status.configured && status.dictionaryPending > 0,
    );
    if (unresolvedDictionary) {
      openMappingField(unresolvedDictionary.index);
      setStep(4);
      return fail(
        `${unresolvedDictionary.field.label}还有 ${unresolvedDictionary.dictionaryPending} 个来源字典值未确认，请完成匹配或明确忽略后再校验`,
      );
    }
    const invalidMapping = targetFields
      .map((field) => ({
        field,
        parsed: parseValueMappings(rules[field.key]?.valueMappingsText),
      }))
      .find(({ parsed }) => parsed.invalidLines.length);
    if (invalidMapping) {
      return fail(
        `${invalidMapping.field.label}的值映射第 ${invalidMapping.parsed.invalidLines.join("、")} 行格式不正确，请使用“旧值 = 新值”`,
      );
    }
    const invalidAdvancedRule = targetFields
      .map((field) => ({ field, rule: rules[field.key] || {} }))
      .find(({ rule }) => {
        const operator = rule.conditionOperator || "ALWAYS";
        if (operator === "ALWAYS") return false;
        if (!rule.conditionField) return true;
        if (
          ["EQUALS", "NOT_EQUALS", "CONTAINS"].includes(operator) &&
          !`${rule.conditionValue ?? ""}`.trim()
        ) {
          return true;
        }
        return (
          rule.conditionElse === "DEFAULT" &&
          !`${rule.defaultValue ?? ""}`.trim()
        );
      });
    if (invalidAdvancedRule) {
      return fail(
        `${invalidAdvancedRule.field.label}的条件规则不完整，请补充判断字段、比较值或默认值`,
      );
    }
    const mappings = targetFields
      .filter((field) => mapping[field.key] || rules[field.key]?.defaultValue)
      .map((field) => {
        const rule = rules[field.key] || {};
        return {
          sourceField: mapping[field.key] || "",
          additionalSourceFields: (rule.additionalSourceFields || []).filter(
            Boolean,
          ),
          joinSeparator: rule.joinSeparator || "",
          targetField: field.key,
          transform: rule.transform || defaultTransformForField(field),
          maxLength: Number(rule.maxLength) || 0,
          truncateMode: rule.truncateMode || "KEEP_START",
          conditionField: rule.conditionField || "",
          conditionOperator: rule.conditionOperator || "ALWAYS",
          conditionValue: rule.conditionValue || "",
          conditionElse: rule.conditionElse || "KEEP",
          defaultValue: rule.defaultValue || "",
          valueMappings: parseValueMappings(rule.valueMappingsText).mappings,
          valueMappingCaseInsensitive:
            rule.valueMappingCaseInsensitive || false,
        };
      });
    const preparedRows = rows.map((row) => ({
      ...row,
      _sourceKey: activeAdapterProvidesSourceKey
        ? (row[activeMedicineWorkflow.sourceKeyField] ?? "")
        : encodeSourceKey(row, sourceKeyFields),
    }));
    const stableSourceName = activeSourceAdapter?.automaticDetection
      ? sourceAdapterIdentity(
          activeSourceAdapter.name,
          sourceProfile,
          sourceAdapterInspection?.schema ||
            sourceProfile.schema ||
            sourceProfile.username ||
            "SOURCE",
        )
      : sourceName;
    const declaredBatchSourceType = activeMedicineWorkflow.batchSourceType;
    prepareBatchLock.current = true;
    setBusy("prepare");
    try {
      await new Promise((resolve) =>
        window.requestAnimationFrame(() =>
          window.requestAnimationFrame(resolve),
        ),
      );
      await persistSourceMappingProfile();
      const detail = await command("prepare_migration_batch", {
        request: {
          batchName: `${sourceName}-药品迁移`,
          sourceType:
            declaredBatchSourceType && declaredBatchSourceType !== "GENERIC"
              ? declaredBatchSourceType
              : sourceMode === "database"
                ? "DATABASE"
                : "FILE",
          sourceName: stableSourceName,
          sourceDescription: sourceDescription || query,
          conflictStrategy,
          allowCreateFactory,
          idempotencyKey: objectId(),
          mappings,
          costMergeMappings,
          rows: preparedRows,
        },
      });
      setBatchDetail(detail);
      setSkipInvalidRowsOnExecute(true);
      setValidationResultFilter(
        detail.batch.failCount > 0 ? "INVALID" : "VALIDATED",
      );
      setOverwritePreview(null);
      setSelectedOverwriteRowIds([]);
      notify(`校验完成：${detail.batch.validCount} 行可迁移`);
    } catch (error) {
      fail(error);
    } finally {
      prepareBatchLock.current = false;
      setBusy("");
    }
  }

  function openValidationFieldMapping(field, row) {
    const nextIndex = targetFields.findIndex((item) => item.key === field.key);
    if (nextIndex < 0) return;
    setFieldIndex(nextIndex);
    setExpert(false);
    setValidationFieldContext({
      fieldKey: field.key,
      fieldLabel: field.label,
      rowNo: row.rowNo,
      sourceKey: row.sourceKey,
      errorMessage: row.errorMessage,
    });
    if (row.rowNo > 0 && row.rowNo <= rows.length) {
      setMappingSampleIndex(row.rowNo - 1);
    }
    setStep(4);
  }

  async function testTarget() {
    setBusy("target-test");
    try {
      const result = await command("test_database_connection", {
        profile: targetProfile,
      });
      const readiness = await command("inspect_target_schema", {
        profile: targetProfile,
      });
      await rememberTargetDatabaseConnection();
      notify(
        `${result.message} · 已核对 ${readiness.checkedTables.length} 张药品表 · ${result.latencyMs}ms`,
      );
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  async function previewOverwrite() {
    if (!batchDetail) return;
    setBusy("overwrite-preview");
    try {
      const preview = await command("preview_overwrite_batch", {
        request: {
          batchId: batchDetail.batch.batchId,
          target: targetProfile,
        },
      });
      setOverwritePreview(preview);
      setSelectedOverwriteRowIds(
        preview.rows
          .filter((row) => row.action !== "UNCHANGED")
          .map((row) => row.rowId),
      );
      notify(preview.message);
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  async function execute(failedOnly = false) {
    if (!batchDetail || migrationExecutionLock.current) return;
    if (!tenantId.trim() || !operatorId.trim())
      return fail("请填写新系统租户 ID 和操作人 ID");
    const overwrite = batchDetail.batch.conflictStrategy === "OVERWRITE";
    if (overwrite && !overwritePreview)
      return fail("覆盖迁移必须先生成并确认目标字段差异");
    if (overwrite && !selectedOverwriteRowIds.length)
      return fail("请至少勾选一条需要执行的记录");
    if (!failedOnly && formalMigrationNeedsTrial)
      return fail("请先在迁移明细中选择一条数据完成单条试迁移");
    migrationExecutionLock.current = true;
    setBusy(failedOnly ? "retry" : "execute");
    try {
      const detail = await command("execute_migration_batch", {
        request: {
          batchId: batchDetail.batch.batchId,
          target: targetProfile,
          failedOnly,
          tenantId,
          operatorId,
          organizationId: "",
          selectedRowIds: overwrite ? selectedOverwriteRowIds : [],
          overwritePreviewConfirmed: overwrite && Boolean(overwritePreview),
          skipInvalidRows: skipInvalidRowsOnExecute,
        },
      });
      setBatchDetail(detail);
      setStep(6);
      notify(failedOnly ? "失败行重试完成" : "迁移执行完成");
    } catch (error) {
      fail(error);
    } finally {
      migrationExecutionLock.current = false;
      setBusy("");
    }
  }

  async function trialMigrateRow(row) {
    if (!batchDetail || trialMigrationLock.current) return;
    if (!tenantId.trim() || !operatorId.trim())
      return fail("请先完成新系统认证并确认目标库连接");
    trialMigrationLock.current = true;
    setTrialRowId(row.rowId);
    try {
      const response = await command("trial_migration_row", {
        request: {
          batchId: batchDetail.batch.batchId,
          rowId: row.rowId,
          target: targetProfile,
          tenantId,
          operatorId,
          organizationId: "",
        },
      });
      setBatchDetail(response.detail);
      setActiveResultTab("rows");
      notify(
        response.result.message,
        response.result.ok ? "success" : "danger",
      );
    } catch (error) {
      fail(error);
    } finally {
      trialMigrationLock.current = false;
      setTrialRowId("");
    }
  }

  async function undoBatch() {
    if (!batchDetail) return;
    const confirmed = window.confirm(
      "仅会删除本批次由工具新增、且尚未被后续数据引用的记录；复用数据不会删除。确认撤销？",
    );
    if (!confirmed) return;
    setBusy("undo");
    try {
      const detail = await command("undo_migration_batch", {
        request: {
          batchId: batchDetail.batch.batchId,
          target: targetProfile,
        },
      });
      setBatchDetail(detail);
      setActiveResultTab("audit");
      notify(
        detail.batch.status === "UNDONE"
          ? "该批次新增数据已安全撤销"
          : "撤销已完成；有后续引用的数据已保留，请查看审计日志",
      );
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  return (
    <div className="app-shell">
      <Header
        runtime={runtime}
        auth={targetAuth}
        sectionLabel={
          migrationType === "INVENTORY" ? "机构库存初始化" : "药品基础数据"
        }
        connectionCount={databaseConnections.length}
        historyCount={historyBatches.length}
        onOpenConnections={() => setConnectionManagerOpen(true)}
        onOpenHistory={openMigrationHistory}
      />
      <Stepper active={step} />
      <main className="workspace" ref={workspaceRef}>
        <SystemConnectionScreen
          context={{
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
          }}
        />

        <TaskSelectionScreen
          context={{
            migrationType,
            setMigrationType,
            setStep,
            sourceAdapters,
            step,
            targetAuth,
          }}
        />

        {step === 2 && migrationType === "INVENTORY" && (
          <Suspense
            fallback={<DeferredModuleLoading label="正在打开机构库存工作台…" />}
          >
            <LazyInventoryMigrationScreen
              context={{
                busy,
                databaseConnections,
                databaseDrivers,
                driverPacks,
                executeInventoryBatch,
                exportInventorySourceAdapterDiagnostic,
                exportInventorySourceAdapterSupportPackage,
                forgetSourceConnection,
                forgetTargetDatabaseConnection,
                hasSavedSourceConnection,
                hasSavedTargetDatabase,
                inspectInventorySourceAdapter,
                inventoryBatchDetail,
                inventoryExecutionSeconds,
                inventoryLocationMappings,
                inventoryMappingExpanded,
                inventoryOrganizationMappings,
                inventoryPreflightExpanded,
                inventoryReadiness,
                inventorySourceDiagnostics,
                inventoryResolvedLocations,
                inventoryReviewConfirmed,
                inventoryTrialStatus,
                inventorySourceExpanded,
                inventoryTargetExpanded,
                inventorySourceCatalog,
                loadInventoryTargetStorages,
                migrationType,
                prepareInventoryBatch,
                previewInventoryUndo,
                rememberSourcePassword,
                rememberTargetDatabasePassword,
                inventoryRenderContext,
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
                setInventorySourceDiagnostics,
                setInventoryTargetExpanded,
                setInventorySourceCatalog,
                selectedInventoryAdapterId,
                setSelectedInventoryAdapterId,
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
                sourceAdapters,
                sourceProfile,
                step,
                targetAuth,
                targetConnectionEditing,
                targetOrganizationCatalog,
                targetProfile,
                targetStorageCatalog,
                tenantId,
                trialInventoryRow,
              }}
            />
          </Suspense>
        )}

        <MedicineSourceScreen
          context={{
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
            sourceAdapterDiagnostics,
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
            selectedSourceConnectionId,
            selectedMedicineAdapterId,
            selectedSourceObject,
            sourceObjectPreview,
            sourceObjectCount,
            sourceObjectCountBusy,
            sourceObjectSurvey,
            surveyLikelySourceObjects,
            setConnectionManagerOpen,
            setSourceAdapterInspection,
            setLegacyScope,
            setQuery,
            setRememberSourcePassword,
            setSelectedSourceConnectionId,
            setSelectedMedicineAdapterId: (adapterId) => {
              setSelectedMedicineAdapterId(adapterId);
              setSourceAdapterInspection(null);
              setSourceAdapterDiagnostics([]);
            },
            setSelectedSourceObject: (objectName) => {
              setSelectedSourceObject(objectName);
              invalidateSourceObjectPreview();
            },
            setShowCustomQuery,
            setSourceConnectionEditing,
            setSourceMode,
            setSourceProfile: (profile) => {
              setSourceProfile(profile);
              setSourceAdapterDiagnostics([]);
              setSourceObjects([]);
              setSelectedSourceObject("");
              invalidateSourceObjectPreview();
              invalidateSourceObjectSurvey();
              setQuery("");
            },
            setStep,
            showCustomQuery,
            sourceConnectionEditing,
            sourceAdapters,
            sourceMode,
            sourceObjects,
            sourceProfile,
            step,
            testSourceConnection,
          }}
        />

        <SourceRecognitionScreen
          context={{
            activeSourceAdapterId,
            beginMapping,
            columnMetadata,
            columns,
            medicinePreviewColumns,
            medicinePreviewLabels,
            mappingProfileStatus,
            sourceTemplateImport,
            cancelSourceTemplateImport: () => setSourceTemplateImport(null),
            confirmSourceMappingTemplateImport,
            exportSourceMappingTemplate,
            previewSourceMappingTemplateFile,
            rows,
            setSourceKeyFields: (nextSourceKeyFields) => {
              setSourceKeyFields(nextSourceKeyFields);
              setSourceKeyConfirmed(activeAdapterProvidesSourceKey);
            },
            setSourceKeyConfirmed,
            setStep,
            sourceDescription,
            sourceAdapters,
            sourceKeyFields,
            sourceKeyConfirmed,
            sourceName,
            step,
          }}
        />

        {step === 4 && (
          <section className="screen mapping-screen">
            <div className="mapping-top">
              <div className="mapping-progress">
                <Rows size={26} weight="duotone" />
                <div>
                  <strong>
                    来源已配置 {mappedCount} / {targetFields.length} 个目标字段
                  </strong>
                  <div className="progress-track">
                    <span
                      style={{
                        width: `${(mappedCount / targetFields.length) * 100}%`,
                      }}
                    />
                  </div>
                </div>
              </div>
              <div className="mapping-top__actions">
                <SearchableSelect
                  ariaLabel="快速跳转目标字段"
                  className="mapping-quick-jump"
                  onChange={(next) =>
                    openMappingField(
                      targetFields.findIndex((field) => field.key === next),
                    )
                  }
                  options={mappingStatuses.map((status) => ({
                    value: status.field.key,
                    label: status.field.label,
                    description: `${status.field.group} · ${status.label}`,
                    meta: status.sourceField
                      ? `来源：${status.sourceField}`
                      : targetLocations(status.field).join(" · "),
                    keywords: `${status.field.key} ${status.field.group} ${status.sourceField}`,
                  }))}
                  searchPlaceholder="按名称、字段、分组或来源查找"
                  value={currentField.key}
                />
                <button
                  className="expert-switch"
                  onClick={() => setExpert(true)}
                >
                  <Code size={19} />
                  全部字段总览
                </button>
              </div>
            </div>
            <div className="mapping-workspace">
              <FieldMappingNavigator
                currentFieldKey={currentField.key}
                onSelectField={openMappingField}
                statuses={mappingStatuses}
                targetLocations={targetLocations}
              />
              <div className="mapping-field-workbench">
                <div className="source-line">
                  <Database size={20} />
                  <span>来源：</span>
                  <strong>{sourceName}</strong>
                  <span className="source-chip">唯一标识 {sourceKey}</span>
                </div>
                {validationFieldContext?.fieldKey === currentField.key && (
                  <div className="validation-jump-context">
                    <Warning size={18} weight="fill" />
                    <div>
                      <strong>
                        从校验失败定位：第 {validationFieldContext.rowNo} 行 ·
                        来源键 {validationFieldContext.sourceKey}
                      </strong>
                      <span>{validationFieldContext.errorMessage}</span>
                    </div>
                    <button
                      className="button button--secondary"
                      type="button"
                      onClick={() => setStep(5)}
                    >
                      返回校验结果
                    </button>
                  </div>
                )}
                <div className="question-copy">
                  <div className="question-copy__meta">
                    <span className="field-group">{currentField.group}</span>
                    <div className="target-destination">
                      <span>
                        <Database size={14} weight="duotone" />
                        新系统落点
                      </span>
                      {targetLocations(currentField).map((location) => (
                        <code key={location}>{location}</code>
                      ))}
                    </div>
                  </div>
                  <h1>
                    老系统里，哪个字段代表“{currentField.label}”？
                    {currentField.required && <em>必填</em>}
                  </h1>
                  <p>{currentField.hint}</p>
                </div>
                {currentDictionary && (
                  <div className="dictionary-guide">
                    <div>
                      <strong>新系统标准字典</strong>
                      <code>{currentDictionary.dicId}</code>
                      <span>{currentDictionary.items.length} 个可用值</span>
                    </div>
                    <div className="dictionary-guide__items">
                      {currentDictionary.items.slice(0, 12).map((item) => (
                        <span key={item.id || dictionaryItemValue(item)}>
                          <b>{dictionaryItemValue(item)}</b>
                          {item.text || item.na}
                        </span>
                      ))}
                      {currentDictionary.items.length > 12 && (
                        <em>另有 {currentDictionary.items.length - 12} 项</em>
                      )}
                    </div>
                    <button
                      className="button button--secondary"
                      type="button"
                      disabled={!mapping[currentField.key]}
                      onClick={() => autoMapDictionary(currentField)}
                    >
                      <LinkSimple size={17} />
                      按含义自动生成转换
                    </button>
                  </div>
                )}
                <div className="suggestion-heading">
                  <span>智能推荐</span>
                  <small>只展示字段名或别名达到可信阈值的老系统字段</small>
                  <Info size={15} />
                </div>
                <div className="suggestions">
                  {suggestions.map((item, index) => (
                    <button
                      className={`suggestion-row ${mapping[currentField.key] === item.column ? "suggestion-row--selected" : ""}`}
                      onClick={() =>
                        setMapping((current) => ({
                          ...current,
                          [currentField.key]: item.column,
                        }))
                      }
                      key={item.column}
                    >
                      <span className="radio">
                        {mapping[currentField.key] === item.column && <i />}
                      </span>
                      <span className="suggestion-field">
                        <strong>
                          {sourceFieldDisplayName(
                            item.column,
                            columnMetadata[item.column],
                          )}
                        </strong>
                        <small>
                          {sourceFieldPhysicalOrigin(
                            columnMetadata[item.column],
                          )
                            ? `来源：${sourceFieldPhysicalOrigin(columnMetadata[item.column])}`
                            : "来源数据字段"}
                        </small>
                      </span>
                      <span>
                        <small>当前药品样例</small>
                        {sourceValueLabel(
                          mappingSample[item.column],
                          columnMetadata[item.column],
                        )}
                      </span>
                      <b
                        className={
                          index === 0
                            ? "badge badge--good"
                            : "badge badge--possible"
                        }
                      >
                        {index === 0 ? "推荐" : "可能匹配"}
                      </b>
                      <strong className="score">{item.score}%</strong>
                    </button>
                  ))}
                  {!suggestions.length && (
                    <div className="suggestions-empty">
                      <Info size={18} />
                      <span>
                        <strong>没有达到可信阈值的推荐字段</strong>
                        请在下方按字段注释、物理来源和当前药品样例人工选择。
                      </span>
                    </div>
                  )}
                </div>
                <div className="inline-select">
                  <span>没有合适的推荐？</span>
                  <SearchableSelect
                    ariaLabel={`${currentField.label}来源字段`}
                    value={mapping[currentField.key] || ""}
                    onChange={(next) =>
                      setMapping((current) => ({
                        ...current,
                        [currentField.key]: next,
                      }))
                    }
                    options={[
                      {
                        value: "",
                        label: "不映射 / 稍后使用默认值",
                      },
                      ...sourceFieldOptions,
                    ]}
                    searchPlaceholder="按字段名、注释、表名或样例值过滤"
                  />
                </div>
                {currentDictionary && currentSourceField && (
                  <DictionaryMappingEditor
                    dictionaryScopeLabel={currentDictionaryScopeLabel}
                    dictionary={currentDictionary}
                    field={currentField}
                    onAutoMap={() => autoMapDictionary(currentField)}
                    onChange={(sourceValue, targetValue) =>
                      setDictionaryMapping(
                        currentField,
                        sourceValue,
                        targetValue,
                      )
                    }
                    onClear={() => clearDictionaryMappings(currentField)}
                    onSaveSourceDictionary={(items) =>
                      saveSourceDictionaryOverride(
                        currentSourceDictionary,
                        items,
                      )
                    }
                    rows={currentDictionaryRows}
                    sourceDictionary={currentSourceDictionary}
                    valueMappingsText={
                      rules[currentField.key]?.valueMappingsText
                    }
                  />
                )}
                <div className="sample-toolbar">
                  <div>
                    <strong>预览哪一条待迁移药品？</strong>
                    <span>推荐和即时预览都会跟随此处切换</span>
                  </div>
                  <button
                    className="button button--secondary"
                    disabled={rows.length < 2}
                    onClick={() =>
                      setMappingSampleIndex((current) =>
                        randomRowIndex(rows.length, current),
                      )
                    }
                    type="button"
                  >
                    <ArrowCounterClockwise size={17} />
                    随机换一条
                  </button>
                  <SearchableSelect
                    ariaLabel="指定预览药品"
                    className="sample-row-select"
                    value={`${mappingSampleIndex}`}
                    onChange={(next) => setMappingSampleIndex(Number(next))}
                    options={sampleRowOptions}
                    searchPlaceholder="按药品名、规格、厂家或来源键查找"
                  />
                </div>
                <div className="preview-card">
                  <div className="preview-card__heading">
                    <LinkSimple size={17} />
                    <strong>即时预览</strong>
                    <span>
                      {mapping[currentField.key]
                        ? sourceFieldDisplayName(
                            mapping[currentField.key],
                            columnMetadata[mapping[currentField.key]],
                          )
                        : "未选择来源字段"}{" "}
                      → {currentField.key}
                    </span>
                  </div>
                  <div className="preview-medicine-context">
                    {sampleContext.length ? (
                      sampleContext.map(([label, value]) => (
                        <span key={label}>
                          <small>{label}</small>
                          <strong>{value}</strong>
                        </span>
                      ))
                    ) : (
                      <span>
                        <small>待迁移记录</small>
                        <strong>
                          {medicineSampleLabel(
                            mappingSample,
                            mappingSampleIndex,
                          )}
                        </strong>
                      </span>
                    )}
                  </div>
                  <div className="preview-conversion">
                    <div>
                      <span>
                        {activeSourceAdapter?.automaticDetection
                          ? `${activeSourceAdapter.name}来源值`
                          : "三方来源值"}
                        {currentMappingPreview?.sourceDictionaryName
                          ? ` · ${currentMappingPreview.sourceDictionaryName}`
                          : ""}
                      </span>
                      <strong>
                        {currentMappingPreview
                          ? currentMappingPreview.sourceDictionaryText
                            ? `${currentMappingPreview.sourceDictionaryText}（${currentMappingPreview.original}）`
                            : `${currentMappingPreview.original ?? "空值"}`
                          : "尚未选择来源字段"}
                      </strong>
                    </div>
                    <ArrowRight size={20} />
                    <div>
                      <span>清洗 / 转换后</span>
                      <strong>
                        {currentMappingPreview
                          ? currentMappingPreview.ignored
                            ? "已忽略，不写入新系统"
                            : `${currentMappingPreview.converted ?? "空值"}`
                          : "—"}
                      </strong>
                    </div>
                    {currentDictionary && (
                      <div className="preview-dictionary-value">
                        <span>新系统字典含义</span>
                        <strong>
                          {currentMappingPreview?.ignored
                            ? "该来源值已确认忽略，不参与目标字典校验"
                            : currentMappingPreview?.dictionaryText
                              ? `${currentMappingPreview.dictionaryText}（${currentMappingPreview.dictionaryCode}）`
                              : currentMappingPreview
                                ? "当前值尚未匹配到新系统字典"
                                : "选择来源字段后显示"}
                        </strong>
                      </div>
                    )}
                  </div>
                </div>
                <div className="screen-actions">
                  <button
                    className="button button--secondary"
                    onClick={() =>
                      fieldIndex ? setFieldIndex(fieldIndex - 1) : setStep(3)
                    }
                  >
                    <ArrowLeft />
                    上一个
                  </button>
                  <div>
                    <button
                      className="button button--ghost"
                      onClick={() =>
                        setMapping((current) => ({
                          ...current,
                          [currentField.key]: "",
                        }))
                      }
                    >
                      此字段先不匹配
                    </button>
                    <button
                      className="button button--primary"
                      disabled={busy === "mapping-save"}
                      onClick={continueFieldMapping}
                    >
                      {busy === "mapping-save"
                        ? "正在固化映射…"
                        : fieldIndex < targetFields.length - 1
                          ? `确认，继续匹配${targetFields[fieldIndex + 1].label}`
                          : unresolvedDictionaryValueCount
                            ? `处理 ${unresolvedDictionaryValueCount} 个待确认字典值`
                            : "完成映射，进入校验"}
                      <ArrowRight />
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </section>
        )}

        {step === 5 && (
          <section className="screen">
            <div className="screen-heading">
              <span className="eyebrow">第 6 步 · 校验与修正</span>
              <h1>先试迁移，再决定是否正式写入</h1>
              <p>
                此阶段只把原始数据、转换结果和问题写入本地暂存库，不会修改新系统业务表。
              </p>
            </div>
            <div className="rules-layout">
              <div className="rules-card cost-merge-card">
                <div className="section-title">
                  <LinkSimple size={21} />
                  <div>
                    <strong>药品类型 → 费用归并</strong>
                    <small>
                      费用归并不是标准字典；手动确认后会按当前新系统租户自动保存在本机，下次登录优先恢复
                    </small>
                  </div>
                </div>
                <div className="cost-merge-grid">
                  {(articleTypeDictionary?.items || []).map((article) => {
                    const articleKey = dictionaryItemValue(article);
                    return (
                      <label key={articleKey}>
                        <span>
                          <b>{articleKey}</b>
                          {article.text || article.na}
                        </span>
                        <SearchableSelect
                          ariaLabel={`${article.text || article.na}费用归并`}
                          value={costMergeMappings[articleKey] || ""}
                          onChange={(next) =>
                            updateCostMergeMapping(articleKey, next)
                          }
                          options={[
                            {
                              value: "",
                              label: "未设置，相关药品将校验失败",
                            },
                            ...(costMergeCatalog?.items || []).map((cost) => ({
                              value: cost.key,
                              label: `${cost.text} · ${cost.key}`,
                            })),
                          ]}
                          searchPlaceholder="过滤费用归并"
                        />
                      </label>
                    );
                  })}
                </div>
              </div>
              <div className="rules-card">
                <div className="section-title">
                  <Gear size={21} />
                  <div>
                    <strong>迁移策略</strong>
                    <small>这些设置会随批次进入审计记录</small>
                  </div>
                </div>
                <label className="option-row">
                  <span>
                    <strong>目标重复时</strong>
                    <small>
                      增量模式会跳过来源未变化的数据；检测到来源已变化时阻止静默覆盖。
                      二系列药品按名称、规格、最小单位自动合并，来源映射仍逐条保留
                    </small>
                  </span>
                  <SearchableSelect
                    ariaLabel="目标重复时"
                    value={conflictStrategy}
                    onChange={setConflictStrategy}
                    options={[
                      {
                        value: "INCREMENTAL",
                        label: "新增增量迁移（推荐）",
                      },
                      { value: "FAIL", label: "发现重复即报错" },
                      {
                        value: "OVERWRITE",
                        label: "覆盖迁移（保存原值，可撤销）",
                      },
                    ]}
                    searchPlaceholder="过滤迁移策略"
                  />
                </label>
                {conflictStrategy === "OVERWRITE" && (
                  <div className="strategy-warning">
                    仅覆盖本工具台账确认管理的药品和商品；共享厂家、包装单位不会直接改写。
                    修改前字段值会进入审计快照，撤销时自动恢复。
                  </div>
                )}
                <label className="option-row">
                  <span>
                    <strong>
                      {activeMedicineWorkflow.factoryPolicy ===
                      "ADAPTER_MANAGED"
                        ? "同步缺失的生产厂家基础数据"
                        : "未匹配到生产厂家"}
                    </strong>
                    <small>
                      {activeUsesPhis27Preset
                        ? "按 YK_YPCD.YPCD 关联 YK_CDDZ，厂家先迁入 HI_BD_FAC，商品再引用新厂家主键"
                        : "关闭时该行失败，不会静默制造厂家脏数据"}
                    </small>
                  </span>
                  <input
                    className="switch"
                    type="checkbox"
                    checked={allowCreateFactory}
                    disabled={
                      activeMedicineWorkflow.factoryPolicy === "ADAPTER_MANAGED"
                    }
                    onChange={(event) =>
                      setAllowCreateFactory(event.target.checked)
                    }
                  />
                </label>
              </div>
              <div className="rules-card">
                <div className="section-title">
                  <FloppyDisk size={21} />
                  <div>
                    <strong>字段转换</strong>
                    <small>
                      支持清洗、组合、长度限制、简单条件、默认值和值字典映射
                    </small>
                  </div>
                </div>
                <div className="rule-list">
                  {targetFields
                    .filter(
                      (field) =>
                        mapping[field.key] ||
                        field.required ||
                        rules[field.key]?.defaultValue ||
                        rules[field.key]?.valueMappingsText,
                    )
                    .map((field) => (
                      <div className="rule-row" key={field.key}>
                        <span>
                          <strong>
                            {field.label}
                            {field.required && <em>*</em>}
                            <span className="rule-target-location">
                              （{targetPhysicalLocationText(field)}）
                            </span>
                          </strong>
                          <small>
                            {mapping[field.key]
                              ? `来自 ${sourceFieldDisplayName(mapping[field.key], columnMetadata[mapping[field.key]])}${sourceFieldPhysicalOrigin(columnMetadata[mapping[field.key]]) ? `（${sourceFieldPhysicalOrigin(columnMetadata[mapping[field.key]])}）` : ""}${fieldRulePreview(field) ? `；${fieldRulePreview(field)}` : ""}`
                              : "尚未匹配来源字段"}
                          </small>
                        </span>
                        <SearchableSelect
                          ariaLabel={`${field.label}转换规则`}
                          value={
                            rules[field.key]?.transform ||
                            defaultTransformForField(field)
                          }
                          onChange={(next) =>
                            setRules((current) => ({
                              ...current,
                              [field.key]: {
                                ...current[field.key],
                                transform: next,
                              },
                            }))
                          }
                          options={[
                            { value: "TRIM", label: "清理首尾空格" },
                            {
                              value: "COLLAPSE_WHITESPACE",
                              label: "合并连续空格",
                            },
                            {
                              value: "REMOVE_WHITESPACE",
                              label: "移除全部空格",
                            },
                            { value: "INTEGER", label: "转为整数" },
                            { value: "DECIMAL", label: "转为数字" },
                            {
                              value: "BOOLEAN_01",
                              label: "常见标志转 1/0（含 1/2、RX/OTC）",
                            },
                            { value: "UPPER", label: "转大写" },
                            { value: "LOWER", label: "转小写" },
                            {
                              value: "DATE_YYYY_MM_DD",
                              label: "日期转 YYYY-MM-DD",
                            },
                          ]}
                          searchPlaceholder="过滤转换规则"
                        />
                        {dictionaryForField(field) ? (
                          <SearchableSelect
                            ariaLabel={`${field.label}缺失时默认值`}
                            value={rules[field.key]?.defaultValue || ""}
                            onChange={(next) =>
                              setRules((current) => ({
                                ...current,
                                [field.key]: {
                                  ...current[field.key],
                                  defaultValue: next,
                                },
                              }))
                            }
                            options={[
                              {
                                value: "",
                                label: "缺失时不设置默认值",
                              },
                              ...dictionaryForField(field).items.map(
                                (item) => ({
                                  value: dictionaryItemValue(item),
                                  label: `${dictionaryItemValue(item)} · ${item.text || item.na}`,
                                }),
                              ),
                            ]}
                            searchPlaceholder="过滤字典默认值"
                          />
                        ) : (
                          <input
                            placeholder="缺失时使用默认值"
                            value={rules[field.key]?.defaultValue || ""}
                            onChange={(event) =>
                              setRules((current) => ({
                                ...current,
                                [field.key]: {
                                  ...current[field.key],
                                  defaultValue: event.target.value,
                                },
                              }))
                            }
                          />
                        )}
                        <FieldAdvancedRuleEditor
                          fieldLabel={field.label}
                          primarySourceField={mapping[field.key] || ""}
                          rule={rules[field.key] || {}}
                          sourceOptions={sourceFieldOptions}
                          onChange={(nextRule) =>
                            setRules((current) => ({
                              ...current,
                              [field.key]: nextRule,
                            }))
                          }
                        />
                        <details className="value-mapping-editor">
                          <summary>
                            <span>字段值映射（旧值 → 新值）</span>
                            <small>
                              {Object.keys(
                                parseValueMappings(
                                  rules[field.key]?.valueMappingsText,
                                ).mappings,
                              ).length || "未配置"}
                            </small>
                          </summary>
                          <div>
                            {dictionaryForField(field) && (
                              <div className="dictionary-editor-head">
                                <span>
                                  目标字典：{field.dictionaryId} ·{" "}
                                  {dictionaryForField(field).items.length} 项
                                </span>
                                <button
                                  type="button"
                                  className="button button--ghost"
                                  onClick={() => autoMapDictionary(field)}
                                >
                                  自动匹配当前来源值
                                </button>
                              </div>
                            )}
                            <textarea
                              rows={4}
                              placeholder={
                                "西药 = 1\n中成药 = 2\n无需迁移 = <忽略>"
                              }
                              value={rules[field.key]?.valueMappingsText || ""}
                              onChange={(event) =>
                                setRules((current) => ({
                                  ...current,
                                  [field.key]: {
                                    ...current[field.key],
                                    valueMappingsText: event.target.value,
                                  },
                                }))
                              }
                            />
                            <label>
                              <input
                                type="checkbox"
                                checked={
                                  rules[field.key]
                                    ?.valueMappingCaseInsensitive || false
                                }
                                onChange={(event) =>
                                  setRules((current) => ({
                                    ...current,
                                    [field.key]: {
                                      ...current[field.key],
                                      valueMappingCaseInsensitive:
                                        event.target.checked,
                                    },
                                  }))
                                }
                              />
                              英文字母忽略大小写
                            </label>
                            <small>
                              每行一条，格式为“旧值 =
                              新值”；如需明确忽略某个来源值，可填写“旧值 =
                              &lt;忽略&gt;”。
                            </small>
                          </div>
                        </details>
                      </div>
                    ))}
                </div>
              </div>
            </div>
            {batchDetail && (
              <>
                <SummaryCards batch={batchDetail.batch} />
                <ValidationResults
                  detail={batchDetail}
                  filter={validationResultFilter}
                  onFilterChange={setValidationResultFilter}
                  onEditField={openValidationFieldMapping}
                />
              </>
            )}
            {busy === "prepare" && (
              <div
                className="validation-running"
                role="status"
                aria-live="polite"
              >
                <CircleNotch className="is-spinning" size={25} weight="bold" />
                <div>
                  <strong>正在校验 {rows.length} 条待迁移数据</strong>
                  <span>
                    正在逐行转换、核对字典并保存校验结果，已用时{" "}
                    {prepareElapsedSeconds} 秒
                  </span>
                </div>
                <small>窗口会保持响应，请勿重复点击或关闭应用</small>
              </div>
            )}
            {batchDetail && validationFailureCount > 0 && (
              <label className="validation-skip-choice">
                <input
                  type="checkbox"
                  checked={skipInvalidRowsOnExecute}
                  onChange={(event) =>
                    setSkipInvalidRowsOnExecute(event.target.checked)
                  }
                />
                <span>
                  <strong>
                    仅迁移校验通过的 {batchDetail.batch.validCount} 条数据
                  </strong>
                  <small>
                    跳过 {validationFailureCount}{" "}
                    条校验失败数据；保留完整失败原因并写入本地审计，目标库不会写入这些记录。
                  </small>
                </span>
                <em>跳过 {validationFailureCount} 条</em>
              </label>
            )}
            <div className="screen-actions">
              <button
                className="button button--secondary"
                disabled={busy === "prepare"}
                onClick={() => setStep(4)}
              >
                <ArrowLeft />
                返回映射
              </button>
              <div>
                {batchDetail && (
                  <button
                    className="button button--secondary"
                    disabled={busy === "prepare"}
                    onClick={() => setBatchDetail(null)}
                  >
                    调整后重新校验
                  </button>
                )}
                <button
                  className="button button--primary"
                  disabled={
                    busy === "prepare" ||
                    (Boolean(batchDetail) && !batchDetail.batch.validCount) ||
                    (validationFailureCount > 0 && !skipInvalidRowsOnExecute)
                  }
                  onClick={batchDetail ? () => setStep(6) : prepareBatch}
                >
                  {busy === "prepare"
                    ? `正在校验 ${prepareElapsedSeconds} 秒`
                    : batchDetail
                      ? validationFailureCount > 0
                        ? `确认跳过 ${validationFailureCount} 条，配置目标库`
                        : "确认结果，配置目标库"
                      : "开始试迁移校验"}
                  <ArrowRight />
                </button>
              </div>
            </div>
          </section>
        )}

        {step === 6 && (
          <section className="screen">
            <div className="screen-heading screen-heading--row">
              <div>
                <span className="eyebrow">第 7 步 · 执行与审计</span>
                <h1>确认目标库，正式写入药品表</h1>
                <p>
                  每行使用独立事务。失败行不会影响成功行，可修正后单独重试。
                </p>
              </div>
              {batchDetail && (
                <span
                  className={`batch-status status-pill--${statusMeta(batchDetail.batch.status)[1]}`}
                >
                  {statusMeta(batchDetail.batch.status)[0]}
                </span>
              )}
            </div>
            <SummaryCards batch={batchDetail?.batch} />
            {batchDetail && validationFailureCount > 0 && (
              <div className="execution-scope-notice">
                <CheckCircle size={21} weight="fill" />
                <div>
                  <strong>
                    本次只写入 {batchDetail.batch.validCount} 条校验通过数据
                  </strong>
                  <span>
                    {validationFailureCount}{" "}
                    条校验失败数据将在执行时记为“已跳过”，不会连接目标表执行写入。
                  </span>
                </div>
              </div>
            )}
            {batchDetail &&
              !["SUCCESS", "PARTIAL", "UNDONE", "UNDO_PARTIAL"].includes(
                batchDetail.batch.status,
              ) && (
                <div className="target-card">
                  <ConnectionPicker
                    purpose="TARGET"
                    entries={databaseConnections}
                    selectedId={selectedTargetConnectionId}
                    onSelect={(connectionId) =>
                      selectDatabaseConnection(connectionId, "TARGET")
                    }
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
                        setOverwritePreview(null);
                        setSelectedOverwriteRowIds([]);
                      }}
                      title="新系统目标数据库"
                      drivers={databaseDrivers}
                      driverPacks={driverPacks}
                      rememberPassword={rememberTargetDatabasePassword}
                      onRememberPasswordChange={
                        setRememberTargetDatabasePassword
                      }
                      hasSavedConnection={hasSavedTargetDatabase}
                      onForgetSaved={forgetTargetDatabaseConnection}
                      showCredentialPreference={false}
                    />
                  )}
                  <div className="target-context">
                    <Field label="租户 ID">
                      <input
                        value={tenantId}
                        disabled
                        title="来自已认证的新系统租户"
                      />
                    </Field>
                    <Field label="操作人 ID">
                      <input
                        value={operatorId}
                        disabled
                        title="来自已认证的 system 用户"
                      />
                    </Field>
                  </div>
                  <div
                    className={`trial-gate ${targetTrialPassed ? "trial-gate--passed" : ""}`}
                  >
                    <ShieldCheck size={22} weight="fill" />
                    <div>
                      <strong>
                        {targetTrialPassed
                          ? "单条试迁移已通过，可以执行全量迁移"
                          : "先从迁移明细选择一条数据进行试迁移"}
                      </strong>
                      <span>
                        {targetTrialPassed
                          ? "已在当前目标库验证真实字段、约束和关联写入；测试事务已自动回滚。"
                          : "试迁移会执行完整目标写入并强制回滚，不留下测试药品；通过后才开放正式迁移。"}
                      </span>
                    </div>
                    <em>
                      {targetTrialPassed ? "已通过 · 已回滚" : "等待单条验证"}
                    </em>
                  </div>
                  <div className="target-actions">
                    <button
                      className="button button--secondary"
                      disabled={busy === "target-test"}
                      onClick={testTarget}
                    >
                      <Plugs />
                      {busy === "target-test" ? "测试中…" : "测试目标库连接"}
                    </button>
                    {batchDetail.batch.conflictStrategy === "OVERWRITE" && (
                      <button
                        className="button button--secondary"
                        disabled={busy === "overwrite-preview"}
                        onClick={previewOverwrite}
                      >
                        <ListMagnifyingGlass />
                        {busy === "overwrite-preview"
                          ? "正在读取差异…"
                          : overwritePreview
                            ? "重新读取覆盖差异"
                            : "读取并确认覆盖差异"}
                      </button>
                    )}
                    <button
                      className="button button--danger"
                      disabled={
                        busy === "execute" ||
                        Boolean(trialRowId) ||
                        formalMigrationNeedsTrial ||
                        !batchDetail.batch.validCount ||
                        (batchDetail.batch.conflictStrategy === "OVERWRITE" &&
                          (!overwritePreview ||
                            !selectedOverwriteRowIds.length))
                      }
                      onClick={() => execute(false)}
                    >
                      <Play weight="fill" />
                      {busy === "execute"
                        ? "正在逐行写入…"
                        : formalMigrationNeedsTrial
                          ? "请先完成单条试迁移"
                          : batchDetail.batch.status === "RUNNING"
                            ? `重新执行中断批次 ${batchDetail.batch.validCount} 行`
                            : batchDetail.batch.conflictStrategy === "OVERWRITE"
                              ? `执行已确认的 ${selectedOverwriteRowIds.length} 行`
                              : validationFailureCount > 0
                                ? `正式迁移 ${batchDetail.batch.validCount} 行（跳过 ${validationFailureCount} 行）`
                                : `正式迁移 ${batchDetail.batch.validCount} 行`}
                    </button>
                  </div>
                </div>
              )}
            {overwritePreview &&
              batchDetail?.batch.conflictStrategy === "OVERWRITE" &&
              !["SUCCESS", "UNDONE", "UNDO_PARTIAL"].includes(
                batchDetail.batch.status,
              ) && (
                <div className="overwrite-preview-card">
                  <div className="overwrite-preview-heading">
                    <div>
                      <span className="eyebrow">覆盖前确认</span>
                      <h2>逐条核对目标库字段变化</h2>
                      <p>{overwritePreview.message}</p>
                    </div>
                    <label>
                      <input
                        type="checkbox"
                        checked={
                          selectedOverwriteRowIds.length > 0 &&
                          selectedOverwriteRowIds.length ===
                            overwritePreview.rows.filter(
                              (row) => row.action !== "UNCHANGED",
                            ).length
                        }
                        onChange={(event) =>
                          setSelectedOverwriteRowIds(
                            event.target.checked
                              ? overwritePreview.rows
                                  .filter((row) => row.action !== "UNCHANGED")
                                  .map((row) => row.rowId)
                              : [],
                          )
                        }
                      />
                      全选有变化记录
                    </label>
                  </div>
                  <div className="overwrite-preview-list">
                    {overwritePreview.rows.map((row) => {
                      const selected = selectedOverwriteRowIds.includes(
                        row.rowId,
                      );
                      return (
                        <article
                          key={row.rowId}
                          className={selected ? "selected" : ""}
                        >
                          <header>
                            <label>
                              <input
                                type="checkbox"
                                disabled={row.action === "UNCHANGED"}
                                checked={selected}
                                onChange={(event) =>
                                  setSelectedOverwriteRowIds((current) =>
                                    event.target.checked
                                      ? [...new Set([...current, row.rowId])]
                                      : current.filter(
                                          (id) => id !== row.rowId,
                                        ),
                                  )
                                }
                              />
                              <strong>
                                #{row.rowNo} · {row.sourceKey}
                              </strong>
                            </label>
                            <span
                              className={`diff-action diff-action--${row.action.toLowerCase()}`}
                            >
                              {row.action === "INSERT"
                                ? "新增"
                                : row.action === "UPDATE"
                                  ? `覆盖 ${row.changes.length} 项`
                                  : "无变化"}
                            </span>
                          </header>
                          <p>{row.message}</p>
                          {row.changes.length > 0 && (
                            <div className="field-diff-list">
                              {row.changes.map((change) => (
                                <div key={`${change.table}.${change.column}`}>
                                  <strong>{change.label}</strong>
                                  <code>{diffValue(change.before)}</code>
                                  <ArrowRight />
                                  <code>{diffValue(change.after)}</code>
                                </div>
                              ))}
                            </div>
                          )}
                        </article>
                      );
                    })}
                  </div>
                </div>
              )}
            {batchDetail && (
              <div className="result-panel">
                <div className="result-tabs">
                  <button
                    className={activeResultTab === "rows" ? "active" : ""}
                    onClick={() => setActiveResultTab("rows")}
                  >
                    迁移明细
                  </button>
                  <button
                    className={activeResultTab === "audit" ? "active" : ""}
                    onClick={() => setActiveResultTab("audit")}
                  >
                    审计日志 <span>{batchDetail.audits.length}</span>
                  </button>
                  <div />
                  {batchDetail.batch.failCount > 0 && (
                    <button
                      className="retry-button"
                      disabled={
                        busy === "retry" ||
                        ["UNDONE", "UNDO_PARTIAL"].includes(
                          batchDetail.batch.status,
                        )
                      }
                      onClick={() => execute(true)}
                    >
                      {busy === "retry" ? "重试中…" : "仅重试失败行"}
                    </button>
                  )}
                  {["SUCCESS", "PARTIAL"].includes(
                    batchDetail.batch.status,
                  ) && (
                    <button
                      className="undo-button"
                      disabled={busy === "undo"}
                      onClick={undoBatch}
                    >
                      <ArrowCounterClockwise />
                      {busy === "undo" ? "撤销中…" : "撤销本批次新增数据"}
                    </button>
                  )}
                </div>
                {activeResultTab === "rows" ? (
                  <div
                    className="result-table"
                    role="region"
                    aria-label="迁移明细列表"
                    tabIndex={0}
                  >
                    <div className="result-table__row result-table__head">
                      <span>来源行</span>
                      <span>状态</span>
                      <span>药品主键</span>
                      <span>商品主键</span>
                      <span>结果说明</span>
                      <span>单条试迁移</span>
                    </div>
                    {batchDetail.rows.map((row) => {
                      const trialAudit = trialResultByRow.get(row.rowId);
                      const trialRunning = trialRowId === row.rowId;
                      const trialEligible = ["VALIDATED", "FAILED"].includes(
                        row.status,
                      );
                      return (
                        <div className="result-table__row" key={row.rowId}>
                          <span>
                            #{row.rowNo} · {row.sourceKey}
                          </span>
                          <span>
                            <i
                              className={`status-dot status-dot--${statusMeta(row.status)[1]}`}
                            />
                            {statusMeta(row.status)[0]}
                          </span>
                          <code>{row.idMed || "—"}</code>
                          <code>{row.idMedPro || "—"}</code>
                          <span title={row.errorMessage}>
                            {row.errorMessage ||
                              (row.status === "SUCCESS"
                                ? "直接写表完成"
                                : "等待执行")}
                          </span>
                          <span className="trial-row-action">
                            {trialAudit && (
                              <small
                                className={
                                  trialAudit.result === "SUCCESS"
                                    ? "trial-result trial-result--success"
                                    : "trial-result trial-result--danger"
                                }
                                title={trialAudit.message}
                              >
                                {trialAudit.result === "SUCCESS"
                                  ? "通过 · 已回滚"
                                  : "未通过 · 已回滚"}
                              </small>
                            )}
                            {trialEligible && (
                              <button
                                className="trial-row-button"
                                disabled={
                                  Boolean(trialRowId) ||
                                  busy === "execute" ||
                                  busy === "retry"
                                }
                                onClick={() => trialMigrateRow(row)}
                              >
                                {trialRunning ? (
                                  <CircleNotch className="spin" />
                                ) : (
                                  <ShieldCheck />
                                )}
                                {trialRunning
                                  ? "验证中…"
                                  : trialAudit
                                    ? "重新验证"
                                    : "试迁移"}
                              </button>
                            )}
                          </span>
                        </div>
                      );
                    })}
                  </div>
                ) : (
                  <div className="audit-timeline">
                    {batchDetail.audits.map((audit) => (
                      <div key={audit.auditId}>
                        <span
                          className={`audit-icon audit-icon--${audit.result === "FAILED" ? "danger" : "success"}`}
                        >
                          {audit.result === "FAILED" ? <X /> : <Check />}
                        </span>
                        <div>
                          <strong>
                            {audit.operation} · {audit.targetTable}
                          </strong>
                          <p>{audit.message}</p>
                          <small>
                            {new Date(audit.operatedAt).toLocaleString("zh-CN")}{" "}
                            · Trace {audit.traceId}
                          </small>
                        </div>
                        <code>{audit.targetId || "—"}</code>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}
          </section>
        )}
      </main>

      {expert && (
        <>
          <div
            className="panel-backdrop"
            onClick={() => {
              setExpert(false);
              setExpertDictionaryFieldKey("");
            }}
          />
          <aside className="expert-panel">
            <div className="expert-panel__header">
              <div>
                <span className="eyebrow">专业模式</span>
                <h2>全部字段配置总览</h2>
              </div>
              <button
                className="icon-button"
                onClick={() => {
                  setExpert(false);
                  setExpertDictionaryFieldKey("");
                }}
              >
                <X />
              </button>
            </div>
            <p>
              来源字段和字典项都可在此维护；也可直接进入任一字段的完整配置。
            </p>
            <div className="expert-overview-toolbar">
              <SearchableSelect
                ariaLabel="专业模式快速定位字段"
                onChange={(next) =>
                  openMappingField(
                    targetFields.findIndex((field) => field.key === next),
                    true,
                  )
                }
                options={mappingStatuses.map((status) => ({
                  value: status.field.key,
                  label: status.field.label,
                  description: `${status.field.group} · ${status.label}`,
                  meta: status.sourceField
                    ? `来源：${status.sourceField}`
                    : targetLocations(status.field).join(" · "),
                  keywords: `${status.field.key} ${status.field.group} ${status.sourceField}`,
                }))}
                placeholder="快速查找并进入字段详情"
                searchPlaceholder="搜索目标字段、物理字段或来源"
                value=""
              />
              <span>
                <b>
                  {
                    mappingStatuses.filter((item) => item.state === "ready")
                      .length
                  }
                </b>
                已完成
              </span>
              <span className="is-warning">
                <b>
                  {
                    mappingStatuses.filter(
                      (item) => item.state === "dictionary",
                    ).length
                  }
                </b>
                字典待确认
              </span>
              <span>
                <b>
                  {
                    mappingStatuses.filter((item) =>
                      ["pending", "optional"].includes(item.state),
                    ).length
                  }
                </b>
                未配置
              </span>
            </div>
            <div className="expert-grid">
              <div className="expert-grid__head">
                <span>新系统字段 / 物理落点</span>
                <span>三方来源字段</span>
                <span>状态</span>
                <span>配置</span>
              </div>
              {mappingStatuses.map((status) => (
                <div className="expert-field-block" key={status.field.key}>
                  <div className="expert-grid__row">
                    <button
                      className="expert-field-title"
                      onClick={() => openMappingField(status.index, true)}
                      type="button"
                    >
                      <strong>{status.field.label}</strong>
                      <code>{status.field.key}</code>
                      <span className="target-destination target-destination--compact">
                        {targetLocations(status.field).map((location) => (
                          <code key={location}>{location}</code>
                        ))}
                      </span>
                    </button>
                    <SearchableSelect
                      ariaLabel={`${status.field.label}来源字段`}
                      value={mapping[status.field.key] || ""}
                      onChange={(next) =>
                        setMapping((current) => ({
                          ...current,
                          [status.field.key]: next,
                        }))
                      }
                      options={[
                        { value: "", label: "不映射" },
                        ...sourceFieldOptions,
                      ]}
                      searchPlaceholder="按字段名、注释、表名或样例值过滤"
                    />
                    <span
                      className={`status-pill status-pill--${status.state === "ready" ? "ready" : status.state === "dictionary" ? "warning" : status.field.required ? "danger" : "muted"}`}
                    >
                      {status.label}
                    </span>
                    <span className="expert-row-actions">
                      {status.dictionary && (
                        <button
                          className="button button--secondary"
                          disabled={!status.sourceField}
                          onClick={() =>
                            setExpertDictionaryFieldKey((current) =>
                              current === status.field.key
                                ? ""
                                : status.field.key,
                            )
                          }
                          type="button"
                        >
                          字典 {status.dictionaryHandled}/
                          {status.dictionaryTotal}
                        </button>
                      )}
                      <button
                        className="button button--ghost"
                        onClick={() => openMappingField(status.index, true)}
                        type="button"
                      >
                        详细配置
                      </button>
                    </span>
                  </div>
                  {expertDictionaryFieldKey === status.field.key &&
                    status.dictionary &&
                    status.sourceField && (
                      <div className="expert-dictionary-editor">
                        <DictionaryMappingEditor
                          dictionaryScopeLabel={currentDictionaryScopeLabel}
                          dictionary={status.dictionary}
                          field={status.field}
                          onAutoMap={() => autoMapDictionary(status.field)}
                          onChange={(sourceValue, targetValue) =>
                            setDictionaryMapping(
                              status.field,
                              sourceValue,
                              targetValue,
                            )
                          }
                          onClear={() => clearDictionaryMappings(status.field)}
                          onSaveSourceDictionary={(items) =>
                            saveSourceDictionaryOverride(
                              columnMetadata[status.sourceField]
                                ?.sourceDictionary,
                              items,
                            )
                          }
                          rows={status.dictionaryRows}
                          sourceDictionary={
                            columnMetadata[status.sourceField]
                              ?.sourceDictionary || null
                          }
                          valueMappingsText={
                            rules[status.field.key]?.valueMappingsText
                          }
                        />
                      </div>
                    )}
                </div>
              ))}
            </div>
            <div className="expert-actions">
              <button
                className="button button--secondary"
                onClick={() => {
                  setExpert(false);
                  setExpertDictionaryFieldKey("");
                }}
              >
                返回引导模式
              </button>
              <button
                className="button button--primary"
                disabled={busy === "mapping-save"}
                onClick={saveExpertMappingAndContinue}
              >
                {busy === "mapping-save"
                  ? "正在固化映射…"
                  : unresolvedDictionaryValueCount
                    ? `处理 ${unresolvedDictionaryValueCount} 个待确认字典值`
                    : "保存并进入校验"}
              </button>
            </div>
          </aside>
        </>
      )}
      <ConnectionManager
        open={connectionManagerOpen}
        entries={databaseConnections}
        drivers={databaseDrivers}
        driverPacks={driverPacks}
        busy={busy}
        onClose={() => setConnectionManagerOpen(false)}
        onSave={saveManagedDatabaseConnection}
        onDelete={deleteManagedDatabaseConnection}
        onTest={testManagedDatabaseConnection}
        onUse={useDatabaseConnection}
      />
      {migrationHistoryOpen && (
        <Suspense
          fallback={<DeferredModuleLoading dialog label="正在打开迁移历史…" />}
        >
          <LazyMigrationHistory
            open
            batches={historyBatches}
            detail={historyBatchDetail}
            busy={busy}
            onClose={() => setMigrationHistoryOpen(false)}
            onRefresh={openMigrationHistory}
            onSelect={loadHistoryBatch}
          />
        </Suspense>
      )}
      {notice && (
        <div
          className={`toast toast--${notice.tone}`}
          role={notice.tone === "danger" ? "alert" : "status"}
        >
          {notice.tone === "danger" ? (
            <Warning weight="fill" />
          ) : (
            <CheckCircle weight="fill" />
          )}
          <span className="toast__message">{notice.message}</span>
          {notice.tone === "danger" && (
            <button
              type="button"
              className="toast__close"
              aria-label="关闭错误提示"
              onClick={dismissNotice}
            >
              <X size={16} />
            </button>
          )}
        </div>
      )}
    </div>
  );
}
