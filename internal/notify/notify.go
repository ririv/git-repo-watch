// Package notify sends macOS notifications without interpolating executable script text.
package notify

import (
	"context"
	"fmt"
	"gw/internal/model"
	"os/exec"
	"path/filepath"
	"time"
)

const script = `on run argv
 display notification (item 1 of argv) with title "GW 自动推送失败" subtitle (item 2 of argv) sound name "Basso"
end run`

type Sender interface {
	Failure(context.Context, string, model.PushRecord) model.Notice
}
type MacOS struct{}

func (MacOS) Failure(ctx context.Context, path string, failure model.PushRecord) model.Notice {
	ctx, cancel := context.WithTimeout(ctx, 15*time.Second)
	defer cancel()
	runes := []rune(failure.Error)
	if len(runes) > 240 {
		runes = runes[:240]
	}
	message := string(runes) + "\n详情：gw report --pushes"
	subtitle := filepath.Base(path) + " / " + failure.Target.Branch
	data, e := exec.CommandContext(ctx, "/usr/bin/osascript", "-e", script, "--", message, subtitle).CombinedOutput()
	if e != nil {
		return model.NotificationFailed(path, failure.Target.Branch, fmt.Sprintf("%v：%s", e, data))
	}
	return model.Accepted(path, failure.Target.Branch)
}
