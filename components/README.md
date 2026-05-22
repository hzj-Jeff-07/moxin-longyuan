# components/ · 元件目录

每个文件定义一个**元件类型**的电气抽象。完整 schema 见 [`../docs/component-schema.md`](../docs/component-schema.md)。

## 谁维护什么

| 字段 | 谁维护 |
|---|---|
| `[component_type]` 段 | 开发部 |
| `[[pin]]` 段 | 开发部（建模部不要直接改） |
| `[[parameter]]` 段 | 开发部主导，前端可提需求 |
| `[[state_field]]` 段 | 开发部 |

建模部如果觉得引脚命名 / 数量有问题，**提 issue 让开发部改 toml**，不要自己改建模文件里的 pin 名。

## 当前元件清单（v1.0）

| 文件 | 元件 | 对应建模任务书章节 |
|---|---|---|
| `led.toml` | 5mm LED | 5.1 (2) |
| `button.toml` | 轻触按钮 | 5.1 (3) |
| `resistor.toml` | 色环电阻 | 5.1 (4) |
| `breadboard.toml` | 半尺寸面包板 | 5.1 (5) |
| `dupont.toml` | 杜邦线 | 5.1 (6) |
| `seven_segment.toml` | 数码管 | 5.2 (7) |
| `buzzer.toml` | 无源蜂鸣器 | 5.2 (8) |
| `potentiometer.toml` | 电位器 | 5.2 (9) |

**Arduino Uno R3 不在此目录**——它是开发板，由 `src/boards/arduino_uno.rs` 维护，引脚锚点见 `../pin-anchors-template/arduino_uno.json`。

## 添加新元件的步骤

1. 在本目录创建 `<elem_id>.toml`，参考 `led.toml` 结构
2. 在 `../pin-anchors-template/` 同步生成 `<elem_id>.json` 模板
3. 通知建模部接单
4. 在 schema 文档第 14 节"未决问题"里如果有相关条目，标记为已解决

## CI 校验（待开发部实现）

提交时会跑：
- TOML 格式合法性
- `[component_type].id` 与文件名一致
- 所有 `electrical` 字段值在 schema v1.0 枚举内
- pins[].name 唯一性、aliases 不冲突
- 同名 pin 在 pin-anchors-template/ 必须存在
