import Dispatch
import Foundation
import PrexpCore
import PrexpTUI
import Tint

// prexp — the Swift terminal UI. Wires tint's run loop to the PrexpTUI model over
// the native process source. A background timer refreshes the (expensive) full
// snapshot on a cadence; tint redraws the current model at its own frame rate.

// --interval <seconds> — refresh cadence (default 2.0, floored at 0.5).
var interval: Double = 2.0
var args = Array(CommandLine.arguments.dropFirst())
if let i = args.firstIndex(of: "--interval"), i + 1 < args.count, let v = Double(args[i + 1]) {
    interval = max(0.5, v)
}
if args.contains("--help") || args.contains("-h") {
    print("""
    prexp — process explorer (Swift TUI)

    USAGE: prexp [--interval <seconds>]

    Keys: j/k move · Enter detail · / search · s/S sort · g stats · R refresh · q quit · ? help
    """)
    exit(0)
}

let model = AppModel(source: NativeSource())
model.refresh(now: Date())

// --snapshot [WxH] — render one live frame to stdout and exit (no raw mode / TTY).
// Handy for previews, docs, and debugging the layout without an interactive terminal.
if let s = args.firstIndex(of: "--snapshot") {
    var w = 110, h = 34
    if s + 1 < args.count, let x = args[s + 1].firstIndex(of: "x"),
       let pw = Int(args[s + 1][..<x]), let ph = Int(args[s + 1][args[s + 1].index(after: x)...]) {
        w = pw; h = ph
    }
    if args.contains("--stats") { model.showStats = true }
    Thread.sleep(forTimeInterval: 0.15)
    model.refresh(now: Date())   // second read so CPU%/core deltas populate
    var buf = Buffer(area: Rect(x: 0, y: 0, width: w, height: h))
    View.render(model, area: buf.area, buffer: &buf)
    print(buf.allText().joined(separator: "\n"))
    exit(0)
}

let app = Application(theme: model.theme)

let refresh = DispatchSource.makeTimerSource(queue: .main)
refresh.schedule(deadline: .now() + interval, repeating: interval)
refresh.setEventHandler {
    model.refresh(now: Date())
    app.requestRender()
}
refresh.resume()

app.run(render: { area, buffer in
    View.render(model, area: area, buffer: &buffer)
}, onKey: { key in
    if !model.isSearching, key == .char("R") {
        model.refresh(now: Date())
    } else {
        model.handle(key)
    }
    if model.quitRequested { app.quit() }
})
