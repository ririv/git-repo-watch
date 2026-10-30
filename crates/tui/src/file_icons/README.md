# 文件类型图标

图标与配色映射取自 nvim-tree/nvim-web-devicons 的固定提交 `ec309a0eaff6ea967e58c275996155ab5c16938d`，按 MIT 许可使用，原文见同目录 `LICENSE`。字形由终端的 Nerd Font 支持提供，本项目不分发字体文件。

`data.rs` 包含归一化后的 221 条文件名映射和 493 条扩展名映射。查找时先匹配完整文件名，再按最长复合扩展名匹配；未收录类型明确使用通用文件图标。匹配不读取文件元数据或内容，旧提交路径及已删除文件也可使用。

## 更新数据

在项目根目录执行，版本参数必须为上游完整提交哈希：

```bash
python3 tools/update-file-icons.py --revision ec309a0eaff6ea967e58c275996155ab5c16938d
cargo test -p tui file_icons -- --test-threads=4
```

更新脚本只下载、解析固定 Lua 表中的名称、单字符图标与 RGB 颜色，不执行 Lua。遇到未识别条目或大小写映射冲突会停止；生成表排序且去重，测试验证字符格宽和匹配优先级。同步更新本说明及 THIRD_PARTY_NOTICES.md 中的上游提交。构建与运行均不需要网络。
