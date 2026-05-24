# B3 · assert DSL grammar 设计

## 任务

设计 moxin assert DSL 的语法,产出 `docs/design/assert-dsl.md`。不写解析器代码,只定义语法。

DSL 用途：让用户在 moxin.toml 或单独 .assert 文件里声明"运行时应该看到什么",CI 跑 `moxin assert` 自动验证。

最小语法集合 (起步够用)：

```
# 单点断言
assert led1.level == "on" after 100ms

# 区间断言
assert pot1.voltage_mv in [2400..2600] at t=500ms

# 序列断言
sequence "blink" {
  led1.level == "on" at 0ms
  led1.level == "off" at 500ms
  led1.level == "on" at 1000ms
  tolerance: 50ms
}

# 计数断言
assert btn1 pressed >= 3 within 2s
```

## 允许动的文件

- 新增 `docs/design/assert-dsl.md`(完整 grammar 描述,含 EBNF 或类似规范)
- 新增 `examples/assert-demo/*.assert`(3-5 个示例文件)
- 不动 src/

## 验收

```powershell
Test-Path docs/design/assert-dsl.md
(Get-ChildItem examples/assert-demo -Filter *.assert).Count -ge 3
# 文档可读,grammar 节有 EBNF 或类似
Select-String -Path docs/design/assert-dsl.md -Pattern "^## " | Measure-Object
# 至少 5 个章节: 概述 / grammar / 语义 / 示例 / 未决问题
```

## 约束

- 这一票纯设计,不写代码
- grammar 要简单,能用 pest 或 nom 一晚上写出来
- 时间单位只支持 ms / s,不支持 us / min
- 第一版不要泛型化,等 B4 实现完再回来加 feature

## commit message

`docs(B3): assert DSL grammar 设计文档`
