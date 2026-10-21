// Package gitcmd executes cancellable Git commands with binary-safe output.
package gitcmd

import (
	"bytes"
	"context"
	"fmt"
	"os"
	"os/exec"
	"strings"
	"syscall"
	"time"
)

type Runner interface {
	Run(context.Context, string, ...string) ([]byte, error)
}
type OSRunner struct{ Timeout time.Duration }

func (r OSRunner) Run(ctx context.Context, dir string, args ...string) ([]byte, error) {
	ctx, cancel := context.WithTimeout(ctx, r.Timeout)
	defer cancel()
	all := append([]string{"-C", dir}, args...)
	cmd := exec.CommandContext(ctx, "/usr/bin/git", all...)
	cmd.Env = append(os.Environ(), "GIT_TERMINAL_PROMPT=0", "GIT_OPTIONAL_LOCKS=0", "GIT_SSH_COMMAND=/usr/bin/ssh -o BatchMode=yes -o ConnectTimeout=15")
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
	cmd.Cancel = func() error {
		err := syscall.Kill(-cmd.Process.Pid, syscall.SIGKILL)
		if err == syscall.ESRCH {
			return os.ErrProcessDone
		}
		return err
	}
	cmd.WaitDelay = 2 * time.Second
	var stdout, stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr
	err := cmd.Run()
	if err != nil {
		if ctx.Err() != nil {
			return nil, fmt.Errorf("git 命令未完成：%w", ctx.Err())
		}
		return nil, fmt.Errorf("git %s 失败：%w\n%s\n%s", strings.Join(args, " "), err, strings.TrimSpace(stderr.String()), strings.TrimSpace(stdout.String()))
	}
	return stdout.Bytes(), nil
}
func Text(ctx context.Context, r Runner, dir string, args ...string) (string, error) {
	data, err := r.Run(ctx, dir, args...)
	return strings.TrimSuffix(string(data), "\n"), err
}
