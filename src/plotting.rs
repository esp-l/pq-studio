//! Plotting for simulation results. One rendering path draws the on-screen
//! chart and the exported PNG/SVG: the screen shows the export figure
//! rendered at a smaller scale, so both have the same layout, text and
//! legend. White background, 3:2 figure, print-safe colours regardless of
//! the app's dark UI.
//!
//! Every plotted line (a Y column, per simulation when there are several)
//! is a series with its own style. A new series gets the lowest free
//! automatic style: twelve colours, then the same colours again with another
//! line type and marker shape. The user can override any part of it.
//!
//! The editor for all of this is in plot_ui.rs; the settings are saved next
//! to the project file (see `settings_path`).

use plotters::prelude::*;
use plotters::style::text_anchor::{HPos, Pos, VPos};
use plotters::style::{FontStyle, register_font};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Once;

const FONT_REGULAR: &[u8] = include_bytes!("../assets/fonts/DejaVuSans.ttf");
const FONT_BOLD: &[u8] = include_bytes!("../assets/fonts/DejaVuSans-Bold.ttf");
static FONT_INIT: Once = Once::new();

/// plotters' `ab_glyph` text backend (chosen over `ttf`/font-kit, which fails
/// to build here against this system's fontconfig bindings) does no system
/// font discovery at all — every font family name used in chart code must be
/// explicitly registered with real font bytes first, or text drawing panics
/// with `FontUnavailable`. Bundles DejaVu Sans (SIL/Bitstream-Vera-style
/// permissive license, see assets/fonts/LICENSE) so this works identically
/// on Linux and macOS without depending on whatever's installed system-wide.
fn ensure_fonts_registered() {
    FONT_INIT.call_once(|| {
        let _ = register_font("sans-serif", FontStyle::Normal, FONT_REGULAR);
        let _ = register_font("sans-serif", FontStyle::Bold, FONT_BOLD);
    });
}

/// Automatic series colours, in order. The first eight are the dataviz
/// skill's validated categorical order (blue, orange, aqua, yellow, magenta,
/// green, violet, red); sixteen more hues extend it to twenty-four.
pub const CATEGORICAL_PALETTE: [(u8, u8, u8); 24] = [
    (0x2a, 0x78, 0xd6), // blue
    (0xeb, 0x68, 0x34), // orange
    (0x1b, 0xaf, 0x7a), // aqua
    (0xed, 0xa1, 0x00), // yellow
    (0xe8, 0x7b, 0xa4), // magenta
    (0x00, 0x83, 0x00), // green
    (0x62, 0x50, 0xd6), // violet
    (0xe3, 0x49, 0x48), // red
    (0x8c, 0x56, 0x4b), // brown
    (0x3c, 0x3c, 0x3c), // charcoal
    (0xa0, 0x2b, 0x93), // plum
    (0x8a, 0x8a, 0x14), // olive
    (0x1f, 0x3a, 0x93), // navy
    (0x00, 0x80, 0x80), // teal
    (0xb8, 0x86, 0x0b), // dark gold
    (0xa5, 0x00, 0x26), // crimson
    (0x66, 0xa6, 0x1e), // lime
    (0x56, 0xb4, 0xe9), // sky
    (0xf4, 0x8f, 0x6a), // salmon
    (0x70, 0x80, 0x90), // slate
    (0xc0, 0x7c, 0xd8), // orchid
    (0xc4, 0x4e, 0x00), // burnt orange
    (0x2e, 0x8b, 0x57), // sea green
    (0x4b, 0x00, 0x82), // indigo
];

/// Minerals and gases that always get the same palette colour by default,
/// in all their columns (si_Calcite, d_Calcite, Calcite), so they look the
/// same in every report figure. Their colours are kept free for them: no
/// other column gets one automatically.
pub const FIXED_COLOURS: [(&str, usize); 13] = [
    ("Calcite", 0),       // blue
    ("Aragonite", 1),     // orange
    ("Dolomite", 2),      // aqua
    ("Anhydrite", 3),     // yellow
    ("Gypsum", 4),        // magenta
    ("Magnesite", 5),     // green
    ("Witherite", 6),     // violet
    ("CO2(g)", 7),        // red
    ("Barite", 8),        // brown
    ("Halite", 9),        // charcoal
    ("Strontianite", 10), // plum
    ("Celestite", 12),    // navy
    ("Siderite", 14),     // dark gold
];

fn fixed_colour(family: &str) -> Option<usize> {
    FIXED_COLOURS.iter().find(|(name, _)| *name == family).map(|&(_, i)| i)
}

/// The mineral or gas a column is about, so all its columns share a colour:
/// `si_X`, `d_X` and `g_X` are about X, and so is a bare `X` (the phase
/// amount) when X is a fixed mineral or `si_X`/`d_X` is also known. Any
/// other column is its own family.
pub fn family(key: &str, known: impl Fn(&str) -> bool) -> String {
    for prefix in ["si_", "d_", "g_"] {
        if let Some(rest) = key.strip_prefix(prefix) {
            return rest.to_string();
        }
    }
    let mineral = fixed_colour(key).is_some() || known(&format!("si_{key}")) || known(&format!("d_{key}"));
    if mineral { key.to_string() } else { format!("column {key}") }
}

/// Default figure size: 3:2, a common report figure shape.
pub const REPORT_WIDTH: u32 = 1200;
pub const REPORT_HEIGHT: u32 = 800;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineType {
    #[default]
    Solid,
    Dashed,
    Dotted,
    DashDot,
    None,
}

impl LineType {
    pub const ALL: [LineType; 5] = [LineType::Solid, LineType::Dashed, LineType::Dotted, LineType::DashDot, LineType::None];

    pub fn label(self) -> &'static str {
        match self {
            LineType::Solid => "Solid",
            LineType::Dashed => "Dashed",
            LineType::Dotted => "Dotted",
            LineType::DashDot => "Dash-dot",
            LineType::None => "None",
        }
    }

    /// On/off lengths in multiples of the line width; empty is a solid line.
    fn pattern(self) -> &'static [f64] {
        match self {
            LineType::Solid | LineType::None => &[],
            LineType::Dashed => &[4.0, 2.5],
            LineType::Dotted => &[1.0, 1.8],
            LineType::DashDot => &[5.0, 2.0, 1.0, 2.0],
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarkerShape {
    #[default]
    Circle,
    Square,
    Triangle,
    Diamond,
    None,
}

impl MarkerShape {
    pub const ALL: [MarkerShape; 5] =
        [MarkerShape::Circle, MarkerShape::Square, MarkerShape::Triangle, MarkerShape::Diamond, MarkerShape::None];

    pub fn label(self) -> &'static str {
        match self {
            MarkerShape::Circle => "Circle",
            MarkerShape::Square => "Square",
            MarkerShape::Triangle => "Triangle",
            MarkerShape::Diamond => "Diamond",
            MarkerShape::None => "None",
        }
    }
}

/// How one series is drawn. Sizes are in figure pixels (see `PlotConfig`
/// export size), so they scale with the rest of the chart.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SeriesStyle {
    pub color: [u8; 3],
    pub line: LineType,
    pub line_width: f32,
    pub marker: MarkerShape,
    pub marker_size: f32,
}

impl SeriesStyle {
    /// The `index`th automatic style: a solid line with circles in the
    /// `index`th palette colour.
    pub fn automatic(index: usize) -> Self {
        let (r, g, b) = CATEGORICAL_PALETTE[index % CATEGORICAL_PALETTE.len()];
        SeriesStyle { color: [r, g, b], line: LineType::Solid, line_width: 2.5, marker: MarkerShape::Circle, marker_size: 5.0 }
    }

    fn rgb(&self) -> RGBColor {
        RGBColor(self.color[0], self.color[1], self.color[2])
    }
}

/// A series' remembered style: its automatic style number, the user's own
/// style if they changed it, and a legend label replacing the automatic one.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SeriesEntry {
    pub auto: usize,
    pub custom: Option<SeriesStyle>,
    pub label: String,
}

impl SeriesEntry {
    pub fn style(&self) -> SeriesStyle {
        self.custom.unwrap_or_else(|| SeriesStyle::automatic(self.auto))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LegendPosition {
    TopLeft,
    TopCenter,
    #[default]
    TopRight,
    LeftCenter,
    RightCenter,
    BottomLeft,
    BottomCenter,
    BottomRight,
    OutsideRight,
    Hidden,
}

impl LegendPosition {
    pub const ALL: [LegendPosition; 10] = [
        LegendPosition::TopLeft,
        LegendPosition::TopCenter,
        LegendPosition::TopRight,
        LegendPosition::LeftCenter,
        LegendPosition::RightCenter,
        LegendPosition::BottomLeft,
        LegendPosition::BottomCenter,
        LegendPosition::BottomRight,
        LegendPosition::OutsideRight,
        LegendPosition::Hidden,
    ];

    pub fn label(self) -> &'static str {
        match self {
            LegendPosition::TopLeft => "Top left",
            LegendPosition::TopCenter => "Top center",
            LegendPosition::TopRight => "Top right",
            LegendPosition::LeftCenter => "Left center",
            LegendPosition::RightCenter => "Right center",
            LegendPosition::BottomLeft => "Bottom left",
            LegendPosition::BottomCenter => "Bottom center",
            LegendPosition::BottomRight => "Bottom right",
            LegendPosition::OutsideRight => "Outside right",
            LegendPosition::Hidden => "Hidden",
        }
    }

    /// Reads a label or a short name such as "top-left" or "outside".
    pub fn parse(s: &str) -> Option<Self> {
        let key: String = s.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase();
        Self::ALL.into_iter().find(|p| {
            let name: String = p.label().chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase();
            name == key || (key == "outside" && *p == LegendPosition::OutsideRight)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LegendSettings {
    pub position: LegendPosition,
    pub font_size: f32,
    pub frame: bool,
    /// Background opacity, 0 (none) to 1 (solid white).
    pub opacity: f32,
}

impl Default for LegendSettings {
    fn default() -> Self {
        Self { position: LegendPosition::TopRight, font_size: 18.0, frame: true, opacity: 0.85 }
    }
}

/// Chart options, field set drawn from the real GUI's USER_GRAPH editor
/// dialog (PhreeqcI dialog resource 617: "Chart title", per-axis
/// "Title"/"Min"/"Max"/"logarithmic"), plus series styles, legend and export
/// size. Applied to this app's own SELECTED_OUTPUT-driven plotting rather
/// than the USER_GRAPH keyword itself — that keyword embeds custom BASIC
/// PLOT_XY scripting into the PHREEQC input, which is out of scope here for
/// the same reason RATES/custom BASIC is out of scope for Kinetics.
///
/// Saved next to the project file, except the fields that come from the run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlotConfig {
    pub chart_title: String,
    pub x_axis_title: String,
    pub y_axis_title: String,
    pub x_min: String,
    pub x_max: String,
    pub y_min: String,
    pub y_max: String,
    pub x_log: bool,
    pub y_log: bool,
    pub title_font_size: f32,
    pub axis_font_size: f32,
    pub tick_font_size: f32,
    pub grid: bool,
    /// A dashed line at SI = 0 when saturation indices are plotted.
    pub zero_line: bool,
    pub legend: LegendSettings,
    /// Styles by series key (see `Series::key`).
    pub series: BTreeMap<String, SeriesEntry>,
    /// Simulation numbers left out of the chart.
    pub hidden_sims: BTreeSet<i64>,
    /// The X and Y columns by name, filled in when the settings are saved.
    pub x_column: Option<String>,
    pub y_columns: Vec<String>,
    /// Figure size in pixels at 1×; the export resolution multiplies it.
    pub export_width: u32,
    pub export_height: u32,
    pub export_scale: u32,
    /// The input file's TITLE, used as the chart title when none is typed.
    #[serde(skip)]
    pub document_title: String,
    /// A name per simulation number, from the input as it was run.
    #[serde(skip)]
    pub sim_names: BTreeMap<i64, String>,
}

impl Default for PlotConfig {
    fn default() -> Self {
        Self {
            chart_title: String::new(),
            x_axis_title: String::new(),
            y_axis_title: String::new(),
            x_min: String::new(),
            x_max: String::new(),
            y_min: String::new(),
            y_max: String::new(),
            x_log: false,
            y_log: false,
            title_font_size: 28.0,
            axis_font_size: 22.0,
            tick_font_size: 20.0,
            grid: true,
            zero_line: true,
            legend: LegendSettings::default(),
            series: BTreeMap::new(),
            hidden_sims: BTreeSet::new(),
            x_column: None,
            y_columns: Vec::new(),
            export_width: REPORT_WIDTH,
            export_height: REPORT_HEIGHT,
            export_scale: 2,
            document_title: String::new(),
            sim_names: BTreeMap::new(),
        }
    }
}

impl PlotConfig {
    /// Figure size in pixels at 1×, kept within sane bounds.
    pub fn figure_size(&self) -> (u32, u32) {
        (self.export_width.clamp(300, 4000), self.export_height.clamp(200, 4000))
    }

    pub fn export_scale(&self) -> u32 {
        self.export_scale.clamp(1, 4)
    }
}

/// The plot settings file saved beside a project: `run.pqi` →
/// `run.pqstudio.json`. The input file itself stays plain PHREEQC.
pub fn settings_path(project: &Path) -> PathBuf {
    project.with_extension("pqstudio.json")
}

#[derive(Serialize, Deserialize)]
struct SettingsFile {
    app: String,
    version: u32,
    plot: PlotConfig,
}

pub fn read_settings(path: &Path) -> Result<PlotConfig, Box<dyn std::error::Error>> {
    let file: SettingsFile = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    Ok(file.plot)
}

pub fn write_settings(path: &Path, config: &PlotConfig) -> std::io::Result<()> {
    let file = SettingsFile { app: crate::phreeqc::APP_NAME.to_string(), version: 1, plot: config.clone() };
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(&file)?)?;
    std::fs::rename(tmp, path)
}


#[derive(Debug, Clone, Default)]
pub struct DataTable {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<f64>>,
    /// Per row: an initial-solution line (state `i_soln`), which the chart
    /// leaves out when the file also has reaction or transport lines.
    pub initial: Vec<bool>,
    /// The file as PHREEQC wrote it, for exporting the table unchanged.
    pub source: String,
}

impl DataTable {
    fn is_initial(&self, row: usize) -> bool {
        self.initial.get(row).copied().unwrap_or(false)
    }
}

/// Parse a PHREEQC SELECTED_OUTPUT file: tab-separated, header row first,
/// numeric cells; non-numeric cells (e.g. a trailing blank column PHREEQC
/// sometimes emits) become NaN and are skipped by the plotter.
pub fn load_selected_output(path: &Path) -> std::io::Result<DataTable> {
    let text = std::fs::read_to_string(path)?;
    let source = text.clone();
    let mut lines = text.lines();
    let header_line = lines.next().unwrap_or("");
    let headers: Vec<String> = header_line
        .split('\t')
        .map(|h| h.trim().to_string())
        .collect();
    let state_col = headers.iter().position(|h| h == "state");
    let mut rows = Vec::new();
    let mut initial = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        initial.push(state_col.and_then(|c| line.split('\t').nth(c)).is_some_and(|s| s.trim() == "i_soln"));
        let row: Vec<f64> = line
            .split('\t')
            .map(|cell| cell.trim().parse::<f64>().unwrap_or(f64::NAN))
            .collect();
        rows.push(row);
    }

    // PHREEQC's selected-output format ends each line with a trailing tab,
    // producing a trailing empty header and column; drop it from both sides
    // consistently so header/row indices stay aligned.
    let mut headers = headers;
    if headers.last().map(|h| h.is_empty()).unwrap_or(false) {
        headers.pop();
        for row in &mut rows {
            row.pop();
        }
    }

    for (col, header) in headers.iter().enumerate() {
        // -99 marks "not applicable" in the standard columns; -999.999 is
        // PHREEQC's "not computed" for saturation indices and log activities.
        let sentinel = if SENTINEL_COLUMNS.contains(&header.as_str()) {
            Some(-99.0)
        } else if header.starts_with("si_") || header.starts_with("la_") {
            Some(-999.999)
        } else {
            None
        };
        if let Some(s) = sentinel {
            for row in &mut rows {
                if let Some(v) = row.get_mut(col) {
                    if (*v - s).abs() < 1e-9 {
                        *v = f64::NAN;
                    }
                }
            }
        }
    }

    Ok(DataTable { headers, rows, initial, source })
}

/// The simulation numbers with rows the chart can show (initial-solution
/// rows aside), in order.
pub fn plotted_simulations(table: &DataTable) -> Vec<i64> {
    let Some(sim_col) = table.headers.iter().position(|h| h == "sim") else {
        return Vec::new();
    };
    let any_reacting = (0..table.rows.len()).any(|r| !table.is_initial(r));
    let mut sims: Vec<i64> = Vec::new();
    for (r, row) in table.rows.iter().enumerate() {
        if any_reacting && table.is_initial(r) {
            continue;
        }
        if let Some(v) = row.get(sim_col).filter(|v| v.is_finite()) {
            let n = *v as i64;
            if !sims.contains(&n) {
                sims.push(n);
            }
        }
    }
    sims
}

/// The table as text for a spreadsheet, with a `name` column first giving
/// each row's simulation name. Values are exactly as PHREEQC wrote them.
pub fn table_text(table: &DataTable, names: &BTreeMap<i64, String>, separator: char) -> String {
    let quote = |cell: &str| {
        if cell.contains(separator) || cell.contains('"') {
            format!("\"{}\"", cell.replace('"', "\"\""))
        } else {
            cell.to_string()
        }
    };
    let mut lines = table.source.lines().filter(|l| !l.trim().is_empty());
    let Some(header) = lines.next() else {
        return String::new();
    };
    let cells = |line: &str| -> Vec<String> {
        let mut cells: Vec<String> = line.split('\t').map(|c| c.trim().to_string()).collect();
        if cells.last().is_some_and(|c| c.is_empty()) {
            cells.pop();
        }
        cells
    };
    let headers = cells(header);
    let sim_col = headers.iter().position(|h| h == "sim");
    let mut out = String::new();
    let mut push_row = |first: &str, rest: &[String]| {
        let row: Vec<String> = std::iter::once(quote(first)).chain(rest.iter().map(|c| quote(c))).collect();
        out.push_str(&row.join(&separator.to_string()));
        out.push('\n');
    };
    push_row("name", &headers);
    for line in lines {
        let row = cells(line);
        let name = sim_col
            .and_then(|c| row.get(c))
            .and_then(|v| v.parse::<f64>().ok())
            .and_then(|v| names.get(&(v as i64)))
            .map(String::as_str)
            .unwrap_or("");
        push_row(name, &row);
    }
    out
}

pub struct PlotState {
    pub table: Option<DataTable>,
    pub x_col: usize,
    pub y_cols: Vec<usize>,
    pub config: PlotConfig,
    /// Column headers the current axis choice was made for; a rerun with the
    /// same columns keeps the user's choice.
    axes_for: Vec<String>,
    /// The chart rendered for the screen, and the scale it was rendered at.
    pub(crate) texture: Option<(egui::TextureHandle, f64)>,
    pub(crate) dirty: bool,
    /// The plot window, where the chart and all its settings are.
    pub window_open: bool,
    /// The user closed the plot window, so a run doesn't reopen it.
    pub window_dismissed: bool,
    /// The user changed a plot setting since the project was last saved.
    pub touched: bool,
    /// The project has a plot settings file, so saving keeps it up to date.
    pub has_settings_file: bool,
    /// Why the last run gave nothing to plot, shown in place of the chart.
    pub note: Option<String>,
    /// The series styles changed and should be written to the style library.
    pub styles_changed: bool,
    /// Saved plot presets (app-wide), and the preset form's name and choice.
    pub presets: Vec<PlotPreset>,
    pub preset_name: String,
    pub preset_choice: usize,
}

impl Default for PlotState {
    fn default() -> Self {
        Self {
            table: None,
            x_col: 0,
            y_cols: Vec::new(),
            config: PlotConfig::default(),
            axes_for: Vec::new(),
            texture: None,
            dirty: true,
            window_open: false,
            window_dismissed: false,
            touched: false,
            has_settings_file: false,
            note: None,
            styles_changed: false,
            presets: Vec::new(),
            preset_name: String::new(),
            preset_choice: 0,
        }
    }
}

impl PlotState {
    /// Drops the data but keeps the axis choice for the next run.
    pub fn clear_data(&mut self) {
        self.table = None;
        self.texture = None;
        self.dirty = true;
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// A setting changed: redraw, and the project has unsaved plot changes.
    pub fn changed(&mut self) {
        self.dirty = true;
        self.touched = true;
    }

    /// A series style changed: as `changed`, and the style library is saved.
    pub fn style_changed(&mut self) {
        self.changed();
        self.styles_changed = true;
    }

    pub fn set_table(&mut self, table: DataTable) {
        if table.headers != self.axes_for {
            let index = |name: &str| table.headers.iter().position(|h| h == name);
            // Saved columns win when the table still has them.
            let saved_x = self.config.x_column.as_deref().and_then(index);
            let saved_ys: Vec<usize> = self.config.y_columns.iter().filter_map(|n| index(n)).collect();
            match saved_x {
                Some(x) if !saved_ys.is_empty() => {
                    self.x_col = x;
                    self.y_cols = saved_ys;
                }
                _ => {
                    let (x, ys) = default_axes(&table);
                    self.x_col = x;
                    self.y_cols = ys;
                }
            }
            self.axes_for = table.headers.clone();
        }
        self.table = Some(table);
        self.texture = None;
        self.dirty = true;
    }

    /// The settings to save, with the axis columns by name.
    pub fn saved_config(&self) -> PlotConfig {
        let mut config = self.config.clone();
        if let Some(table) = &self.table {
            config.x_column = table.headers.get(self.x_col).cloned();
            config.y_columns = self.y_cols.iter().filter_map(|&c| table.headers.get(c).cloned()).collect();
        }
        config
    }

    /// Settings for another project (or none): forget this one's, except the
    /// series styles, which are shared by every plot. Styles the file has
    /// that the library lacks are added to it.
    pub fn load_config(&mut self, mut config: PlotConfig, from_file: bool) {
        let mut library = std::mem::take(&mut self.config.series);
        let before = library.len();
        merge_styles(&mut library, std::mem::take(&mut config.series));
        self.styles_changed |= library.len() != before;
        config.series = library;
        self.config = config;
        self.normalize_styles();
        self.axes_for.clear();
        self.y_cols.clear();
        self.x_col = 0;
        self.touched = false;
        self.has_settings_file = from_file;
        self.dirty = true;
    }

    /// Gives each series on the chart that has no style yet the lowest
    /// automatic style no other remembered series has. It keeps it from then
    /// on, in every plot.
    pub fn assign_styles(&mut self) {
        let Some(table) = &self.table else {
            return;
        };
        let keys: Vec<String> =
            build_series(table, self.x_col, &self.y_cols, &self.config).into_iter().map(|s| s.key).collect();
        for key in keys {
            if !self.config.series.contains_key(&key) {
                self.config.series.insert(key, SeriesEntry { auto: usize::MAX, ..Default::default() });
            }
        }
        self.normalize_styles();
    }

    /// One automatic colour per mineral (all its columns) or other column:
    /// the fixed one for the common minerals, else the one it already has if
    /// nothing else has it, else the lowest free one.
    pub fn normalize_styles(&mut self) {
        let n = CATEGORICAL_PALETTE.len();
        let keys: BTreeSet<String> = self.config.series.keys().cloned().collect();
        let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for key in &keys {
            groups.entry(family(key, |k| keys.contains(k))).or_default().push(key.clone());
        }
        // The si_ column's colour is the one a mineral keeps.
        for members in groups.values_mut() {
            members.sort_by_key(|k| !k.starts_with("si_"));
        }
        let mut used: BTreeSet<usize> = FIXED_COLOURS.iter().map(|&(_, i)| i).collect();
        let mut chosen: BTreeMap<String, usize> = BTreeMap::new();
        for family in groups.keys() {
            if let Some(i) = fixed_colour(family) {
                chosen.insert(family.clone(), i);
            }
        }
        for (family, members) in &groups {
            if chosen.contains_key(family) {
                continue;
            }
            let kept = members
                .iter()
                .map(|k| self.config.series[k].auto)
                .find(|&a| a != usize::MAX && !used.contains(&(a % n)));
            if let Some(a) = kept {
                used.insert(a % n);
                chosen.insert(family.clone(), a % n);
            }
        }
        for family in groups.keys() {
            if !chosen.contains_key(family) {
                // Past 24 families the colours have to repeat.
                let auto = (0..n).find(|i| !used.contains(i)).unwrap_or(chosen.len() % n);
                used.insert(auto);
                chosen.insert(family.clone(), auto);
            }
        }
        for (family, members) in &groups {
            for key in members {
                let entry = self.config.series.get_mut(key).expect("listed above");
                if entry.auto != chosen[family] {
                    entry.auto = chosen[family];
                    self.styles_changed = true;
                }
            }
        }
    }

    /// Applies a preset's columns, axes, legend and export settings; the
    /// chart title and the simulations shown stay as they are.
    pub fn apply_preset(&mut self, preset: &PlotConfig) {
        let c = &mut self.config;
        c.x_axis_title = preset.x_axis_title.clone();
        c.y_axis_title = preset.y_axis_title.clone();
        c.x_min = preset.x_min.clone();
        c.x_max = preset.x_max.clone();
        c.y_min = preset.y_min.clone();
        c.y_max = preset.y_max.clone();
        c.x_log = preset.x_log;
        c.y_log = preset.y_log;
        c.title_font_size = preset.title_font_size;
        c.axis_font_size = preset.axis_font_size;
        c.tick_font_size = preset.tick_font_size;
        c.grid = preset.grid;
        c.zero_line = preset.zero_line;
        c.legend = preset.legend.clone();
        c.export_width = preset.export_width;
        c.export_height = preset.export_height;
        c.export_scale = preset.export_scale;
        if let Some(table) = &self.table {
            let index = |name: &str| table.headers.iter().position(|h| h == name);
            if let Some(x) = preset.x_column.as_deref().and_then(index) {
                self.x_col = x;
            }
            let ys: Vec<usize> = preset.y_columns.iter().filter_map(|n| index(n)).collect();
            if !ys.is_empty() {
                self.y_cols = ys;
            }
        }
        self.changed();
    }
}

/// Adds styles from a project's settings file to the library, keeping the
/// library's own. Styles that older versions kept per simulation
/// (`si_Barite @ sim 3`) become the column's style if it has none.
pub fn merge_styles(library: &mut BTreeMap<String, SeriesEntry>, from: BTreeMap<String, SeriesEntry>) {
    for (key, entry) in from {
        match key.split_once(" @ sim ") {
            Some((column, _)) => {
                if entry.custom.is_some() || !entry.label.is_empty() {
                    library.entry(column.to_string()).or_insert(entry);
                }
            }
            None => {
                library.entry(key).or_insert(entry);
            }
        }
    }
}

/// The series styles shared by every project, in the app's config folder.
pub fn style_library_path() -> Option<PathBuf> {
    crate::settings::config_dir().map(|d| d.join("plot_styles.json"))
}

pub fn read_style_library() -> BTreeMap<String, SeriesEntry> {
    style_library_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn write_style_library(styles: &BTreeMap<String, SeriesEntry>) -> std::io::Result<()> {
    let Some(path) = style_library_path() else {
        return Ok(());
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(styles)?)?;
    std::fs::rename(tmp, path)
}

/// Columns PHREEQC writes itself (as opposed to ones the user asked for).
pub(crate) const STANDARD_COLUMNS: [&str; 16] = [
    "sim", "state", "soln", "dist_x", "time", "step", "pH", "pe", "reaction", "temp", "temp(C)",
    "Alk(eq/kgw)", "mu", "mass_H2O", "charge(eq)", "pct_err",
];

/// Standard columns where PHREEQC writes -99 for "not applicable".
const SENTINEL_COLUMNS: [&str; 4] = ["dist_x", "time", "step", "reaction"];

fn varies(table: &DataTable, col: usize) -> bool {
    let vals: Vec<f64> = table
        .rows
        .iter()
        .filter_map(|r| r.get(col).copied())
        .filter(|v| v.is_finite())
        .collect();
    vals.len() > 1 && vals.iter().any(|v| *v != vals[0])
}

/// A useful first view: X = the first of reaction/time/distance/temperature/
/// step that changes; Y = the user's own columns that change (up to four), else pH.
pub fn default_axes(table: &DataTable) -> (usize, Vec<usize>) {
    let index = |name: &str| table.headers.iter().position(|h| h == name);
    let x = ["reaction", "time", "dist_x", "temp", "temp(C)", "step"]
        .iter()
        .filter_map(|n| index(n))
        .find(|&c| varies(table, c))
        .or_else(|| index("step"))
        .unwrap_or(0);
    // Only columns of the same kind as the first, so the default view never
    // puts e.g. saturation indices and molalities on one scale.
    let candidates: Vec<usize> = (0..table.headers.len())
        .filter(|&c| c != x && !STANDARD_COLUMNS.contains(&table.headers[c].as_str()))
        .filter(|&c| varies(table, c))
        .collect();
    let first_kind = candidates.first().map(|&c| describe_column(table, c));
    let mut ys: Vec<usize> = candidates
        .into_iter()
        .filter(|&c| {
            let info = describe_column(table, c);
            first_kind
                .as_ref()
                .is_some_and(|f| info.kind == f.kind && info.unit == f.unit)
        })
        .take(4)
        .collect();
    if ys.is_empty() {
        ys.extend(index("pH").filter(|&c| varies(table, c)));
    }
    (x, ys)
}

/// What a SELECTED_OUTPUT column measures, read from PHREEQC's header naming
/// conventions. `kind` groups columns that can share one axis label.
#[derive(Debug, Clone, PartialEq)]
struct ColumnInfo {
    name: String,
    unit: Option<&'static str>,
    kind: &'static str,
}

fn describe_column(table: &DataTable, col: usize) -> ColumnInfo {
    let h = table.headers.get(col).map(String::as_str).unwrap_or("");
    let info = |name: String, unit, kind| ColumnInfo { name, unit, kind };
    let standard = match h {
        "pH" => Some(("pH", None)),
        "pe" => Some(("pe", None)),
        "reaction" => Some(("Reaction", Some("mol"))),
        "step" => Some(("Step", None)),
        "time" => Some(("Time", Some("s"))),
        "dist_x" => Some(("Distance", Some("m"))),
        "temp" | "temp(C)" => Some(("Temperature", Some("°C"))),
        "Alk(eq/kgw)" => Some(("Alkalinity", Some("eq/kgw"))),
        "mu" => Some(("Ionic strength", Some("mol/kgw"))),
        "mass_H2O" => Some(("Mass of water", Some("kg"))),
        "charge(eq)" => Some(("Charge imbalance", Some("eq"))),
        "pct_err" => Some(("Charge balance error", Some("%"))),
        "soln" => Some(("Solution", None)),
        "sim" => Some(("Simulation", None)),
        _ => None,
    };
    if let Some((name, unit)) = standard {
        return info(name.to_string(), unit, "standard");
    }
    let has = |name: String| table.headers.iter().any(|x| *x == name);
    if let Some(rest) = h.strip_prefix("si_") {
        return info(format!("SI {rest}"), None, "Saturation index");
    }
    if let Some(rest) = h.strip_prefix("la_") {
        return info(format!("log a {rest}"), None, "log activity");
    }
    if let Some(rest) = h.strip_prefix("m_") {
        return info(rest.to_string(), Some("mol/kgw"), "Molality");
    }
    if let Some(rest) = h.strip_prefix("g_") {
        return info(rest.to_string(), Some("mol"), "Gas amount");
    }
    if let Some(rest) = h.strip_prefix("d_") {
        if has(rest.to_string()) {
            return info(format!("Δ {rest}"), Some("mol"), "Phase change");
        }
    }
    if let Some(rest) = h.strip_prefix("dk_") {
        if has(format!("k_{rest}")) {
            return info(format!("Δ {rest}"), Some("mol"), "Kinetic reaction");
        }
    }
    if let Some(rest) = h.strip_prefix("k_") {
        if has(format!("dk_{rest}")) {
            return info(rest.to_string(), Some("mol"), "Kinetic reactant");
        }
    }
    // Equilibrium-phase amounts come paired with a d_ column; otherwise a bare
    // name is an element total (-totals), unless it's a USER_PUNCH heading.
    if has(format!("d_{h}")) {
        return info(h.to_string(), Some("mol"), "Phase amount");
    }
    let looks_like_element = h
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_uppercase())
        && h.len() <= 8
        && h.chars().all(|c| c.is_ascii_alphanumeric() || "()+-".contains(c));
    if looks_like_element {
        return info(h.to_string(), Some("mol/kgw"), "Total");
    }
    info(h.to_string(), None, "other")
}

fn with_unit(name: &str, unit: Option<&str>) -> String {
    match unit {
        Some(u) => format!("{name} ({u})"),
        None => name.to_string(),
    }
}

/// Default x-axis title when the user hasn't typed one.
pub(crate) fn default_x_label(table: &DataTable, x_col: usize) -> String {
    let c = describe_column(table, x_col);
    with_unit(&c.name, c.unit)
}

/// Default y-axis title: the series itself when there's one, a shared
/// category when several of the same kind are plotted, else "Value".
pub(crate) fn default_y_label(table: &DataTable, y_cols: &[usize]) -> String {
    let infos: Vec<ColumnInfo> = y_cols.iter().map(|&c| describe_column(table, c)).collect();
    match infos.as_slice() {
        [] => "Value".to_string(),
        [one] => with_unit(&one.name, one.unit),
        [first, rest @ ..] => {
            let same_kind = rest.iter().all(|c| c.kind == first.kind);
            let same_unit = rest.iter().all(|c| c.unit == first.unit);
            if same_kind && !matches!(first.kind, "standard" | "other") {
                with_unit(first.kind, first.unit)
            } else if same_unit && first.unit.is_some() {
                with_unit("Value", first.unit)
            } else {
                "Value".to_string()
            }
        }
    }
}

/// Padding added to each end of an axis. A zero-width range (one data point,
/// or a constant series) gets padding proportional to the value instead of a
/// near-zero span, which would otherwise label every tick identically.
fn axis_pad(min: f64, max: f64, frac: f64) -> f64 {
    let span = (max - min).abs();
    if span > 1e-12 * max.abs().max(min.abs()).max(1.0) {
        span * frac
    } else {
        (max.abs() * 0.5).max(1.0)
    }
}

/// Parse a user-entered axis bound; blank or unparsable means "auto" (None).
fn parse_bound(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        None
    } else {
        s.parse::<f64>().ok()
    }
}

/// One line on the chart: a Y column for one simulation.
#[derive(Debug, Clone)]
pub struct Series {
    /// The style it is drawn with, which also names it in the style list:
    /// the column (`si_Barite`), so a column looks the same in every plot;
    /// or, when one column is plotted for several simulations, the
    /// simulation (`sim: Brine A`), each in its own colour.
    pub key: String,
    /// Automatic name for that style (the column or the simulation).
    pub label: String,
    /// The simulation's name.
    pub sim_name: String,
    /// With several columns over several simulations, the column gives the
    /// colour and line, and each simulation gets its own marker shape.
    pub marker_override: Option<MarkerShape>,
    /// Points in plot coordinates (log10 already applied on log axes).
    pub points: Vec<(f64, f64)>,
    /// Its Y column, and which run of rows (simulation) it comes from.
    pub column: usize,
    pub sim_index: usize,
}

/// The rows the chart shows, split into one run per simulation. Initial-
/// solution lines are left out once there is anything else, so each
/// simulation is its own line rather than one zig-zagging line.
fn segments(table: &DataTable, config: &PlotConfig) -> Vec<Vec<usize>> {
    let any_reacting = (0..table.rows.len()).any(|r| !table.is_initial(r));
    let sim_col = table.headers.iter().position(|h| h == "sim");
    let sim = |r: usize| sim_col.and_then(|c| table.rows[r].get(c).copied()).filter(|v| v.is_finite());
    let mut segments: Vec<Vec<usize>> = Vec::new();
    for r in 0..table.rows.len() {
        if (any_reacting && table.is_initial(r)) || sim(r).is_some_and(|v| config.hidden_sims.contains(&(v as i64))) {
            continue;
        }
        let new_sim = match segments.last().and_then(|s| s.last()) {
            Some(&prev) => sim(prev) != sim(r),
            None => true,
        };
        if new_sim {
            segments.push(Vec::new());
        }
        segments.last_mut().expect("pushed above").push(r);
    }
    segments
}

fn log_xform(log: bool, v: f64) -> f64 {
    if log { v.log10() } else { v }
}

/// The series the chart draws, in legend order: by Y column, then by
/// simulation. Series without a single drawable point are left out.
pub fn build_series(table: &DataTable, x_col: usize, y_cols: &[usize], config: &PlotConfig) -> Vec<Series> {
    let segments = segments(table, config);
    let column = |name: &str| table.headers.iter().position(|h| h == name);
    let value = |row: usize, col: Option<usize>| col.and_then(|c| table.rows[row].get(c).copied()).filter(|v| v.is_finite());
    let sim_col = column("sim");
    let soln_col = column("soln");
    let solutions: Vec<Option<f64>> = segments.iter().map(|s| value(s[0], soln_col)).collect();
    let distinct_solutions = solutions.iter().all(Option::is_some)
        && (1..solutions.len()).all(|i| !solutions[..i].contains(&solutions[i]));
    let segment_name = |s: usize| {
        let sim = value(segments[s][0], sim_col);
        if let Some(name) = sim.and_then(|n| config.sim_names.get(&(n as i64))) {
            return name.clone();
        }
        match (distinct_solutions, solutions[s], sim) {
            (true, Some(n), _) => format!("Solution {n}"),
            (_, _, Some(n)) => format!("Simulation {n}"),
            _ => format!("Series {}", s + 1),
        }
    };
    let mut out = Vec::new();
    for &yc in y_cols {
        let header = table.headers.get(yc).cloned().unwrap_or_else(|| format!("col{yc}"));
        for (s, segment) in segments.iter().enumerate() {
            let points: Vec<(f64, f64)> = segment
                .iter()
                .filter_map(|&r| {
                    let x = log_xform(config.x_log, *table.rows[r].get(x_col)?);
                    let y = log_xform(config.y_log, *table.rows[r].get(yc)?);
                    (x.is_finite() && y.is_finite()).then_some((x, y))
                })
                .collect();
            if points.is_empty() {
                continue;
            }
            out.push(Series {
                key: header.clone(),
                label: header.clone(),
                sim_name: segment_name(s),
                marker_override: None,
                points,
                column: yc,
                sim_index: s,
            });
        }
    }
    let columns: BTreeSet<usize> = out.iter().map(|s| s.column).collect();
    let sims: Vec<usize> = out.iter().map(|s| s.sim_index).collect::<BTreeSet<_>>().into_iter().collect();
    for series in &mut out {
        match (columns.len() > 1, sims.len() > 1) {
            (false, true) => {
                series.key = format!("sim: {}", series.sim_name);
                series.label = series.sim_name.clone();
            }
            (true, true) => {
                let rank = sims.iter().position(|&s| s == series.sim_index).unwrap_or(0);
                series.marker_override = Some(SIM_MARKERS[rank % SIM_MARKERS.len()]);
            }
            _ => {}
        }
    }
    out
}

/// Marker shapes telling simulations apart when several columns are plotted.
const SIM_MARKERS: [MarkerShape; 4] = [MarkerShape::Circle, MarkerShape::Square, MarkerShape::Triangle, MarkerShape::Diamond];

/// The styles on a chart, once each, in legend order: key, legend label and
/// style. An automatic colour that another style on the chart already has
/// is swapped for a free one, so the chart never shows two the same.
pub fn chart_styles(config: &PlotConfig, series: &[Series]) -> Vec<(String, String, SeriesStyle)> {
    let mut out: Vec<(String, String, SeriesStyle)> = Vec::new();
    for s in series {
        if out.iter().any(|(k, _, _)| *k == s.key) {
            continue;
        }
        let entry = config.series.get(&s.key);
        let mut style = entry.map(SeriesEntry::style).unwrap_or_else(|| SeriesStyle::automatic(out.len()));
        let custom = entry.is_some_and(|e| e.custom.is_some());
        if !custom && out.iter().any(|(_, _, other)| other.color == style.color) {
            if let Some(&(r, g, b)) =
                CATEGORICAL_PALETTE.iter().find(|&&(r, g, b)| out.iter().all(|(_, _, o)| o.color != [r, g, b]))
            {
                style.color = [r, g, b];
            }
        }
        let label = match entry.map(|e| e.label.trim()) {
            Some(custom) if !custom.is_empty() => custom.to_string(),
            _ => s.label.clone(),
        };
        out.push((s.key.clone(), label, style));
    }
    out
}

/// Render into an in-memory RGB buffer at `scale` × the figure size; the
/// screen shows this, and PNG export writes the same drawing.
pub fn render_rgb(
    table: &DataTable,
    x_col: usize,
    y_cols: &[usize],
    config: &PlotConfig,
    scale: f64,
) -> Result<(Vec<u8>, u32, u32), Box<dyn std::error::Error>> {
    let (w, h) = config.figure_size();
    let (width, height) = ((w as f64 * scale).round() as u32, (h as f64 * scale).round() as u32);
    let mut buffer = vec![0u8; (width * height * 3) as usize];
    {
        let root = BitMapBackend::with_buffer(&mut buffer, (width, height)).into_drawing_area();
        draw_chart(&root, table, x_col, y_cols, config, scale)?;
        root.present()?;
    }
    Ok((buffer, width, height))
}

/// Cuts a polyline to the visible rectangle (Liang–Barsky per segment), so
/// lines never run into the margins when the axis range is set by hand.
fn clip_polyline(points: &[(f64, f64)], xr: (f64, f64), yr: (f64, f64)) -> Vec<Vec<(f64, f64)>> {
    let clip = |a: (f64, f64), b: (f64, f64)| -> Option<((f64, f64), (f64, f64))> {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let (mut t0, mut t1) = (0.0f64, 1.0f64);
        for (p, q) in [(-dx, a.0 - xr.0), (dx, xr.1 - a.0), (-dy, a.1 - yr.0), (dy, yr.1 - a.1)] {
            if p == 0.0 {
                if q < 0.0 {
                    return None;
                }
            } else {
                let t = q / p;
                if p < 0.0 {
                    t0 = t0.max(t);
                } else {
                    t1 = t1.min(t);
                }
            }
        }
        if t0 > t1 {
            return None;
        }
        let at = |t: f64| if t == 0.0 { a } else if t == 1.0 { b } else { (a.0 + t * dx, a.1 + t * dy) };
        Some((at(t0), at(t1)))
    };
    let mut out = Vec::new();
    let mut current: Vec<(f64, f64)> = Vec::new();
    for pair in points.windows(2) {
        match clip(pair[0], pair[1]) {
            Some((a, b)) => {
                if current.last() != Some(&a) {
                    if current.len() > 1 {
                        out.push(std::mem::take(&mut current));
                    }
                    current.clear();
                    current.push(a);
                }
                current.push(b);
            }
            None => {
                if current.len() > 1 {
                    out.push(std::mem::take(&mut current));
                }
                current.clear();
            }
        }
    }
    if current.len() > 1 {
        out.push(current);
    }
    out
}

/// Splits a pixel path into the "on" pieces of a dash pattern (lengths in
/// pixels, alternating on/off). An empty pattern is a solid line.
fn dash(path: &[(f64, f64)], pattern: &[f64]) -> Vec<Vec<(f64, f64)>> {
    if pattern.is_empty() || pattern.iter().any(|l| *l <= 0.0) {
        return vec![path.to_vec()];
    }
    let mut pieces = Vec::new();
    let mut index = 0;
    let mut left = pattern[0];
    let mut current: Vec<(f64, f64)> = path.first().map(|&p| vec![p]).unwrap_or_default();
    for pair in path.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let length = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        let mut done = 0.0;
        while length - done > left {
            done += left;
            let t = done / length;
            let p = (a.0 + t * (b.0 - a.0), a.1 + t * (b.1 - a.1));
            if index % 2 == 0 {
                current.push(p);
                pieces.push(std::mem::take(&mut current));
            } else {
                current = vec![p];
            }
            index = (index + 1) % pattern.len();
            left = pattern[index];
        }
        left -= length - done;
        if index % 2 == 0 {
            current.push(b);
        }
    }
    if index % 2 == 0 && current.len() > 1 {
        pieces.push(current);
    }
    pieces
}

fn draw_line_px<DB: DrawingBackend>(
    root: &DrawingArea<DB, plotters::coord::Shift>,
    path: &[(f64, f64)],
    style: &SeriesStyle,
    scale: f64,
) -> Result<(), Box<dyn std::error::Error>>
where
    DB::ErrorType: 'static,
{
    if style.line == LineType::None || style.line_width <= 0.0 || path.len() < 2 {
        return Ok(());
    }
    let width = (style.line_width as f64 * scale).max(1.0);
    let pattern: Vec<f64> = style.line.pattern().iter().map(|l| (l * width).max(1.5)).collect();
    let stroke = style.rgb().stroke_width(width.round().max(1.0) as u32);
    for piece in dash(path, &pattern) {
        let pixels: Vec<(i32, i32)> = piece.iter().map(|&(x, y)| (x.round() as i32, y.round() as i32)).collect();
        root.draw(&PathElement::new(pixels, stroke))?;
    }
    Ok(())
}

fn draw_marker_px<DB: DrawingBackend>(
    root: &DrawingArea<DB, plotters::coord::Shift>,
    (x, y): (f64, f64),
    style: &SeriesStyle,
    scale: f64,
) -> Result<(), Box<dyn std::error::Error>>
where
    DB::ErrorType: 'static,
{
    let r = style.marker_size as f64 * scale;
    if r < 0.5 {
        return Ok(());
    }
    let fill = style.rgb().filled();
    let p = |dx: f64, dy: f64| ((x + dx).round() as i32, (y + dy).round() as i32);
    match style.marker {
        MarkerShape::None => {}
        MarkerShape::Circle => root.draw(&Circle::new(p(0.0, 0.0), r.round().max(1.0) as i32, fill))?,
        MarkerShape::Square => {
            let s = r * 0.9;
            root.draw(&Rectangle::new([p(-s, -s), p(s, s)], fill))?
        }
        MarkerShape::Triangle => {
            let s = r * 1.25;
            root.draw(&Polygon::new(vec![p(0.0, -s), p(s * 0.866, s * 0.5), p(-s * 0.866, s * 0.5)], fill))?
        }
        MarkerShape::Diamond => {
            let s = r * 1.2;
            root.draw(&Polygon::new(vec![p(0.0, -s), p(s, 0.0), p(0.0, s), p(-s, 0.0)], fill))?
        }
    }
    Ok(())
}

/// Legend layout: box size and each row's height, in pixels.
struct LegendLayout {
    width: f64,
    height: f64,
    row: f64,
    pad: f64,
    swatch: f64,
}

fn legend_font(config: &PlotConfig, scale: f64) -> TextStyle<'static> {
    ("sans-serif", config.legend.font_size.clamp(6.0, 60.0) as f64 * scale).into_font().into()
}

fn legend_layout<DB: DrawingBackend>(
    root: &DrawingArea<DB, plotters::coord::Shift>,
    entries: &[(String, SeriesStyle)],
    config: &PlotConfig,
    scale: f64,
) -> LegendLayout
where
    DB::ErrorType: 'static,
{
    let font = legend_font(config, scale);
    let size = config.legend.font_size.clamp(6.0, 60.0) as f64 * scale;
    let text_width = entries
        .iter()
        .map(|(label, _)| root.estimate_text_size(label, &font).map(|s| s.0 as f64).unwrap_or(size * label.len() as f64 * 0.6))
        .fold(0.0, f64::max);
    let biggest_marker = entries.iter().map(|(_, s)| s.marker_size as f64 * scale).fold(0.0, f64::max);
    let row = (size * 1.45).max(biggest_marker * 2.6);
    let pad = 0.6 * size;
    let swatch = 2.4 * size;
    LegendLayout {
        width: pad + swatch + 0.5 * size + text_width + pad,
        height: pad + row * entries.len() as f64 + pad * 0.6,
        row,
        pad,
        swatch,
    }
}

fn draw_legend<DB: DrawingBackend>(
    root: &DrawingArea<DB, plotters::coord::Shift>,
    entries: &[(String, SeriesStyle)],
    layout: &LegendLayout,
    plot: (std::ops::Range<i32>, std::ops::Range<i32>),
    config: &PlotConfig,
    scale: f64,
) -> Result<(), Box<dyn std::error::Error>>
where
    DB::ErrorType: 'static,
{
    let (x0, x1) = (plot.0.start as f64, plot.0.end as f64);
    let (y0, y1) = (plot.1.start as f64, plot.1.end as f64);
    let inset = 12.0 * scale;
    let (w, h) = (layout.width, layout.height);
    let left = x0 + inset;
    let center_x = (x0 + x1 - w) / 2.0;
    let right = x1 - inset - w;
    let top = y0 + inset;
    let middle = (y0 + y1 - h) / 2.0;
    let bottom = y1 - inset - h;
    let (bx, by) = match config.legend.position {
        LegendPosition::TopLeft => (left, top),
        LegendPosition::TopCenter => (center_x, top),
        LegendPosition::TopRight => (right, top),
        LegendPosition::LeftCenter => (left, middle),
        LegendPosition::RightCenter => (right, middle),
        LegendPosition::BottomLeft => (left, bottom),
        LegendPosition::BottomCenter => (center_x, bottom),
        LegendPosition::BottomRight => (right, bottom),
        LegendPosition::OutsideRight => (x1 + inset, y0),
        LegendPosition::Hidden => return Ok(()),
    };
    let corner = |x: f64, y: f64| (x.round() as i32, y.round() as i32);
    let opacity = config.legend.opacity.clamp(0.0, 1.0) as f64;
    if opacity > 0.0 {
        root.draw(&Rectangle::new([corner(bx, by), corner(bx + w, by + h)], WHITE.mix(opacity).filled()))?;
    }
    if config.legend.frame {
        let frame = RGBColor(110, 110, 110).stroke_width((scale.round() as u32).max(1));
        root.draw(&Rectangle::new([corner(bx, by), corner(bx + w, by + h)], frame))?;
    }
    let font = legend_font(config, scale).pos(Pos::new(HPos::Left, VPos::Center));
    let size = config.legend.font_size.clamp(6.0, 60.0) as f64 * scale;
    for (i, (label, style)) in entries.iter().enumerate() {
        let cy = by + layout.pad + layout.row * (i as f64 + 0.5);
        let sx = bx + layout.pad;
        draw_line_px(root, &[(sx, cy), (sx + layout.swatch, cy)], style, scale)?;
        draw_marker_px(root, (sx + layout.swatch / 2.0, cy), style, scale)?;
        root.draw_text(label, &font, corner(sx + layout.swatch + 0.5 * size, cy))?;
    }
    Ok(())
}

fn draw_chart<DB: DrawingBackend>(
    root: &DrawingArea<DB, plotters::coord::Shift>,
    table: &DataTable,
    x_col: usize,
    y_cols: &[usize],
    config: &PlotConfig,
    scale: f64,
) -> Result<(), Box<dyn std::error::Error>>
where
    DB::ErrorType: 'static,
{
    ensure_fonts_registered();
    root.fill(&WHITE)?;
    let px = |v: f64| (v * scale).round().max(0.0) as u32;
    let font = |size: f32| ("sans-serif", size.clamp(6.0, 80.0) as f64 * scale);
    let message = |text: &str| root.titled(text, font(26.0)).map(|_| ());

    if table.rows.is_empty() || y_cols.is_empty() {
        message("No data to plot")?;
        return Ok(());
    }
    let series = build_series(table, x_col, y_cols, config);
    if segments(table, config).is_empty() {
        message("All simulations are hidden")?;
        return Ok(());
    }
    let all_points = || series.iter().flat_map(|s| s.points.iter().copied());
    let bound = |text: &str, log: bool| parse_bound(text).map(|v| log_xform(log, v)).filter(|v| v.is_finite());
    let x_min = bound(&config.x_min, config.x_log).unwrap_or_else(|| all_points().map(|p| p.0).fold(f64::INFINITY, f64::min));
    let x_max = bound(&config.x_max, config.x_log).unwrap_or_else(|| all_points().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max));
    let y_min = bound(&config.y_min, config.y_log).unwrap_or_else(|| all_points().map(|p| p.1).fold(f64::INFINITY, f64::min));
    let y_max = bound(&config.y_max, config.y_log).unwrap_or_else(|| all_points().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max));
    if !x_min.is_finite() || !x_max.is_finite() || !y_min.is_finite() || !y_max.is_finite() {
        message("No numeric data to plot")?;
        return Ok(());
    }
    // Typed bounds are used as they are; automatic ones get some room.
    let x_pad = axis_pad(x_min, x_max, 0.05);
    let y_pad = axis_pad(y_min, y_max, 0.08);
    let x_range = (
        if parse_bound(&config.x_min).is_some() { x_min } else { x_min - x_pad },
        if parse_bound(&config.x_max).is_some() { x_max } else { x_max + x_pad },
    );
    let y_range = (
        if parse_bound(&config.y_min).is_some() { y_min } else { y_min - y_pad },
        if parse_bound(&config.y_max).is_some() { y_max } else { y_max + y_pad },
    );
    if x_range.0 >= x_range.1 || y_range.0 >= y_range.1 {
        message("Axis minimum must be below the maximum")?;
        return Ok(());
    }

    let mut x_label = match config.x_axis_title.trim() {
        "" => default_x_label(table, x_col),
        t => t.to_string(),
    };
    if config.x_log {
        x_label = format!("log10({x_label})");
    }
    let mut y_label = match config.y_axis_title.trim() {
        "" => default_y_label(table, y_cols),
        t => t.to_string(),
    };
    if config.y_log {
        y_label = format!("log10({y_label})");
    }
    let title = [&config.chart_title, &config.document_title]
        .iter()
        .map(|t| t.trim())
        .find(|t| !t.is_empty())
        .unwrap_or("PHREEQC results")
        .to_string();

    let styles = chart_styles(config, &series);
    let style_of = |s: &Series| {
        let mut style = styles.iter().find(|(k, _, _)| *k == s.key).map(|t| t.2).unwrap_or(SeriesStyle::automatic(0));
        if let Some(marker) = s.marker_override.filter(|_| style.marker != MarkerShape::None) {
            style.marker = marker;
        }
        style
    };
    let drawn: Vec<SeriesStyle> = series.iter().map(style_of).collect();
    // One legend row per style; with several columns over several
    // simulations, the columns (as lines) and then the simulations (as their
    // marker shapes).
    let mut entries: Vec<(String, SeriesStyle)> = Vec::new();
    if series.iter().any(|s| s.marker_override.is_some()) {
        for (_, label, style) in &styles {
            let marker = if style.line == LineType::None { style.marker } else { MarkerShape::None };
            entries.push((label.clone(), SeriesStyle { marker, ..*style }));
        }
        for s in &series {
            if !entries.iter().any(|(l, _)| *l == s.sim_name) {
                let marker = s.marker_override.unwrap_or(MarkerShape::Circle);
                let grey = SeriesStyle { color: [90, 90, 90], line: LineType::None, line_width: 2.0, marker, marker_size: 5.0 };
                entries.push((s.sim_name.clone(), grey));
            }
        }
    } else {
        entries = styles.iter().map(|(_, label, style)| (label.clone(), *style)).collect();
    }
    // With one series the y-axis title already names it.
    let legend_shown = config.legend.position != LegendPosition::Hidden && entries.len() > 1;
    let layout = legend_layout(root, &entries, config, scale);
    let outside = legend_shown && config.legend.position == LegendPosition::OutsideRight;

    let tick = config.tick_font_size as f64;
    let axis = config.axis_font_size as f64;
    let mut chart = ChartBuilder::on(root)
        .margin(px(20.0))
        .margin_right(if outside { px(20.0) + (layout.width + 24.0 * scale).round() as u32 } else { px(28.0) })
        .x_label_area_size(px(tick * 1.3 + axis * 1.3 + 8.0))
        .y_label_area_size(px(tick * 3.9 + axis * 1.7 + 10.0))
        .caption(title, font(config.title_font_size))
        .build_cartesian_2d(x_range.0..x_range.1, y_range.0..y_range.1)?;

    let mut mesh = chart.configure_mesh();
    mesh.x_desc(x_label)
        .y_desc(y_label)
        .label_style(font(config.tick_font_size))
        .axis_desc_style(font(config.axis_font_size))
        .x_labels(8)
        .y_labels(8)
        .light_line_style(RGBColor(235, 235, 235))
        .bold_line_style(RGBColor(205, 205, 205));
    if !config.grid {
        mesh.disable_mesh();
    }
    mesh.draw()?;

    // Lines first, then every marker on top, then the legend over both;
    // none of it changes the axis ranges set above.
    let to_px = |&(x, y): &(f64, f64)| {
        let (px, py) = chart.backend_coord(&(x, y));
        (px as f64, py as f64)
    };
    let saturation = y_cols.iter().any(|&c| table.headers.get(c).is_some_and(|h| h.starts_with("si_")));
    if config.zero_line && saturation && !config.y_log && y_range.0 < 0.0 && y_range.1 > 0.0 {
        let line = SeriesStyle {
            color: [95, 95, 95],
            line: LineType::Dashed,
            line_width: 1.6,
            marker: MarkerShape::None,
            marker_size: 0.0,
        };
        draw_line_px(root, &[to_px(&(x_range.0, 0.0)), to_px(&(x_range.1, 0.0))], &line, scale)?;
    }
    for (s, style) in series.iter().zip(&drawn) {
        for piece in clip_polyline(&s.points, x_range, y_range) {
            let path: Vec<(f64, f64)> = piece.iter().map(to_px).collect();
            draw_line_px(root, &path, style, scale)?;
        }
    }
    let inside = |&(x, y): &(f64, f64)| x >= x_range.0 && x <= x_range.1 && y >= y_range.0 && y <= y_range.1;
    for (s, style) in series.iter().zip(&drawn) {
        for point in s.points.iter().filter(|p| inside(p)) {
            draw_marker_px(root, to_px(point), style, scale)?;
        }
    }
    if legend_shown {
        draw_legend(root, &entries, &layout, chart.plotting_area().get_pixel_range(), config, scale)?;
    }
    Ok(())
}

pub fn export_png(
    table: &DataTable,
    x_col: usize,
    y_cols: &[usize],
    config: &PlotConfig,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let (buffer, width, height) = render_rgb(table, x_col, y_cols, config, config.export_scale() as f64)?;
    let image = image::RgbImage::from_raw(width, height, buffer).ok_or("could not build the image")?;
    image.save_with_format(path, image::ImageFormat::Png)?;
    Ok(())
}

pub fn export_svg(
    table: &DataTable,
    x_col: usize,
    y_cols: &[usize],
    config: &PlotConfig,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = SVGBackend::new(path, config.figure_size()).into_drawing_area();
    draw_chart(&root, table, x_col, y_cols, config, 1.0)?;
    root.present()?;
    Ok(())
}

/// Export the current chart; PNG or SVG chosen by the file extension.
pub fn export(state: &PlotState, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let table = state.table.as_ref().ok_or("no data to plot yet")?;
    if state.y_cols.is_empty() {
        return Err("choose at least one Y series first".into());
    }
    let is_svg = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg"));
    if is_svg {
        export_svg(table, state.x_col, &state.y_cols, &state.config, path)
    } else {
        export_png(table, state.x_col, &state.y_cols, &state.config, path)
    }
}

/// What the plot panel asks the app to do this frame.
#[derive(PartialEq)]
pub enum PlotAction {
    None,
    ExportPlot,
    ExportTable,
    ExportData,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_phreeqc_selected_output_format() {
        // Verbatim shape of what the real `phreeqc` binary writes: tab-separated,
        // trailing tab on every line (including the header).
        let text = concat!(
            "         sim\t       state\t          pH\t          Ca\t\n",
            "           1\t      i_soln\t           7\t  0.0000e+00\t\n",
            "           1\t       react\t     8.52904\t  7.6607e-04\t\n",
        );
        let dir = std::env::temp_dir().join("pq_studio_test_selected_output");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("selected.out");
        std::fs::write(&path, text).unwrap();

        let table = load_selected_output(&path).unwrap();
        assert_eq!(table.headers, vec!["sim", "state", "pH", "Ca"]);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[1][2], 8.52904);
    }
}

#[cfg(test)]
mod render_smoke_test {
    use super::*;

    #[test]
    fn default_axes_pick_reaction_and_user_columns() {
        let dir = std::env::temp_dir().join(format!("pq_studio_axes_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let data = dir.join("sel.out");
        std::fs::write(
            &data,
            concat!(
                "sim\tstate\tstep\tpH\treaction\tCa\tC(4)\tNa\t\n",
                "1\ti_soln\t-99\t7\t-99\t0\t0\t1\t\n",
                "1\treact\t1\t8.3\t5e-4\t5e-4\t1e-3\t1\t\n",
                "1\treact\t2\t7.9\t1e-3\t8e-4\t2e-3\t1\t\n",
            ),
        )
        .unwrap();
        let table = load_selected_output(&data).unwrap();
        assert!(table.rows[0][4].is_nan(), "-99 in 'reaction' is not applicable");
        let (x, ys) = default_axes(&table);
        assert_eq!(table.headers[x], "reaction");
        let names: Vec<&str> = ys.iter().map(|&c| table.headers[c].as_str()).collect();
        assert_eq!(names, ["Ca", "C(4)"], "constant Na is skipped");

        let mut state = PlotState::default();
        state.set_table(table.clone());
        state.y_cols = vec![3];
        state.clear_data();
        state.set_table(table);
        assert_eq!(state.y_cols, vec![3], "user choice survives a rerun");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn axis_pad_gives_single_points_a_real_range() {
        assert_eq!(axis_pad(1.0, 1.0, 0.05), 1.0);
        assert_eq!(axis_pad(0.0, 0.0, 0.05), 1.0);
        assert_eq!(axis_pad(400.0, 400.0, 0.05), 200.0);
        assert!((axis_pad(0.0, 10.0, 0.05) - 0.5).abs() < 1e-12);
    }

    #[test]
    fn calcite_run_defaults_ignore_undefined_saturation_index() {
        // Header and first rows as the real engine wrote them for the
        // CO2-into-calcite test input.
        let dir = std::env::temp_dir().join(format!("pqstudio-si-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("selected.out");
        std::fs::write(
            &path,
            concat!(
                "sim\tstate\tsoln\tdist_x\ttime\tstep\tpH\tpe\treaction\tCa\tC(4)\tsi_Calcite\t\n",
                "1\ti_soln\t1\t-99\t-99\t-99\t7\t4\t-99\t0.0000e+00\t0.0000e+00\t-999.9990\t\n",
                "1\treact\t1\t-99\t0\t1\t9.91622\t7.77\t0.0000e+00\t1.2689e-04\t1.2689e-04\t0.0000\t\n",
                "1\treact\t1\t-99\t0\t2\t8.28271\t-2.4\t5.0000e-04\t5.0703e-04\t1.0070e-03\t0.0000\t\n",
                "1\treact\t1\t-99\t0\t3\t7.76215\t10.0\t1.0000e-03\t9.4418e-04\t1.9442e-03\t0.0000\t\n",
            ),
        )
        .unwrap();
        let table = load_selected_output(&path).unwrap();
        std::fs::remove_dir_all(&dir).ok();

        let col = |name: &str| table.headers.iter().position(|h| h == name).unwrap();
        assert!(table.rows[0][col("si_Calcite")].is_nan());
        let (x, ys) = default_axes(&table);
        assert_eq!(x, col("reaction"));
        assert_eq!(ys, vec![col("Ca"), col("C(4)")]);
        assert_eq!(default_y_label(&table, &ys), "Total (mol/kgw)");
    }

    #[test]
    fn default_labels_read_phreeqc_column_names() {
        let headers = [
            "sim", "state", "soln", "dist_x", "time", "step", "pH", "pe", "reaction",
            "Ca", "C(4)", "Calcite", "d_Calcite", "si_Calcite", "m_CO3-2", "la_Ca+2",
            "g_CO2(g)", "my_punch_value",
        ];
        let table = DataTable {
            headers: headers.iter().map(|h| h.to_string()).collect(),
            rows: vec![],
            initial: vec![],
            source: String::new(),
        };
        let col = |name: &str| headers.iter().position(|h| *h == name).unwrap();

        assert_eq!(default_x_label(&table, col("reaction")), "Reaction (mol)");
        assert_eq!(default_x_label(&table, col("time")), "Time (s)");
        assert_eq!(default_y_label(&table, &[col("pH")]), "pH");
        assert_eq!(default_y_label(&table, &[col("Ca")]), "Ca (mol/kgw)");
        assert_eq!(default_y_label(&table, &[col("Ca"), col("C(4)")]), "Total (mol/kgw)");
        assert_eq!(default_y_label(&table, &[col("si_Calcite")]), "SI Calcite");
        assert_eq!(default_y_label(&table, &[col("Calcite")]), "Calcite (mol)");
        assert_eq!(default_y_label(&table, &[col("d_Calcite")]), "Δ Calcite (mol)");
        assert_eq!(default_y_label(&table, &[col("m_CO3-2")]), "CO3-2 (mol/kgw)");
        assert_eq!(default_y_label(&table, &[col("la_Ca+2")]), "log a Ca+2");
        assert_eq!(default_y_label(&table, &[col("g_CO2(g)")]), "CO2(g) (mol)");
        assert_eq!(default_y_label(&table, &[col("my_punch_value")]), "my_punch_value");
        assert_eq!(default_y_label(&table, &[col("pH"), col("Ca")]), "Value");
        assert_eq!(default_y_label(&table, &[col("Ca"), col("m_CO3-2")]), "Value (mol/kgw)");
    }

    #[test]
    fn export_picks_format_from_extension() {
        let dir = std::env::temp_dir().join(format!("pq_studio_export_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let data = dir.join("sel.out");
        std::fs::write(&data, "step\tpH\t\n1\t7.0\t\n2\t7.5\t\n3\t8.1\t\n").unwrap();
        let mut state = PlotState::default();
        state.table = Some(load_selected_output(&data).unwrap());
        assert!(export(&state, &dir.join("none.png")).is_err(), "no Y series chosen yet");
        state.x_col = 0;
        state.y_cols = vec![1];
        export(&state, &dir.join("chart.svg")).unwrap();
        export(&state, &dir.join("chart.png")).unwrap();
        let svg = std::fs::read_to_string(dir.join("chart.svg")).unwrap();
        assert!(svg.starts_with("<svg") || svg.contains("<svg"));
        let png = std::fs::read(dir.join("chart.png")).unwrap();
        assert_eq!(&png[1..4], b"PNG");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn renders_png_and_svg_with_working_fonts() {
        // Self-contained fixture shaped like real SELECTED_OUTPUT: sim, state,
        // soln, dist_x, time, step, pH, pe, Ca — mirrors the real column
        // order/format this app actually consumes (see
        // parses_real_phreeqc_selected_output_format for the raw-format check).
        let text = concat!(
            "sim\tstate\tsoln\tdist_x\ttime\tstep\tpH\tpe\tCa\t\n",
            "1\ti_soln\t1\t-99\t-99\t-99\t7.0\t4.0\t0.0\t\n",
            "1\treact\t1\t-99\t8.64e6\t1\t8.52904\t-1.33556\t7.6607e-04\t\n",
            "1\treact\t1\t-99\t1.728e7\t2\t8.42063\t-1.43538\t1.0111e-03\t\n",
            "1\treact\t1\t-99\t2.592e7\t3\t8.33025\t-1.57252\t1.1720e-03\t\n",
        );
        let dir = std::env::temp_dir().join("pq_studio_render_smoke");
        std::fs::create_dir_all(&dir).unwrap();
        let data_path = dir.join("fixture_selected.out");
        std::fs::write(&data_path, text).unwrap();
        let table = load_selected_output(&data_path).unwrap();
        assert!(!table.rows.is_empty());

        let x_col = 4; // time
        let y_cols = vec![6usize, 8]; // pH, Ca
        let png = dir.join("smoke.png");
        let svg = dir.join("smoke.svg");
        let config = PlotConfig::default();
        export_png(&table, x_col, &y_cols, &config, &png).expect("png export failed");
        export_svg(&table, x_col, &y_cols, &config, &svg).expect("svg export failed");
        let png_meta = std::fs::metadata(&png).unwrap();
        let svg_meta = std::fs::metadata(&svg).unwrap();
        assert!(
            png_meta.len() > 1000,
            "png suspiciously small: {}",
            png_meta.len()
        );
        assert!(
            svg_meta.len() > 500,
            "svg suspiciously small: {}",
            svg_meta.len()
        );
        eprintln!("png: {} ({} bytes)", png.display(), png_meta.len());
        eprintln!("svg: {} ({} bytes)", svg.display(), svg_meta.len());
    }
}

#[cfg(test)]
mod config_smoke_test {
    use super::*;

    #[test]
    fn renders_with_custom_titles_bounds_and_log_scale() {
        let table = DataTable {
            headers: vec!["time".into(), "Ca".into()],
            rows: vec![vec![1.0, 1e-5], vec![10.0, 1e-4], vec![100.0, 1e-3]],
            initial: vec![],
            source: String::new(),
        };
        let config = PlotConfig {
            chart_title: "Custom title".into(),
            x_axis_title: "Time (s)".into(),
            y_axis_title: "Calcium (mol/kgw)".into(),
            x_log: true,
            y_log: true,
            ..Default::default()
        };
        let dir = std::env::temp_dir().join("pq_studio_config_smoke");
        std::fs::create_dir_all(&dir).unwrap();
        let png = dir.join("config_smoke.png");
        export_png(&table, 0, &[1], &config, &png).expect("log-scale png export failed");
        assert!(std::fs::metadata(&png).unwrap().len() > 1000);
    }

    #[test]
    fn several_simulations_plot_against_temperature_one_line_each() {
        let dir = std::env::temp_dir().join(format!("pq-studio-sims-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sel.out");
        let mut text = String::from("sim\tstate\tsoln\tstep\tpH\ttemp\tsi_Halite\t\n");
        for soln in 1..=2 {
            text.push_str(&format!("1\ti_soln\t{soln}\t-99\t7\t25\t-3\t\n"));
        }
        for (sim, soln) in [(2, 2), (3, 5)] {
            for (step, temp) in [25, 40, 60, 80, 100, 120].iter().enumerate() {
                text.push_str(&format!("{sim}\treact\t{soln}\t{}\t6.5\t{temp}\t-{soln}.{step}\t\n", step + 1));
            }
        }
        std::fs::write(&path, text).unwrap();
        let table = load_selected_output(&path).unwrap();
        assert_eq!(table.initial.iter().filter(|&&i| i).count(), 2);
        let (x, ys) = default_axes(&table);
        assert_eq!(table.headers[x], "temp");
        assert_eq!(ys.iter().map(|&c| table.headers[c].as_str()).collect::<Vec<_>>(), ["si_Halite"]);
        let svg = dir.join("sims.svg");
        let state = PlotState {
            x_col: x,
            y_cols: ys,
            table: Some(table),
            ..Default::default()
        };
        export(&state, &svg).unwrap();
        let drawn = std::fs::read_to_string(&svg).unwrap();
        assert!(drawn.contains("Solution 2") && drawn.contains("Solution 5"));
        assert!(drawn.contains("Temperature (°C)"));

        // Named simulations, several columns (one marker shape per
        // simulation), one simulation hidden, and the table as CSV.
        let mut state = state;
        let table = state.table.clone().unwrap();
        state.y_cols = vec![table.headers.iter().position(|h| h == "pH").unwrap(), state.y_cols[0]];
        state.config.sim_names = [(2, "NaCl · Calcite".to_string()), (3, "SW, warm".to_string())].into();
        export(&state, &svg).unwrap();
        let drawn = std::fs::read_to_string(&svg).unwrap();
        assert!(drawn.contains("NaCl · Calcite") && drawn.contains("si_Halite"));
        state.config.hidden_sims.insert(2);
        export(&state, &svg).unwrap();
        assert!(!std::fs::read_to_string(&svg).unwrap().contains("NaCl · Calcite"));
        assert_eq!(plotted_simulations(&table), vec![2, 3]);

        let csv = table_text(&table, &state.config.sim_names, ',');
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "name,sim,state,soln,step,pH,temp,si_Halite");
        assert_eq!(lines[1], ",1,i_soln,1,-99,7,25,-3");
        assert_eq!(lines[3], "NaCl · Calcite,2,react,2,1,6.5,25,-2.0");
        assert!(lines[9].starts_with("\"SW, warm\",3,react,5,1,"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}


#[cfg(test)]
mod style_tests {
    use super::*;

    /// Nine SI columns against temperature, one simulation.
    fn nine_si_table() -> DataTable {
        let names = ["Anhydrite", "Gypsum", "Celestite", "Barite", "Aragonite", "Calcite", "Witherite", "Dolomite", "Strontianite"];
        let mut headers = vec!["sim".to_string(), "state".to_string(), "temp".to_string()];
        headers.extend(names.iter().map(|n| format!("si_{n}")));
        let rows = (0..6)
            .map(|step| {
                let mut row = vec![1.0, f64::NAN, 25.0 + 25.0 * step as f64];
                row.extend((0..names.len()).map(|i| i as f64 * 0.3 - 1.0 + step as f64 * 0.1));
                row
            })
            .collect();
        DataTable { headers, rows, initial: vec![false; 6], source: String::new() }
    }

    #[test]
    fn automatic_styles_are_solid_with_circles_in_distinct_colours() {
        let colours: BTreeSet<[u8; 3]> = (0..24).map(|i| SeriesStyle::automatic(i).color).collect();
        assert_eq!(colours.len(), 24);
        assert!((0..24).all(|i| {
            let s = SeriesStyle::automatic(i);
            s.line == LineType::Solid && s.marker == MarkerShape::Circle
        }));
    }

    fn colour_of(state: &PlotState, key: &str) -> [u8; 3] {
        let series = build_series(state.table.as_ref().unwrap(), state.x_col, &state.y_cols, &state.config);
        chart_styles(&state.config, &series).into_iter().find(|(k, _, _)| k == key).unwrap().2.color
    }

    #[test]
    fn a_column_keeps_its_style_across_plots() {
        let mut state = PlotState::default();
        state.set_table(nine_si_table());
        state.x_col = 2;
        state.y_cols = (3..12).collect();
        state.assign_styles();
        let series = build_series(state.table.as_ref().unwrap(), state.x_col, &state.y_cols, &state.config);
        let colours: BTreeSet<[u8; 3]> = chart_styles(&state.config, &series).iter().map(|t| t.2.color).collect();
        assert_eq!(colours.len(), 9);

        // si_Barite (column 6) made black stays black when unticked and
        // ticked again, and in a plot with other columns.
        let black = SeriesStyle { color: [0, 0, 0], ..SeriesStyle::automatic(0) };
        state.config.series.get_mut("si_Barite").unwrap().custom = Some(black);
        let calcite = colour_of(&state, "si_Calcite");
        state.y_cols = vec![4, 8];
        state.assign_styles();
        assert_eq!(colour_of(&state, "si_Calcite"), calcite, "automatic colours stay too");
        state.y_cols = vec![6, 10];
        state.assign_styles();
        assert_eq!(colour_of(&state, "si_Barite"), [0, 0, 0]);

        // A new project's settings keep the library's styles.
        state.load_config(PlotConfig::default(), false);
        state.set_table(nine_si_table());
        state.x_col = 2;
        state.y_cols = vec![6];
        assert_eq!(colour_of(&state, "si_Barite"), [0, 0, 0]);

        // Reset brings back the automatic style.
        state.config.series.get_mut("si_Barite").unwrap().custom = None;
        let auto = SeriesStyle::automatic(state.config.series["si_Barite"].auto);
        assert_eq!(colour_of(&state, "si_Barite"), auto.color);
    }

    #[test]
    fn styles_follow_the_column_across_simulations() {
        // Two simulations: with several columns the column keeps its style
        // whichever simulations are shown; with one column each simulation
        // has its own (named) style.
        let mut table = nine_si_table();
        let more: Vec<Vec<f64>> = table.rows.iter().map(|r| {
            let mut r = r.clone();
            r[0] = 2.0;
            r
        }).collect();
        table.rows.extend(more);
        table.initial = vec![false; 12];
        let mut state = PlotState::default();
        state.config.sim_names = [(1, "DW".to_string()), (2, "SW".to_string())].into();
        state.set_table(table);
        state.x_col = 2;
        state.y_cols = vec![6, 7];
        state.assign_styles();
        state.config.series.get_mut("si_Barite").unwrap().custom = Some(SeriesStyle { color: [0, 0, 0], ..SeriesStyle::automatic(0) });
        assert_eq!(colour_of(&state, "si_Barite"), [0, 0, 0]);
        state.config.hidden_sims.insert(1);
        assert_eq!(colour_of(&state, "si_Barite"), [0, 0, 0]);
        state.config.hidden_sims.clear();
        let series = build_series(state.table.as_ref().unwrap(), 2, &[6, 7], &state.config);
        assert_eq!(series.len(), 4);
        assert!(series.iter().all(|s| s.marker_override.is_some()));
        state.y_cols = vec![6];
        state.assign_styles();
        let keys: Vec<String> = build_series(state.table.as_ref().unwrap(), 2, &[6], &state.config).into_iter().map(|s| s.key).collect();
        assert_eq!(keys, ["sim: DW", "sim: SW"]);
        assert_ne!(colour_of(&state, "sim: DW"), colour_of(&state, "sim: SW"));
    }

    #[test]
    fn old_per_simulation_styles_become_column_styles() {
        let black = SeriesStyle { color: [0, 0, 0], ..SeriesStyle::automatic(0) };
        let mut library = BTreeMap::new();
        merge_styles(
            &mut library,
            [
                ("si_Barite @ sim 3".to_string(), SeriesEntry { auto: 4, custom: Some(black), label: String::new() }),
                ("si_Gypsum @ sim 3".to_string(), SeriesEntry { auto: 5, ..Default::default() }),
            ]
            .into(),
        );
        assert_eq!(library.keys().collect::<Vec<_>>(), ["si_Barite"]);
        assert_eq!(library["si_Barite"].custom, Some(black));
    }

    #[test]
    fn legend_positions_never_change_the_axes_and_png_matches_the_screen() {
        let table = nine_si_table();
        let y: Vec<usize> = (3..12).collect();
        let mut config = PlotConfig::default();
        config.legend.position = LegendPosition::Hidden;
        let (bare, _, _) = render_rgb(&table, 2, &y, &config, 1.0).unwrap();
        for position in LegendPosition::ALL {
            config.legend.position = position;
            let (image, w, h) = render_rgb(&table, 2, &y, &config, 1.0).unwrap();
            assert_eq!((w, h), (REPORT_WIDTH, REPORT_HEIGHT));
            if position != LegendPosition::OutsideRight {
                // Same plot area: the bottom axis line row is identical
                // outside the legend box, so nothing moved.
                let differing = image.iter().zip(&bare).filter(|(a, b)| a != b).count();
                assert!(position == LegendPosition::Hidden || differing > 0);
                assert!(differing < image.len() / 6, "{position:?} changed too much of the chart");
            }
        }

        // The PNG file holds exactly the drawing the screen shows at that scale.
        let dir = std::env::temp_dir().join(format!("pq-studio-png-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        config.export_scale = 2;
        let png = dir.join("chart.png");
        export_png(&table, 2, &y, &config, &png).unwrap();
        let file = image::open(&png).unwrap().into_rgb8();
        let (screen, w, h) = render_rgb(&table, 2, &y, &config, 2.0).unwrap();
        assert_eq!((file.width(), file.height()), (w, h));
        assert!(file.into_raw() == screen);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn settings_round_trip_through_the_file_beside_the_project() {
        let dir = std::env::temp_dir().join(format!("pq-studio-plotcfg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let project = dir.join("part1.pqi");
        assert_eq!(settings_path(&project), dir.join("part1.pqstudio.json"));

        let mut state = PlotState::default();
        state.set_table(nine_si_table());
        state.x_col = 2;
        state.y_cols = vec![5, 8];
        state.config.legend = LegendSettings { position: LegendPosition::OutsideRight, font_size: 24.0, frame: false, opacity: 0.4 };
        state.config.y_min = "-3".into();
        state.config.y_log = false;
        state.config.export_scale = 3;
        state.assign_styles();
        state.config.series.get_mut("si_Celestite").unwrap().custom = Some(SeriesStyle {
            color: [9, 9, 9],
            line: LineType::Dotted,
            line_width: 4.0,
            marker: MarkerShape::Diamond,
            marker_size: 8.0,
        });
        state.config.document_title = "not saved".into();
        write_settings(&settings_path(&project), &state.saved_config()).unwrap();

        let loaded = read_settings(&settings_path(&project)).unwrap();
        assert_eq!(loaded.legend, state.config.legend);
        assert_eq!(loaded.series, state.config.series);
        assert_eq!(loaded.y_min, "-3");
        assert_eq!(loaded.export_scale, 3);
        assert_eq!(loaded.x_column.as_deref(), Some("temp"));
        assert_eq!(loaded.y_columns, ["si_Celestite", "si_Calcite"]);
        assert!(loaded.document_title.is_empty());

        // Reopened: the next run's table gets the saved columns back.
        let mut reopened = PlotState::default();
        reopened.load_config(loaded, true);
        reopened.set_table(nine_si_table());
        assert_eq!((reopened.x_col, reopened.y_cols.clone()), (2, vec![5, 8]));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn dashes_and_clipping() {
        let pieces = dash(&[(0.0, 0.0), (10.0, 0.0)], &[2.0, 3.0]);
        assert_eq!(pieces, vec![vec![(0.0, 0.0), (2.0, 0.0)], vec![(5.0, 0.0), (7.0, 0.0)]]);
        assert_eq!(dash(&[(0.0, 0.0), (4.0, 0.0)], &[]), vec![vec![(0.0, 0.0), (4.0, 0.0)]]);
        let clipped = clip_polyline(&[(-1.0, 0.5), (0.5, 0.5), (2.0, 0.5)], (0.0, 1.0), (0.0, 1.0));
        assert_eq!(clipped, vec![vec![(0.0, 0.5), (0.5, 0.5), (1.0, 0.5)]]);
        assert!(clip_polyline(&[(2.0, 2.0), (3.0, 3.0)], (0.0, 1.0), (0.0, 1.0)).is_empty());
    }
}

#[cfg(test)]
mod fixed_colour_tests {
    use super::*;

    #[test]
    fn common_minerals_have_fixed_colours_and_others_avoid_them() {
        // A library like one written before fixed colours existed.
        let entry = |auto| SeriesEntry { auto, ..Default::default() };
        let mut state = PlotState::default();
        state.config.series = [
            ("si_Anhydrite", 0), ("si_Aragonite", 21), ("si_Barite", 8), ("si_Calcite", 16),
            ("si_Celestite", 12), ("si_Dolomite", 26), ("si_Gypsum", 1), ("si_Strontianite", 34),
            ("si_Witherite", 30), ("pH", 3), ("si_Halite", 8),
        ]
        .into_iter()
        .map(|(k, a)| (k.to_string(), entry(a)))
        .collect();
        state.normalize_styles();
        let colour = |k: &str| SeriesStyle::automatic(state.config.series[k].auto).color;
        let rgb = |i: usize| {
            let (r, g, b) = CATEGORICAL_PALETTE[i];
            [r, g, b]
        };
        assert_eq!(colour("si_Calcite"), rgb(0));
        assert_eq!(colour("si_Aragonite"), rgb(1));
        assert_eq!(colour("si_Dolomite"), rgb(2));
        assert_eq!(colour("si_Anhydrite"), rgb(3));
        assert_eq!(colour("si_Gypsum"), rgb(4));
        assert_eq!(colour("si_Witherite"), rgb(6));
        assert_eq!(colour("si_Barite"), rgb(8));
        assert_eq!(colour("si_Strontianite"), rgb(10));
        assert_eq!(colour("si_Celestite"), rgb(12));
        // pH and si_Halite, on fixed colours, move to free ones.
        let all: BTreeSet<[u8; 3]> = state.config.series.keys().map(|k| colour(k)).collect();
        assert_eq!(all.len(), state.config.series.len());
        assert!(state.styles_changed);

        // A custom colour still wins over the fixed one, and reset returns to it.
        state.config.series.get_mut("si_Calcite").unwrap().custom =
            Some(SeriesStyle { color: [0, 0, 0], ..SeriesStyle::automatic(0) });
        assert_eq!(state.config.series["si_Calcite"].style().color, [0, 0, 0]);
        state.config.series.get_mut("si_Calcite").unwrap().custom = None;
        assert_eq!(state.config.series["si_Calcite"].style().color, rgb(0));

        // A brand-new column never takes a fixed colour.
        state.config.series.insert("si_Fluorite".into(), SeriesEntry { auto: usize::MAX, ..Default::default() });
        state.normalize_styles();
        let fixed: BTreeSet<usize> = FIXED_COLOURS.iter().map(|f| f.1).collect();
        assert!(!fixed.contains(&state.config.series["si_Fluorite"].auto));
    }
}

/// A named plot set-up, reusable in any project: the X/Y columns, axes,
/// legend and export settings (not the chart title, the simulations shown
/// or the series styles, which are kept separately).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlotPreset {
    pub name: String,
    pub config: PlotConfig,
}

pub fn presets_path() -> Option<PathBuf> {
    crate::settings::config_dir().map(|d| d.join("plot_presets.json"))
}

pub fn read_presets() -> Vec<PlotPreset> {
    presets_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn write_presets(presets: &[PlotPreset]) -> std::io::Result<()> {
    let Some(path) = presets_path() else {
        return Ok(());
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(presets)?)?;
    std::fs::rename(tmp, path)
}

/// The preset to save for the current plot.
pub fn preset_from(state: &PlotState, name: &str) -> PlotPreset {
    let mut config = state.saved_config();
    config.series.clear();
    config.hidden_sims.clear();
    config.chart_title.clear();
    PlotPreset { name: name.trim().to_string(), config }
}

/// The data the chart shows, for plotting elsewhere: one row per plotted
/// point in time order, with the simulation's name and number, the X
/// column and each Y column, values exactly as PHREEQC wrote them. Rows
/// and simulations left out of the chart are left out here too.
pub struct PlottedData {
    pub headers: Vec<String>,
    /// Per simulation: its name, number and rows of values (NaN = empty).
    pub groups: Vec<(String, Option<i64>, Vec<Vec<f64>>)>,
}

pub fn plotted_data(table: &DataTable, x_col: usize, y_cols: &[usize], config: &PlotConfig) -> PlottedData {
    let column = |name: &str| table.headers.iter().position(|h| h == name);
    let sim_col = column("sim");
    let names: BTreeMap<usize, String> =
        build_series(table, x_col, y_cols, config).into_iter().map(|s| (s.sim_index, s.sim_name)).collect();
    let mut headers = vec!["simulation".to_string(), "sim".to_string()];
    headers.extend(std::iter::once(x_col).chain(y_cols.iter().copied()).filter_map(|c| table.headers.get(c).cloned()));
    let groups = segments(table, config)
        .into_iter()
        .enumerate()
        .map(|(s, rows)| {
            let sim = sim_col.and_then(|c| table.rows[rows[0]].get(c).copied()).filter(|v| v.is_finite()).map(|v| v as i64);
            let name = names.get(&s).cloned().unwrap_or_else(|| match sim {
                Some(n) => config.sim_names.get(&n).cloned().unwrap_or_else(|| format!("Simulation {n}")),
                None => String::new(),
            });
            let values = rows
                .iter()
                .map(|&r| {
                    std::iter::once(x_col)
                        .chain(y_cols.iter().copied())
                        .map(|c| table.rows[r].get(c).copied().unwrap_or(f64::NAN))
                        .collect()
                })
                .collect();
            (name, sim, values)
        })
        .collect();
    PlottedData { headers, groups }
}

fn number_text(v: f64) -> String {
    if v.is_finite() { format!("{v}") } else { String::new() }
}

/// The plotted data as CSV.
pub fn plotted_csv(data: &PlottedData) -> String {
    let quote = |cell: &str| {
        if cell.contains(',') || cell.contains('"') {
            format!("\"{}\"", cell.replace('"', "\"\""))
        } else {
            cell.to_string()
        }
    };
    let mut out = data.headers.iter().map(|h| quote(h)).collect::<Vec<_>>().join(",");
    out.push('\n');
    for (name, sim, rows) in &data.groups {
        for row in rows {
            let mut cells = vec![quote(name), sim.map(|n| n.to_string()).unwrap_or_default()];
            cells.extend(row.iter().map(|&v| number_text(v)));
            out.push_str(&cells.join(","));
            out.push('\n');
        }
    }
    out
}

/// The plotted data as an Excel workbook: every row on the first sheet, and
/// with several simulations one sheet per simulation (X and Y columns side
/// by side, ready for a scatter chart).
pub fn write_plotted_xlsx(data: &PlottedData, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use rust_xlsxwriter::{Format, Workbook, Worksheet};
    let bold = Format::new().set_bold();
    let mut workbook = Workbook::new();
    let fill = |sheet: &mut Worksheet, headers: &[String], rows: &[Vec<(Option<String>, f64)>]| -> Result<(), rust_xlsxwriter::XlsxError> {
        for (c, h) in headers.iter().enumerate() {
            sheet.write_string_with_format(0, c as u16, h, &bold)?;
            sheet.set_column_width(c as u16, (h.chars().count() as f64 + 2.0).max(11.0))?;
        }
        for (r, row) in rows.iter().enumerate() {
            for (c, (text, v)) in row.iter().enumerate() {
                let (r, c) = (r as u32 + 1, c as u16);
                match text {
                    Some(t) => {
                        sheet.write_string(r, c, t)?;
                    }
                    None if v.is_finite() => {
                        sheet.write_number(r, c, *v)?;
                    }
                    None => {}
                }
            }
        }
        sheet.set_freeze_panes(1, 0)?;
        Ok(())
    };
    let all: Vec<Vec<(Option<String>, f64)>> = data
        .groups
        .iter()
        .flat_map(|(name, sim, rows)| {
            rows.iter().map(move |row| {
                let mut cells = vec![(Some(name.clone()), f64::NAN), (None, sim.map(|n| n as f64).unwrap_or(f64::NAN))];
                cells.extend(row.iter().map(|&v| (None, v)));
                cells
            })
        })
        .collect();
    let sheet = workbook.add_worksheet().set_name("Plotted data")?;
    fill(sheet, &data.headers, &all)?;
    if data.groups.len() > 1 {
        let mut used: BTreeSet<String> = ["Plotted data".to_string()].into();
        for (i, (name, sim, rows)) in data.groups.iter().enumerate() {
            // Excel sheet names: at most 31 characters, none of []:*?/\.
            let clean: String = name.chars().map(|c| if "[]:*?/\\".contains(c) { '-' } else { c }).collect();
            let base = match sim {
                Some(n) if clean.is_empty() => format!("Simulation {n}"),
                _ if clean.trim().is_empty() => format!("Simulation {}", i + 1),
                _ => clean.trim().to_string(),
            };
            let mut title: String = base.chars().take(31).collect();
            let mut k = 2;
            while used.contains(&title.to_lowercase()) {
                let suffix = format!(" ({k})");
                title = base.chars().take(31 - suffix.len()).collect::<String>() + &suffix;
                k += 1;
            }
            used.insert(title.to_lowercase());
            let cells: Vec<Vec<(Option<String>, f64)>> = rows.iter().map(|row| row.iter().map(|&v| (None, v)).collect()).collect();
            let sheet = workbook.add_worksheet().set_name(&title)?;
            fill(sheet, &data.headers[2..], &cells)?;
        }
    }
    workbook.save(path)?;
    Ok(())
}

#[cfg(test)]
mod export_preset_tests {
    use super::*;

    fn two_sim_table() -> DataTable {
        let headers: Vec<String> = ["sim", "state", "soln", "temp", "pH", "si_Calcite", "d_Calcite", "Calcite"]
            .iter()
            .map(|h| h.to_string())
            .collect();
        let mut rows = vec![vec![1.0, f64::NAN, 1.0, 25.0, 7.0, -0.5, f64::NAN, f64::NAN]];
        let mut initial = vec![true];
        for sim in [2.0, 3.0] {
            for (i, t) in [25.0, 50.0, 75.0].iter().enumerate() {
                rows.push(vec![sim, f64::NAN, sim - 1.0, *t, 7.0 - i as f64 * 0.1, 0.1 * i as f64 - sim / 10.0, 1e-4, 0.01]);
                initial.push(false);
            }
        }
        DataTable { headers, rows, initial, source: String::new() }
    }

    #[test]
    fn plotted_data_holds_only_what_the_chart_shows() {
        let table = two_sim_table();
        let mut config = PlotConfig::default();
        config.sim_names = [(2, "DW, cold".to_string()), (3, "SW".to_string())].into();
        let data = plotted_data(&table, 3, &[5, 4], &config);
        assert_eq!(data.headers, ["simulation", "sim", "temp", "si_Calcite", "pH"]);
        assert_eq!(data.groups.len(), 2, "the initial solution row is left out");
        let csv = plotted_csv(&data);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 7);
        assert_eq!(lines[1], "\"DW, cold\",2,25,-0.2,7");
        config.hidden_sims.insert(2);
        let csv = plotted_csv(&plotted_data(&table, 3, &[5], &config));
        assert_eq!(csv.lines().count(), 4);
        assert!(csv.lines().nth(1).unwrap().starts_with("SW,3,25,"));

        config.hidden_sims.clear();
        let dir = std::env::temp_dir().join(format!("pq-studio-xlsx-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("plotted.xlsx");
        write_plotted_xlsx(&plotted_data(&table, 3, &[5, 4], &config), &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[..2], b"PK", "an xlsx file is a zip archive");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_mineral_shares_its_colour_across_its_columns() {
        let mut state = PlotState::default();
        let entry = |auto| SeriesEntry { auto, ..Default::default() };
        state.config.series = [("si_Halite", 15), ("d_Halite", 3), ("Halite", 20), ("si_Fluorite", 17), ("d_Fluorite", 18), ("Fluorite", 19), ("pH", 17)]
            .into_iter()
            .map(|(k, a)| (k.to_string(), entry(a)))
            .collect();
        state.normalize_styles();
        let auto = |k: &str| state.config.series[k].auto;
        assert_eq!((auto("si_Halite"), auto("d_Halite"), auto("Halite")), (9, 9, 9), "Halite is fixed");
        assert_eq!((auto("si_Fluorite"), auto("d_Fluorite"), auto("Fluorite")), (17, 17, 17), "the si_ colour is kept");
        assert_ne!(auto("pH"), 17, "another column moves off it");
        assert_eq!(family("Ca", |_| false), "column Ca");
        assert_eq!(family("g_CO2(g)", |_| false), "CO2(g)");
    }

    #[test]
    fn presets_carry_columns_and_axes_but_not_the_title_or_simulations() {
        let mut state = PlotState::default();
        state.set_table(two_sim_table());
        state.x_col = 3;
        state.y_cols = vec![5];
        state.config.y_min = "-1".into();
        state.config.zero_line = false;
        state.config.legend.position = LegendPosition::BottomLeft;
        state.config.chart_title = "DW only".into();
        state.config.hidden_sims.insert(3);
        let preset = preset_from(&state, " SI vs T ");
        assert_eq!(preset.name, "SI vs T");

        let mut other = PlotState::default();
        other.set_table(two_sim_table());
        other.config.chart_title = "SW".into();
        other.config.hidden_sims.insert(2);
        other.apply_preset(&preset.config);
        assert_eq!((other.x_col, other.y_cols.clone()), (3, vec![5]));
        assert_eq!(other.config.y_min, "-1");
        assert!(!other.config.zero_line);
        assert_eq!(other.config.legend.position, LegendPosition::BottomLeft);
        assert_eq!(other.config.chart_title, "SW");
        assert!(other.config.hidden_sims.contains(&2));
        let json = serde_json::to_string(&vec![preset.clone()]).unwrap();
        let back: Vec<PlotPreset> = serde_json::from_str(&json).unwrap();
        assert_eq!(back[0], preset);
    }
}
