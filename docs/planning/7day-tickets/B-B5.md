# B5 · `moxin assert` CLI + e2e 测试

## 任务

加 CLI 子命令 `moxin assert <project-dir>`：

1. 找到目录下所有 `.assert` 文件,parse 出全部断言
2. 跑仿真,matcher 实时检查
3. 仿真跑完或所有断言出结果后退出
4. 控制台输出 pass/fail 列表,失败的标红
5. exit code: 全 pass 0,任一 fail 1,parse error 2

写至少 3 个 e2e 测试：blink 序列断言、电位器区间断言、按钮计数断言。

## 允许动的文件

- 新增 `src/cmd_assert.rs`
- `src/main.rs`(注册子命令)
- 新增 `tests/assert_e2e.rs`
- 新增 `examples/assert-demo/`(可以复用 B3 创建的)

## 验收

```powershell
cargo test assert_e2e
cargo clippy --all-targets
moxin assert examples/assert-demo
# 输出类似:
# [PASS] led1.level == "on" after 100ms
# [PASS] sequence "blink"
# [FAIL] pot1.voltage_mv in [2400..2600] at t=500ms (got 2700)
# 2 passed, 1 failed
$LASTEXITCODE   # 应该是 1
```

## 约束

- CLI 用现有的 clap 框架,不引入新的 CLI crate
- 输出彩色用 anstyle 或 console crate (按现有项目惯例)
- 不动 run / status 等其它子命令

## commit message

`feat(B5): moxin assert CLI 入口 + e2e 测试`
