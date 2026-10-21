// Package cli exposes command routing while keeping all repository operations in app.
package cli

import (
	"context"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"gw/internal/app"
	"gw/internal/discovery"
	"gw/internal/model"
	"gw/internal/scheduler"
	"gw/internal/state"
	"gw/internal/tui"
	"io"
	"os"
	"path/filepath"
)

const Help = `gw：Git 仓库状态查询与配置管理

用法：gw [--home 绝对路径] [命令]
不传命令时进入 TUI；状态查询使用 gw status。检查需在 TUI 中确认、显式 check 或系统定时触发。

  tui                              打开交互界面，不自动检查
  status [--json]                  查看任务、配置和上次结果
  report [--json] [筛选]            读取完整报告
    --problems                     只看需关注或检查失败
    --no-upstream                  单独查看无远程上游的分支
    --pushes                       只看自动推送结果
  config                           查看配置 JSON
  list                             查看仓库列表
  exclusions                       查看排除目录
  add 路径...                      添加仓库根目录
  remove 路径                      移除仓库
  exclude 路径                     排除目录并移除其下仓库
  unexclude 路径                   取消排除，不自动添加仓库
  scan 路径                        按 Git 忽略规则扫描，写报告
  import-tree 路径                 扫描并导入列表
  interval 6h                      保存频率（支持 s、m、h、d）
  autopush disabled|fast_forward    保存自动推送策略
  check                            执行检查与配置的自动推送
  install                          安装命令并应用调度，不立即检查
  uninstall                        卸载定时任务，保留数据与命令
`

func encode(w io.Writer, value any) error {
	encoder := json.NewEncoder(w)
	encoder.SetIndent("", "  ")
	encoder.SetEscapeHTML(false)
	return encoder.Encode(value)
}

func ParseInterval(value string) (int, error) { return app.ParseInterval(value) }

type filterFlag struct {
	selected *app.Filter
	choice   app.Filter
}

func (f filterFlag) String() string   { return string(*f.selected) }
func (f filterFlag) IsBoolFlag() bool { return true }
func (f filterFlag) Set(value string) error {
	if value != "true" {
		return fmt.Errorf("筛选参数不接受布尔赋值")
	}
	if *f.selected != app.All && *f.selected != f.choice {
		return fmt.Errorf("报告筛选参数互斥")
	}
	*f.selected = f.choice
	return nil
}

type Installer interface {
	Install(context.Context, string, string, int) error
	Uninstall(context.Context) error
}

func Execute(ctx context.Context, args []string, out, errOut io.Writer) int {
	global := flag.NewFlagSet("gw", flag.ContinueOnError)
	global.SetOutput(errOut)
	home := global.String("home", "", "显式指定配置与报告所在目录")
	global.Usage = func() { fmt.Fprint(out, Help) }
	if e := global.Parse(args); e != nil {
		if errors.Is(e, flag.ErrHelp) {
			return 0
		}
		return 2
	}
	manager, e := scheduler.New()
	if e != nil {
		fmt.Fprintln(errOut, e)
		return 2
	}
	if *home == "" {
		location, e := manager.ReadLocation()
		if e != nil {
			fmt.Fprintf(errOut, "无法读取安装位置，请在项目中运行 ./install.sh 或显式传入 --home：%v\n", e)
			return 2
		}
		*home = location.Home
	}
	if !filepath.IsAbs(*home) {
		fmt.Fprintln(errOut, "--home 必须是绝对路径")
		return 2
	}
	service := app.New(filepath.Clean(*home), manager)
	source, e := os.Executable()
	if e != nil {
		fmt.Fprintln(errOut, e)
		return 2
	}
	return Run(ctx, service, manager, source, global.Args(), out, errOut)
}
func Run(ctx context.Context, s *app.Service, installer Installer, source string, args []string, out, errOut io.Writer) int {
	command := "tui"
	rest := []string{}
	if len(args) > 0 {
		command = args[0]
		rest = args[1:]
	}
	if command == "help" {
		fmt.Fprint(out, Help)
		return 0
	}
	code, e := run(ctx, s, installer, source, command, rest, out, errOut)
	if e != nil {
		if errors.Is(e, context.Canceled) {
			fmt.Fprintln(errOut, "操作已取消")
			return 130
		}
		fmt.Fprintln(errOut, "错误：", e)
		return 2
	}
	return code
}
func run(ctx context.Context, s *app.Service, installer Installer, source, command string, args []string, out, errOut io.Writer) (int, error) {
	one := func() error {
		if len(args) != 1 {
			return fmt.Errorf("%s 要求一个参数", command)
		}
		return nil
	}
	none := func() error {
		if len(args) != 0 {
			return fmt.Errorf("%s 不接受位置参数", command)
		}
		return nil
	}
	switch command {
	case "tui":
		if e := none(); e != nil {
			return 2, e
		}
		apply := func(ctx context.Context) error {
			c, e := s.Config()
			if e != nil {
				return e
			}
			return installer.Install(ctx, s.Store.Home, source, c.Interval)
		}
		return 0, tui.Run(ctx, s, apply, os.Stdin, out)
	case "status", "report":
		flags := flag.NewFlagSet(command, flag.ContinueOnError)
		flags.SetOutput(errOut)
		machine := flags.Bool("json", false, "输出 JSON")
		filter := app.All
		if command == "report" {
			flags.Var(filterFlag{&filter, app.Problems}, "problems", "只看问题")
			flags.Var(filterFlag{&filter, app.WithoutUpstream}, "no-upstream", "无上游分支")
			flags.Var(filterFlag{&filter, app.Pushes}, "pushes", "自动推送结果")
		}
		if e := flags.Parse(args); e != nil {
			if errors.Is(e, flag.ErrHelp) {
				return 0, nil
			}
			return 2, e
		}
		if len(flags.Args()) > 0 {
			return 2, fmt.Errorf("查询命令不接受位置参数")
		}
		if command == "status" {
			snapshot, e := s.Snapshot(ctx)
			if e != nil {
				return 2, e
			}
			if *machine {
				return 0, encode(out, snapshot)
			}
			RenderStatus(out, snapshot)
			return 0, nil
		}
		r, e := s.Report()
		if errors.Is(e, state.ErrNoReport) {
			fmt.Fprintln(out, "尚未生成完整报告；等待定时执行或显式运行 check")
			return 1, nil
		}
		if e != nil {
			return 2, e
		}
		if *machine {
			return 0, encode(out, app.FilterReport(r, filter))
		}
		RenderReport(out, r, filter)
		return 0, nil
	case "config", "list", "exclusions":
		if e := none(); e != nil {
			return 2, e
		}
		c, e := s.Config()
		if e != nil {
			return 2, e
		}
		if command == "config" {
			return 0, encode(out, c)
		}
		paths := c.Repositories
		if command == "exclusions" {
			paths = c.Excluded
		}
		for _, p := range paths {
			fmt.Fprintln(out, p)
		}
		return 0, nil
	case "add":
		if len(args) == 0 {
			return 2, fmt.Errorf("add 至少需要一个仓库路径")
		}
		n, e := s.Add(ctx, args)
		if e == nil {
			fmt.Fprintf(out, "已添加 %d 个仓库\n", n)
		}
		return 0, e
	case "remove", "exclude", "unexclude":
		if e := one(); e != nil {
			return 2, e
		}
		var e error
		switch command {
		case "remove":
			e = s.Remove(args[0])
		case "exclude":
			e = s.Exclude(args[0])
		case "unexclude":
			e = s.Unexclude(args[0])
		}
		if e == nil {
			fmt.Fprintln(out, "配置已保存；不会立即检查或自动导入仓库")
		}
		return 0, e
	case "interval":
		if e := one(); e != nil {
			return 2, e
		}
		seconds, e := ParseInterval(args[0])
		if e != nil {
			return 2, e
		}
		if e = s.Interval(seconds); e != nil {
			return 2, e
		}
		fmt.Fprintln(out, "频率已保存，运行 gw install 应用；不会立即检查")
		return 0, nil
	case "autopush":
		if e := one(); e != nil {
			return 2, e
		}
		policy, e := model.ParsePolicy(args[0])
		if e != nil {
			return 2, e
		}
		if e = s.Policy(policy); e != nil {
			return 2, e
		}
		fmt.Fprintln(out, "自动推送策略已保存，下次显式检查或定时检查生效")
		return 0, nil
	case "scan", "import-tree":
		if e := one(); e != nil {
			return 2, e
		}
		mode := app.Preview
		if command == "import-tree" {
			mode = app.Import
		}
		report, e := s.Scan(ctx, args[0], mode)
		if e != nil {
			return 2, e
		}
		failures := 0
		for _, repo := range report.Repositories {
			if repo.Status == discovery.Unreadable {
				failures++
			}
		}
		fmt.Fprintf(out, "发现 %d 个仓库，其中 %d 个无法读取；详见 var/import-report.json\n", len(report.Repositories), failures)
		if failures > 0 {
			return 1, nil
		}
		return 0, nil
	case "check":
		if e := none(); e != nil {
			return 2, e
		}
		r, e := s.Run(ctx, func(event app.Event) {
			switch event := event.(type) {
			case app.Checking:
				fmt.Fprintln(out, "正在检查："+event.Path)
			case app.Checked:
				if e := encode(out, event.Repository); e != nil {
					fmt.Fprintln(errOut, e)
				}
			case app.NotificationProblem:
				fmt.Fprintln(errOut, "系统通知发送失败："+event.Notice.Error)
			}
		})
		if e != nil {
			return 2, e
		}
		fmt.Fprintf(out, "检查完成：%s，共 %d 个仓库\n", r.Status, len(r.Repositories))
		if r.Status != model.Clean {
			return 1, nil
		}
		return 0, nil
	case "install":
		if e := none(); e != nil {
			return 2, e
		}
		c, e := s.Config()
		if e != nil {
			return 2, e
		}
		if e = installer.Install(ctx, s.Store.Home, source, c.Interval); e != nil {
			return 2, e
		}
		fmt.Fprintf(out, "已安装原生 gw 命令与系统任务，每 %g 小时检查一次；未触发检查\n", float64(c.Interval)/3600)
		return 0, nil
	case "uninstall":
		if e := none(); e != nil {
			return 2, e
		}
		if e := installer.Uninstall(ctx); e != nil {
			return 2, e
		}
		fmt.Fprintln(out, "已卸载定时任务，保留配置、报告和管理命令")
		return 0, nil
	default:
		return 2, fmt.Errorf("未知命令 %q；运行 gw --help 查看帮助", command)
	}
}
