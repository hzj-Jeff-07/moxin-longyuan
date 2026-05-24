# A6 · 被动元件 schema 层

## 任务

为电阻、面包板、杜邦线这三类被动元件补 schema 文件。被动元件在仿真层不产生事件,只用于电路连通性校验。

- **电阻 (resistor)**：2 引脚,electrical 都是 `passive`,参数 `resistance_ohm: u32`
- **面包板 (breadboard)**：30 行 × 5 列 = 150 个孔位 (简化版,400 孔位 Phase 2),引脚 name 用 `a1`/`a2`/.../`e30`,electrical 全 `passive`
- **杜邦线 (dupont)**：2 引脚,纯连接器,无参数

## 允许动的文件

- `components/resistor.toml`
- `components/breadboard.toml`
- `components/dupont.toml`
- `pin-anchors-template/resistor.json`
- `pin-anchors-template/breadboard.json`
- `pin-anchors-template/dupont.json`
- `src/sim/wire.rs`(走线校验时把 passive 元件视为短路连接)
- `tests/passive_components.rs`

## 验收

```powershell
cargo test passive
cargo clippy --all-targets
python scripts/check_schema.py    # 12/12 components OK
```

测试要点：
- 电阻可作为分压元件被识别 (仿真上视为短路即可,数值不影响 demo)
- 面包板 a1-a5 同一行连通,a1-b1 跨列不连通(参考真实面包板拓扑)
- 杜邦线两端等电位

## 约束

- 不实现真实电阻分压计算 (留 Phase 2)
- 面包板拓扑：每行(a-e 列)5 孔连通;左右两条电源轨各自连通;两轨之间不连通
- 不动现有元件 schema

## commit message

`feat(A6): 被动元件 schema (电阻/面包板/杜邦线)`
