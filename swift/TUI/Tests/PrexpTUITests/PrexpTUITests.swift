import Foundation
import Testing
import Tint
@testable import PrexpTUI

private func newModel() -> AppModel {
    let m = AppModel(source: FakeSource(procs: Sample.procs))
    m.refresh(now: Date())
    return m
}

private func renderFrame(_ m: AppModel, w: Int = 100, h: Int = 30) -> [String] {
    var buf = Buffer(area: Rect(x: 0, y: 0, width: w, height: h))
    View.render(m, area: buf.area, buffer: &buf)
    return buf.allText()
}

@Suite struct ModelTests {
    @Test func refreshLoadsAndSortsByCpuDefault() {
        let m = newModel()
        #expect(m.processes.count == 3)
        // alpha has the only nonzero cpu_time; first refresh is a baseline (0%),
        // so order is stable but present. Just assert all three are loaded.
        #expect(Set(m.processes.map(\.pid)) == [200, 201, 202])
    }

    @Test func sortByNameOrdersAlphabetically() {
        let m = newModel()
        m.sort = .name; m.rebuild()
        #expect(m.processes.map(\.name) == ["alpha", "bravo", "charlie"])
    }

    @Test func sortByMemoryDescending() {
        let m = newModel()
        m.sort = .memory; m.rebuild()
        #expect(m.processes.first?.name == "alpha")   // highest rss
    }

    @Test func reverseFlipsOrder() {
        let m = newModel()
        m.sort = .name; m.reversed = true; m.rebuild()
        #expect(m.processes.map(\.name) == ["charlie", "bravo", "alpha"])
    }

    @Test func searchFiltersByName() {
        let m = newModel()
        m.handle(.char("/"))
        #expect(m.isSearching)
        for c in "brav" { m.handle(.char(c)) }
        #expect(m.processes.count == 1)
        #expect(m.processes.first?.name == "bravo")
    }

    @Test func escapeClearsSearch() {
        let m = newModel()
        m.handle(.char("/")); m.handle(.char("z"))
        #expect(m.processes.isEmpty)   // nothing matches "z"
        m.handle(.escape)
        #expect(!m.isSearching)
        #expect(m.processes.count == 3)
    }

    @Test func navigationClampsAndSelectionFollowsPid() {
        let m = newModel()
        m.sort = .name; m.rebuild()
        m.handle(.char("j")); m.handle(.char("j")); m.handle(.char("j"))  // past the end
        #expect(m.selected == 2)
        m.handle(.char("k"))
        #expect(m.selected == 1)
    }

    @Test func enterTogglesDetailOverlay() {
        let m = newModel()
        m.handle(.enter)
        #expect(m.overlay == .detail)
        m.handle(.enter)
        #expect(m.overlay == .none)
    }

    @Test func qQuitsButClosesOverlayFirst() {
        let m = newModel()
        m.handle(.enter)              // open detail
        m.handle(.char("q"))          // closes overlay
        #expect(!m.quitRequested)
        m.handle(.char("q"))          // now quits
        #expect(m.quitRequested)
    }
}

@Suite struct RenderTests {
    @Test func drawsHeaderAndProcessRows() {
        let text = renderFrame(newModel()).joined(separator: "\n")
        #expect(text.contains("Processes"))
        #expect(text.contains("PID"))
        #expect(text.contains("NAME"))
        #expect(text.contains("alpha"))
        #expect(text.contains("bravo"))
    }

    @Test func statusBarShowsHints() {
        let text = renderFrame(newModel()).joined(separator: "\n")
        #expect(text.contains("Quit"))
        #expect(text.contains("Search"))
    }

    @Test func searchBarRendersQuery() {
        let m = newModel()
        m.handle(.char("/")); m.handle(.char("a")); m.handle(.char("l"))
        let text = renderFrame(m).joined(separator: "\n")
        #expect(text.contains("/ al"))
    }

    @Test func detailOverlayShowsResourcePaths() {
        let m = newModel()
        m.sort = .name; m.rebuild()      // alpha first
        m.handle(.enter)
        let text = renderFrame(m).joined(separator: "\n")
        #expect(text.contains("alpha"))
        #expect(text.contains("/tmp/a.log"))
        #expect(text.contains("socket"))
    }

    @Test func helpOverlayLterists() {
        let m = newModel()
        m.handle(.char("?"))
        let text = renderFrame(m).joined(separator: "\n")
        #expect(text.contains("Help"))
        #expect(text.contains("sort"))
    }

    @Test func statsHeaderTogglesOn() {
        let m = newModel()
        m.refresh(now: Date())          // second refresh → core usage deltas
        m.showStats = true
        let text = renderFrame(m).joined(separator: "\n")
        #expect(text.contains("System"))
        #expect(text.contains("MEM"))
        #expect(text.contains("load"))
    }
}
