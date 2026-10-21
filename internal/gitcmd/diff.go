package gitcmd

import (
	"context"
	"errors"
	"os/exec"
)

// Diff accepts Git's documented exit status 1 for differences and preserves actual failures.
func Diff(ctx context.Context, g Runner, path string, args ...string) ([]byte, error) {
	output, e := g.Run(ctx, path, append([]string{"diff"}, args...)...)
	var status *exec.ExitError
	if errors.As(e, &status) && status.ExitCode() == 1 {
		return output, nil
	}
	return output, e
}
