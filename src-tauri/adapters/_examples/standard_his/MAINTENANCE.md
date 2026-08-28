# 标准教学 HIS 适配器维护

## 版本与模板兼容

- `sourceCompatibility` 仅声明教学结构契约，不代表任何厂商产品或版本获得生产认证。
- 修改表关系、来源身份或输出字段时递增 `manifest.json` 的版本，并同步调整 `templateCompatibleFromVersion`。
- 示例只使用 `super::sdk::*` 暴露的读取与统一模型，不直接依赖 ODBC、PostgreSQL 协议或中央迁移编排模块。
- 库存位置只输出标准 `WAREHOUSE` / `PHARMACY`，来源库存行、库房和药品商品必须分别保留稳定键；不得用名称或当前行号代替。
- 库存数量、单价和金额必须与 `HIS_INVENTORY.SALE_UNIT/UNIT_SALE_FACTOR` 使用同一管理包装口径；商品包装只用于对照。
- 复制为真实适配器时必须更换稳定 ID、名称、物理表字段、验收契约和项目变体，不能将示例表名当成厂商事实。

## 项目差异处理

1. 运行 `npm run adapter:support -- --file <支持包.json>` 核对完整性、脱敏状态和结构影响。
2. 推荐运行 `npm run adapter:handoff -- --file <支持包.json>` 一次生成维护实施计划、独立契约差异草稿、脱敏项目变体草稿和 SHA-256 文件清单；接收方先运行 `npm run adapter:handoff-verify -- --dir <交接目录>` 验证清单自校验、文件完整性、任务一致性和隐私边界，不直接覆盖 `acceptance.json` 或正式夹具。
3. 仅需单独重生成时，可分别运行 `npm run adapter:contract-draft` 和 `npm run adapter:fixture-draft`；完成业务预期和真实测试路径后再把夹具移入 `fixtures`。
4. 补齐业务用途、必要性与安全降级，并同步维护药品及库存脱敏夹具和可执行测试。
5. 库存适配只负责读取来源模型；机构/库房映射、药品台账、目标空库检查、逐库试迁移、首次盘点和安全撤销必须继续使用公共流程。

## 交付门禁

运行 `npm run adapter:verify`。教学示例还必须通过 `npm run adapter:examples`，保证示例可编译、契约可验证且不会进入正式运行时注册表。
