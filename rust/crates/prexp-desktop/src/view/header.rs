//! The system-stats header — a card above the process table with headline stat
//! tiles (CPU / MEM / LOAD / BATTERY), a per-core CPU strip, and a CPU-history
//! line chart. Toggled with the "Stats" control; reads everything from
//! [`App::system`](crate::app::App::system).

use iced::widget::{column, container, Column, Row};
use iced::{Color, Length};
use prexp_core::system::CpuKind;
use rime::theme::{tokens, Palette};
use rime::widgets::{caption, card, line_chart, pill, stat, LineChart, Series};

use super::fmt;
use crate::app::{App, Message};

/// Pills-per-row in the per-core strip, so a many-core machine wraps instead of
/// overflowing the window width.
const CORES_PER_ROW: usize = 8;
/// The CPU-history chart's height; it spans the full card width below the tiles.
const CHART_HEIGHT: f32 = 84.0;

pub fn header(app: &App) -> iced::Element<'_, Message> {
    let s = app.system();
    let p = tokens();

    // Headline tiles. Battery only appears on machines that have one.
    let mut tiles: Vec<iced::Element<'_, Message>> = vec![
        stat("CPU", format!("{:.0}%", s.aggregate_cpu())),
        stat("MEM", mem_label(app)),
        stat("LOAD", load_label(s.load())),
    ];
    if let Some(b) = s.battery() {
        let bolt = if b.charging { " ⚡" } else { "" };
        tiles.push(stat("BATTERY", format!("{:.0}%{bolt}", b.percent)));
    }
    let tiles_row = Row::with_children(tiles).spacing(40);

    // Tiles, then the per-core strip, then a full-width CPU-history chart.
    card(
        column![tiles_row, core_strip(app, &p), cpu_chart(app, p.accent)]
            .spacing(12)
            .width(Length::Fill),
    )
}

/// A full-width CPU-history line chart on a `0..100%` scale. Shows a placeholder
/// until at least two samples give the curve something to draw.
fn cpu_chart<'a>(app: &App, accent: Color) -> iced::Element<'a, Message> {
    let history = app.system().cpu_history();
    if history.len() < 2 {
        return container(caption("CPU history — sampling…"))
            .height(Length::Fixed(CHART_HEIGHT))
            .center_y(Length::Fixed(CHART_HEIGHT))
            .into();
    }
    let points = history
        .iter()
        .enumerate()
        .map(|(i, &v)| (i as f64, v as f64))
        .collect();
    line_chart(
        LineChart {
            title: "CPU %".to_string(),
            series: vec![Series {
                points,
                color: accent,
            }],
            y_max: Some(100.0),
            y_max_label: Some("100%".to_string()),
            hover_format: Some(format_pct),
        },
        CHART_HEIGHT,
    )
}

fn format_pct(y: f64) -> String {
    format!("{y:.0}%")
}

/// A wrapped grid of per-core usage chips, each colored by load. Empty until the
/// second sample gives us a tick delta to compute usage from.
fn core_strip<'a>(app: &'a App, p: &Palette) -> iced::Element<'a, Message> {
    let s = app.system();
    let cores = s.per_core();
    if cores.is_empty() {
        return caption("sampling CPU…");
    }

    let levels = s.perf_levels();
    let mut grid = Column::new().spacing(6);
    for (chunk_idx, chunk) in cores.chunks(CORES_PER_ROW).enumerate() {
        let mut r = Row::new().spacing(6);
        for (local, &pct) in chunk.iter().enumerate() {
            let i = chunk_idx * CORES_PER_ROW + local;
            let label = format!("{}{i} {pct:.0}%", cluster_prefix(levels.get(i)));
            r = r.push(pill(&label, load_color(p, pct)));
        }
        grid = grid.push(r);
    }
    grid.width(Length::Shrink).into()
}

fn mem_label(app: &App) -> String {
    match app.system().memory() {
        Some(m) => format!("{} / {}", fmt::bytes(m.used), fmt::bytes(m.total)),
        None => "—".to_string(),
    }
}

fn load_label(load: Option<[f64; 3]>) -> String {
    match load {
        Some([one, five, fifteen]) => format!("{one:.2}  {five:.2}  {fifteen:.2}"),
        None => "—".to_string(),
    }
}

/// The single-letter core prefix: `P`/`E` on heterogeneous Apple Silicon, `C`
/// when the cluster type is unknown (Intel) or the topology wasn't reported.
fn cluster_prefix(kind: Option<&CpuKind>) -> &'static str {
    match kind {
        Some(CpuKind::Performance) => "P",
        Some(CpuKind::Efficiency) => "E",
        _ => "C",
    }
}

/// Grade a core's usage: healthy → caution → hot.
fn load_color(p: &Palette, pct: f64) -> Color {
    if pct >= 80.0 {
        p.danger
    } else if pct >= 50.0 {
        p.warn
    } else {
        p.success
    }
}
