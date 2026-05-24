# W1 · README 首屏改造 + GitHub Release v0.5.0（A 窗口领）

## 任务

1. **README 首屏改造**：当前 README 像内部文档。改成开源项目首屏：项目一句话定位 → demo gif/视频链接 → quickstart 三步 → feature 列表 → 文档索引。前 100 行内必须让访客看明白这是什么、为什么用、怎么开始。
2. **GitHub Release v0.5.0**：写 release notes，列出 7 天新增的 feature、修复的 bug、贡献者。挂上 C9 出的视频。

## 允许动的文件

- `README.md`（整个重写）
- `CHANGELOG.md`（如果有就更新，没有就新建）
- `Cargo.toml`（version 升 0.5.0）
- 不动其它源码

## 验收

```powershell
cargo build --release
git tag v0.5.0
# 在本地预览 README 渲染
# 用 gh CLI 起草 release(不要直接 publish,先 draft)
gh release create v0.5.0 --draft --notes-file CHANGELOG.md
```

README 首屏要求：
- 第一行 H1 项目名 + 一句话定位
- 第二行 badges（CI / crates.io / license）
- demo 视频/gif 链接放在 H1 下面，不超过 5 行
- Quickstart 三步：install、run example、see result

## 约束

- 不动 docs/ 下的任何文件（那是 C 窗口的 W3 范围）
- 不动 source code，只动文档和版本号
- Release 先 draft，等 W2 财经部素材 + W3 backlog 都齐了再 publish

## commit message

`release(W1): README 首屏改造 + v0.5.0 release`
