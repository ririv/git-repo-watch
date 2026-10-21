package repoaction

import (
	"context"
	"fmt"
	"strings"

	"gw/internal/gitcmd"
)

type Area uint8

const (
	Staged Area = iota
	Unstaged
	Untracked
)

type IndexAction uint8

const (
	Stage IndexAction = iota
	Unstage
)

type location interface {
	paths() []string
	name() string
	display() string
}
type plainPath struct{ path string }
type renamedPath struct{ before, after string }
type copiedPath struct{ before, after string }

func (p plainPath) paths() []string   { return []string{p.path} }
func (p plainPath) name() string      { return p.path }
func (p plainPath) display() string   { return fmt.Sprintf("%q", p.path) }
func (p renamedPath) paths() []string { return []string{p.before, p.after} }
func (p renamedPath) name() string    { return p.after }
func (p renamedPath) display() string { return fmt.Sprintf("%q → %q", p.before, p.after) }
func (p copiedPath) paths() []string  { return []string{p.after} }
func (p copiedPath) name() string     { return p.after }
func (p copiedPath) display() string  { return fmt.Sprintf("%q → %q", p.before, p.after) }

type Change interface {
	change()
	Area() Area
	Name() string
	Display() string
	Kind() byte
	Key() string
}
type fileChange struct {
	owner    *files
	area     Area
	kind     byte
	location location
}

func (*fileChange) change()           {}
func (c *fileChange) Area() Area      { return c.area }
func (c *fileChange) Name() string    { return c.location.name() }
func (c *fileChange) Display() string { return c.location.display() }
func (c *fileChange) Kind() byte      { return c.kind }
func (c *fileChange) Key() string     { return fmt.Sprintf("%d:%s", c.area, c.Name()) }

type Files interface {
	files()
	Path() string
	Branch() string
	Changes() []Change
}
type files struct {
	path    string
	head    gitcmd.Head
	tree    string
	changes []Change
}

func (*files) files()              {}
func (f *files) Path() string      { return f.path }
func (f *files) Branch() string    { return f.head.Branch() }
func (f *files) Changes() []Change { return append([]Change{}, f.changes...) }

func ReadFiles(ctx context.Context, g gitcmd.Runner, path string) (Files, error) {
	if e := gitcmd.RequireIdle(ctx, g, path); e != nil {
		return nil, e
	}
	head, e := gitcmd.ReadHead(ctx, g, path)
	if e != nil {
		return nil, e
	}
	tree, e := gitcmd.Text(ctx, g, path, "write-tree")
	if e != nil {
		return nil, e
	}
	data, e := g.Run(ctx, path, "status", "--porcelain=v1", "-z", "--untracked-files=all", "--ignore-submodules=none")
	if e != nil {
		return nil, e
	}
	f := &files{path: path, head: head, tree: tree, changes: []Change{}}
	if e = parseChanges(f, data); e != nil {
		return nil, e
	}
	if e = validateFiles(ctx, g, f); e != nil {
		return nil, e
	}
	return f, nil
}
func parseChanges(f *files, data []byte) error {
	records := strings.Split(string(data), "\x00")
	for i := 0; i < len(records); i++ {
		row := records[i]
		if row == "" {
			continue
		}
		if len(row) < 4 || row[2] != ' ' {
			return fmt.Errorf("Git 文件状态记录格式错误")
		}
		name := row[3:]
		x, y := row[0], row[1]
		if x == '?' && y == '?' {
			f.changes = append(f.changes, &fileChange{f, Untracked, '?', plainPath{name}})
			continue
		}
		switch string([]byte{x, y}) {
		case "DD", "AU", "UD", "UA", "DU", "AA", "UU":
			return fmt.Errorf("仓库存在未解决的冲突")
		}
		before := ""
		if x == 'R' || x == 'C' || y == 'R' || y == 'C' {
			i++
			if i >= len(records) || records[i] == "" {
				return fmt.Errorf("Git 重命名记录缺少原路径")
			}
			before = records[i]
		}
		for _, part := range []struct {
			area Area
			kind byte
		}{{Staged, x}, {Unstaged, y}} {
			if part.kind == ' ' {
				continue
			}
			if !strings.ContainsRune("MADRCT", rune(part.kind)) {
				return fmt.Errorf("不支持的 Git 文件状态：%c", part.kind)
			}
			var place location = plainPath{name}
			if part.kind == 'R' {
				place = renamedPath{before, name}
			} else if part.kind == 'C' {
				place = copiedPath{before, name}
			}
			f.changes = append(f.changes, &fileChange{f, part.area, part.kind, place})
		}
	}
	return nil
}
func validateFiles(ctx context.Context, g gitcmd.Runner, f *files) error {
	if e := gitcmd.RequireIdle(ctx, g, f.path); e != nil {
		return e
	}
	head, e := gitcmd.ReadHead(ctx, g, f.path)
	if e != nil {
		return e
	}
	if head != f.head {
		return fmt.Errorf("分支或 HEAD 已变化，请刷新文件列表")
	}
	tree, e := gitcmd.Text(ctx, g, f.path, "write-tree")
	if e != nil {
		return e
	}
	if tree != f.tree {
		return fmt.Errorf("暂存区已变化，请刷新文件列表")
	}
	return nil
}

func EditIndex(ctx context.Context, g gitcmd.Runner, snapshot Files, operation IndexAction, selection []Change) (Files, error) {
	f := snapshot.(*files)
	if len(selection) == 0 {
		return nil, fmt.Errorf("请先选择文件")
	}
	paths := []string{}
	seen := map[string]struct{}{}
	for _, item := range selection {
		change := item.(*fileChange)
		if change.owner != f {
			return nil, fmt.Errorf("文件来自不同的状态快照，请刷新后重新选择")
		}
		switch operation {
		case Stage:
			if change.area == Staged {
				return nil, fmt.Errorf("请选择未暂存或未跟踪文件")
			}
		case Unstage:
			if change.area != Staged {
				return nil, fmt.Errorf("请选择已暂存文件")
			}
		default:
			return nil, fmt.Errorf("未知暂存操作")
		}
		for _, path := range change.location.paths() {
			if _, ok := seen[path]; !ok {
				seen[path] = struct{}{}
				paths = append(paths, path)
			}
		}
	}
	if e := validateFiles(ctx, g, f); e != nil {
		return nil, e
	}
	args := []string{"--literal-pathspecs"}
	switch operation {
	case Stage:
		args = append(args, "add", "-A", "--")
	case Unstage:
		switch head := f.head.(type) {
		case gitcmd.InitialHead:
			args = append(args, "update-index", "--force-remove", "--")
		case gitcmd.ExistingHead:
			args = append(args, "restore", "--staged", "--source="+head.OID, "--")
		}
	}
	if _, e := g.Run(ctx, f.path, append(args, paths...)...); e != nil {
		return nil, e
	}
	result, e := ReadFiles(ctx, g, f.path)
	if e != nil {
		return nil, fmt.Errorf("暂存操作已执行，但刷新失败：%w", e)
	}
	return result, nil
}
