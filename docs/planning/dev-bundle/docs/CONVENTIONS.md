# MoXin 编码约定

> 全员（含 AI）写 moxin-longyuan 代码必须遵守的规范。
> 若与现有代码冲突，以**多数现有代码风格**为准（不要为新规范一次性大改老代码）。

---

## 一、Rust 命名

| 类别 | 规则 | 例 |
|---|---|---|
| 类型（struct/enum/trait） | UpperCamelCase | `RunState`、`BridgeEvent`、`BoardImpl` |
| 函数 / 方法 / 字段 | snake_case | `apply_event`、`last_pin_event_t_us` |
| 常量 | SCREAMING_SNAKE | `SERIAL_BUFFER_CAP` |
| 模块 | snake_case 单数 | `sim`、`board`、`shell` |
| Cargo features | kebab-case | `tui-experimental` |

## 二、模块组织

- 一个文件一个模块，文件名 = 模块名
- 子模块用目录：`src/boards/mod.rs` + `src/boards/<name>.rs`
- 模块导出尽量私有，需要跨模块用 `pub(crate)`
- **不要**用 `pub use crate::xxx::*;` 这种通配重导出
- 测试在同文件底部 `#[cfg(test)] mod tests { use super::*; ... }`

## 三、错误处理

```rust
use anyhow::{Result, bail, Context};

// ✅ 正常返回
pub fn parse_pin(s: &str) -> Result<PinRef> {
    // ...
    if invalid {
        bail!("invalid pin reference `{}` — expected `board.D13` or `<id>.<pin>`", s);
    }
    Ok(PinRef::BoardName(s.into()))
}

// ✅ 链式 context
let project = Project::load(&path)
    .with_context(|| format!("failed to load project at {}", path.display()))?;

// ❌ 不要用 unwrap()
let x = foo().unwrap();   // 禁止（除非有 // SAFETY 注释解释 invariant）

// ❌ 不要 panic!() 在 lib 代码
panic!("unreachable");    // 禁止
```

错误消息：
- 全小写开头（除专有名词）
- 加修复建议 `— <how to fix>`
- 用 backtick 包变量名 / 路径
- 中文还是英文？**英文**（AI 生成的代码也保持英文，便于 grep 和拼写检查）

## 四、注释

- 模块顶部注释：1-3 行说明本模块的职责
- 公开 API 的 `///` doc comment：必填（即使一句话）
- 私有函数：复杂的写 `// ` 一句话说明意图
- 不写 "what" 写 "why"：代码已经说了 what，注释解释 why
- TODO 格式：`// TODO(D<n>-<m>): ...` 或 `// TODO(phase-2): ...`

```rust
// ✅ 好
// Use sleep(50ms) instead of yield_now() because tighter loops
// caused 30% CPU on macOS with bridge process producing < 1 event/sec.
std::thread::sleep(Duration::from_millis(50));

// ❌ 差
// Sleep 50ms
std::thread::sleep(Duration::from_millis(50));
```

## 五、依赖管理

- **Cargo.toml 现有依赖**：见 AI-CONTEXT §二
- **加新依赖**：必须在 ticket 里明说"允许加 `xxx = ` 依赖"，否则一律不加
- **版本钉法**：用 `"4"` 而非 `"4.5.13"`，让 patch 自动升
- **features**：能少则少，例如 `serde = { version = "1", features = ["derive"] }` 不加多余 feature

## 六、字符串处理

- 引用用 `&str`，存储用 `String`
- 函数参数尽量 `&str`，除非确实要 owned
- 拼接：少量用 `format!("...")`，循环大量用 `String::with_capacity + write!()`
- 路径用 `Path` / `PathBuf`，不要用 `String` 存路径

## 七、并发

- 项目坚持**同步设计**，**不引入 async / tokio**
- 共享状态：`Arc<Mutex<...>>`，目前只有 `Arc<Mutex<RunState>>` 一处
- 线程：`std::thread::spawn`，必须保存 JoinHandle，禁止 detach（这是 D2-3 修的 bug）
- 超时：用 `crossbeam-channel` 或 `std::sync::mpsc` 的 `recv_timeout`

## 八、TUI 代码（src/tui.rs）

- 用 ratatui 0.30 的 API（不要混用旧版语法）
- 每帧渲染必须无副作用 / 无 IO
- 状态读取通过 `state.lock()`，**立刻**释放锁，**不要**持锁渲染
- 快捷键的 KeyCode 处理放在一个 match 块里，不要散布

## 九、测试约定

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // 命名：<被测函数>_<场景>
    #[test]
    fn pin_ref_parse_board_digital() {
        let pr = PinRef::parse("pin13").unwrap();
        assert!(matches!(pr, PinRef::BoardDigital(13)));
    }

    #[test]
    fn pin_ref_parse_invalid_returns_err() {
        assert!(PinRef::parse("garbage").is_err());
    }
}
```

- 测试可以 `.unwrap()`
- 一个测试只测一件事
- 不要写 `#[test] fn it_works()` 这种无意义命名
- 集成测试（D7 起加）放 `tests/` 顶层目录

## 十、Bridge C 代码（bridge/*.c）

- C11 标准（`-std=c11`）
- 错误处理：返回 -1 + `fprintf(stderr, "...\n")`
- JSON 输出：手写 printf，不用 cJSON 等库（保持 bridge 轻量）
- 行末 `\n` + `fflush(stdout)`，否则 Rust 端读不到
- 不要引入新 C 依赖

## 十一、Commit message

```
<scope>(D<n>-<m>): <一句话主题>

<可选：详细说明 2-5 行>
- 改了什么
- 为什么改

ref: docs/sprint-plan.md D<n>-<m>
```

例：
```
sim(D1-2): 修 BridgeEvent::Button 的 _t_us 字段名拼错

之前字段叫 _t_us，但 bridge 发的 JSON key 是 t_us，serde 反序列化必失败，
button 事件被 reader_loop 静默吞掉。

- src/sim.rs: BridgeEvent::Button 字段改为 t_us（去掉下划线）
- src/sim.rs: apply_event 的 Button 分支同步更新 last_event_t_us
- 新增单测 apply_event_button_updates_state

ref: docs/sprint-plan.md D1-2
```

scope 用模块名：`sim` / `shell` / `tui` / `boards` / `docs` / `examples` / `schema` ...

## 十二、PR / 单次产出大小

- 一个 ticket = 一个 commit = 一次 PR（如果用 PR 流程）
- 单次 commit < 300 行 diff（除非 ticket 本身就是大文件如 schema 文档）
- 超过 300 行：拆 ticket，不要硬塞

## 十三、AI 行为红线（再次强调）

| 红线 | 后果 |
|---|---|
| 改了 ticket 未明说的文件 | 整 PR 拒收，重做 |
| 加了 ticket 未允许的依赖 | 整 PR 拒收，重做 |
| `cargo test` 不绿就交付 | 整 PR 拒收，重做 |
| `cargo clippy` 新增警告 | 修完再交，可作为单独的"fix clippy"轮 |
| 静默修改了 public API | 整 PR 拒收，重做 |
| 写"看起来对"但没跑过的代码 | 该轮 review 不计入工时，全责重做 |

## 十四、关于"风格不一致"的优先级

- 现有代码风格 > 本文档新约定
- 本文档约定 > AI 自己习惯
- 任何冲突先问用户

10 天内不做风格统一重构（那是 Phase 2 任务）。
