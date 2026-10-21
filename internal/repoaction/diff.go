package repoaction

import (
	"context"
	"fmt"
	"os"
	"path/filepath"

	"gw/internal/gitcmd"
)

func FileDiff(ctx context.Context, g gitcmd.Runner, snapshot Files, item Change) (string, error) {
	f := snapshot.(*files)
	change := item.(*fileChange)
	if change.owner != f {
		return "", fmt.Errorf("文件来自不同状态快照，请刷新列表")
	}
	if e := validateFiles(ctx, g, f); e != nil {
		return "", e
	}
	args := []string{"--no-color", "--no-ext-diff", "--no-textconv"}
	var output []byte
	var e error
	if change.area == Untracked {
		path := filepath.Join(f.path, change.Name())
		info, err := os.Lstat(path)
		if err != nil {
			return "", err
		}
		if info.Mode()&os.ModeSymlink != 0 {
			target, err := os.Readlink(path)
			return fmt.Sprintf("新符号链接 %q → %q", change.Name(), target), err
		}
		if !info.Mode().IsRegular() {
			return fmt.Sprintf("%q 是目录或其他特殊文件，无法显示文本 diff。", change.Name()), nil
		}
		if info.Size() > 1024*1024 {
			return fmt.Sprintf("%q 为未跟踪大文件（%d 字节），请使用外部工具查看完整内容。", change.Name(), info.Size()), nil
		}
		args = append(args, "--no-index", "--", "/dev/null", path)
		output, e = gitcmd.Diff(ctx, g, f.path, args...)
	} else {
		if change.area == Staged {
			args = append(args, "--cached")
		}
		args = append(args, "--")
		args = append(args, change.location.paths()...)
		output, e = g.Run(ctx, f.path, append([]string{"--literal-pathspecs", "diff"}, args...)...)
	}
	if e != nil {
		return "", e
	}
	const limit = 128 * 1024
	if len(output) > limit {
		return string(output[:limit]) + "\n… diff 超过 128 KiB，仅显示前段。", nil
	}
	return string(output), nil
}
