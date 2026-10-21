// Package cli adapts application operations to terminal text and JSON output.
package cli

import (
	"fmt"
	"gw/internal/app"
	"gw/internal/model"
	"gw/internal/scheduler"
	"gw/internal/state"
	"io"
	"strings"
	"time"
)

func local(value string) string {
	t, e := time.Parse(time.RFC3339Nano, value)
	if e != nil {
		return value
	}
	return t.Local().Format("2006-01-02 15:04:05 MST")
}
func RenderStatus(w io.Writer, s app.Snapshot) {
	registration := "未加载"
	if s.Registration == scheduler.Loaded {
		registration = "已加载"
	}
	activity := "空闲"
	if s.Activity == state.Running {
		activity = "运行中"
	}
	policy := "已关闭"
	if s.Policy == model.FastForward {
		policy = "仅领先且不落后时普通推送"
	}
	fmt.Fprintf(w, "系统任务：%s\n检查进程：%s\n配置频率：每 %g 小时\n仓库数量：%d\n自动推送：%s\n", registration, activity, float64(s.Interval)/3600, s.RepositoryCount, policy)
	switch saved := s.Schedule.(type) {
	case scheduler.Missing:
		fmt.Fprintln(w, "系统任务配置文件：不存在")
	case scheduler.Saved:
		fmt.Fprintf(w, "系统任务频率：每 %g 小时\n", float64(saved.Interval)/3600)
		if saved.Interval != s.Interval {
			fmt.Fprintln(w, "频率尚未应用，请运行 gw install")
		}
	}
	if len(s.Excluded) > 0 {
		fmt.Fprintln(w, "排除目录："+strings.Join(s.Excluded, "、"))
	}
	switch saved := s.Report.(type) {
	case app.MissingReport:
		fmt.Fprintln(w, "最近报告：尚未生成完整报告")
	case app.AvailableReport:
		stats := app.Stats(saved.Report)
		fmt.Fprintf(w, "最近检查：%s\n最近结果：%d 个正常，%d 个需关注，%d 个检查失败\n", local(saved.Report.FinishedAt), stats.Counts[model.Clean], stats.Counts[model.Attention], stats.Counts[model.Failed])
		fmt.Fprintf(w, "最近自动推送：%d 个分支成功，%d 个分支失败\n", stats.Pushes["pushed"], stats.Pushes["failed"])
		if stats.NotificationFailures > 0 {
			fmt.Fprintf(w, "系统通知发送失败：%d 条，详情见 gw report --pushes\n", stats.NotificationFailures)
		}
		if saved.Report.PushPolicy == model.Disabled && s.Policy == model.FastForward {
			fmt.Fprintln(w, "最近报告来自自动推送启用前；将在下次检查时应用")
		}
		fmt.Fprintf(w, "未配置远程上游（单独统计）：%d 个分支，涉及 %d 个仓库\n", stats.Branches.Branches, stats.Branches.Repositories)
		if stats.Branches.Unavailable > 0 {
			fmt.Fprintf(w, "另有 %d 个仓库检查失败，此项统计不完整\n", stats.Branches.Unavailable)
		}
		if saved.Report.Status == model.Unconfigured {
			fmt.Fprintln(w, "该轮检查未配置仓库")
		}
		if saved.Scope == app.Outdated {
			fmt.Fprintln(w, "报告范围与当前仓库列表不同，等待下一轮检查或显式运行 check")
		}
	}
}
func RenderReport(w io.Writer, r model.Report, filter app.Filter) {
	fmt.Fprintln(w, "检查数据时间："+local(r.FinishedAt))
	if r.ClassificationUpdatedAt != "" {
		fmt.Fprintln(w, "分类规则更新时间："+local(r.ClassificationUpdatedAt))
	}
	labels := map[model.Status]string{model.Clean: "正常", model.Attention: "需关注", model.Failed: "检查失败"}
	for _, repo := range app.FilterReport(r, filter).Repositories {
		if filter == app.WithoutUpstream {
			fmt.Fprintln(w, "[未配置远程上游] "+repo.Path)
		} else {
			fmt.Fprintf(w, "[%s] %s\n", labels[repo.Status], repo.Path)
			if repo.Status == model.Attention && filter != app.Pushes {
				for _, issue := range repo.Issues {
					fmt.Fprintln(w, "  - "+issue.Detail)
				}
			}
			if repo.Status == model.Failed {
				fmt.Fprintln(w, "  - "+repo.Error)
			}
		}
		if filter != app.WithoutUpstream {
			for _, pushed := range repo.Pushes {
				target := pushed.Target
				if pushed.Status == "pushed" {
					fmt.Fprintf(w, "  - 已推送 %s → %s/%s，%d 个提交\n", target.Branch, target.Remote, target.RemoteRef, target.Ahead)
				} else {
					fmt.Fprintf(w, "  - 推送失败 %s（%s）：%s\n", target.Branch, pushed.Phase, pushed.Error)
				}
			}
			for _, notice := range r.Notifications {
				if notice.Path == repo.Path && notice.Status == "failed" {
					fmt.Fprintln(w, "  - 系统通知发送失败："+notice.Error)
				}
			}
		}
		if repo.Status != model.Failed && (filter == app.All || filter == app.WithoutUpstream) {
			for _, branch := range repo.Branches {
				reason := "未配置上游"
				if branch.Reason == model.LocalUpstream {
					reason = "上游是本地分支"
				}
				fmt.Fprintf(w, "  - %s：%s（单独统计）\n", branch.Branch, reason)
			}
		}
	}
}
