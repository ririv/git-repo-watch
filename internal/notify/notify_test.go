package notify

import (
	"context"
	"gw/internal/model"
	"strings"
	"testing"
)

func TestCanceledNotificationIsExplicitFailure(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	cancel()
	failure := model.PushRecord{Status: "failed", Target: model.PushTarget{Branch: "main"}, Phase: model.Transfer, Error: "server rejected"}
	r := (MacOS{}).Failure(ctx, "/path/to/repo", failure).Record()
	if r.Status != "failed" || !strings.Contains(r.Error, "context canceled") {
		t.Fatal(r)
	}
}
