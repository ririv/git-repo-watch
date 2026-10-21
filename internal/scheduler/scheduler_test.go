package scheduler_test

import (
	"bytes"
	"encoding/xml"
	"gw/internal/scheduler"
	"io"
	"testing"
)

func TestPlistEscapesPathsAndDoesNotRunAtLoad(t *testing.T) {
	root := "/path/to/资料 & workspace"
	data := scheduler.Plist(root, "/path/to/gw", 21600)
	saved, e := scheduler.ParseSchedule(data)
	if e != nil {
		t.Fatal(e)
	}
	if got := saved.(scheduler.Saved).Interval; got != 21600 {
		t.Fatal(got)
	}
	decoder := xml.NewDecoder(bytes.NewReader(data))
	foundRoot := false
	runKey := false
	foundFalse := false
	for {
		token, e := decoder.Token()
		if e == io.EOF {
			break
		}
		if e != nil {
			t.Fatal(e)
		}
		start, ok := token.(xml.StartElement)
		if !ok {
			continue
		}
		if start.Name.Local == "string" || start.Name.Local == "key" {
			var value string
			if e = decoder.DecodeElement(&value, &start); e != nil {
				t.Fatal(e)
			}
			if value == root {
				foundRoot = true
			}
			if value == "RunAtLoad" {
				runKey = true
			}
		}
		if runKey && start.Name.Local == "false" {
			foundFalse = true
		}
	}
	if !foundRoot || !foundFalse {
		t.Fatal("路径或加载行为未保留")
	}
}
