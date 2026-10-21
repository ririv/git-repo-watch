// Package app defines shared configuration input validation.
package app

import (
	"fmt"
	"regexp"
	"strconv"
)

var durationPattern = regexp.MustCompile(`^([1-9][0-9]*)([smhd]?)$`)

func ParseInterval(value string) (int, error) {
	match := durationPattern.FindStringSubmatch(value)
	if match == nil {
		return 0, fmt.Errorf("频率格式应为 6h、30m、1d 或整数秒")
	}
	n, e := strconv.ParseInt(match[1], 10, 64)
	if e != nil {
		return 0, e
	}
	scale := map[string]int64{"": 1, "s": 1, "m": 60, "h": 3600, "d": 86400}[match[2]]
	limit := int64(^uint(0) >> 1)
	if n > limit/scale {
		return 0, fmt.Errorf("检查间隔过大")
	}
	seconds := int(n * scale)
	if seconds < 60 {
		return 0, fmt.Errorf("检查间隔至少为 60 秒")
	}
	return seconds, nil
}
