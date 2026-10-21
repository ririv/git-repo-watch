// Package discovery delegates in-repository exclusions to Git's file enumeration.
package discovery

import (
	"context"
	"errors"
	"fmt"
	"gw/internal/gitcmd"
	"gw/internal/state"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

type Kind string

const (
	Scanned    Kind = "scanned"
	Unreadable Kind = "failed"
)

type Record struct {
	Status Kind   `json:"status"`
	Path   string `json:"path"`
	Error  string `json:"error,omitempty"`
}
type Report struct {
	Root         string   `json:"root"`
	ScannedAt    string   `json:"scanned_at"`
	Excluded     []string `json:"excluded_roots"`
	Repositories []Record `json:"repositories"`
	Markers      []string `json:"non_repository_markers"`
}
type marker int

const (
	absent marker = iota
	repository
	nonRepositoryFile
)

func metadata(dir string) (marker, error) {
	p := filepath.Join(dir, ".git")
	info, e := os.Lstat(p)
	if os.IsNotExist(e) {
		return absent, nil
	}
	if e != nil {
		return absent, e
	}
	if info.IsDir() {
		return repository, nil
	}
	if info.Mode().IsRegular() {
		file, e := os.Open(p)
		if e != nil {
			return absent, e
		}
		defer file.Close()
		b := make([]byte, 8)
		n, e := file.Read(b)
		if e != nil && n == 0 && !errors.Is(e, io.EOF) {
			return absent, e
		}
		if string(b[:n]) == "gitdir: " {
			return repository, nil
		}
		return nonRepositoryFile, nil
	}
	return absent, nil
}

type tree map[string]tree

func include(t tree, path string, directory bool) {
	parts := strings.Split(strings.TrimSuffix(path, "/"), "/")
	if !directory {
		parts = parts[:len(parts)-1]
	}
	for _, part := range parts {
		if t[part] == nil {
			t[part] = tree{}
		}
		t = t[part]
	}
}
func directoryTree(ctx context.Context, g gitcmd.Runner, root string) (tree, error) {
	t := tree{}
	data, e := g.Run(ctx, root, "ls-files", "--cached", "--stage", "-z")
	if e != nil {
		return nil, e
	}
	for _, row := range strings.Split(string(data), "\x00") {
		if row == "" {
			continue
		}
		meta, path, ok := strings.Cut(row, "\t")
		if !ok {
			return nil, fmt.Errorf("Git 已跟踪文件格式错误")
		}
		include(t, path, strings.HasPrefix(meta, "160000 "))
	}
	data, e = g.Run(ctx, root, "ls-files", "--others", "--exclude-standard", "-z")
	if e != nil {
		return nil, e
	}
	for _, path := range strings.Split(string(data), "\x00") {
		if path != "" {
			include(t, path, strings.HasSuffix(path, "/"))
		}
	}
	return t, nil
}
func Discover(ctx context.Context, g gitcmd.Runner, root string, excluded []string) (Report, error) {
	root, e := filepath.EvalSymlinks(root)
	if e != nil {
		return Report{}, e
	}
	root, e = filepath.Abs(root)
	if e != nil {
		return Report{}, e
	}
	info, e := os.Stat(root)
	if e != nil {
		return Report{}, e
	}
	if !info.IsDir() {
		return Report{}, fmt.Errorf("扫描起点不是目录：%s", root)
	}
	report := Report{Root: root, Excluded: excluded, Repositories: []Record{}, Markers: []string{}}
	isExcluded := func(p string) bool {
		for _, x := range excluded {
			if state.Within(p, x) {
				return true
			}
		}
		return false
	}
	if isExcluded(root) {
		return report, nil
	}
	var visit func(string, tree) error
	visit = func(dir string, allowed tree) error {
		if e := ctx.Err(); e != nil {
			return e
		}
		if isExcluded(dir) {
			return nil
		}
		kind, e := metadata(dir)
		if e != nil {
			return e
		}
		if kind == nonRepositoryFile {
			report.Markers = append(report.Markers, filepath.Join(dir, ".git"))
		}
		if kind == repository {
			actual, e := gitcmd.Text(ctx, g, dir, "rev-parse", "--show-toplevel")
			if e == nil {
				actual, e = filepath.EvalSymlinks(actual)
				if e == nil && actual != dir {
					e = fmt.Errorf("仓库根目录不匹配：%s", actual)
				}
			}
			if e == nil {
				allowed, e = directoryTree(ctx, g, dir)
			}
			if e != nil {
				report.Repositories = append(report.Repositories, Record{Unreadable, dir, e.Error()})
				return nil
			}
			report.Repositories = append(report.Repositories, Record{Status: Scanned, Path: dir})
		}
		names := []string{}
		if allowed == nil {
			entries, e := os.ReadDir(dir)
			if e != nil {
				return e
			}
			for _, entry := range entries {
				if entry.Name() != ".git" && entry.IsDir() {
					names = append(names, entry.Name())
				}
			}
		} else {
			for name := range allowed {
				names = append(names, name)
			}
		}
		sort.Strings(names)
		for _, name := range names {
			child := filepath.Join(dir, name)
			info, e := os.Lstat(child)
			if os.IsNotExist(e) {
				continue
			}
			if e != nil {
				return e
			}
			if info.IsDir() {
				var next tree
				if allowed != nil {
					next = allowed[name]
				}
				if e = visit(child, next); e != nil {
					return e
				}
			}
		}
		return nil
	}
	var initial tree
	kind, e := metadata(root)
	if e != nil {
		return report, e
	}
	if kind != repository {
		for ancestor := filepath.Dir(root); ; ancestor = filepath.Dir(ancestor) {
			kind, e := metadata(ancestor)
			if e != nil {
				return report, e
			}
			if kind == repository {
				initial, e = directoryTree(ctx, g, ancestor)
				if e != nil {
					return report, e
				}
				rel, _ := filepath.Rel(ancestor, root)
				for _, part := range strings.Split(rel, string(os.PathSeparator)) {
					var ok bool
					initial, ok = initial[part]
					if !ok {
						return report, nil
					}
				}
				break
			}
			if filepath.Dir(ancestor) == ancestor {
				break
			}
		}
	}
	if e = visit(root, initial); e != nil {
		return report, e
	}
	sort.Slice(report.Repositories, func(i, j int) bool { return report.Repositories[i].Path < report.Repositories[j].Path })
	sort.Strings(report.Markers)
	return report, nil
}
