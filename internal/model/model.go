// Package model defines closed domain outcomes and versioned serialization records.
package model

import (
	"encoding/json"
	"fmt"
)

type PushPolicy string

const (
	Disabled    PushPolicy = "disabled"
	FastForward PushPolicy = "fast_forward"
)

func ParsePolicy(value string) (PushPolicy, error) {
	p := PushPolicy(value)
	if p != Disabled && p != FastForward {
		return "", fmt.Errorf("未知自动推送策略：%s", value)
	}
	return p, nil
}

type Status string

const (
	Clean        Status = "clean"
	Attention    Status = "attention"
	Failed       Status = "failed"
	Unconfigured Status = "unconfigured"
)

type IssueKind string

const (
	Dirty           IssueKind = "dirty"
	Detached        IssueKind = "detached_head"
	NoCommits       IssueKind = "no_commits"
	MissingUpstream IssueKind = "missing_upstream"
	OutOfSync       IssueKind = "out_of_sync"
	PushError       IssueKind = "push_failed"
)

type Issue struct {
	Kind   IssueKind `json:"kind"`
	Detail string    `json:"detail"`
}
type UpstreamReason string

const (
	NotConfigured UpstreamReason = "not_configured"
	LocalUpstream UpstreamReason = "local_upstream"
)

type BranchWithoutUpstream struct {
	Branch string         `json:"branch"`
	Reason UpstreamReason `json:"reason"`
}
type Phase string

const (
	Validate Phase = "validate"
	Transfer Phase = "transfer"
	Verify   Phase = "verify"
)

type PushTarget struct {
	Branch    string `json:"branch"`
	LocalRef  string `json:"local_ref"`
	LocalOID  string `json:"local_oid"`
	Remote    string `json:"remote"`
	RemoteRef string `json:"remote_ref"`
	RemoteOID string `json:"remote_oid"`
	Ahead     int    `json:"ahead_commits"`
}

// Push is a closed union: an acknowledged publication or a failed attempt.
type Push interface {
	push()
	Record() PushRecord
}
type pushed struct{ target PushTarget }
type pushFailed struct {
	target PushTarget
	phase  Phase
	err    string
}

func (pushed) push()                                         {}
func (pushFailed) push()                                     {}
func Published(t PushTarget) Push                            { return pushed{t} }
func PublicationFailed(t PushTarget, p Phase, e string) Push { return pushFailed{t, p, e} }
func (p pushed) Record() PushRecord                          { return PushRecord{Status: "pushed", Target: p.target} }
func (p pushFailed) Record() PushRecord {
	return PushRecord{Status: "failed", Target: p.target, Phase: p.phase, Error: p.err}
}

type PushRecord struct {
	Status string     `json:"status"`
	Target PushTarget `json:"target"`
	Phase  Phase      `json:"phase,omitempty"`
	Error  string     `json:"error,omitempty"`
}

// Result keeps mutually exclusive repository outcomes separate from wire DTOs.
type Result interface {
	result()
	Record() Repository
}
type clean struct {
	path     string
	branches []BranchWithoutUpstream
	pushes   []PushRecord
}
type attention struct {
	path     string
	first    Issue
	rest     []Issue
	branches []BranchWithoutUpstream
	pushes   []PushRecord
}
type failed struct {
	path, err string
	pushes    []PushRecord
}

func (clean) result()                                                      {}
func (attention) result()                                                  {}
func (failed) result()                                                     {}
func Normal(path string, b []BranchWithoutUpstream, p []PushRecord) Result { return clean{path, b, p} }
func NeedsAttention(path string, first Issue, rest []Issue, b []BranchWithoutUpstream, p []PushRecord) Result {
	return attention{path, first, rest, b, p}
}
func Failure(path, err string, p []PushRecord) Result { return failed{path, err, p} }
func (r clean) Record() Repository {
	return Repository{Status: Clean, Path: r.path, Branches: r.branches, Pushes: r.pushes}
}
func (r attention) Record() Repository {
	return Repository{Status: Attention, Path: r.path, Issues: append([]Issue{r.first}, r.rest...), Branches: r.branches, Pushes: r.pushes}
}
func (r failed) Record() Repository {
	return Repository{Status: Failed, Path: r.path, Error: r.err, Pushes: r.pushes}
}

// Repository is the JSON boundary, never the state machine used by the checker.
type Repository struct {
	Status   Status
	Path     string
	Issues   []Issue
	Branches []BranchWithoutUpstream
	Error    string
	Pushes   []PushRecord
}

func nonnil[T any](v []T) []T {
	if v == nil {
		return []T{}
	}
	return v
}
func (r Repository) MarshalJSON() ([]byte, error) {
	base := map[string]any{"status": r.Status, "path": r.Path, "pushes": nonnil(r.Pushes)}
	switch r.Status {
	case Clean:
		base["branches_without_upstream"] = nonnil(r.Branches)
	case Attention:
		base["issues"] = r.Issues
		base["branches_without_upstream"] = nonnil(r.Branches)
	case Failed:
		base["error"] = r.Error
	default:
		return nil, fmt.Errorf("未知仓库状态：%s", r.Status)
	}
	return json.Marshal(base)
}
func (r *Repository) UnmarshalJSON(data []byte) error {
	var wire struct {
		Status   Status                  `json:"status"`
		Path     string                  `json:"path"`
		Issues   []Issue                 `json:"issues"`
		Branches []BranchWithoutUpstream `json:"branches_without_upstream"`
		Error    string                  `json:"error"`
		Pushes   []PushRecord            `json:"pushes"`
	}
	if err := json.Unmarshal(data, &wire); err != nil {
		return err
	}
	switch wire.Status {
	case Clean:
		if len(wire.Issues) > 0 || wire.Error != "" {
			return fmt.Errorf("正常仓库包含错误状态")
		}
	case Attention:
		if len(wire.Issues) == 0 || wire.Error != "" {
			return fmt.Errorf("需关注仓库缺少有效问题列表")
		}
	case Failed:
		if wire.Error == "" || len(wire.Issues) > 0 || len(wire.Branches) > 0 {
			return fmt.Errorf("失败仓库的状态字段无效")
		}
	default:
		return fmt.Errorf("未知仓库状态：%s", wire.Status)
	}
	*r = Repository{wire.Status, wire.Path, wire.Issues, wire.Branches, wire.Error, wire.Pushes}
	return nil
}

type Notice interface {
	notice()
	Record() NoticeRecord
}
type accepted struct{ path, branch string }
type noticeFailed struct{ path, branch, err string }

func (accepted) notice()                                 {}
func (noticeFailed) notice()                             {}
func Accepted(path, branch string) Notice                { return accepted{path, branch} }
func NotificationFailed(path, branch, err string) Notice { return noticeFailed{path, branch, err} }
func (n accepted) Record() NoticeRecord {
	return NoticeRecord{Status: "accepted", Path: n.path, Branch: n.branch}
}
func (n noticeFailed) Record() NoticeRecord {
	return NoticeRecord{Status: "failed", Path: n.path, Branch: n.branch, Error: n.err}
}

type NoticeRecord struct {
	Status string `json:"status"`
	Path   string `json:"path"`
	Branch string `json:"branch"`
	Error  string `json:"error,omitempty"`
}
type Report struct {
	SchemaVersion           int            `json:"schema_version"`
	PushPolicy              PushPolicy     `json:"push_policy"`
	Notifications           []NoticeRecord `json:"notifications"`
	StartedAt               string         `json:"started_at"`
	FinishedAt              string         `json:"finished_at"`
	Status                  Status         `json:"status"`
	Repositories            []Repository   `json:"repositories"`
	ClassificationUpdatedAt string         `json:"classification_updated_at,omitempty"`
}

func (r Report) Validate() error {
	if r.SchemaVersion != 3 {
		return fmt.Errorf("不支持的报告版本：%d", r.SchemaVersion)
	}
	if _, e := ParsePolicy(string(r.PushPolicy)); e != nil {
		return e
	}
	switch r.Status {
	case Clean, Attention, Unconfigured:
	default:
		return fmt.Errorf("未知报告状态：%s", r.Status)
	}
	return nil
}
