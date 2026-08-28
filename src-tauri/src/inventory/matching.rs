use std::cmp::Ordering;
use std::time::Instant;
use unicode_normalization::UnicodeNormalization;

fn inventory_target_medicine_from_values(
    value: impl Fn(usize) -> String,
) -> InventoryTargetMedicine {
    let private = value(10).trim() == "1";
    InventoryTargetMedicine {
        id_med_pro: value(0),
        id_med: value(1),
        drug_name: value(2),
        specification: value(3),
        minimum_unit: value(4),
        product_name: value(5),
        sale_specification: value(6),
        factory_name: value(7),
        external_code: value(8),
        approval_code: value(9),
        private,
        organization_id: if private { value(11) } else { String::new() },
    }
}

const INVENTORY_MEDICINE_CATALOG_SQL: &str =
    "SELECT p.id_med_pro,p.id_med,m.na_med,COALESCE(m.spec,''),COALESCE(m.unit_pre,''),\
     COALESCE(p.na_med_pro,''),COALESCE(p.spec_sale,''),COALESCE(f.na_fac,''),\
     COALESCE(p.cd_med_pro,''),COALESCE(p.cd_appr,''),COALESCE(p.fg_pri,'0'),\
     COALESCE(p.id_org_pri,'') FROM hi_bd_med_pro p \
     INNER JOIN hi_bd_med m ON m.id_med=p.id_med AND m.id_tet=p.id_tet \
     LEFT JOIN hi_bd_fac f ON f.id_fac=p.id_fac AND f.id_tet=p.id_tet \
     WHERE p.id_tet=? AND m.id_tet=? AND p.fg_active='1' AND m.fg_active='1'";

fn load_target_medicine_catalog_odbc(
    profile: &ConnectionProfile,
    tenant_id: &str,
) -> Result<Vec<InventoryTargetMedicine>, String> {
    with_connection(profile, |connection| {
        configure_target_session(connection, profile)?;
        let rows = query_rows_strings(
            connection,
            INVENTORY_MEDICINE_CATALOG_SQL,
            vec![tenant_id.into(), tenant_id.into()],
            50_000,
        )?;
        Ok(rows
            .into_iter()
            .map(|row| {
                inventory_target_medicine_from_values(|index| {
                    row.get(index)
                        .and_then(Clone::clone)
                        .unwrap_or_default()
                        .trim()
                        .to_string()
                })
            })
            .collect())
    })
}

async fn load_target_medicine_catalog_pg(
    profile: &ConnectionProfile,
    tenant_id: &str,
) -> Result<Vec<InventoryTargetMedicine>, String> {
    let pool = crate::pg_protocol::connect(profile).await?;
    let sql = INVENTORY_MEDICINE_CATALOG_SQL
        .replacen('?', "$1", 1)
        .replacen('?', "$2", 1);
    let result = query::<Postgres>(&sql)
        .bind(tenant_id)
        .bind(tenant_id)
        .fetch_all(&pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|row| {
                    inventory_target_medicine_from_values(|index| {
                        row.try_get::<String, _>(index)
                            .unwrap_or_default()
                            .trim()
                            .to_string()
                    })
                })
                .collect::<Vec<_>>()
        })
        .map_err(|error| format!("读取新系统药品目录失败：{error}"));
    pool.close().await;
    result
}

async fn load_target_medicine_catalog_mysql(
    profile: &ConnectionProfile,
    tenant_id: &str,
) -> Result<Vec<InventoryTargetMedicine>, String> {
    let pool = connect_mysql(profile).await?;
    let result = query::<MySql>(INVENTORY_MEDICINE_CATALOG_SQL)
        .bind(tenant_id)
        .bind(tenant_id)
        .fetch_all(&pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|row| {
                    inventory_target_medicine_from_values(|index| {
                        row.try_get::<String, _>(index)
                            .unwrap_or_default()
                            .trim()
                            .to_string()
                    })
                })
                .collect::<Vec<_>>()
        })
        .map_err(|error| format!("读取新系统药品目录失败：{error}"));
    pool.close().await;
    result
}

pub async fn load_target_medicine_catalog(
    profile: &ConnectionProfile,
    tenant_id: &str,
) -> Result<InventoryTargetMedicineCatalog, String> {
    let medicines = match inventory_target_backend(profile) {
        InventoryTargetBackend::PostgreSqlWire => {
            load_target_medicine_catalog_pg(profile, tenant_id).await?
        }
        InventoryTargetBackend::Odbc => {
            let profile = profile.clone();
            let tenant_id = tenant_id.to_string();
            tauri::async_runtime::spawn_blocking(move || {
                load_target_medicine_catalog_odbc(&profile, &tenant_id)
            })
            .await
            .map_err(|error| format!("新系统药品目录读取任务异常：{error}"))??
        }
        InventoryTargetBackend::MySql => {
            load_target_medicine_catalog_mysql(profile, tenant_id).await?
        }
    };
    if medicines.len() >= 50_000 {
        return Err(
            "新系统有效药品商品达到 50000 条，已停止目录匹配；请先清理停用或重复商品后重试"
                .into(),
        );
    }
    Ok(InventoryTargetMedicineCatalog {
        message: format!("已读取 {} 个有效新系统药品商品用于目录匹配", medicines.len()),
        medicines,
    })
}

fn medicine_match_cache_key(profile: &ConnectionProfile, tenant_id: &str) -> String {
    format!("{}::{}", tenant_id.trim(), target_identity(profile))
}

async fn build_medicine_match_index(
    engine: &InventoryMedicineMatchEngine,
    profile: &ConnectionProfile,
    tenant_id: &str,
    force_reload: bool,
) -> Result<(Arc<MedicineMatchIndex>, bool), String> {
    let cache_key = medicine_match_cache_key(profile, tenant_id);
    if !force_reload {
        if let Some(index) = engine
            .indexes
            .read()
            .map_err(|_| "药品匹配目录缓存读取失败".to_string())?
            .get(&cache_key)
            .cloned()
        {
            return Ok((index, true));
        }
    }
    let catalog = load_target_medicine_catalog(profile, tenant_id).await?;
    let index = tauri::async_runtime::spawn_blocking(move || {
        Arc::new(MedicineMatchIndex::new(catalog.medicines))
    })
    .await
    .map_err(|error| format!("新系统药品目录索引任务异常：{error}"))?;
    engine
        .indexes
        .write()
        .map_err(|_| "药品匹配目录缓存写入失败".to_string())?
        .insert(cache_key, index.clone());
    Ok((index, false))
}

pub async fn recommend_inventory_medicine_matches(
    engine: &InventoryMedicineMatchEngine,
    tenant_id: &str,
    request: RecommendInventoryMedicineMatchesRequest,
) -> Result<RecommendInventoryMedicineMatchesResponse, String> {
    if request.sources.is_empty() {
        return Err("当前批次没有需要匹配的新老药品".into());
    }
    let started = Instant::now();
    let (index, cache_hit) =
        build_medicine_match_index(engine, &request.target, tenant_id, false).await?;
    let catalog_count = index.medicines.len();
    let limit = request.limit.clamp(1, 20);
    let (suggestions, compared_count) = tauri::async_runtime::spawn_blocking(move || {
        let mut compared_count = 0usize;
        let suggestions = request
            .sources
            .into_iter()
            .map(|source| {
                let source = prepare_match_source(source);
                let candidate_indexes = index.candidate_indexes(&source);
                compared_count += candidate_indexes.len();
                let mut candidates = candidate_indexes
                    .into_iter()
                    .map(|candidate_index| {
                        score_indexed_candidate(&source, &index.medicines[candidate_index])
                    })
                    .collect::<Vec<_>>();
                candidates.sort_unstable_by(compare_match_candidates);
                candidates.truncate(limit);
                let exact_candidates = candidates
                    .iter()
                    .filter(|candidate| candidate.exact)
                    .collect::<Vec<_>>();
                let auto_candidate = (exact_candidates.len() == 1)
                    .then(|| exact_candidates[0].target.clone());
                let confidence = if auto_candidate.is_some() {
                    "EXACT"
                } else if candidates.first().is_some_and(|candidate| candidate.score >= 0.78) {
                    "HIGH"
                } else if candidates.first().is_some_and(|candidate| candidate.score >= 0.55) {
                    "MEDIUM"
                } else {
                    "LOW"
                };
                InventoryMedicineMatchSuggestion {
                    source: source.source,
                    candidates,
                    auto_candidate,
                    confidence: confidence.to_string(),
                }
            })
            .collect::<Vec<_>>();
        (suggestions, compared_count)
    })
    .await
    .map_err(|error| format!("药品相似度后台计算任务异常：{error}"))?;
    let elapsed_ms = started.elapsed().as_millis();
    Ok(RecommendInventoryMedicineMatchesResponse {
        message: format!(
            "已索引 {catalog_count} 个新系统药品商品，后台完成 {} 种老系统药品推荐",
            suggestions.len()
        ),
        suggestions,
        catalog_count,
        compared_count,
        elapsed_ms,
        cache_hit,
    })
}

pub async fn search_inventory_target_medicines(
    engine: &InventoryMedicineMatchEngine,
    tenant_id: &str,
    request: SearchInventoryTargetMedicinesRequest,
) -> Result<SearchInventoryTargetMedicinesResponse, String> {
    let (index, _) = build_medicine_match_index(engine, &request.target, tenant_id, false).await?;
    let catalog_count = index.medicines.len();
    let organization_id = request.target_organization_id;
    let query = request.query;
    let limit = request.limit;
    let medicines = tauri::async_runtime::spawn_blocking(move || {
        index.search(&organization_id, &query, limit)
    })
    .await
    .map_err(|error| format!("新系统药品目录搜索任务异常：{error}"))?;
    Ok(SearchInventoryTargetMedicinesResponse {
        message: format!("找到 {} 个可选药品商品", medicines.len()),
        medicines,
        catalog_count,
    })
}

fn inventory_row_text(row: &MigrationRow, field: &str) -> String {
    row.raw_data
        .get(field)
        .map(crate::normalize::value_text)
        .unwrap_or_default()
        .trim()
        .to_string()
}

pub async fn save_inventory_medicine_matches(
    engine: &InventoryMedicineMatchEngine,
    store: &LocalStore,
    tenant_id: &str,
    operator_id: &str,
    request: SaveInventoryMedicineMatchesRequest,
) -> Result<SaveInventoryMedicineMatchesResponse, String> {
    if request.matches.is_empty() {
        return Err("请至少确认一个药品目录匹配结果".into());
    }
    let detail = store.load_batch(&request.batch_id)?;
    if !is_inventory_batch_source_type(&detail.batch.source_type) {
        return Err("只能为机构库存首次盘点批次维护药品目录匹配".into());
    }
    let target_identity_value = target_identity(&request.target);
    let target_identity_text = target_identity_value.to_string();
    let batch_mapping = store.load_batch_mapping(&request.batch_id)?;
    if batch_mapping.get("targetIdentity") != Some(&target_identity_value) {
        return Err("目标数据库已变化，请重新读取机构库存后再匹配药品目录".into());
    }
    let (catalog, _) = build_medicine_match_index(engine, &request.target, tenant_id, false).await?;
    let candidates = catalog
        .medicines
        .iter()
        .map(|medicine| (medicine.medicine.id_med_pro.as_str(), &medicine.medicine))
        .collect::<HashMap<_, _>>();
    let mut sources = HashMap::<(String, String), &MigrationRow>::new();
    for row in &detail.rows {
        let source_product_key = inventory_row_text(row, "sourceProductKey");
        let organization_id = row
            .normalized_data
            .get("idOrg")
            .map(crate::normalize::value_text)
            .unwrap_or_default();
        if !source_product_key.is_empty() && !organization_id.is_empty() {
            sources
                .entry((source_product_key, organization_id))
                .or_insert(row);
        }
    }
    let mut saved = HashSet::new();
    for selection in &request.matches {
        let source_key = selection.source_product_key.trim();
        let target_org = selection.target_organization_id.trim();
        if !saved.insert((source_key.to_string(), target_org.to_string())) {
            return Err(format!("药品 {source_key} 在同一目标机构中存在重复匹配选择"));
        }
        let source_row = sources
            .get(&(source_key.to_string(), target_org.to_string()))
            .copied()
            .ok_or_else(|| format!("药品 {source_key} 不属于当前库存批次或目标机构已变化"))?;
        let target = candidates
            .get(selection.id_med_pro.trim())
            .copied()
            .ok_or_else(|| format!("目标药品商品 {} 已停用或不存在", selection.id_med_pro))?;
        if target.id_med != selection.id_med {
            return Err(format!(
                "目标药品商品 {} 的基础药品关系已变化，请重新读取候选",
                selection.id_med_pro
            ));
        }
        if target.private && target.organization_id != target_org {
            return Err(format!(
                "目标药品商品 {} 不属于当前目标机构",
                selection.id_med_pro
            ));
        }
        let match_method = match selection.match_method.trim() {
            "EXACT_AUTO" => "EXACT_AUTO",
            _ => "MANUAL",
        };
        let source_snapshot = serde_json::json!({
            "sourceProductKey":source_key,
            "drugName":inventory_row_text(source_row, "drugName"),
            "specification":inventory_row_text(source_row, "specification"),
            "factoryName":inventory_row_text(source_row, "factoryName"),
            "productName":inventory_row_text(source_row, "productName"),
            "targetOrganizationId":target_org
        });
        let target_snapshot = serde_json::to_value(target).map_err(|error| error.to_string())?;
        store.save_inventory_medicine_mapping(
            tenant_id,
            &detail.batch.source_name,
            source_key,
            &target_identity_text,
            target_org,
            &target.id_med,
            &target.id_med_pro,
            match_method,
            &source_snapshot,
            &target_snapshot,
        )?;
        store.audit_event(
            &request.batch_id,
            &source_row.row_id,
            "INVENTORY_MEDICINE_MATCH",
            "migration_inventory_medicine_mapping",
            &target.id_med_pro,
            "SUCCESS",
            Value::Null,
            serde_json::json!({
                "source":source_snapshot,
                "target":target_snapshot,
                "matchMethod":match_method,
                "targetIdentity":target_identity_value
            }),
            "已确认老系统药品与新系统药品商品目录关系",
            operator_id,
            &new_object_id(),
        )?;
    }
    Ok(SaveInventoryMedicineMatchesResponse {
        saved_count: saved.len(),
        message: format!("已保存 {} 项药品目录匹配，可重新核对本批库存", saved.len()),
    })
}
const INVENTORY_MATCH_CANDIDATE_POOL: usize = 600;
const INVENTORY_MATCH_MINIMUM_POOL: usize = 80;

#[derive(Debug, Clone)]
struct PreparedMatchText {
    normalized: String,
    bigrams: Vec<String>,
}

impl PreparedMatchText {
    fn new(value: &str) -> Self {
        let normalized = normalize_inventory_match_text(value);
        let mut bigrams = inventory_match_bigrams(&normalized);
        bigrams.sort_unstable();
        Self { normalized, bigrams }
    }
}

#[derive(Debug)]
struct PreparedTargetMedicine {
    medicine: InventoryTargetMedicine,
    drug_name: PreparedMatchText,
    product_name: PreparedMatchText,
    specification: PreparedMatchText,
    sale_specification: PreparedMatchText,
    factory_name: PreparedMatchText,
    search_blob: String,
}

#[derive(Debug)]
struct PreparedMatchSource {
    source: InventoryMedicineMatchSource,
    drug_name: PreparedMatchText,
    product_name: PreparedMatchText,
    specification: PreparedMatchText,
    factory_name: PreparedMatchText,
}

#[derive(Debug)]
struct MedicineMatchIndex {
    medicines: Vec<PreparedTargetMedicine>,
    name_postings: HashMap<String, Vec<usize>>,
    exact_names: HashMap<String, Vec<usize>>,
    exact_specifications: HashMap<String, Vec<usize>>,
    exact_factories: HashMap<String, Vec<usize>>,
}

fn normalize_inventory_match_text(value: &str) -> String {
    let mut normalized = String::with_capacity(value.len());
    for character in value.nfkc().flat_map(char::to_lowercase) {
        match character {
            '×' | '✕' | '＊' | '*' => normalized.push('x'),
            'μ' | 'µ' => normalized.push('u'),
            character
                if character.is_whitespace()
                    || matches!(
                        character,
                        '·' | '•' | ',' | '，' | '。' | '.' | ';' | '；' | ':' | '：'
                            | '/' | '\\' | '_' | '-' | '(' | ')' | '（' | '）' | '【' | '】'
                            | '［' | '］'
                    ) => {}
            character => normalized.push(character),
        }
    }
    normalized
}

fn inventory_match_bigrams(normalized: &str) -> Vec<String> {
    let characters = normalized.chars().collect::<Vec<_>>();
    match characters.len() {
        0 => Vec::new(),
        1 => vec![characters[0].to_string()],
        _ => characters
            .windows(2)
            .map(|pair| pair.iter().collect::<String>())
            .collect(),
    }
}

fn prepared_text_similarity(left: &PreparedMatchText, right: &PreparedMatchText) -> f64 {
    if left.normalized.is_empty() || right.normalized.is_empty() {
        return 0.0;
    }
    if left.normalized == right.normalized {
        return 1.0;
    }
    let mut left_index = 0;
    let mut right_index = 0;
    let mut overlap = 0;
    while left_index < left.bigrams.len() && right_index < right.bigrams.len() {
        match left.bigrams[left_index].cmp(&right.bigrams[right_index]) {
            Ordering::Less => left_index += 1,
            Ordering::Greater => right_index += 1,
            Ordering::Equal => {
                overlap += 1;
                left_index += 1;
                right_index += 1;
            }
        }
    }
    let pair_count = left.bigrams.len() + right.bigrams.len();
    let dice = if pair_count == 0 {
        0.0
    } else {
        (2 * overlap) as f64 / pair_count as f64
    };
    let containment = if left.normalized.contains(&right.normalized)
        || right.normalized.contains(&left.normalized)
    {
        left.normalized.chars().count().min(right.normalized.chars().count()) as f64
            / left.normalized.chars().count().max(right.normalized.chars().count()) as f64
    } else {
        0.0
    };
    dice.max(containment * 0.92)
}

fn push_exact(map: &mut HashMap<String, Vec<usize>>, value: &str, index: usize) {
    if !value.is_empty() {
        map.entry(value.to_string()).or_default().push(index);
    }
}

impl MedicineMatchIndex {
    fn new(medicines: Vec<InventoryTargetMedicine>) -> Self {
        let mut prepared = Vec::with_capacity(medicines.len());
        let mut name_postings = HashMap::<String, Vec<usize>>::new();
        let mut exact_names = HashMap::<String, Vec<usize>>::new();
        let mut exact_specifications = HashMap::<String, Vec<usize>>::new();
        let mut exact_factories = HashMap::<String, Vec<usize>>::new();
        for medicine in medicines {
            let drug_name = PreparedMatchText::new(&medicine.drug_name);
            let product_name = PreparedMatchText::new(&medicine.product_name);
            let specification = PreparedMatchText::new(&medicine.specification);
            let sale_specification = PreparedMatchText::new(&medicine.sale_specification);
            let factory_name = PreparedMatchText::new(&medicine.factory_name);
            let index = prepared.len();
            push_exact(&mut exact_names, &drug_name.normalized, index);
            push_exact(&mut exact_names, &product_name.normalized, index);
            push_exact(&mut exact_specifications, &specification.normalized, index);
            push_exact(
                &mut exact_specifications,
                &sale_specification.normalized,
                index,
            );
            push_exact(&mut exact_factories, &factory_name.normalized, index);
            let unique_name_bigrams = drug_name
                .bigrams
                .iter()
                .chain(&product_name.bigrams)
                .cloned()
                .collect::<HashSet<_>>();
            for bigram in unique_name_bigrams {
                name_postings.entry(bigram).or_default().push(index);
            }
            let search_blob = normalize_inventory_match_text(&format!(
                "{} {} {} {} {} {} {}",
                medicine.drug_name,
                medicine.product_name,
                medicine.specification,
                medicine.sale_specification,
                medicine.factory_name,
                medicine.external_code,
                medicine.approval_code
            ));
            prepared.push(PreparedTargetMedicine {
                medicine,
                drug_name,
                product_name,
                specification,
                sale_specification,
                factory_name,
                search_blob,
            });
        }
        Self {
            medicines: prepared,
            name_postings,
            exact_names,
            exact_specifications,
            exact_factories,
        }
    }

    fn eligible(&self, index: usize, organization_id: &str) -> bool {
        let medicine = &self.medicines[index].medicine;
        !medicine.private || medicine.organization_id == organization_id
    }

    fn add_exact_candidates(
        &self,
        map: &HashMap<String, Vec<usize>>,
        value: &str,
        organization_id: &str,
        candidates: &mut HashSet<usize>,
    ) {
        if let Some(indexes) = map.get(value) {
            candidates.extend(
                indexes
                    .iter()
                    .copied()
                    .filter(|index| self.eligible(*index, organization_id)),
            );
        }
    }

    fn candidate_indexes(&self, source: &PreparedMatchSource) -> Vec<usize> {
        let organization_id = &source.source.target_organization_id;
        let mut candidates = HashSet::<usize>::new();
        for name in [&source.drug_name.normalized, &source.product_name.normalized] {
            self.add_exact_candidates(
                &self.exact_names,
                name,
                organization_id,
                &mut candidates,
            );
        }
        let exact_name_candidates = candidates.clone();

        let source_bigrams = source
            .drug_name
            .bigrams
            .iter()
            .chain(&source.product_name.bigrams)
            .collect::<HashSet<_>>();
        let mut rare_bigrams = source_bigrams
            .into_iter()
            .filter_map(|bigram| {
                self.name_postings
                    .get(bigram)
                    .map(|postings| (postings.len(), bigram))
            })
            .collect::<Vec<_>>();
        rare_bigrams.sort_unstable_by_key(|(count, _)| *count);
        let mut overlap_counts = HashMap::<usize, u8>::new();
        for (_, bigram) in rare_bigrams.into_iter().take(8) {
            if let Some(postings) = self.name_postings.get(bigram) {
                for index in postings {
                    if self.eligible(*index, organization_id) {
                        *overlap_counts.entry(*index).or_default() += 1;
                    }
                }
            }
        }
        let mut ranked_overlap = overlap_counts.into_iter().collect::<Vec<_>>();
        ranked_overlap.sort_unstable_by(|(left_index, left_count), (right_index, right_count)| {
            right_count
                .cmp(left_count)
                .then_with(|| left_index.cmp(right_index))
        });
        for (index, _) in ranked_overlap {
            candidates.insert(index);
            if candidates.len() >= INVENTORY_MATCH_CANDIDATE_POOL {
                break;
            }
        }

        if candidates.len() < INVENTORY_MATCH_MINIMUM_POOL {
            self.add_exact_candidates(
                &self.exact_specifications,
                &source.specification.normalized,
                organization_id,
                &mut candidates,
            );
            self.add_exact_candidates(
                &self.exact_factories,
                &source.factory_name.normalized,
                organization_id,
                &mut candidates,
            );
        }
        if candidates.len() < INVENTORY_MATCH_MINIMUM_POOL {
            for index in 0..self.medicines.len() {
                if self.eligible(index, organization_id) {
                    candidates.insert(index);
                }
                if candidates.len() >= INVENTORY_MATCH_MINIMUM_POOL {
                    break;
                }
            }
        }
        let mut exact_candidates = exact_name_candidates.iter().copied().collect::<Vec<_>>();
        exact_candidates.sort_unstable();
        let mut remaining = candidates
            .into_iter()
            .filter(|index| !exact_name_candidates.contains(index))
            .collect::<Vec<_>>();
        remaining.sort_unstable();
        remaining.truncate(INVENTORY_MATCH_CANDIDATE_POOL.saturating_sub(exact_candidates.len()));
        exact_candidates.extend(remaining);
        exact_candidates
    }

    fn search(
        &self,
        organization_id: &str,
        query: &str,
        limit: usize,
    ) -> Vec<InventoryTargetMedicine> {
        let normalized_query = normalize_inventory_match_text(query);
        if normalized_query.is_empty() {
            return Vec::new();
        }
        let mut matches = self
            .medicines
            .iter()
            .enumerate()
            .filter(|(index, medicine)| {
                self.eligible(*index, organization_id)
                    && medicine.search_blob.contains(&normalized_query)
            })
            .map(|(index, medicine)| {
                let rank = if medicine.drug_name.normalized == normalized_query
                    || medicine.product_name.normalized == normalized_query
                {
                    3
                } else if medicine.drug_name.normalized.starts_with(&normalized_query)
                    || medicine.product_name.normalized.starts_with(&normalized_query)
                {
                    2
                } else {
                    1
                };
                (rank, index)
            })
            .collect::<Vec<_>>();
        matches.sort_unstable_by(|(left_rank, left_index), (right_rank, right_index)| {
            right_rank
                .cmp(left_rank)
                .then_with(|| {
                    self.medicines[*left_index]
                        .medicine
                        .drug_name
                        .cmp(&self.medicines[*right_index].medicine.drug_name)
                })
                .then_with(|| left_index.cmp(right_index))
        });
        matches
            .into_iter()
            .take(limit.clamp(1, 100))
            .map(|(_, index)| self.medicines[index].medicine.clone())
            .collect()
    }
}

fn prepare_match_source(source: InventoryMedicineMatchSource) -> PreparedMatchSource {
    PreparedMatchSource {
        drug_name: PreparedMatchText::new(&source.drug_name),
        product_name: PreparedMatchText::new(&source.product_name),
        specification: PreparedMatchText::new(&source.specification),
        factory_name: PreparedMatchText::new(&source.factory_name),
        source,
    }
}

fn score_indexed_candidate(
    source: &PreparedMatchSource,
    target: &PreparedTargetMedicine,
) -> InventoryMedicineMatchCandidate {
    let name = prepared_text_similarity(&source.drug_name, &target.drug_name)
        .max(prepared_text_similarity(
            &source.drug_name,
            &target.product_name,
        ))
        .max(prepared_text_similarity(
            &source.product_name,
            &target.product_name,
        ));
    let specification = prepared_text_similarity(&source.specification, &target.specification)
        .max(prepared_text_similarity(
            &source.specification,
            &target.sale_specification,
        ));
    let factory = prepared_text_similarity(&source.factory_name, &target.factory_name);
    let score = name * 0.5 + specification * 0.3 + factory * 0.2;
    let name_exact = !source.drug_name.normalized.is_empty() && name == 1.0;
    let specification_exact =
        !source.specification.normalized.is_empty() && specification == 1.0;
    let factory_exact = !source.factory_name.normalized.is_empty() && factory == 1.0;
    let exact = name_exact && specification_exact && factory_exact;
    let reason = format!(
        "{} · {} · {}",
        if name_exact {
            "名称一致".to_string()
        } else {
            format!("名称 {:.0}%", name * 100.0)
        },
        if specification_exact {
            "规格一致".to_string()
        } else {
            format!("规格 {:.0}%", specification * 100.0)
        },
        if factory_exact {
            "厂家一致".to_string()
        } else {
            format!("厂家 {:.0}%", factory * 100.0)
        }
    );
    InventoryMedicineMatchCandidate {
        target: target.medicine.clone(),
        score,
        score_percent: (score * 100.0).round().clamp(0.0, 100.0) as u8,
        exact,
        reason,
    }
}

fn compare_match_candidates(
    left: &InventoryMedicineMatchCandidate,
    right: &InventoryMedicineMatchCandidate,
) -> Ordering {
    right
        .exact
        .cmp(&left.exact)
        .then_with(|| right.score.partial_cmp(&left.score).unwrap_or(Ordering::Equal))
        .then_with(|| left.target.drug_name.cmp(&right.target.drug_name))
}

#[cfg(test)]
mod medicine_match_index_tests {
    use super::*;

    fn target(
        index: usize,
        name: &str,
        specification: &str,
        factory: &str,
    ) -> InventoryTargetMedicine {
        InventoryTargetMedicine {
            id_med: format!("med-{index}"),
            id_med_pro: format!("product-{index}"),
            drug_name: name.into(),
            specification: specification.into(),
            minimum_unit: "支".into(),
            product_name: name.into(),
            sale_specification: specification.into(),
            factory_name: factory.into(),
            external_code: format!("CODE-{index}"),
            approval_code: format!("APPROVAL-{index}"),
            private: false,
            organization_id: String::new(),
        }
    }

    fn source(name: &str, specification: &str, factory: &str) -> PreparedMatchSource {
        prepare_match_source(InventoryMedicineMatchSource {
            key: "101:4003::org-1".into(),
            source_product_key: "101:4003".into(),
            target_organization_id: "org-1".into(),
            drug_name: name.into(),
            specification: specification.into(),
            factory_name: factory.into(),
            product_name: name.into(),
            row_count: 1,
        })
    }

    #[test]
    fn normalizes_full_width_units_and_punctuation_once() {
        assert_eq!(
            normalize_inventory_match_text(" ２ml：２０mg（注射液） "),
            "2ml20mg注射液"
        );
        assert_eq!(normalize_inventory_match_text("0.25g×24粒"), "025gx24粒");
    }

    #[test]
    fn indexed_match_keeps_exact_medicine_first() {
        let index = MedicineMatchIndex::new(vec![
            target(0, "多巴胺注射液", "2ml:20mg", "上海禾丰制药有限公司"),
            target(1, "多巴酚丁胺注射液", "2ml:20mg", "其他厂家"),
        ]);
        let source = source(
            "多巴胺注射液",
            "2ml：20mg",
            "上海禾丰制药有限公司",
        );
        let mut candidates = index
            .candidate_indexes(&source)
            .into_iter()
            .map(|candidate| score_indexed_candidate(&source, &index.medicines[candidate]))
            .collect::<Vec<_>>();
        candidates.sort_unstable_by(compare_match_candidates);
        assert_eq!(candidates[0].target.id_med_pro, "product-0");
        assert!(candidates[0].exact);
        assert_eq!(candidates[0].score_percent, 100);
    }

    #[test]
    fn private_products_are_scoped_to_the_target_organization() {
        let mut allowed = target(0, "阿莫西林胶囊", "0.25g*24粒", "甲药业");
        allowed.private = true;
        allowed.organization_id = "org-1".into();
        let mut rejected = allowed.clone();
        rejected.id_med_pro = "product-other-org".into();
        rejected.organization_id = "org-2".into();
        let index = MedicineMatchIndex::new(vec![allowed, rejected]);
        let source = source("阿莫西林胶囊", "0.25g*24粒", "甲药业");
        let candidates = index.candidate_indexes(&source);
        assert_eq!(candidates, vec![0]);
    }

    #[test]
    fn large_catalog_limits_fuzzy_scoring_pool() {
        let medicines = (0..10_000)
            .map(|index| {
                target(
                    index,
                    &format!("测试药品{index}注射液"),
                    &format!("{}ml:{}mg", index % 20, index % 100),
                    &format!("生产厂家{}", index % 80),
                )
            })
            .collect();
        let index = MedicineMatchIndex::new(medicines);
        let source = source("测试药品9999注射液", "19ml:99mg", "生产厂家79");
        let candidates = index.candidate_indexes(&source);
        assert!(candidates.len() <= INVENTORY_MATCH_CANDIDATE_POOL);
        assert!(candidates.contains(&9_999));
    }

    #[test]
    fn nine_hundred_source_medicines_do_not_restore_all_pairs_work() {
        let medicines = (0..10_000)
            .map(|index| {
                target(
                    index,
                    &format!("规模测试药品{index}注射液"),
                    &format!("{}ml:{}mg", index % 20, index % 100),
                    &format!("规模厂家{}", index % 80),
                )
            })
            .collect();
        let index = MedicineMatchIndex::new(medicines);
        let mut compared = 0usize;
        for source_index in 0..900 {
            let source = source(
                &format!("规模测试药品{source_index}注射液"),
                &format!("{}ml:{}mg", source_index % 20, source_index % 100),
                &format!("规模厂家{}", source_index % 80),
            );
            let candidate_indexes = index.candidate_indexes(&source);
            compared += candidate_indexes.len();
            let mut candidates = candidate_indexes
                .into_iter()
                .map(|candidate| score_indexed_candidate(&source, &index.medicines[candidate]))
                .collect::<Vec<_>>();
            candidates.sort_unstable_by(compare_match_candidates);
            candidates.truncate(8);
            assert!(!candidates.is_empty());
        }
        assert!(compared <= 900 * INVENTORY_MATCH_CANDIDATE_POOL);
        assert!(compared < 900 * 10_000 / 10);
    }

    #[test]
    fn on_demand_search_obeys_private_scope_and_limit() {
        let mut private = target(1, "维生素D滴剂", "400IU*36粒", "示例厂家");
        private.private = true;
        private.organization_id = "org-2".into();
        let index = MedicineMatchIndex::new(vec![
            target(0, "维生素D滴剂", "400IU*36粒", "示例厂家"),
            private,
        ]);
        let results = index.search("org-1", "维生素D", 60);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id_med_pro, "product-0");
    }
}
