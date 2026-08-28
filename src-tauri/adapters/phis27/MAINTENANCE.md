# 二系列phis适配器维护

## 版本与模板兼容

- `sourceCompatibility.mode` 固定为 `STRUCTURE_CONTRACT`：二系列项目版本只作现场记录，不把数据库连接成功或版本名称当成已认证。每个药品/库存任务都必须通过当前来源表字段契约和项目变体回归。
- 修改药品、机构、药库药房或库存来源行为时，递增 `manifest.json` 的 `version`，并在 `changes` 增加当前版本说明。
- 只有旧字段映射和字典决策仍可安全复用时，才保留 `templateCompatibleFromVersion`；核心来源含义变化时应提高最低兼容版本。
- 来自未来版本的模板不能降级套用，低于兼容范围的模板必须重新核对。

## 项目差异处理

1. 请现场人员在来源识别区导出“适配器支持包”。
2. 运行 `npm run adapter:support -- --file <支持包.json>`，检查校验值、脱敏状态、缺失对象和历次结构变化。当前版本会根据支持包中的药品基础/机构库存任务标记自动选择契约；旧支持包才需要人工增加 `--task MEDICINE_BASE` 或 `--task INVENTORY`。
3. 推荐运行 `npm run adapter:handoff -- --file <支持包.json>` 一次生成当前任务的维护实施计划、契约差异草稿、项目变体夹具草稿和 SHA-256 文件清单；旧支持包需同时指定 `--task MEDICINE_BASE` 或 `--task INVENTORY`。接收方先运行 `npm run adapter:handoff-verify -- --dir <交接目录>`，验证清单自校验、文件完整性、任务一致性和隐私边界。交接包不会修改正式文件，也不携带来源指纹、Schema、样例值、SQL 或连接信息。
4. 仅需单独重生成时，可分别运行 `npm run adapter:contract-draft` 和 `npm run adapter:fixture-draft`。`REVIEW` 夹具必须人工决定为 `PASS` 或 `BLOCK`，改写标题与业务检查项并填写真实 `rustTest` 后，才可移入 `fixtures`。
5. 删除无关候选并补齐每项的业务用途、REQUIRED / CONDITIONAL / OPTIONAL 判定和安全降级方式，再在 `legacy_phis27` 的可执行 Rust 测试中覆盖实际字段能力。
6. 核心关联键、库存包装或金额证据变化时，同步维护 `acceptance.json`；不得以项目连接或固定 Schema 代替能力识别。
7. 药库与药房是独立库存分支：保留 `objectGroups` 的至少一个库存表门禁，并通过 `requiredWhenObjectsPresent` 只在对应库存表存在时要求该分支的包装和位置字段。

## 交付门禁

运行 `npm run adapter:verify`。清单、实现绑定、维护说明、验收证据、脱敏夹具和可执行回归测试必须全部通过。
