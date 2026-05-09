# CLI 形态北极星

![CLI Vision](cli-vision.png)

## 1. 总览

这是 MoXin CLI 形态的视觉北极星,跨 v1 / v2a / v2b / 后续 sprint 实现。
四块面板分别对应不同 sprint 的边界:板形 + 接线、Serial Monitor、AI Inspector、`moxin >` 输入条。
每块独立演进,合起来构成 `moxin shell` 完整体验。

## 2. 四个面板

### `[Arduino Uno]` 板形 + 引脚连线
**语义**:渲染当前板子的引脚布局与组件连线;`PIN13 ●——[LED: ON #]` 形式表达"哪个引脚连了什么、当前状态是什么"。
**状态**:v1 已做板载 L LED + truecolor;板形升级与连线可视化为 v2b 候选。
**最小验收**:`add led red --id led1` + `wire pin13 -> led1.a` 后,面板自动出现 `PIN13 ●——[LED]`,run 时 LED 状态实时刷新。

### `[Serial Monitor]` 程序 printf 流
**语义**:被仿真程序 `Serial.print*` 的输出实时滚动显示。
**状态**:v2b 候选,bridge 协议需新增 `serial` 事件类型。
**最小验收**:程序里 `Serial.println("Hello")` 在 run 时即时出现在面板,顺序与发送顺序一致。

### `[AI Inspector]` 状态/诊断/建议
**语义**:把当前仿真器状态(电压、引脚、计数)结构化呈现给外接 LLM,渲染模型返回的诊断与建议(`Status: OK` / `No issues detected.`)。
**状态**:新 sprint 候选,优先级未定。
**最小验收**:面板能展示一组结构化状态行 + 一段模型生成文本;模型未连接时面板降级为纯状态展示,不报错。

### `moxin >` 输入条
**语义**:始终在底部的命令输入入口,toast 形式回馈结果。
**状态**:v1 已做。
**最小验收**:已在 `feature/tui-v1` 验收通过。

## 3. 已定的设计决策

- **AI Inspector 走外接模型**:LLM API / MCP server 出来的结果,MoXin 不自训、不内置模型,只负责提供结构化状态 + 渲染外部模型回答。
- **接线靠命令驱动 + 自动布线**:用户写 `wire pin13 -> led1.a`,layout 算法自动布线,不手填坐标。算法选型未定。

## 4. 待决问题

- Serial Monitor 是否需要染色 / 过滤 / 搜索。
- 自动布线算法选型(候选:dagre / graphviz / 手写最简栅格),以及栅格 vs 节点图。

## 5. 与 sprint 路线的映射

| 面板/能力 | sprint | 备注 |
|---|---|---|
| 板载 L LED + truecolor + 输入条 | v1 | 已做 |
| Pico 接入 + Mcu/Toolchain trait | v2a | 进行中 |
| 接线可视化 + Serial Monitor + 板形升级 | v2b | 候选 |
| AI Inspector(MCP/外接模型驱动) | 新 sprint | 候选,优先级未定 |
