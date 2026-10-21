package gitcmd_test

import (
	"context"
	"errors"
	"gw/internal/gitcmd"
	"gw/internal/testkit"
	"testing"
	"time"
)

func TestTimeoutReapsProcessGroup(t *testing.T) {
	testkit.Isolate(t)
	g := gitcmd.OSRunner{Timeout: 200 * time.Millisecond}
	started := time.Now()
	_, e := g.Run(context.Background(), t.TempDir(), "-c", "alias.pause=!sleep 20", "pause")
	if !errors.Is(e, context.DeadlineExceeded) {
		t.Fatal(e)
	}
	if time.Since(started) > 3*time.Second {
		t.Fatal("子进程未按超时回收")
	}
}
func TestCancellation(t *testing.T) {
	testkit.Isolate(t)
	ctx, cancel := context.WithCancel(context.Background())
	cancel()
	_, e := gitcmd.OSRunner{Timeout: time.Second}.Run(ctx, t.TempDir(), "status")
	if !errors.Is(e, context.Canceled) {
		t.Fatal(e)
	}
}
