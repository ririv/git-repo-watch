// Package app provides stable report statistics for CLI and future TUI consumers.
package app

import (
	"encoding/json"
	"gw/internal/model"
)

type BranchStats struct {
	Branches     int `json:"branches"`
	Repositories int `json:"repositories"`
	Unavailable  int `json:"unavailable_repositories"`
}
type Statistics struct {
	Counts               map[model.Status]int
	Pushes               map[string]int
	Branches             BranchStats
	NotificationFailures int
}

func Stats(r model.Report) Statistics {
	s := Statistics{Counts: map[model.Status]int{}, Pushes: map[string]int{}}
	for _, repo := range r.Repositories {
		s.Counts[repo.Status]++
		if repo.Status == model.Failed {
			s.Branches.Unavailable++
		} else {
			if len(repo.Branches) > 0 {
				s.Branches.Repositories++
			}
			s.Branches.Branches += len(repo.Branches)
		}
		for _, pushed := range repo.Pushes {
			s.Pushes[pushed.Status]++
		}
	}
	for _, notice := range r.Notifications {
		if notice.Status == "failed" {
			s.NotificationFailures++
		}
	}
	return s
}
func (r AvailableReport) MarshalJSON() ([]byte, error) {
	s := Stats(r.Report)
	return json.Marshal(map[string]any{"status": "available", "outcome": r.Report.Status, "scope": r.Scope, "started_at": r.Report.StartedAt, "finished_at": r.Report.FinishedAt, "counts": s.Counts, "branches_without_upstream": s.Branches, "push_policy": r.Report.PushPolicy, "push_counts": s.Pushes, "notification_failures": s.NotificationFailures})
}

type Filter string

const (
	All             Filter = "all"
	Problems        Filter = "problems"
	WithoutUpstream Filter = "no_upstream"
	Pushes          Filter = "pushes"
)

func FilterReport(r model.Report, f Filter) model.Report {
	copy := r
	copy.Repositories = []model.Repository{}
	for _, repo := range r.Repositories {
		include := false
		switch f {
		case All:
			include = true
		case Problems:
			include = repo.Status != model.Clean
		case WithoutUpstream:
			include = repo.Status != model.Failed && len(repo.Branches) > 0
		case Pushes:
			include = len(repo.Pushes) > 0
		}
		if include {
			copy.Repositories = append(copy.Repositories, repo)
		}
	}
	return copy
}
