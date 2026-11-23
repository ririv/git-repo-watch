import { expect, mock, test } from "claude-code/testing";

test("打开 Rivet Git 面板并切换改动项", async ($, on) => {
  mock.env(on, {
    RIVET_GIT_CLAUDE_PANEL: "/tools/rvgit-claude-panel",
    RIVET_GIT_CLAUDE_HOME: "/profiles/demo",
  });
  on("session.cwd", () => ({ value: "/work" }));
  on("ui.open", ($, event) => {
    expect(event.id).toBe("rvgit");
    expect(event.focus).toBe(true);
    expect(event.backgroundColor).toBe("transparent");
    return { value: { isPlaced: true } };
  });
  let renderedView = "";
  on("process.run", ($, event) => {
    expect(event.init.cwd).toBe("/work");
    expect(event.argv.slice(0, 3)).toEqual(["/tools/rvgit-claude-panel", "--home", "/profiles/demo"]);
    const [, , , view, width, , position] = event.argv;
    renderedView = view;
    const index = Number(position);
    const total = view === "changes" ? 2 : 0;
    const text = ["文件一", "文件二"][index];
    return {
      value: {
        exitCode: 0,
        stderr: "",
        stdout: JSON.stringify({
          width: Number(width),
          position: index,
          total,
          rows: [{ kind: "text", runs: [{ text, fg: "suggestion", bg: null }] }],
        }),
      },
    };
  });

  await $.command.run({ command: "rvgit", args: "" });
  const ui = await $.ui.mount({
    plugin: "rvgit-panel",
    component: "Pane",
    requestId: "rvgit",
    surface: "terminal",
    viewport: { columns: 160, rows: 40 },
    props: {
      title: "Rivet Git",
      isFocused: true,
      placement: "inline",
      bodyColumns: 80,
      scroll: { offset: 0, bodyRows: 25 },
      view: {},
    },
  });

  expect(renderedView).toBe("overview");
  await ui.press({ key: "changes" });
  expect(await ui.find({ type: "Text", text: "1/2" })).toBeDefined();
  await ui.press({ key: "next" });
  expect((await ui.find({ type: "Text", text: "文件二" }))?.props.color).toBe("suggestion");
  await ui.press({ key: "history" });
  expect(renderedView).toBe("history");
  await ui.unmount();
});
