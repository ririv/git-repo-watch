# 内置扩展语法

基础集合固定为 two-face 0.5.2+bat-0.26.1 的 Oniguruma 版本，补充以下定义。构建只读取仓库内文件，不请求网络；运行时不解析 YAML。

| 文件 | 上游与固定版本 | 本项目修改 |
| --- | --- | --- |
| `JSONC.sublime-syntax` | 本项目维护，MIT | 显式命名 JSONC 并复用基础 JSON 已包含的注释与尾逗号规则；注册 `.jsonc` 和 `.code-workspace` |
| `JSON5.sublime-syntax` | [jonrandy/JSON5Syntax](https://github.com/jonrandy/JSON5Syntax/tree/31121d1140068e881a957b3b8f0db31904378a49)，MIT | 修正 YAML 将 `infinity` 解析为数值的问题、数字捕获键类型；补齐 Unicode 键、正负号、十六进制、小数、科学计数法、JSON5 转义与续行 |
| `Astro.sublime-syntax` | [SublimeText/Astro](https://github.com/SublimeText/Astro/tree/a41132afc7da386ca96589f3b801a55775616a2a)，ST3 分支，MIT | 将 SCSS、Stylus、Markdown 引用连接到当前包的实际作用域；脚本使用 TypeScript；插值采用嵌套上下文，避免对象、字符串内部的 `}` 提前结束插值；frontmatter 只在开头识别，避免正文分隔线重新进入脚本 |

JSON5 词法规则参照 [JSON5 规范](https://spec.json5.org/)。第三方版权和许可原文完整保留在仓库根目录 [THIRD_PARTY_NOTICES.md](../../../THIRD_PARTY_NOTICES.md)，发布时随二进制分发。

采用 Astro 的独立 ST3 定义，是因为当前 Syntect 不实现较新 Sublime 语法的 `extends` 继承机制。直接放入依赖继承的新版定义会丢失基础规则，不能当成已支持。

构建脚本显式列出补充文件，并拒绝新增的未链接上下文。文件名及复合扩展名识别由 `src/syntax.rs` 单独维护；源码界限、取消和逐行解析由 `src/source.rs` 负责。语言扩展不改变 TUI 与引擎接口。
