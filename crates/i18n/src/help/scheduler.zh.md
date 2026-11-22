
## 开发者功能 · 系统定时任务
- 流程编辑页勾选步骤即可加入，取消勾选即可移除；`↑↓` 选择步骤与底部保存、取消，`Enter`／空格操作当前项，`←→` 选择独立“操作”面板中的上移、下移。点击步骤也可直接勾选，保存后生效。
- `/schedule` 打开定时任务页。状态、间隔、时间基准和执行流程各自显示当前值与对应按钮，单击按钮即可修改或进入启停确认；↑↓ 选择，Enter 操作。
- `/schedule actions` 编辑执行流程；`/schedule interval <duration>` 设置间隔，如 6h。
- `/schedule anchor <datetime|auto>` 设置本地时间基准，可填 HH:MM[:SS] 或 YYYY-MM-DD HH:MM[:SS]；auto 在启用时重新生成。
- 基准缺省时，首次启用会自动生成并保存，后续启停和重启沿用；定时任务页显示基准与当前已应用任务的下次触发。
- 间隔支持 s / m / h / d / w（秒、分、小时、天、周，至少 60 秒），保存后用 `/schedule apply` 应用。
- `/schedule apply` 应用配置；`/schedule enabled [true|false]` 选择启用或停用，不填参数时显示选项菜单并标记当前状态，确认后执行。
> 定时流程默认 fetch → inspect；可添加 pull、push 并调整顺序。inspect 固定最后，push 前先 fetch 或 pull。某步失败即停止此仓库的后续步骤，详情保存在后台任务中。手动检查仍只执行 fetch 和本地读取。
