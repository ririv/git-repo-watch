package tui

import (
	"context"
	"fmt"
	"path/filepath"

	"gw/internal/app"
	"gw/internal/model"
)

func actionTitle(a action) string {
	switch a {
	case stageSelected:
		return "读取或暂存文件"
	case unstageSelected:
		return "读取或取消暂存文件"
	case diffSelected:
		return "查看文件 diff"
	case commitSelected:
		return "提交选中仓库的暂存内容"
	case pushSelected:
		return "推送选中仓库的当前分支"
	case search:
		return "搜索仓库路径"
	case add:
		return "添加仓库根目录"
	case remove:
		return "移除仓库（保留磁盘文件）"
	case exclude:
		return "排除目录并移除其下注册仓库"
	case unexclude:
		return "取消排除目录（不自动导入）"
	case scan:
		return "扫描目录（只写扫描报告）"
	case importTree:
		return "扫描并导入仓库"
	case interval:
		return "保存检查频率（如 6h，之后需应用调度）"
	case policy:
		return "保存推送策略（disabled / fast_forward）"
	case check:
		return "立即检查所有仓库，并按当前策略自动推送"
	case applySchedule:
		return "重新安装 gw 并应用调度（不立即检查）"
	}
	panic("unknown TUI action")
}
func validate(a action, value string) error {
	switch a {
	case add, exclude, unexclude, scan, importTree:
		if !filepath.IsAbs(value) {
			return fmt.Errorf("请输入绝对路径")
		}
	case interval:
		_, e := app.ParseInterval(value)
		return e
	case policy:
		_, e := model.ParsePolicy(value)
		return e
	}
	return nil
}
func perform(ctx context.Context, b Backend, apply func(context.Context) error, a action, value string, emit func(app.Event)) (string, error) {
	switch a {
	case add:
		n, e := b.Add(ctx, []string{value})
		return fmt.Sprintf("已添加 %d 个仓库", n), e
	case remove:
		return "已移除注册项，磁盘文件保留", b.Remove(value)
	case exclude:
		return "已排除目录并移除其下注册项", b.Exclude(value)
	case unexclude:
		return "已取消排除，未自动导入", b.Unexclude(value)
	case interval:
		seconds, e := app.ParseInterval(value)
		if e != nil {
			return "", e
		}
		return "频率已保存；在设置页按 l 应用调度", b.Interval(seconds)
	case policy:
		p, e := model.ParsePolicy(value)
		if e != nil {
			return "", e
		}
		return "推送策略已保存，下次检查生效", b.Policy(p)
	case scan, importTree:
		mode := app.Preview
		if a == importTree {
			mode = app.Import
		}
		r, e := b.Scan(ctx, value, mode)
		if e != nil {
			return "", e
		}
		failed := 0
		for _, repo := range r.Repositories {
			if repo.Error != "" {
				failed++
			}
		}
		return fmt.Sprintf("发现 %d 个仓库，%d 个无法读取；详见 var/import-report.json", len(r.Repositories), failed), nil
	case check:
		r, e := b.Run(ctx, emit)
		if e != nil {
			return "", e
		}
		s := app.Stats(r)
		return fmt.Sprintf("检查完成：%d 正常 · %d 需关注 · %d 失败", s.Counts[model.Clean], s.Counts[model.Attention], s.Counts[model.Failed]), nil
	case applySchedule:
		return "系统调度已应用；未触发检查", apply(ctx)
	}
	return "", fmt.Errorf("此操作不能执行")
}
