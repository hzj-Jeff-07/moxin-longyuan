# assert-blink-toggles

演示 `moxin assert --pin D13 --toggles` 的最小用例。

## 一句话

让 D13 每秒翻转一次，用 `--toggles --within 3s` 验证"灯在闪"。

## 30 秒跑通

```bash
cd examples/assert-blink-toggles
moxin build
moxin run --output json > /tmp/events.jsonl &
RUN_PID=$!
moxin assert --pin D13 --toggles --within 3s
echo "exit=$?"   # 期望 0 (PASS)
kill $RUN_PID || true
```

## 退出码语义

| 退出码 | 含义 | 触发条件 |
|---|---|---|
| 0 | PASS | 3 秒窗口内观察到至少一次 D13 翻转 |
| 1 | FAIL | (本断言模式不返回 1) |
| 2 | TIMEOUT | 窗口耗尽 D13 状态未变 → 程序卡死 / 没真正 digitalWrite |
| 其他 | anyhow 错误 | 板子不对 / artifact 未编译 / 引脚不可观测 |

## 故意做坏（回归验证）

把 `src/main.ino` 里的 `digitalWrite(LED, LOW);` 注释掉，重新 build + assert：

```bash
moxin build
moxin run --output json > /tmp/events.jsonl &
moxin assert --pin D13 --toggles --within 3s
echo "exit=$?"   # 期望 2 (TIMEOUT)
```

→ 证明断言确实能抓到"灯不闪"。

## 适用场景

CI 烟囱测试：blink 类程序最廉价的"代码没烧坏"信号。AI agent 改完 GPIO 相关代码后，先用这个 assert 跑一遍，再去做更细的验证。
