//! Explicit command catalog independent of terminal rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Stage,
    Unstage,
    Diff,
    Commit,
    Push,
    CheckAll,
    Refresh,
    Settings,
    Add,
    Remove,
    Exclude,
    Unexclude,
    Scan,
    Import,
    Interval,
    Autopush,
    Concurrency,
    Apply,
    Help,
    Quit,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Repository,
    Global,
    View,
}
#[derive(Debug, Clone, Copy)]
pub struct Command {
    pub name: &'static str,
    pub action: Action,
    pub scope: Scope,
    pub description: &'static str,
}
use Action::*;
use Scope::*;
pub const COMMANDS: &[Command] = &[
    Command {
        name: "stage",
        action: Stage,
        scope: Repository,
        description: "选择工作区文件并暂存",
    },
    Command {
        name: "unstage",
        action: Unstage,
        scope: Repository,
        description: "选择暂存区文件并取消暂存",
    },
    Command {
        name: "diff",
        action: Diff,
        scope: Repository,
        description: "查看光标文件的差异",
    },
    Command {
        name: "commit",
        action: Commit,
        scope: Repository,
        description: "预览暂存内容，输入说明并确认提交",
    },
    Command {
        name: "push",
        action: Push,
        scope: Repository,
        description: "核对当前分支和远程上游后确认推送",
    },
    Command {
        name: "check all",
        action: CheckAll,
        scope: Global,
        description: "检查全部仓库，按当前配置自动推送",
    },
    Command {
        name: "refresh",
        action: Refresh,
        scope: View,
        description: "重新读取报告或文件列表",
    },
    Command {
        name: "settings",
        action: Settings,
        scope: Global,
        description: "打开设置",
    },
    Command {
        name: "add",
        action: Add,
        scope: Global,
        description: "添加仓库根目录",
    },
    Command {
        name: "remove",
        action: Remove,
        scope: Repository,
        description: "移除注册项，保留磁盘文件",
    },
    Command {
        name: "exclude",
        action: Exclude,
        scope: Global,
        description: "排除目录并移除其下注册项",
    },
    Command {
        name: "unexclude",
        action: Unexclude,
        scope: Global,
        description: "取消目录排除，不自动导入",
    },
    Command {
        name: "scan",
        action: Scan,
        scope: Global,
        description: "按 Git 忽略规则扫描预览",
    },
    Command {
        name: "import",
        action: Import,
        scope: Global,
        description: "扫描并导入仓库列表",
    },
    Command {
        name: "interval",
        action: Interval,
        scope: Global,
        description: "保存检查频率",
    },
    Command {
        name: "autopush",
        action: Autopush,
        scope: Global,
        description: "保存自动推送策略",
    },
    Command {
        name: "concurrency",
        action: Concurrency,
        scope: Global,
        description: "设置并发检查上限（1 至 32）",
    },
    Command {
        name: "apply",
        action: Apply,
        scope: Global,
        description: "应用系统调度，不立即检查",
    },
    Command {
        name: "help",
        action: Help,
        scope: Global,
        description: "查看帮助",
    },
    Command {
        name: "quit",
        action: Quit,
        scope: Global,
        description: "退出",
    },
];
pub fn matches(input: &str) -> Vec<&'static Command> {
    COMMANDS
        .iter()
        .filter(|c| c.name.starts_with(input.trim()))
        .collect()
}
pub fn exact(input: &str) -> Option<Action> {
    COMMANDS
        .iter()
        .find(|c| c.name == input.trim())
        .map(|c| c.action)
}
pub const SETTINGS: &[Action] = &[
    Interval,
    Autopush,
    Concurrency,
    Apply,
    Add,
    Exclude,
    Unexclude,
    Scan,
    Import,
];
pub fn description(action: Action) -> &'static str {
    COMMANDS
        .iter()
        .find(|c| c.action == action)
        .expect("catalog covers every action")
        .description
}
