const COMMAND = "rvgit";
const TITLE = "Rivet Git";
const VIEWS = [
  ["overview", "概览", "1"],
  ["changes", "改动", "2"],
  ["history", "历史", "3"],
];

export function register(on) {
  let view = "overview";
  let position = 0;
  let cached;

  on("session.start", async ($, event, next) => {
    await $.command.register({ name: COMMAND, description: "查看当前 Git 仓库" });
    return next(event);
  });

  on("command.run", { command: COMMAND }, async ($) => {
    cached = undefined;
    await $.ui.open({
      id: COMMAND,
      title: TITLE,
      focus: true,
      closeOnEscape: true,
      backgroundColor: "transparent",
      rows: 30,
      columns: 88,
    });
    return {};
  });

  on("ui.render", { component: "Pane" }, async ($, event, next) => {
    if (event.requestId !== COMMAND) return next(event);
    const { Box, Text, Button, Raster } = $.ui.resolve(event);
    const message = (text) => Text({ children: [text] });
    if (event.surface !== "terminal") return message(`${TITLE} 面板需要 Claude Code 终端版`);

    const columns = Math.min(160, event.props.bodyColumns);
    if (columns < 28) return message(`请加宽终端以显示 ${TITLE} 面板`);
    const rows = Math.max(10, Math.min(48, event.props.scroll.bodyRows - 2));
    const binary = await $.env.get("RIVET_GIT_CLAUDE_PANEL");
    if (!binary) return message("请将 RIVET_GIT_CLAUDE_PANEL 设为 rvgit-claude-panel 的绝对路径");

    const home = await $.env.get("RIVET_GIT_CLAUDE_HOME");
    const cwd = await $.session.cwd();
    const profile = home ? ["--home", home] : [];
    const request = [binary, ...profile, view, String(columns), String(rows), String(position)];
    const key = JSON.stringify([cwd, request]);
    if (cached?.key !== key) {
      cached = { key, result: $.process.run(request, { cwd, timeoutMs: 8000 }) };
    }

    let frame;
    try {
      const result = await cached.result;
      if (result.exitCode !== 0) throw new Error(result.stderr.trim() || `退出码 ${result.exitCode}`);
      if (result.isStdoutTruncated) throw new Error("画面数据被截断");
      frame = JSON.parse(result.stdout);
    } catch (error) {
      return message(`无法读取 ${TITLE} 面板：${error.message}`);
    }

    const redraw = () => $.ui.invalidate("ui.render");
    const tabs = VIEWS.map(([name, label, hotkey]) =>
      Button({
        key: name,
        label,
        hotkey,
        plain: true,
        dimColor: view !== name,
        onPress: () => {
          view = name;
          position = 0;
          redraw();
        },
      }),
    );
    const controls = [
      Button({
        key: "refresh",
        label: "刷新",
        hotkey: "r",
        plain: true,
        onPress: () => {
          cached = undefined;
          redraw();
        },
      }),
    ];
    if (frame.total > 0) {
      const move = (step) => {
        position = Math.max(0, Math.min(frame.total - 1, frame.position + step));
        redraw();
      };
      controls.unshift(
        Button({ key: "previous", label: "上项", hotkey: "k", plain: true, onPress: () => move(-1) }),
        Button({ key: "next", label: "下项", hotkey: "j", plain: true, onPress: () => move(1) }),
        message(`${frame.position + 1}/${frame.total}`),
      );
    }
    return Box({
      flexDirection: "column",
      children: [
        Box({ flexDirection: "row", columnGap: 2, children: tabs }),
        Box({ flexDirection: "row", columnGap: 2, children: controls }),
        ...frame.rows.map((row, index) => drawRow(Box, Text, Raster, row, frame.width, index)),
      ],
    });
  });
}

function drawRow(Box, Text, Raster, row, width, index) {
  const key = `row-${index}`;
  if (row.kind === "raster") return Raster({ key, columns: width, rows: 1, cells: row.cells });
  return Box({
    key,
    flexDirection: "row",
    height: 1,
    children: row.runs.map((run) =>
      Text({
        color: paint(run.fg),
        backgroundColor: paint(run.bg),
        bold: run.bold,
        italic: run.italic,
        dimColor: run.dim,
        wrap: "truncate-end",
        children: [run.text],
      }),
    ),
  });
}

function paint(value) {
  if (value === null) return undefined;
  if (typeof value === "string") return value;
  return `#${value.toString(16).padStart(6, "0")}`;
}
