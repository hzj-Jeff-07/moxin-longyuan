# AI 接入指南 — 让 Claude Code 通过 MoXin 调试硬件

> 目标读者：使用 Claude Code / Cursor / 其他 AI Agent 的开发者，希望让 AI 直接驱动 MoXin CLI 完成"写代码 → 编译 → 仿真 → 验证"的全闭环。
>
> 本指南面向 v0.3.0。本版本只稳定承诺 Arduino Uno (simavr) + STM32F405 (qemu netduinoplus2)，断言能力仅覆盖 D13 LED + 串口输出。

---

## 1. 设计立场：为什么 MoXin 适合给 AI 用

MoXin 的 CLI 不是为人类舒适设计的，是为 AI 闭环设计的。三个关键约定：

1. **JSON Lines 事件流**：`moxin run --output json` 把 bridge 事件流原样透传到 stdout（每行一条 JSON）。AI 只需逐行 `JSON.parse`，不需要解析 ANSI、TUI 或文本表格。
2. **退出码即结论**：`moxin assert` 用退出码表达结果，0=pass / 1=fail / 2=timeout。AI 不需要"理解"输出，直接看 `$?` 即可决策下一步。
3. **状态快照文件**：`moxin run --output json` 运行期间，会在 `build/.moxin-state.json` 持续写最新状态。需要"现在 D13 什么电平"时，直接读这个文件，不必维护事件流的状态机。

这三点叠加 → AI 可以把 MoXin 当作"硬件世界的 pytest"：写代码 → 触发 → 拿 exit code 决断 → 进入下一轮。

---

## 2. 标准闭环 — 四步法

任何 AI Agent 对 MoXin 项目做改动，建议固定走这四步：

```text
┌──────────┐   ┌──────────┐   ┌──────────────┐   ┌──────────────────┐
│ 1. edit  │ → │ 2. build │ → │ 3. run/json  │ → │ 4. assert (exit) │
└──────────┘   └──────────┘   └──────────────┘   └──────────────────┘
   写代码        moxin build      moxin run         moxin assert
                                  --output json     退出码 0/1/2
```

每一步对 AI 的指令都极简：

| 步骤 | 命令 | AI 读什么 |
|---|---|---|
| build | `moxin build` | stdout 末尾是否 `OK`；非零 exit code 即失败 |
| run | `moxin run --output json` | stdout 每行一条 JSON，解析 `event` 字段 |
| assert | `moxin assert ...` | **只看退出码**：0=pass, 1=fail, 2=timeout |

---

## 3. `moxin run --output json` 输出契约

bridge 子进程每行输出一条 JSON。事件类型固定为五种：

```jsonl
{"event":"ready","mcu":"atmega328p","freq":16000000}
{"event":"pin","t_us":1234,"port":"B","bit":5,"value":1}
{"event":"serial","t_us":1235,"line":"hello"}
{"event":"button","t_us":1240,"pressed":true}
{"event":"exit","state":0}
```

字段语义（详见 `docs/design/bridge-protocol.md`）：

- `event` — 事件种类，是唯一的 dispatch key
- `t_us` — bridge 启动后的微秒时间戳（不是 Unix epoch）
- `pin.port` + `pin.bit` — Arduino: B/5 = D13；STM32: GPIO/13 = PA13
- `serial.line` — 一行 UART 输出（不含换行符）
- `exit.state` — bridge 退出码

> ⚠️ 未知事件类型会被 Rust 端静默忽略。AI 在解析时也建议 `default: ignore`，不要为新事件类型 panic。

### 3.1 状态快照文件 `build/.moxin-state.json`

`moxin run --output json` 运行期间，会把当前已知状态合并写入 `build/.moxin-state.json`。结构示意：

```json
{
  "pin_states": { "B:5": 1 },
  "serial_lines": [[1235, "hello"]],
  "bridge_exited": false
}
```

当 AI 只需要"快照"（而非全量事件流），直接读这个文件即可，免去维护事件状态机。

---

## 4. `moxin assert` — AI 验证的主武器

v0.3.0 提供三种断言模式，互斥使用：

### 4.1 引脚电平等值（PinEq）

```bash
moxin assert --pin D13 --eq HIGH --after 1s
```

语义：仿真启动后等 1 秒，读 D13 当前电平，等于 HIGH → exit 0；否则 exit 1；状态未知 → exit 2。

- `--eq` 接受 `HIGH | LOW | 1 | 0`（大小写不敏感）
- `--after` 默认 `1s`，支持 `500ms` / `2s` / `1m`

### 4.2 引脚翻转（PinToggles）

```bash
moxin assert --pin D13 --toggles --within 3s
```

语义：在 3 秒窗口内观察到 D13 至少翻转一次（0→1 或 1→0）→ exit 0；窗口耗尽未翻转 → exit 2。

- 适合验证 `blink` 类程序：只要灯在闪，无论高低相位都 pass
- `--within` 默认 `3s`

### 4.3 串口子串（SerialContains）

```bash
moxin assert --serial-contains "hello" --within 2s
```

语义：在 2 秒窗口内任意一行串口输出包含 `"hello"` → exit 0；超时 → exit 2。

- 子串匹配，不是正则
- `--within` 默认 `2s`

### 4.4 v0.3.0 限制（明文写在错误信息里）

只有 D13 LED 引脚真实可观测。其它引脚的 `--pin` 断言会立刻报错并返回非 0/1/2 的 anyhow 错误：

```text
Error: pin `D7` is not observable by bridge on board arduino-uno (v0.3.0: only D13 LED reports state)
```

> 这条限制是有意保留的反发散约束。要观测更多引脚需要同步改 `bridge/*.c` + `BridgeEvent` + 协议文档，超出 v0.3.0 范围。

### 4.5 退出码总表（CI / AI 决策用）

| 退出码 | 含义 | AI 应该怎么办 |
|---|---|---|
| 0 | PASS — 断言成立 | 继续下一步 |
| 1 | FAIL — 状态可读但不符合预期 | 回去改代码 |
| 2 | TIMEOUT — 窗口耗尽未达成 | 检查仿真是否启动 / 窗口是否过短 |
| 其他 | anyhow 错误（板子不存在、artifact 未编译、参数互斥等） | 读 stderr，修正调用方式 |

---

## 5. 给 Claude Code 的 prompt 模板

下面是把 MoXin 嵌入 Claude Code 工作流的最小 prompt（实测可用）：

```text
你是 MoXin 项目的 AI 调试助手。修改 examples/blink-uno/src/main.ino 后，
按下面四步验证，不要跳步：

1. 切到 examples/blink-uno 目录
2. 跑 `moxin build`，如果非零 exit 退出并报告 stderr
3. 后台跑 `moxin run --output json`，把 stdout 重定向到 events.jsonl
4. 跑 `moxin assert --pin D13 --toggles --within 3s`，根据 exit code 决断：
   - 0 → 报告"D13 闪烁验证通过"
   - 1 → D13 状态固定，回去检查 digitalWrite 调用
   - 2 → 超时，检查 setup() / loop() 是否被卡死
   - 其他 → 读 stderr 报错

全过程使用 Bash 工具，不要用 TUI 模式。
```

要点：

- **永远显式给退出码语义**。AI 不会自己记住"exit 2 = timeout"，把它写进 prompt 里。
- **不要让 AI 用 TUI 模式**（`moxin run` 不带 `--output json`）。TUI 输出是给人看的，AI 会浪费 token 解析颜色码。
- **断言失败时给具体修复方向**。"exit 1 → 回去检查 digitalWrite" 比 "exit 1 → 修代码" 有用 10 倍。

---

## 6. 端到端示例：让 AI 修复一个坏掉的 blink

假设你给 AI 一段坏代码（`digitalWrite(13, LOW)` 后忘了再写 HIGH）：

```cpp
void loop() {
  digitalWrite(13, LOW);
  delay(500);
  // BUG: 漏了 digitalWrite(13, HIGH);
  delay(500);
}
```

AI 的执行轨迹应该是：

```bash
$ moxin build
OK

$ moxin run --output json > events.jsonl &
# 后台运行

$ moxin assert --pin D13 --toggles --within 3s
TIMEOUT
$ echo $?
2
```

退出码 2 → AI 自己判断 "D13 没翻转，loop 里缺一个 HIGH 写入"，编辑代码补回 `digitalWrite(13, HIGH);`，重新跑 build + assert，这次 exit 0 → 闭环完成。

整个过程 AI **不需要看任何日志**，只看退出码。

---

## 7. CI 集成片段（GitHub Actions / 任何 runner）

```yaml
- name: MoXin smoke test
  run: |
    cd examples/blink-uno
    moxin build
    moxin run --output json > /tmp/events.jsonl &
    RUN_PID=$!
    # D13 翻转断言
    moxin assert --pin D13 --toggles --within 3s
    ASSERT_EXIT=$?
    kill $RUN_PID || true
    exit $ASSERT_EXIT
```

退出码会原样冒泡到 CI，0=绿灯，非 0=红灯。**不要写额外的 grep 逻辑**，让退出码做事。

---

## 8. 常见坑（AI 容易踩的）

| 现象 | 根因 | 修复 |
|---|---|---|
| `artifact not found at build/xxx.elf — run \`build\` first` | 直接跑 assert 没 build | 先 `moxin build` |
| `pin \`D7\` is not observable...` | v0.3.0 只有 D13 真观测 | 改用 D13，或改用 `--serial-contains` 让程序自己 print |
| `--toggles` 永远 timeout | 程序根本没翻转过 D13 | 检查 `digitalWrite(13, ...)` 是否真的被调用 |
| `--serial-contains` 找不到 | 串口缓冲未刷出 / `Serial.begin` 漏了 | 在 setup 里加 `Serial.begin(9600)` + 在每次 print 后 `Serial.flush()` |
| `moxin run` stdout 一片空白 | 没加 `--output json`，TUI 模式吃掉了所有输出 | 永远带 `--output json`（AI 场景下） |

---

## 9. 未来演进（v0.4+，不在 v0.3.0 范围）

> 以下能力**当前未实现**。如果你的 AI workflow 强依赖这些，请先在 issue 区登记需求，不要假设它们存在。

- 更多引脚观测（需要扩展 `bridge/*.c`）
- I2C / SPI / OLED / LCD 元件建模
- MCP server 形态：让 AI 通过 MCP 工具调用而非 CLI（设计稿在 v3）
- 多板并行仿真 + 跨板事件路由
- 完整的 ESP32 / RP2040 / Nano 板支持

---

## 10. 快速复制粘贴清单（给 AI 系统提示用）

```text
MoXin CLI 工作流（v0.3.0）：

构建：moxin build
运行：moxin run --output json  → stdout 每行一条 JSON event
快照：build/.moxin-state.json  → 当前状态
断言（看退出码 0/1/2）：
  - moxin assert --pin D13 --eq HIGH --after 1s
  - moxin assert --pin D13 --toggles --within 3s
  - moxin assert --serial-contains "hello" --within 2s

限制：v0.3.0 只有 D13 引脚可观测。
其它板子（GD32VF103）build/run 会 bail "not yet implemented"。
```

把这段贴到 Claude Code 的项目级 prompt 末尾，AI 就具备了驱动 MoXin 的基础能力。
