# 语法高亮覆盖范围

## 当前内置语法

当前固定使用 Syntect 5.3.0、two-face 0.5.2+bat-0.26.1 和 Oniguruma。two-face 的扩展集有 **220 个语法定义**，另内置 JSONC、JSON5 和 Astro 三个定义，合计 **223 个**。定义数量包括嵌入语言、辅助上下文和纯文本，不等于独立编程语言数量。

常用语言覆盖如下；扩展集中的其他语言仍可按内置定义识别。

| 类别 | 常用语言或格式 |
| --- | --- |
| 系统与后端 | Rust、Go、Python、C、C++、C#、Java、Kotlin、Scala、PHP、Ruby、Lua、R、Elixir、Zig |
| Web | JavaScript、JSX、TypeScript、TSX、HTML、CSS、SCSS、Less、Vue、Svelte、Astro |
| 应用开发 | Swift、Dart、Objective-C、Objective-C++ |
| 脚本 | Bash／Zsh、Fish、PowerShell |
| 数据与配置 | JSON、JSONC、JSON5、YAML、TOML、INI、DotENV、SQL、GraphQL、Protocol Buffer |
| 构建与部署 | Makefile、CMake、Dockerfile／Containerfile、Terraform／HCL、Nix、Go 模块与校验文件 |
| 文档与 Git | Markdown、reStructuredText、LaTeX、XML、Git 配置／忽略／提交／补丁 |

新增 JSONC 显式复用内置 JSON 已提供的行注释、块注释和尾逗号词法规则。JSON5 使用独立定义，支持不带引号的 Unicode 键、单引号字符串、转义、跨行续接、十六进制、小数和特殊数字。Astro 使用独立定义组合 TypeScript／TSX、HTML 和样式语法，不按 HTML 文件猜测处理。

语法识别和着色不承担编译、类型检查或格式合法性验证。Vue、Svelte、Astro 的回归用例包括嵌入 TypeScript、注释、标签、属性和 CSS 字符串；JSON5 另检查合法示例不进入错误词法作用域。

## 文件名识别

选择顺序为：显式文件名规则 → 语法包中的完整文件名 → 从长到短的复合扩展名 → 首行的 shebang／模式声明。这里只使用传入的路径和已读取文本，不再次读取文件、不执行命令。

- `Dockerfile`、`Dockerfile.*`、`*.Dockerfile`、`Containerfile`、`Containerfile.*` 使用 Dockerfile 语法。
- `tsconfig.json`、`tsconfig.*.json`、`jsconfig.json`、`jsconfig.*.json`、`*.jsonc` 和 `*.code-workspace` 使用 JSONC。
- `.babelrc`、`*.json5` 使用 JSON5。
- 完整文件名包括 `.env.local`、`CMakeLists.txt`、`Cargo.lock`、`uv.lock`、`go.mod`、`go.work`、`go.sum`、`.gitignore`、`.editorconfig` 等，来自内置语法定义。
- `config.h.in`、`config.hpp.in`、`page.html.j2`、`page.html.eex`、`message.pb.txt`、`style.css.erb` 等优先采用最长匹配后缀，避免只按最后一段识别。

未知语言仍明确显示未识别，只保留差异颜色；不自动改用其他语言语法。

## Syntect 默认集的完整清单

下面直接从 **Syntect 5.3.0 的 `default_newlines.packdump`** 枚举得到，共 **75 个定义**，这是我们扩展之前使用的集合。Syntect 将 Sublime Text 的默认定义预编译为内置数据，详细接口见 [SyntaxSet 文档](https://docs.rs/syntect/5.3.0/syntect/parsing/struct.SyntaxSet.html#method.load_defaults_newlines)。此处不按当前 Sublime Text 的语法目录推断旧版本覆盖范围。

默认集包含常见的 Rust、Go、Python、C／C++、Java、C#、JavaScript 等，但不包含 TypeScript、TSX、JSX 专用语法、Kotlin、Swift、Dart、Vue、Svelte、TOML、Dockerfile、PowerShell 等扩展定义。

“无直接扩展名”通常表示辅助、嵌入或输出语法，可能由其他定义引用。

| 默认定义 | 扩展名或文件名（原始定义） |
| --- | --- |
| ActionScript | `as` |
| AppleScript | `applescript`、`script editor` |
| ASP | `asa` |
| Batch File | `bat`、`cmd` |
| BibTeX | `bib` |
| Bourne Again Shell (bash) | `sh`、`bash`、`zsh`、`fish`、`.bash_aliases`、`.bash_completions`、`.bash_functions`、`.bash_login`、`.bash_logout`、`.bash_profile`、`.bash_variables`、`.bashrc`、`.profile`、`.textmate_init` |
| C | `c`、`h` |
| C# | `cs`、`csx` |
| C++ | `cpp`、`cc`、`cp`、`cxx`、`c++`、`C`、`h`、`hh`、`hpp`、`hxx`、`h++`、`inl`、`ipp` |
| camlp4 | 无直接扩展名 |
| Cargo Build Results | 无直接扩展名 |
| Clojure | `clj` |
| commands-builtin-shell-bash | 无直接扩展名 |
| CSS | `css`、`css.erb`、`css.liquid` |
| D | `d`、`di` |
| Diff | `diff`、`patch` |
| Erlang | `erl`、`hrl`、`Emakefile`、`emakefile` |
| Go | `go` |
| Graphviz (DOT) | `dot`、`DOT`、`gv` |
| Groovy | `groovy`、`gvy`、`gradle` |
| Haskell | `hs` |
| HTML | `html`、`htm`、`shtml`、`xhtml`、`inc`、`tmpl`、`tpl` |
| HTML (ASP) | `asp` |
| HTML (Erlang) | `yaws` |
| HTML (Rails) | `rails`、`rhtml`、`erb`、`html.erb` |
| HTML (Tcl) | `adp` |
| Java | `java`、`bsh` |
| Java Properties | `properties` |
| Java Server Page (JSP) | `jsp` |
| JavaDoc | 无直接扩展名 |
| JavaScript | `js`、`htc` |
| JavaScript (Rails) | `js.erb` |
| JSON | `json`、`sublime-settings`、`sublime-menu`、`sublime-keymap`、`sublime-mousemap`、`sublime-theme`、`sublime-build`、`sublime-project`、`sublime-completions`、`sublime-commands`、`sublime-macro`、`sublime-color-scheme` |
| LaTeX | `tex`、`ltx` |
| LaTeX Log | 无直接扩展名 |
| Lisp | `lisp`、`cl`、`clisp`、`l`、`mud`、`el`、`scm`、`ss`、`lsp`、`fasl` |
| Literate Haskell | `lhs` |
| Lua | `lua` |
| Make Output | 无直接扩展名 |
| Makefile | `make`、`GNUmakefile`、`makefile`、`Makefile`、`OCamlMakefile`、`mak`、`mk` |
| Markdown | `md`、`mdown`、`markdown`、`markdn` |
| MATLAB | `matlab` |
| MultiMarkdown | 无直接扩展名 |
| NAnt Build File | `build` |
| Objective-C | `m`、`h` |
| Objective-C++ | `mm`、`M`、`h` |
| OCaml | `ml`、`mli` |
| OCamllex | `mll` |
| OCamlyacc | `mly` |
| Pascal | `pas`、`p`、`dpr` |
| Perl | `pl`、`pm`、`pod`、`t`、`PL` |
| PHP | `php`、`php3`、`php4`、`php5`、`php7`、`phps`、`phpt`、`phtml` |
| PHP Source | 无直接扩展名 |
| Plain Text | `txt` |
| Python | `py`、`py3`、`pyw`、`pyi`、`pyx`、`pyx.in`、`pxd`、`pxd.in`、`pxi`、`pxi.in`、`rpy`、`cpy`、`SConstruct`、`Sconstruct`、`sconstruct`、`SConscript`、`gyp`、`gypi`、`Snakefile`、`wscript` |
| R | `R`、`r`、`s`、`S`、`Rprofile` |
| R Console | 无直接扩展名 |
| Rd (R Documentation) | `rd` |
| Regular Expression | `re` |
| Regular Expressions (Javascript) | 无直接扩展名 |
| Regular Expressions (Python) | 无直接扩展名 |
| reStructuredText | `rst`、`rest` |
| Ruby | `rb`、`Appfile`、`Appraisals`、`Berksfile`、`Brewfile`、`capfile`、`cgi`、`Cheffile`、`config.ru`、`Deliverfile`、`Fastfile`、`fcgi`、`Gemfile`、`gemspec`、`Guardfile`、`irbrc`、`jbuilder`、`podspec`、`prawn`、`rabl`、`rake`、`Rakefile`、`Rantfile`、`rbx`、`rjs`、`ruby.rail`、`Scanfile`、`simplecov`、`Snapfile`、`thor`、`Thorfile`、`Vagrantfile` |
| Ruby Haml | `haml`、`sass` |
| Ruby on Rails | `rxml`、`builder` |
| Rust | `rs` |
| Scala | `scala`、`sbt` |
| Shell-Unix-Generic | 无直接扩展名 |
| SQL | `sql`、`ddl`、`dml` |
| SQL (Rails) | `erbsql`、`sql.erb` |
| Tcl | `tcl` |
| TeX | `sty`、`cls` |
| Textile | `textile` |
| XML | `xml`、`xsd`、`xslt`、`tld`、`dtml`、`rss`、`opml`、`svg` |
| YAML | `yaml`、`yml`、`sublime-syntax` |

## 维护与发布

基础扩展集来自 [two-face](https://codeberg.org/CosmicHarper/two-face)，它打包了 bat 使用的语法。新增定义的上游版本、修改范围和许可记录在 [语法资源说明](../crates/highlight/syntaxes/README.md) 和 [第三方许可](../THIRD_PARTY_NOTICES.md)。

`highlight` crate 的构建脚本合并并预编译语法，新增定义的未链接引用会使构建失败；运行时仅加载二进制数据，不加载 YAML、不下载语法，也不依赖外部高亮工具。发布构建静态编入 Oniguruma。

更新依赖或语法时，需要核验识别结果、真实 token 颜色、混合语言及跨行上下文，并检查统一／双栏 diff 最终呈现，不能只断言语法名称。
