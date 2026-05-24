# pin-anchors-template/ · 引脚锚点表模板（给建模部）

**这个目录是开发部移交给建模部的"待填表"。**

每份 JSON 都对应一个元件（或开发板），里面的 **元数据已经填好**，**3D 坐标留空（null）**，由建模部建好模型后回填。

## 建模部该做什么

1. 在你电脑上 `git clone` 仓库（或直接下载本目录）
2. 拿一份 JSON 打开，例如 `led.json`
3. 在 Blender / C4D 里建好 LED 模型
4. 量出每个引脚根部（连 PCB 一侧）的 xyz 坐标
5. **只改 `position_mm` / `pin_length_mm` / `bounding_box_mm` / `model_file` 四个字段**
6. 改完 save，交回 `pin-anchors/` 目录（不是本 template 目录）

## 不要做的事

- ❌ 不要改 `pins[].name`
- ❌ 不要改 `pins[].electrical`
- ❌ 不要新增或删除 pin
- ❌ 不要把 `null` 改成 `0`——`null` 是"待填"，`0` 是"零坐标"，含义不同
- ❌ 不要删 `_comment` 字段（虽然不影响功能，但保留方便沟通）

如果在建模过程中觉得引脚名 / 数量有问题，**在群里提**，开发部改 `components/*.toml`，再重新生成对应的 template 给你。

## 坐标系约定

- 单位：毫米 (mm)
- 元件几何中心为原点
- Z 轴朝上，PCB 平面为 XY 平面
- 引脚根部为锚点，引脚朝下伸出时 z 为负

## 当前清单（v1.0）

| 文件 | 元件 | 建模任务书章节 |
|---|---|---|
| `arduino_uno.json` | Arduino Uno R3 | 5.1 (1) |
| `led.json` | 5mm LED | 5.1 (2) |
| `button.json` | 轻触按钮 | 5.1 (3) |
| `resistor.json` | 色环电阻 | 5.1 (4) |
| `breadboard.json` | 半尺寸面包板 | 5.1 (5) |
| `dupont.json` | 杜邦线 | 5.1 (6) |
| `seven_segment.json` | 数码管 | 5.2 (7) |
| `buzzer.json` | 无源蜂鸣器 | 5.2 (8) |
| `potentiometer.json` | 电位器 | 5.2 (9) |

## 交付路径

填完的文件放到仓库的 `pin-anchors/` 目录（**不是** template 目录）：

```
moxin-longyuan/
├── pin-anchors-template/   ← 模板，建模部读取
│   └── led.json
└── pin-anchors/             ← 填好的，建模部提交
    └── led.json
```

CI 会自动校验：
- `pin-anchors/X.json` 的 `pins[].name` 必须跟 `pin-anchors-template/X.json` 完全一致
- 所有 `position_mm` 不能为 null
- `model_file` 不能为 "TODO_BY_MODELER"

## 有问题找谁

- schema 问题（字段含义、命名）→ 开发部
- 模型问题（细节、材质、参考图）→ 建模部内部对齐
- 跨部门冲突 → 阙广平
