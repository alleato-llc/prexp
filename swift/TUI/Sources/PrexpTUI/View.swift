import PrexpCore
import Tint

/// Renders the whole TUI frame from an `AppModel` into a `Buffer`. Pure and
/// side-effect free, so it can be unit-tested by rendering to an offscreen buffer
/// and inspecting the text (the tint/immediate-mode testing style).
public enum View {
    static let headerHeight = 5

    public static func render(_ model: AppModel, area: Rect, buffer: inout Buffer) {
        guard !area.isEmpty else { return }

        var constraints: [Constraint] = []
        let showHeader = model.showStats && area.height > headerHeight + 4
        if showHeader { constraints.append(.fixed(headerHeight)) }
        constraints.append(.fill)        // process table
        constraints.append(.fixed(1))    // status / search bar
        let slots = Layout(direction: .vertical, constraints: constraints).split(area)

        var i = 0
        if showHeader { renderHeader(model, area: slots[i], buffer: &buffer); i += 1 }
        renderTable(model, area: slots[i], buffer: &buffer); i += 1
        renderStatus(model, area: slots[i], buffer: &buffer)

        switch model.overlay {
        case .detail: renderDetail(model, screen: area, buffer: &buffer)
        case .help: renderHelp(model, screen: area, buffer: &buffer)
        case .none: break
        }
    }

    // MARK: Process table

    private static let columns: [Table.Column] = [
        .init("PID", width: .fixed(7)),
        .init("NAME", width: .fill),
        .init("CPU%", width: .fixed(6)),
        .init("MEM", width: .fixed(8)),
        .init("PMEM", width: .fixed(8)),
        .init("THR", width: .fixed(4)),
        .init("FILES", width: .fixed(6)),
        .init("TOTAL", width: .fixed(6)),
    ]

    private static func renderTable(_ model: AppModel, area: Rect, buffer: inout Buffer) {
        let theme = model.theme
        var title = "Processes"
        if model.isSearching || !model.searchQuery.isEmpty { title += " [/\(model.searchQuery)]" }
        title += "  ·  \(model.sort.label) \(model.reversed ? "▲" : "▼")"

        Block(title: title, titleStyle: theme.title, borderStyle: .rounded, style: theme.border)
            .render(area: area, buffer: &buffer)
        let inner = area.inner
        guard !inner.isEmpty else { return }

        let rows = model.processes.map { p -> Table.Row in
            let pct = model.cpuPercent(p.pid)
            let cells = [
                "\(p.pid)", p.name, Format.percent(pct),
                Format.bytes(p.memory.rss), Format.bytes(p.memory.phys),
                "\(p.activity.threadCount)", "\(p.count(of: .file))", "\(p.resources.count)",
            ]
            let style: Style
            if !p.accessible { style = theme.muted }
            else { style = Style(fg: theme.loadColor(pct)) }
            return Table.Row(cells, style: style)
        }

        Table(columns: columns, rows: rows,
              selected: model.processes.isEmpty ? nil : model.selected,
              headerStyle: theme.secondary,
              highlightStyle: theme.highlight,
              columnSpacing: 1)
            .render(area: inner, buffer: &buffer)
    }

    // MARK: Status / search bar

    private static func renderStatus(_ model: AppModel, area: Rect, buffer: inout Buffer) {
        let theme = model.theme
        buffer.fill(area, cell: Cell(character: " ", style: theme.statusBar))

        if model.isSearching {
            let prompt = "/ "
            buffer.write(prompt, x: area.x, y: area.y, style: theme.statusBar)
            TextInput(model.searchQuery, placeholder: "filter by name or pid",
                      style: theme.statusBar,
                      placeholderStyle: theme.statusBar.merging(Style(dim: true)),
                      cursorStyle: Style(reversed: true))
                .render(area: Rect(x: area.x + prompt.count, y: area.y,
                                   width: area.width - prompt.count, height: 1), buffer: &buffer)
            return
        }

        if let err = model.error {
            buffer.write(" error: \(err)".prefix(area.width).description, x: area.x, y: area.y,
                         style: theme.statusBar.merging(Style(fg: .red)))
            return
        }

        let hint = " q Quit  j/k Nav  Enter Detail  / Search  s Sort  g Stats  ? Help "
        let count = "\(model.processes.count) procs "
        buffer.write(String(hint.prefix(area.width)), x: area.x, y: area.y, style: theme.statusBar)
        if area.width > count.count + 2 {
            buffer.write(count, x: area.right - count.count, y: area.y, style: theme.statusBar)
        }
    }

    // MARK: Stats header

    private static func renderHeader(_ model: AppModel, area: Rect, buffer: inout Buffer) {
        let theme = model.theme
        Block(title: "System", titleStyle: theme.title, borderStyle: .rounded, style: theme.border)
            .render(area: area, buffer: &buffer)
        let inner = area.inner
        guard inner.height >= 3 else { return }

        // Line 1: per-core compact usage (index:pct), P/E aware, truncated to width.
        var coreParts: [String] = []
        for (idx, usage) in model.coreUsage.enumerated() {
            let tag = idx < model.perfLevels.count
                ? (model.perfLevels[idx] == .performance ? "P" : model.perfLevels[idx] == .efficiency ? "E" : "")
                : ""
            coreParts.append(String(format: "%@%d:%2.0f%%", tag, idx, usage))
        }
        let coreLine = coreParts.joined(separator: " ")
        buffer.write(String(coreLine.prefix(inner.width)), x: inner.x, y: inner.y, style: theme.primary)

        // Line 2: memory gauge.
        if let mem = model.memory, mem.total > 0 {
            let frac = Double(mem.used) / Double(mem.total)
            let label = "MEM \(Format.bytes(mem.used))/\(Format.bytes(mem.total))"
            Gauge(label: label, progress: frac, labelStyle: theme.secondary,
                  barStyle: Style(fg: theme.loadColor(frac * 100)), percentStyle: theme.secondary)
                .render(area: Rect(x: inner.x, y: inner.y + 1, width: inner.width, height: 1), buffer: &buffer)
        }

        // Line 3: aggregate + load + counts.
        let avg = model.coreUsage.isEmpty ? 0 : model.coreUsage.reduce(0, +) / Double(model.coreUsage.count)
        let threads = model.allProcesses.reduce(0) { $0 + Int($1.activity.threadCount) }
        let line3 = String(format: "CPU %2.0f%%  load %.2f %.2f %.2f  %d procs  %d threads",
                           avg, model.load.0, model.load.1, model.load.2,
                           model.allProcesses.count, threads)
        buffer.write(String(line3.prefix(inner.width)), x: inner.x, y: inner.y + 2, style: theme.secondary)
    }

    // MARK: Overlays

    private static func renderDetail(_ model: AppModel, screen: Rect, buffer: inout Buffer) {
        guard let p = model.selectedProcess else { return }
        let theme = model.theme
        Popup(widthPercent: 84, heightPercent: 80, clearStyle: Style()) {
            Block(title: "\(p.name) (pid \(p.pid))", titleStyle: theme.title,
                  borderStyle: .rounded, style: theme.border)
        }.render(area: screen, buffer: &buffer)

        // Content into the popup's inner area (recompute the same centered rect).
        let rect = screen.centered(widthPercent: 84, heightPercent: 80).inner
        guard !rect.isEmpty else { return }
        var lines: [TextLine] = [
            TextLine([StyledSpan("\(pad("FD", 5)) \(pad("KIND", 8)) PATH", style: theme.secondary)]),
        ]
        for r in p.resources.prefix(max(0, rect.height - 1)) {
            let path = r.path ?? "-"
            lines.append(TextLine("\(pad("\(r.descriptor)", 5)) \(pad(r.kind.rawValue, 8)) \(path)",
                                  style: theme.primary))
        }
        Text(lines: lines).render(area: rect, buffer: &buffer)
    }

    private static func renderHelp(_ model: AppModel, screen: Rect, buffer: inout Buffer) {
        let theme = model.theme
        Popup(width: 48, height: 16, clearStyle: Style()) {
            Block(title: "Help", titleStyle: theme.title, borderStyle: .rounded, style: theme.border)
        }.render(area: screen, buffer: &buffer)
        let rect = screen.centered(width: 48, height: 16).inner
        guard !rect.isEmpty else { return }
        let keys: [(String, String)] = [
            ("j / k, ↑ / ↓", "Move selection"),
            ("Enter", "Open / close detail"),
            ("/", "Search by name or pid"),
            ("s / S", "Cycle sort / reverse"),
            ("g", "Toggle system header"),
            ("R", "Force refresh"),
            ("q / Esc", "Quit / close overlay"),
            ("?", "This help"),
        ]
        let lines = keys.map { TextLine([
            StyledSpan(pad($0.0, 15), style: theme.accent),
            StyledSpan($0.1, style: theme.primary),
        ]) }
        Text(lines: lines).render(area: rect, buffer: &buffer)
    }

    /// Left-pad/truncate a string to a fixed column width.
    private static func pad(_ s: String, _ width: Int) -> String {
        if s.count >= width { return String(s.prefix(width)) }
        return s + String(repeating: " ", count: width - s.count)
    }
}
