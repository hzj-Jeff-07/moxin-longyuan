# assert-serial-hello

演示 `moxin assert --serial-contains` 的最小用例。

## 一句话

让 setup() 打印 `"hello world"`，用 `--serial-contains "hello" --within 2s` 验证"程序确实跑到了 setup 末尾"。

## 30 秒跑通

```bash
cd examples/assert-serial-hello
moxin build
moxin run --output json > /tmp/events.jsonl &
RUN_PID=$!
moxin assert --serial-contains "hello" --within 2s
echo "exit=$?"   # 期望 0 (PASS)
kill $RUN_PID || true
```

## 退出码语义

| 退出码 | 含义 | 触发条件 |
|---|---|---|
| 0 | PASS | 2 秒窗口内任意一行 serial 输出包含 "hello" |
| 1 | FAIL | (本断言模式不返回 1) |
| 2 | TIMEOUT | 窗口耗尽未匹配 → Serial.begin 漏了 / print 没刷出 / 程序在 setup 之前就崩了 |
| 其他 | anyhow 错误 | 板子不对 / artifact 未编译 |

## 故意做坏（回归验证）

把 `src/main.ino` 里的 `Serial.println("hello world")` 改成 `Serial.println("hi world")`，重新 build + assert：

```bash
moxin build
moxin run --output json > /tmp/events.jsonl &
moxin assert --serial-contains "hello" --within 2s
echo "exit=$?"   # 期望 2 (TIMEOUT)，因为 "hi" 不含 "hello"
```

## 适用场景

比"灯闪"更强的代码路径证明 —— 串口输出意味着 `Serial.begin` 走通 + print buffer 刷出 + UART 时钟正确。
AI agent 在重构 setup() 后用此 assert 快速回归。

> ⚠️ 子串匹配，不是正则。`--serial-contains "hello"` 会匹配 `"hello world"` / `"sayhello"` / `"hellofoo"`，全部命中。
