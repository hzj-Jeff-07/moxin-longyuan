# B4 · assert DSL parser + matcher

## 任务

根据 B3 定下的 grammar,实现 parser + runtime matcher。

- parser：用 pest 或 nom,输入 .assert 文件,输出 `Vec<Assertion>` AST
- matcher：订阅 RunState 事件流,每个 tick 检查所有断言,记录 pass/fail/pending

## 允许动的文件

- 新增 `src/assert.rs`(AST + matcher 主体)
- 新增 `src/assert/parser.rs`(parser 子模块)
- 新增 `src/assert/matcher.rs`(matcher 子模块)
- `src/lib.rs`(加 `pub mod assert;`)
- `Cargo.toml`(加 pest 或 nom)
- 新增 `tests/assert_parser.rs`、`tests/assert_matcher.rs`

## 验收

```powershell
cargo test assert_parser
cargo test assert_matcher
cargo clippy --all-targets
# B3 的 5 个示例 .assert 全部能 parse,无错误
```

测试要点：
- B3 文档里的 4 类语法 (单点/区间/序列/计数) 各至少一个测试
- 错误信息要友好：行号 + 列号 + "expected X, got Y"
- matcher 性能：单 tick 内匹配 100 条断言 < 1ms

## 约束

- 只做 parser + matcher,不做 CLI 入口 (那是 B5)
- 解析失败不要 panic,返回 Result<Vec<Assertion>, ParseError>
- 不实现 `tolerance` 字段以外的高级特性

## commit message

`feat(B4): assert DSL parser 与 runtime matcher`
