//! Stable editor identities and supported installation routes.
use super::Interface;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Known {
    Code,
    CodeInsiders,
    Cursor,
    Windsurf,
    Codium,
    Zed,
    Sublime,
    Bbedit,
    CotEditor,
    TextMate,
    Nova,
    TextEdit,
    Kate,
    Gedit,
    Neovim,
    Vim,
    Helix,
    Emacs,
    Nano,
    Micro,
}
pub struct Spec {
    pub token: &'static str,
    pub label: &'static str,
    pub commands: &'static [&'static str],
    pub bundle: Option<&'static str>,
    pub interface: Interface,
    pub folders: bool,
}
impl Known {
    pub const ALL: &[Self] = &[
        Self::Code,
        Self::CodeInsiders,
        Self::Cursor,
        Self::Windsurf,
        Self::Codium,
        Self::Zed,
        Self::Sublime,
        Self::Bbedit,
        Self::CotEditor,
        Self::TextMate,
        Self::Nova,
        Self::TextEdit,
        Self::Kate,
        Self::Gedit,
        Self::Neovim,
        Self::Vim,
        Self::Helix,
        Self::Emacs,
        Self::Nano,
        Self::Micro,
    ];
    pub fn spec(self) -> Spec {
        match self {
            Self::Code => Spec {
                token: "code",
                label: "Visual Studio Code",
                commands: &["code"],
                bundle: Some("com.microsoft.VSCode"),
                interface: Interface::Desktop,
                folders: true,
            },
            Self::CodeInsiders => Spec {
                token: "code-insiders",
                label: "Visual Studio Code Insiders",
                commands: &["code-insiders"],
                bundle: Some("com.microsoft.VSCodeInsiders"),
                interface: Interface::Desktop,
                folders: true,
            },
            Self::Cursor => Spec {
                token: "cursor",
                label: "Cursor",
                commands: &["cursor"],
                bundle: Some("com.todesktop.230313mzl4w4u92"),
                interface: Interface::Desktop,
                folders: true,
            },
            Self::Windsurf => Spec {
                token: "windsurf",
                label: "Windsurf",
                commands: &["windsurf"],
                bundle: Some("com.exafunction.windsurf"),
                interface: Interface::Desktop,
                folders: true,
            },
            Self::Codium => Spec {
                token: "codium",
                label: "VSCodium",
                commands: &["codium"],
                bundle: Some("com.vscodium"),
                interface: Interface::Desktop,
                folders: true,
            },
            Self::Zed => Spec {
                token: "zed",
                label: "Zed",
                commands: &["zed", "zeditor"],
                bundle: Some("dev.zed.Zed"),
                interface: Interface::Desktop,
                folders: true,
            },
            Self::Sublime => Spec {
                token: "subl",
                label: "Sublime Text",
                commands: &["subl"],
                bundle: Some("com.sublimetext.4"),
                interface: Interface::Desktop,
                folders: true,
            },
            Self::Bbedit => Spec {
                token: "bbedit",
                label: "BBEdit",
                commands: &["bbedit"],
                bundle: Some("com.barebones.bbedit"),
                interface: Interface::Desktop,
                folders: true,
            },
            Self::CotEditor => Spec {
                token: "cot",
                label: "CotEditor",
                commands: &["cot"],
                bundle: Some("com.coteditor.CotEditor"),
                interface: Interface::Desktop,
                folders: false,
            },
            Self::TextMate => Spec {
                token: "mate",
                label: "TextMate",
                commands: &["mate"],
                bundle: Some("com.macromates.TextMate"),
                interface: Interface::Desktop,
                folders: true,
            },
            Self::Nova => Spec {
                token: "nova",
                label: "Nova",
                commands: &["nova"],
                bundle: Some("com.panic.Nova"),
                interface: Interface::Desktop,
                folders: true,
            },
            Self::TextEdit => Spec {
                token: "textedit",
                label: "TextEdit",
                commands: &[],
                bundle: Some("com.apple.TextEdit"),
                interface: Interface::Desktop,
                folders: false,
            },
            Self::Kate => Spec {
                token: "kate",
                label: "Kate",
                commands: &["kate"],
                bundle: None,
                interface: Interface::Desktop,
                folders: false,
            },
            Self::Gedit => Spec {
                token: "gedit",
                label: "gedit",
                commands: &["gedit"],
                bundle: None,
                interface: Interface::Desktop,
                folders: false,
            },
            Self::Neovim => Spec {
                token: "nvim",
                label: "Neovim",
                commands: &["nvim"],
                bundle: None,
                interface: Interface::Terminal,
                folders: true,
            },
            Self::Vim => Spec {
                token: "vim",
                label: "Vim",
                commands: &["vim", "vi"],
                bundle: None,
                interface: Interface::Terminal,
                folders: true,
            },
            Self::Helix => Spec {
                token: "hx",
                label: "Helix",
                commands: &["hx"],
                bundle: None,
                interface: Interface::Terminal,
                folders: true,
            },
            Self::Emacs => Spec {
                token: "emacs",
                label: "Emacs",
                commands: &["emacs"],
                bundle: None,
                interface: Interface::Terminal,
                folders: true,
            },
            Self::Nano => Spec {
                token: "nano",
                label: "nano",
                commands: &["nano"],
                bundle: None,
                interface: Interface::Terminal,
                folders: false,
            },
            Self::Micro => Spec {
                token: "micro",
                label: "micro",
                commands: &["micro"],
                bundle: None,
                interface: Interface::Terminal,
                folders: false,
            },
        }
    }
    pub fn named(value: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|known| known.spec().token == value)
    }
    pub fn executable(value: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|known| known.spec().commands.contains(&value))
    }
}
