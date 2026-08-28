# 三方 HIS 适配器维护

## 版本与模板兼容

- 在 `sourceCompatibility` 明确选择 `STRUCTURE_CONTRACT` 或 `DECLARED_VERSION_RANGE`。没有真实版本验收证据时必须使用结构契约模式，不能填写或暗示厂商版本已认证。
- 修改来源读取行为时递增 `manifest.json` 的 `version`，并在 `changes` 增加当前版本说明。
- 使用 `templateCompatibleFromVersion` 声明仍可安全复用的最早模板版本，不兼容变更必须提高该版本。
- 来自未来版本的模板不能降级套用。

## 项目差异处理

1. 从现场取得应用导出的脱敏适配器支持包。
2. 运行 `npm run adapter:support -- --file <支持包.json>` 检查完整性、脱敏状态和结构变化。
3. 推荐运行 `npm run adapter:handoff -- --file <支持包.json> --task MEDICINE_BASE`（或 `INVENTORY`）一次生成维护实施计划、独立契约差异草稿、脱敏项目变体草稿和 SHA-256 文件清单；接收方先运行 `npm run adapter:handoff-verify -- --dir <交接目录>` 验证清单自校验、文件完整性、任务一致性和隐私边界。它不会直接修改验收契约或正式夹具。
4. 仅需单独重生成时，可分别运行 `npm run adapter:contract-draft` 和 `npm run adapter:fixture-draft`。v2 会保留字段名和数据库物理类型，不得加入样例值、注释、SQL 或连接信息。改写标题与业务预期并填写真实 `rustTest` 后再移入 `fixtures`；旧 v1 夹具仍可运行。
5. 删除无关候选，补齐业务用途、必要性和安全降级方式，再将确认过的差异加入验收契约和可执行 Rust 回归测试。
6. 库存存在药库/药房等可选分支时，用 `sourceStructure.objectGroups` 声明至少一个可用来源，并用 `requiredWhenObjectsPresent` 只激活当前分支的包装和位置依赖。
7. 所有动态 Schema/对象名使用 SDK `source_qualified_object`，本批机构/库房文本范围使用 `source_text_filter_list`；禁止在适配器内复制引号、转义或无上限列表拼接逻辑。

## 交付门禁

开发过程中运行 `npm run adapter:status -- --adapter <适配器 ID>` 查看当前阶段和下一步，需要完整错误时运行 `npm run adapter:validate`。每条 `acceptance.evidence` 必须填写所属迁移任务、真实 Rust 测试函数名、完整 `rustTest` 路径和执行策略；清单声明的每个任务都必须有证据覆盖。普通证据使用 `REQUIRED`，需要真实数据库且源码带 `#[ignore]` 的集成测试才可使用 `OPTIONAL_LIVE`。`npm run adapter:evidence` 会拒绝任务遗漏、0 项测试或错误模块路径。最终运行 `npm run adapter:verify`，全部通过后才可交付。
