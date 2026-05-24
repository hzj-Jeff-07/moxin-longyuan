# MoXin 元件 Schema · v1.0

> 状态：草稿，等待建模部 + 开发部联合 review  
> 维护者：开发部  
> 适用范围：MoXin Demo 阶段全部 9 个元件 + 后续扩展件

## 一、为什么需要这份文档

MoXin 由三个部门同时推进，要共用同一份"元件描述"才能不返工：

- **建模部**需要知道：每个元件有几个引脚、引脚叫什么名字、是数字 / 模拟 / 电源 / GND。建好的 3D 模型里要把这些引脚标成可吸附的锚点。
- **开发部**需要知道：每个元件在仿真层的行为——LED 收到 HIGH 就亮、按钮按下回报事件、电位器旋转改变 ADC 读数。
- **未来的前端部**需要知道：拖出来的元件长什么样、属性面板里能改哪些参数、运行时显示什么状态。

如果三边各自维护一套，今天叫 `anode` 明天叫 `+`，下周连线就连不上。这份 schema 把字段、命名、电气属性枚举一次性定死，三边一起遵守。

## 二、数据分层

```
┌─────────────────────────────────────────────────────────┐
│  schema 文档（本文件）—— 契约层                          │
│  字段定义 + 电气属性枚举 + 命名规范                       │
└─────────────────────────────────────────────────────────┘
              │ 约束
    ┌─────────┴─────────┐
    ▼                   ▼
┌─────────────┐   ┌─────────────────────┐
│ 元件目录     │   │ 引脚锚点表           │
│ components/  │   │ pin-anchors/         │
│ *.toml      │   │ *.json              │
│ 开发部维护   │   │ 建模部填充 3D 坐标   │
│ 电气抽象     │   │ 几何抽象             │
└─────────────┘   └─────────────────────┘
        │                   │
        └─────────┬─────────┘
                  ▼
          ┌──────────────┐
          │ moxin 运行时  │
          │ + 未来 GUI    │
          └──────────────┘
```

**关键约束**：两份文件的 `pin.name` 字段必须**严格一致**。建模部填 3D 坐标时不能改名字、不能加减引脚。如果建模发现引脚定义有问题，提 issue 给开发部修 `components/*.toml`，**不要**单方面改 `pin-anchors/*.json`。

## 三、命名规范

| 实体 | 规则 | 示例 |
|---|---|---|
| 元件类型 ID | 小写蛇形 | `led` / `seven_segment` / `dht11` |
| 元件实例 ID | 用户在 moxin.toml 里命名，小写字母数字下划线 | `led1` / `btn_red` / `pot_volume` |
| 引脚名 | 小写蛇形，使用元件领域术语 | `anode` / `cathode` / `wiper` |
| 引脚别名 | 用户友好的简写，列表形式 | `["a", "+"]` / `["c", "-", "k"]` |
| 文件名 | 与元件类型 ID 完全一致 | `led.toml` / `seven_segment.toml` |

**禁止**：
- 大写字母（Linux/Mac 区分大小写，Windows 不区分，会踩坑）
- 中文文件名（建模部交付到 Linux CI 会乱码）
- 别名跟其他元件的正式名冲突（搜索时歧义）

## 四、电气属性枚举（一次定全）

下面这 18 个值是 schema v1.0 唯一允许的电气属性。任何新元件都必须从这个列表里挑，不允许自造。如果发现真有不能覆盖的情况，提 issue 升 v1.1。

| 值 | 用途 | 示例 |
|---|---|---|
| `digital_in` | 元件接收 MCU 数字信号 | LED anode |
| `digital_out` | 元件输出数字信号给 MCU | 按钮触点 |
| `analog_in` | 元件接收模拟信号（少见，主要是接地） | — |
| `analog_out` | 元件输出模拟电压给 MCU ADC | 电位器 wiper、光敏电阻 |
| `pwm_in` | 元件接收 PWM 控制 | RGB LED、舵机、直流电机 |
| `power` | 接 5V / 3V3 电源 | 任何元件的 VCC |
| `gnd` | 接地 | 任何元件的 GND |
| `i2c_sda` | I2C 数据线 | OLED、LCD1602(I2C 版) |
| `i2c_scl` | I2C 时钟线 | OLED、LCD1602(I2C 版) |
| `spi_mosi` | SPI 主出从入 | 扩展件预留 |
| `spi_miso` | SPI 主入从出 | 扩展件预留 |
| `spi_sck` | SPI 时钟 | 扩展件预留 |
| `spi_cs` | SPI 片选 | 扩展件预留 |
| `uart_tx` | UART 发送 | 蓝牙模块、扩展件 |
| `uart_rx` | UART 接收 | 蓝牙模块、扩展件 |
| `one_wire` | 单总线协议 | DHT11、DS18B20 |
| `passive` | 无源元件两端（电阻、面包板触点） | 电阻、面包板孔位 |
| `nc` | 不连接（占位） | 某些按钮的多余引脚 |

## 五、元件目录格式：components/*.toml

每个元件一份 TOML 文件。结构：

```toml
[component_type]
id = "led"                    # 必填，跟文件名一致
display_name = "5mm LED"      # 必填，UI 上显示的名字
category = "output"           # 必填，见下表
description = "5mm 直插式发光二极管"
schema_version = "1.0"        # 必填，跟本文档版本一致

# 引脚定义。顺序无意义但建议按物理顺序排。
[[pin]]
name = "anode"
aliases = ["a", "+"]
electrical = "digital_in"     # 必填，必须是第四节枚举里的值
direction = "in"              # 必填：in / out / bidirectional
required = true               # 默认 true，nc 类型可填 false

[[pin]]
name = "cathode"
aliases = ["c", "-", "k"]
electrical = "gnd"
direction = "in"

# 可调参数（用户在 GUI 属性面板里能改的东西）
[[parameter]]
name = "color"
type = "enum"                 # enum / int / float / string / bool
values = ["red", "green", "yellow", "blue", "white"]
default = "red"
display_name = "颜色"

# 运行时状态字段（仿真器实时更新，前端实时显示）
[[state_field]]
name = "level"
type = "enum"
values = ["off", "on"]
description = "LED 当前亮灭状态"

[[state_field]]
name = "brightness"
type = "int"
range = [0, 255]
default = 0
description = "PWM 亮度 0-255"
```

**`category` 取值**（控制元件库面板的分组）：

| 值 | 含义 |
|---|---|
| `output` | 输出元件（LED、蜂鸣器、数码管、电机、舵机） |
| `input` | 输入元件（按钮、电位器、传感器） |
| `display` | 显示元件（LCD、OLED） |
| `passive` | 被动元件（电阻、电容） |
| `wiring` | 连线辅助（面包板、杜邦线） |
| `power` | 电源相关 |

**`type` 取值**（参数和状态字段的类型）：

| 值 | 说明 |
|---|---|
| `enum` | 枚举，需配 `values` |
| `int` | 整数，可配 `range = [min, max]` |
| `float` | 浮点，可配 `range` |
| `string` | 字符串 |
| `bool` | 布尔 |

## 六、引脚锚点表格式：pin-anchors/*.json

由开发部预生成模板（含元数据），建模部填充 3D 坐标后交回。

```json
{
  "format_version": "1.0",
  "component_id": "led",
  "model_file": "part_led_5mm_red_v1.glb",
  "bounding_box_mm": {
    "min": { "x": -2.5, "y": -2.5, "z": 0 },
    "max": { "x":  2.5, "y":  2.5, "z": 8.6 }
  },
  "pins": [
    {
      "name": "anode",
      "electrical": "digital_in",
      "position_mm": { "x": 1.27, "y": 0, "z": -20 },
      "pin_length_mm": 25,
      "pin_diameter_mm": 0.5,
      "notes": "长脚为正极"
    },
    {
      "name": "cathode",
      "electrical": "gnd",
      "position_mm": { "x": -1.27, "y": 0, "z": -18 },
      "pin_length_mm": 22,
      "pin_diameter_mm": 0.5,
      "notes": "短脚为负极"
    }
  ]
}
```

**字段说明**：

| 字段 | 谁填 | 说明 |
|---|---|---|
| `format_version` | 开发部 | 不要改，便于以后兼容性处理 |
| `component_id` | 开发部 | 与 `components/*.toml` 的 id 一致 |
| `model_file` | 建模部 | 建好后填，glb 文件名按建模部命名规范 |
| `bounding_box_mm` | 建模部 | 元件包围盒，给布局算法用 |
| `pins[].name` | 开发部 | **建模部不要改** |
| `pins[].electrical` | 开发部 | **建模部不要改** |
| `pins[].position_mm` | 建模部 | 引脚根部（连接 PCB 一侧）的坐标 |
| `pins[].pin_length_mm` | 建模部 | 引脚物理长度，给"插入面包板深度"计算用 |
| `pins[].pin_diameter_mm` | 建模部 | 给孔位匹配用 |
| `pins[].notes` | 双方 | 任何需要标注的细节 |

**坐标系约定**（与建模部任务书一致）：
- 单位：毫米 (mm)
- 元件几何中心为原点
- Z 轴朝上，PCB 平面为 XY 平面
- 引脚朝下时 z 为负

## 七、电路连接：moxin.toml 语法（已有，作参考）

为对齐三方理解，再贴一遍现有 `moxin.toml` 的电路声明语法：

```toml
[project]
name = "demo"
board = "arduino-uno"
version = "0.2"

[[component]]
id = "led1"               # 用户起的实例名
type = "led"              # 与 components/*.toml 的 id 一致
color = "red"             # 来自 led.toml 的 parameter

[[wire]]
from = "board.D13"        # 板上引脚
to = "led1.anode"         # 实例.引脚名（或别名）
```

引脚引用语法支持：
- `board.D13` / `board.PA13` / `board.GND` / `board.5V`
- `<instance>.<pin_name>` / `<instance>.<alias>`
- 大小写不敏感

## 八、版本演进规则

- **v1.x**：向后兼容。可以加新的可选字段、新的电气属性枚举值、新的元件类型。
- **v2.0**：允许破坏性变更。届时 `schema_version` 字段会拒绝加载旧文件，要求重写。

**任何修改本文档的 PR 必须**：
1. 同步更新 `schema_version` 字段
2. 在文档底部 CHANGELOG 加一条
3. 由开发部 + 建模部双方 review

## 九、交付与协作流程

```
开发部                        建模部
  │                            │
  ├─ 1. 写 components/X.toml ──┤
  │                            │
  ├─ 2. 生成 pin-anchors-template/X.json
  │     (含元数据，xyz 留空)──→ │
  │                            ├─ 3. 建 3D 模型
  │                            │
  │                            ├─ 4. 填入 xyz 坐标
  │  ←──────────────────────── ┤    交回 pin-anchors/X.json
  │                            │
  ├─ 5. CI 校验：
  │   - JSON 格式合法
  │   - pins[].name 与 .toml 一致
  │   - 没有缺失引脚
  │                            │
  ├─ 6. 通过则合并到 main
  │                            │
```

CI 校验脚本由开发部提供（下个 ticket，本 schema 文档先发了再说）。

## 十、未决问题（v1.1 再讨论）

- **面包板内部连接**：400 个孔位之间哪些是连通的，schema 怎么表达？候选方案：`[[connection_group]]` 段，列出节点组。
- **PWM 等效电压**：LED 用 `pwm_in` 接收 PWM，状态字段里要不要算"等效平均电压"？还是只给 duty cycle？
- **多色 LED**：RGB LED 三个 anode，是定义成一个元件还是三个 LED 实例？
- **传感器模拟值的"环境变量"**：DHT11 的温度、HC-SR04 的距离需要用户在 TUI/GUI 里调节，schema 怎么标记"这是可手动注入的环境量"？

## CHANGELOG

- **v1.0** (2026-05-20) · 初版，覆盖 Demo 阶段 9 个元件 + 扩展电气属性枚举
