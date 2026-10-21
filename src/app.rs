use crate::model::{BLOCK_KINDS, Block, DATABASE_KEYWORDS, Document, Entity, Raw};
use crate::settings::Settings;
use crate::{forms, io, phreeqc, plotting, runner, serialize, simulations, theme, validate};
use egui::{Key, KeyboardShortcut, Modifiers};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const SHORTCUT_NEW: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::N);
const SHORTCUT_OPEN: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::O);
const SHORTCUT_SAVE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
const SHORTCUT_SAVE_AS: KeyboardShortcut = KeyboardShortcut::new(
    Modifiers {
        shift: true,
        ..Modifiers::COMMAND
    },
    Key::S,
);
const SHORTCUT_RUN: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::R);
const SHORTCUT_RUN_F5: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::F5);
const SHORTCUT_QUIT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Q);
const SHORTCUT_DELETE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::Delete);
const SHORTCUT_MOVE_UP: KeyboardShortcut = KeyboardShortcut::new(Modifiers::ALT, Key::ArrowUp);
const SHORTCUT_MOVE_DOWN: KeyboardShortcut = KeyboardShortcut::new(Modifiers::ALT, Key::ArrowDown);

const INPUT_EXTENSIONS: [&str; 3] = ["pqi", "phr", "txt"];

/// Original example inputs bundled with the app (not the USGS examples).
const EXAMPLES: [(&str, &str); 2] = [
    ("Seawater speciation", include_str!("../assets/examples/seawater-speciation.pqi")),
    ("Calcite dissolution as CO₂ is added", include_str!("../assets/examples/calcite-co2.pqi")),
];

/// Official documentation pages, as cited in NOTICE.md.
const PHREEQC_SOFTWARE_PAGE: &str = "https://www.usgs.gov/software/phreeqc-version-3";
const PHREEQC_MANUAL_PAGE: &str = "https://pubs.usgs.gov/tm/06/a43/";

/// Something that would discard the current document, held while the
/// "unsaved changes" prompt is open.
#[derive(Debug, Clone)]
pub(crate) enum Pending {
    New,
    Open,
    OpenPath(PathBuf),
    OpenExample(usize),
    Quit,
}

pub(crate) struct App {
    pub(crate) doc: Document,
    pub(crate) selected: usize,
    pub(crate) settings: Settings,
    pub(crate) engine: Option<PathBuf>,
    pub(crate) database: Option<PathBuf>,
    /// Directory of the engine's own databases (system install or the macOS
    /// bundle), listed in the database picker.
    database_dir: Option<PathBuf>,
    pub(crate) current_file: Option<PathBuf>,
    /// Serialized document as of the last New/Open/Save; "dirty" is simply
    /// "differs from this", so no form needs to report its own edits.
    saved_snapshot: String,
    pub(crate) pending: Option<Pending>,
    allow_close: bool,
    window_title: String,
    pub(crate) output_text: String,
    /// Bumped whenever `output_text` changes; keys the find/layout caches.
    output_generation: u64,
    find_query: String,
    find_current: usize,
    find_scroll_pending: bool,
    find_cache: FindCache,
    galley_cache: Option<(GalleyKey, Arc<egui::Galley>)>,
    /// The engine run in progress, if any, plus the SELECTED_OUTPUT file name
    /// to load for plotting when it finishes.
    pub(crate) run: Option<(runner::RunHandle, Option<String>)>,
    pub(crate) run_error: Option<String>,
    pub(crate) status: Option<(String, bool)>,
    pub(crate) plot: plotting::PlotState,
    pub(crate) screenshot_frame_count: u32,
    pub(crate) screenshot_requested: bool,
    pub(crate) screenshot_seeded: bool,
    /// The form card's height last frame, used to vertically centre a short
    /// card (one frame stale, imperceptible for a static form).
    last_card_height: f32,
    pub(crate) show_about: bool,
    /// Non-blocking advice about the input, shown under the Run bar.
    pub(crate) run_warning: Option<String>,
    pub(crate) sim_dialog: Option<SimDialog>,
    /// Simulation names of the input being run, for the plot legend.
    run_sim_names: std::collections::BTreeMap<i64, String>,
}

/// State of the "Simulations from combinations" dialog while it is open.
pub(crate) struct SimDialog {
    /// Each defined solution and mix: what it is, its label, ticked.
    waters: Vec<((Entity, u32), String, bool)>,
    /// Each reactant type defined in the input, with every choice ("none"
    /// first, then each defined number) and whether it is ticked.
    reactants: Vec<(Entity, Vec<(Option<u32>, String, bool)>)>,
}

impl SimDialog {
    fn plan(&self) -> simulations::Plan {
        simulations::Plan {
            runs: self.waters.iter().filter(|w| w.2).map(|w| w.0).collect(),
            reactants: self
                .reactants
                .iter()
                .map(|(e, choices)| (*e, choices.iter().filter(|c| c.2).map(|c| c.0).collect()))
                .collect(),
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new(Settings::default())
    }
}

impl App {
    pub(crate) fn new(settings: Settings) -> Self {
        let engine = settings
            .engine
            .clone()
            .filter(|p| p.is_file())
            .or_else(phreeqc::find_engine);
        let default_db = phreeqc::find_database(engine.as_deref());
        let database = settings
            .database
            .clone()
            .filter(|p| p.is_file())
            .or_else(|| default_db.clone());
        let doc = Document::default();
        let saved_snapshot = serialize::document_to_pqi(&doc);
        Self {
            doc,
            selected: 0,
            settings,
            engine,
            database,
            database_dir: default_db.and_then(|p| p.parent().map(Path::to_path_buf)),
            current_file: None,
            saved_snapshot,
            pending: None,
            allow_close: false,
            window_title: String::new(),
            output_text: String::new(),
            output_generation: 0,
            find_query: String::new(),
            find_current: 0,
            find_scroll_pending: false,
            find_cache: FindCache::default(),
            galley_cache: None,
            run: None,
            run_error: None,
            status: None,
            plot: {
                let mut plot = plotting::PlotState::default();
                plot.config.series = plotting::read_style_library();
                plot.normalize_styles();
                plot.presets = plotting::read_presets();
                plot
            },
            screenshot_frame_count: 0,
            screenshot_requested: false,
            screenshot_seeded: false,
            last_card_height: 400.0,
            show_about: false,
            run_warning: None,
            sim_dialog: None,
            run_sim_names: Default::default(),
        }
    }

    pub(crate) fn is_dirty(&self) -> bool {
        serialize::document_to_pqi(&self.doc) != self.saved_snapshot || self.plot.touched
    }

    fn mark_clean(&mut self) {
        self.saved_snapshot = serialize::document_to_pqi(&self.doc);
        self.plot.touched = false;
    }

    fn display_name(&self) -> String {
        self.current_file
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".into())
    }

    fn set_status(&mut self, message: impl Into<String>, is_error: bool) {
        self.status = Some((message.into(), is_error));
    }

    fn persist_settings(&mut self) {
        if let Err(e) = self.settings.save() {
            self.set_status(format!("Could not save settings: {e}"), true);
        }
    }

    fn dialog_dir(&self) -> Option<PathBuf> {
        self.current_file
            .as_ref()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .or_else(|| self.settings.last_dir.clone())
            .filter(|d| d.is_dir())
            .or_else(crate::settings::home_dir)
    }

    // ---- document lifecycle ----

    fn request(&mut self, ctx: &egui::Context, action: Pending) {
        if self.is_dirty() {
            self.pending = Some(action);
        } else {
            self.perform(ctx, action);
        }
    }

    fn perform(&mut self, ctx: &egui::Context, action: Pending) {
        match action {
            Pending::New => self.new_document(),
            Pending::Open => {
                let mut dialog = rfd::FileDialog::new()
                    .set_title("Open PHREEQC input")
                    .add_filter("PHREEQC input", &INPUT_EXTENSIONS)
                    .add_filter("All files", &["*"]);
                if let Some(dir) = self.dialog_dir() {
                    dialog = dialog.set_directory(dir);
                }
                if let Some(path) = dialog.pick_file() {
                    self.open_path(&path);
                }
            }
            Pending::OpenPath(path) => self.open_path(&path),
            Pending::OpenExample(index) => self.load_example(index),
            Pending::Quit => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn new_document(&mut self) {
        self.load_document(Document::default(), None);
        self.set_status("New document", false);
    }

    fn load_document(&mut self, doc: Document, path: Option<PathBuf>) {
        self.doc = doc;
        self.selected = 0;
        self.current_file = path;
        self.set_output(String::new());
        self.run_error = None;
        self.plot.clear_data();
        self.plot.note = None;
        // The plot settings saved beside the file, else the defaults.
        let settings_file = self.current_file.as_deref().map(plotting::settings_path).filter(|p| p.is_file());
        match settings_file.map(|p| (plotting::read_settings(&p), p)) {
            Some((Ok(config), _)) => self.plot.load_config(config, true),
            Some((Err(e), p)) => {
                self.plot.load_config(plotting::PlotConfig::default(), false);
                self.set_status(format!("Could not read the plot settings in {}: {e}", p.display()), true);
            }
            None => self.plot.load_config(plotting::PlotConfig::default(), false),
        }
        self.mark_clean();
    }

    pub(crate) fn open_path(&mut self, path: &Path) {
        match io::load(path) {
            Ok(doc) => {
                self.load_document(doc, Some(path.to_path_buf()));
                self.settings.add_recent(path);
                self.persist_settings();
                self.set_status(format!("Opened {}", path.display()), false);
            }
            Err(e) => {
                self.settings.recent_files.retain(|p| p != path);
                self.persist_settings();
                self.set_status(format!("Could not open {}: {e}", path.display()), true);
            }
        }
    }

    /// Returns true if the document was written.
    fn save(&mut self) -> bool {
        match self.current_file.clone() {
            Some(path) => self.save_to(&path),
            None => self.save_as(),
        }
    }

    fn save_as(&mut self) -> bool {
        let file_name = self
            .current_file
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "untitled.pqi".into());
        let mut dialog = rfd::FileDialog::new()
            .set_title("Save PHREEQC input")
            .add_filter("PHREEQC input", &["pqi"])
            .set_file_name(file_name);
        if let Some(dir) = self.dialog_dir() {
            dialog = dialog.set_directory(dir);
        }
        let Some(mut path) = dialog.save_file() else {
            return false;
        };
        if path.extension().is_none() {
            path.set_extension("pqi");
        }
        self.save_to(&path)
    }

    fn save_to(&mut self, path: &Path) -> bool {
        match io::save(&self.doc, path) {
            Ok(()) => {
                self.current_file = Some(path.to_path_buf());
                // Plot settings go in a file beside the input, once there
                // are any; the input itself stays plain PHREEQC.
                let settings_file = plotting::settings_path(path);
                let mut message = format!("Saved {}", path.display());
                if self.plot.touched || self.plot.has_settings_file || settings_file.is_file() {
                    match plotting::write_settings(&settings_file, &self.plot.saved_config()) {
                        Ok(()) => {
                            self.plot.has_settings_file = true;
                            message.push_str(" and its plot settings");
                        }
                        Err(e) => {
                            self.mark_clean();
                            self.plot.touched = true;
                            self.set_status(
                                format!("Saved {}, but not the plot settings: {e}", path.display()),
                                true,
                            );
                            return false;
                        }
                    }
                }
                self.mark_clean();
                self.settings.add_recent(path);
                self.persist_settings();
                self.set_status(message, false);
                true
            }
            Err(e) => {
                self.set_status(format!("Could not save {}: {e}", path.display()), true);
                false
            }
        }
    }

    fn delete_selected_block(&mut self) {
        if self.selected < self.doc.blocks.len() {
            self.doc.blocks.remove(self.selected);
            if self.selected >= self.doc.blocks.len() && self.selected > 0 {
                self.selected -= 1;
            }
        }
    }

    /// New blocks go right after the selected one, so they land in the
    /// selected simulation; with an END selected, just before it.
    /// Definitions (anything but USE/END) never go into the generated USE
    /// groups: with one of those selected they join the end of the
    /// simulation before them, where the solutions are defined.
    pub(crate) fn insert_block(&mut self, block: Block) {
        let generated = simulations::generated_start(&self.doc.blocks);
        let is_definition = !matches!(block, Block::Use(_)) && !simulations::is_end(&block);
        let at = match self.doc.blocks.get(self.selected) {
            None => self.doc.blocks.len(),
            Some(_) if is_definition && generated > 0 && self.selected >= generated => {
                if simulations::is_end(&self.doc.blocks[generated - 1]) { generated - 1 } else { generated }
            }
            Some(b) if simulations::is_end(b) => self.selected,
            Some(_) => self.selected + 1,
        };
        self.doc.blocks.insert(at, block);
        self.selected = at;
    }

    fn move_selected(&mut self, up: bool) {
        let i = self.selected;
        let j = if up { i.checked_sub(1) } else { Some(i + 1) };
        if let Some(j) = j.filter(|&j| j < self.doc.blocks.len()) {
            self.doc.blocks.swap(i, j);
            self.selected = j;
        }
    }

    pub(crate) fn open_sim_dialog(&mut self) {
        let blocks = &self.doc.blocks;
        let plan = simulations::current_plan(blocks);
        let waters: Vec<((Entity, u32), String, bool)> = [Entity::Solution, Entity::Mix]
            .into_iter()
            .flat_map(|e| simulations::defined_numbers(blocks, e).into_iter().map(move |n| (e, n)))
            .map(|water| {
                let (e, n) = water;
                let name = simulations::entity_name(blocks, e, n);
                let label = if name.starts_with(e.label()) { name } else { format!("{} {n} — {name}", e.label()) };
                (water, label, plan.runs.contains(&water))
            })
            .collect();
        if waters.is_empty() {
            self.set_status("Add a SOLUTION block first", true);
            return;
        }
        let reactants = plan
            .reactants
            .iter()
            .filter_map(|(entity, chosen)| {
                let numbers = simulations::defined_numbers(blocks, *entity);
                if numbers.is_empty() {
                    return None;
                }
                let mut choices = vec![(None, "none".to_string(), chosen.is_empty() || chosen.contains(&None))];
                for n in numbers {
                    let name = simulations::entity_name(blocks, *entity, n);
                    let label = if name.starts_with(entity.label()) { n.to_string() } else { format!("{n} — {name}") };
                    choices.push((Some(n), label, chosen.contains(&Some(n))));
                }
                Some((*entity, choices))
            })
            .collect();
        self.sim_dialog = Some(SimDialog { waters, reactants });
    }

    pub(crate) fn apply_sim_plan(&mut self, plan: &simulations::Plan) {
        let applied = simulations::apply(&mut self.doc.blocks, plan);
        self.selected = applied.first_new.min(self.doc.blocks.len().saturating_sub(1));
        let n = if plan.runs.is_empty() { 0 } else { plan.simulation_count() };
        let mut status = match n {
            0 => "Removed the generated simulations".to_string(),
            1 => "Added 1 simulation".to_string(),
            n => format!("Added {n} simulations"),
        };
        if !applied.neutralised.is_empty() {
            status.push_str(&format!(
                "; changed {} before the first END to “none” so that water doesn't run twice",
                applied.neutralised.join(", ")
            ));
        }
        self.set_status(status, false);
    }

    // ---- engine and database ----

    fn choose_database(&mut self, path: PathBuf) {
        self.set_status(format!("Database: {}", path.display()), false);
        self.settings.database = Some(path.clone());
        self.database = Some(path);
        self.persist_settings();
    }

    fn browse_database(&mut self) {
        let mut dialog = rfd::FileDialog::new()
            .set_title("Choose a PHREEQC database")
            .add_filter("PHREEQC database", &["dat"])
            .add_filter("All files", &["*"]);
        if let Some(dir) = self.database.as_ref().and_then(|p| p.parent()) {
            dialog = dialog.set_directory(dir);
        }
        if let Some(path) = dialog.pick_file() {
            self.choose_database(path);
        }
    }

    fn locate_engine(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Locate the phreeqc program")
            .pick_file()
        else {
            return;
        };
        self.settings.engine = Some(path.clone());
        let default_db = phreeqc::find_database(Some(&path));
        self.database_dir = default_db.as_ref().and_then(|p| p.parent().map(Path::to_path_buf));
        if self.database.is_none() {
            self.database = default_db;
        }
        self.engine = Some(path);
        self.persist_settings();
        self.set_status("Engine set", false);
    }

    fn database_choices(&self) -> Vec<PathBuf> {
        let mut choices = self
            .database_dir
            .as_deref()
            .map(phreeqc::list_databases)
            .unwrap_or_default();
        if let Some(current) = &self.database {
            if !choices.contains(current) {
                choices.push(current.clone());
            }
        }
        choices
    }

    pub(crate) fn is_running(&self) -> bool {
        self.run.is_some()
    }

    pub(crate) fn run_simulation(&mut self) {
        if self.is_running() {
            return;
        }
        let Some(engine) = self.engine.clone() else {
            self.run_error = Some(
                "The phreeqc engine was not found. Install PHREEQC (see the README), \
                 or use \"Locate phreeqc…\" next to the Run button."
                    .into(),
            );
            return;
        };
        let Some(database) = self.database.clone() else {
            self.run_error =
                Some("No database selected. Choose one in the Database list next to Run.".into());
            return;
        };
        let problems = validate::problems(&self.doc);
        if !problems.is_empty() {
            let shown: Vec<&str> = problems.iter().take(6).map(String::as_str).collect();
            let more = problems.len().saturating_sub(shown.len());
            let mut message = format!("Fix these fields before running:\n{}", shown.join("\n"));
            if more > 0 {
                message.push_str(&format!("\n…and {more} more"));
            }
            self.run_error = Some(message);
            self.set_status("Not run: some fields are not numbers", true);
            return;
        }
        let unreacted = simulations::unreacted_solutions(&self.doc.blocks);
        self.run_warning = match unreacted.len() {
            0 => None,
            1 => Some(format!(
                "Solution {} is defined but never reacted: PHREEQC reacts one solution per simulation. \
                 To run each solution, use Run → Simulations from combinations…",
                unreacted[0]
            )),
            _ => Some(format!(
                "Solutions {} are defined but never reacted: PHREEQC reacts one solution per simulation. \
                 To run each solution, use Run → Simulations from combinations…",
                simulations::list_numbers(&unreacted)
            )),
        };
        self.run_sim_names = simulations::simulation_names(&self.doc.blocks)
            .into_iter()
            .enumerate()
            .filter_map(|(i, name)| Some((i as i64 + 1, name?)))
            .collect();
        let selected_output = selected_output_file(&self.doc);
        let pqi_body = serialize::document_to_pqi(&self.doc);
        self.run_error = None;
        self.set_status("Running…", false);
        let handle = runner::start(
            &engine,
            &pqi_body,
            &database,
            &phreeqc::default_work_dir(),
            runner::Limits::default(),
        );
        self.run = Some((handle, selected_output));
    }

    fn cancel_run(&mut self) {
        if let Some((handle, _)) = &self.run {
            handle.cancel();
        }
    }

    /// Picks up a finished run; call once per frame.
    pub(crate) fn poll_run(&mut self, ctx: &egui::Context) {
        let Some((handle, _)) = &self.run else {
            return;
        };
        let Some(outcome) = handle.poll() else {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            return;
        };
        let (handle, selected_output) = self.run.take().expect("checked above");
        let seconds = handle.elapsed().as_secs_f32();
        self.plot.clear_data();
        self.plot.note = None;
        match outcome {
            runner::Outcome::Finished { success, output_text, stderr_text } => {
                self.set_output(output_text);
                if success {
                    self.run_error = None;
                    self.set_status(format!("Run finished in {seconds:.1} s"), false);
                    self.plot.note = match &selected_output {
                        None => Some(
                            "No Selected Output block in this input, so there is nothing to plot. \
                             Add one with Insert → Selected Output and run again."
                                .to_string(),
                        ),
                        Some(name) => match plotting::load_selected_output(&handle.work_dir.join(name)) {
                            Err(_) => Some(format!(
                                "The Selected Output file “{name}” was not written by this run. Check that the \
                                 Selected Output block is in the first simulation (before the first END)."
                            )),
                            Ok(table) if table.rows.is_empty() => {
                                Some("The Selected Output table has no rows for this run.".to_string())
                            }
                            Ok(_) => None,
                        },
                    };
                    if let Some(name) = selected_output {
                        if let Ok(table) = plotting::load_selected_output(&handle.work_dir.join(name)) {
                            self.plot.config.document_title = self.doc.title.trim().to_string();
                            let names = std::mem::take(&mut self.run_sim_names);
                            if self.plot.config.sim_names != names {
                                // Hidden simulations belong to the old set,
                                // unless they came with the saved settings.
                                if !self.plot.config.sim_names.is_empty() {
                                    self.plot.config.hidden_sims.clear();
                                }
                                self.plot.config.sim_names = names;
                            }
                            self.plot.set_table(table);
                            if !self.plot.window_dismissed {
                                self.plot.window_open = true;
                            }
                        }
                    }
                } else {
                    self.run_error = Some(phreeqc::summarize_errors(&stderr_text));
                    self.set_status("Run failed — see the error above the output", true);
                }
            }
            runner::Outcome::Cancelled => self.set_status("Run cancelled", true),
            runner::Outcome::TimedOut(limit) => {
                self.run_error = Some(format!(
                    "The run was stopped after {} minutes without finishing.",
                    limit.as_secs() / 60
                ));
                self.set_status("Run timed out", true);
            }
            runner::Outcome::MemoryExceeded(bytes) => {
                self.run_error = Some(format!(
                    "The run was stopped because phreeqc used {:.1} GB of memory (limit {:.0} GB). \
                     This usually means the input makes the engine run away; check the last blocks you changed.",
                    bytes as f64 / 1e9,
                    runner::DEFAULT_MEMORY_LIMIT as f64 / (1u64 << 30) as f64
                ));
                self.set_status("Run stopped: too much memory", true);
            }
            runner::Outcome::Failed(message) => {
                self.run_error = Some(message);
                self.set_status("Run failed", true);
            }
        }
    }

    // ---- per-frame UI ----

    fn handle_window_events(&mut self, ctx: &egui::Context) {
        if let Some(rect) = ctx.input(|i| i.viewport().inner_rect) {
            self.settings.window_size = Some([rect.width(), rect.height()]);
        }
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close && self.is_dirty() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(Pending::Quit);
        }
        let title = format!(
            "{}{} — {}",
            if self.is_dirty() { "*" } else { "" },
            self.display_name(),
            phreeqc::APP_NAME
        );
        if title != self.window_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if self.pending.is_some() || self.sim_dialog.is_some() {
            return;
        }
        let pressed = |s: &KeyboardShortcut| ctx.input_mut(|i| i.consume_shortcut(s));
        // Most specific first: Cmd+Shift+S must not also fire Cmd+S.
        if pressed(&SHORTCUT_SAVE_AS) {
            self.save_as();
        } else if pressed(&SHORTCUT_SAVE) {
            self.save();
        }
        if pressed(&SHORTCUT_NEW) {
            self.request(ctx, Pending::New);
        }
        if pressed(&SHORTCUT_OPEN) {
            self.request(ctx, Pending::Open);
        }
        if pressed(&SHORTCUT_RUN) || pressed(&SHORTCUT_RUN_F5) {
            self.run_simulation();
        }
        if pressed(&SHORTCUT_QUIT) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if !ctx.egui_wants_keyboard_input() {
            if pressed(&SHORTCUT_DELETE) {
                self.delete_selected_block();
            }
            if pressed(&SHORTCUT_MOVE_UP) {
                self.move_selected(true);
            }
            if pressed(&SHORTCUT_MOVE_DOWN) {
                self.move_selected(false);
            }
        }
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let sc = |s: &KeyboardShortcut| ctx.format_shortcut(s);
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.add(egui::Button::new("New").shortcut_text(sc(&SHORTCUT_NEW))).clicked() {
                    self.request(ctx, Pending::New);
                }
                if ui.add(egui::Button::new("Open…").shortcut_text(sc(&SHORTCUT_OPEN))).clicked() {
                    self.request(ctx, Pending::Open);
                }
                ui.menu_button("Open Recent", |ui| {
                    if self.settings.recent_files.is_empty() {
                        ui.add_enabled(false, egui::Button::new("No recent files"));
                    }
                    let mut chosen = None;
                    for path in &self.settings.recent_files {
                        let label = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                        if ui.button(label).on_hover_text(path.display().to_string()).clicked() {
                            chosen = Some(path.clone());
                        }
                    }
                    if let Some(path) = chosen {
                        self.request(ctx, Pending::OpenPath(path));
                    }
                    if !self.settings.recent_files.is_empty() {
                        ui.separator();
                        if ui.button("Clear list").clicked() {
                            self.settings.recent_files.clear();
                            self.persist_settings();
                        }
                    }
                });
                ui.separator();
                if ui.add(egui::Button::new("Save").shortcut_text(sc(&SHORTCUT_SAVE))).clicked() {
                    self.save();
                }
                if ui
                    .add(egui::Button::new("Save As…").shortcut_text(sc(&SHORTCUT_SAVE_AS)))
                    .clicked()
                {
                    self.save_as();
                }
                ui.separator();
                if ui.add(egui::Button::new("Quit").shortcut_text(sc(&SHORTCUT_QUIT))).clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Edit", |ui| {
                let can_delete = self.selected < self.doc.blocks.len();
                if ui
                    .add_enabled(
                        can_delete,
                        egui::Button::new("Delete block").shortcut_text(sc(&SHORTCUT_DELETE)),
                    )
                    .clicked()
                {
                    self.delete_selected_block();
                }
                ui.separator();
                let len = self.doc.blocks.len();
                if ui
                    .add_enabled(
                        self.selected > 0 && self.selected < len,
                        egui::Button::new("Move block up").shortcut_text(sc(&SHORTCUT_MOVE_UP)),
                    )
                    .clicked()
                {
                    self.move_selected(true);
                }
                if ui
                    .add_enabled(
                        self.selected + 1 < len,
                        egui::Button::new("Move block down").shortcut_text(sc(&SHORTCUT_MOVE_DOWN)),
                    )
                    .clicked()
                {
                    self.move_selected(false);
                }
            });
            ui.menu_button("Insert", |ui| {
                for kind in BLOCK_KINDS {
                    if ui.button(format!("{kind}")).clicked() {
                        let number = self.doc.next_number_for(kind);
                        if let Some(b) = Block::new_default(kind, number) {
                            self.insert_block(b);
                        }
                    }
                }
                ui.separator();
                if ui
                    .button("End (start a new simulation)")
                    .on_hover_text("END: blocks after it form the next simulation")
                    .clicked()
                {
                    self.insert_block(simulations::end_block());
                }
                if ui
                    .button("Simulations from combinations…")
                    .on_hover_text("USE + END groups that run each solution or mix through each combination of reactants")
                    .clicked()
                {
                    self.open_sim_dialog();
                }
                ui.separator();
                ui.menu_button("Database Definitions", |ui| {
                    for (label, keyword, template) in DATABASE_KEYWORDS {
                        if ui.button(*label).on_hover_text(*keyword).clicked() {
                            self.insert_block(Block::Raw(Raw {
                                header_line: keyword.to_string(),
                                body: template.to_string(),
                            }));
                        }
                    }
                });
            });
            ui.menu_button("Run", |ui| {
                let running = self.is_running();
                if ui
                    .add_enabled(!running, egui::Button::new("Run").shortcut_text(sc(&SHORTCUT_RUN)))
                    .clicked()
                {
                    self.run_simulation();
                }
                if ui.add_enabled(running, egui::Button::new("Cancel run")).clicked() {
                    self.cancel_run();
                }
                ui.separator();
                if ui.button("Simulations from combinations…").clicked() {
                    self.open_sim_dialog();
                }
            });
            ui.menu_button("Help", |ui| {
                ui.menu_button("Open Example", |ui| {
                    for (index, (name, _)) in EXAMPLES.iter().enumerate() {
                        if ui.button(*name).clicked() {
                            self.request(ctx, Pending::OpenExample(index));
                        }
                    }
                });
                ui.separator();
                if ui.button("PHREEQC documentation (USGS)").on_hover_text(PHREEQC_SOFTWARE_PAGE).clicked() {
                    ctx.open_url(egui::OpenUrl::new_tab(PHREEQC_SOFTWARE_PAGE));
                }
                if ui.button("PHREEQC 3 manual (USGS)").on_hover_text(PHREEQC_MANUAL_PAGE).clicked() {
                    ctx.open_url(egui::OpenUrl::new_tab(PHREEQC_MANUAL_PAGE));
                }
                ui.separator();
                if ui.button("About PQ Studio").clicked() {
                    self.show_about = true;
                }
            });
            if let Some((status, is_error)) = &self.status {
                let color = if *is_error { theme::ERROR } else { theme::ACCENT_BRIGHT };
                ui.colored_label(color, status);
            }
        });
    }

    fn block_tree(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(self.display_name()).strong());
            if self.is_dirty() {
                ui.label(egui::RichText::new("● unsaved").color(theme::WARNING).small())
                    .on_hover_text("This document has changes that are not saved yet");
            }
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Title:");
            ui.text_edit_singleline(&mut self.doc.title);
        });
        ui.add_space(6.0);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let mut remove_idx: Option<usize> = None;
            let mut remove_sim: Option<std::ops::Range<usize>> = None;
            let names = simulations::simulation_names(&self.doc.blocks);
            for (k, range) in simulations::split(&self.doc.blocks).into_iter().enumerate() {
                let mut title = format!("Simulation {}", k + 1);
                if let Some(Some(name)) = names.get(k) {
                    title.push_str(&format!(" · {name}"));
                }
                let header = egui::CollapsingHeader::new(title)
                    .id_salt(("simulation", k))
                    .default_open(true)
                    .show(ui, |ui| {
                        for i in range.clone() {
                            let block = &self.doc.blocks[i];
                            ui.horizontal(|ui| {
                                if ui.selectable_label(self.selected == i, block.label()).clicked() {
                                    self.selected = i;
                                }
                                if ui.small_button("✕").on_hover_text("Delete block").clicked() {
                                    remove_idx = Some(i);
                                }
                            });
                        }
                    });
                header.header_response.on_hover_text("Right-click to delete this simulation").context_menu(|ui| {
                    if ui.button("Delete this simulation").clicked() {
                        remove_sim = Some(range.clone());
                    }
                });
            }
            if let Some(i) = remove_idx {
                self.selected = i;
                self.delete_selected_block();
            }
            if let Some(range) = remove_sim {
                self.doc.blocks.drain(range.clone());
                self.selected = range.start.min(self.doc.blocks.len().saturating_sub(1));
            }
        });
    }

    fn run_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some((handle, _)) = &self.run {
                let elapsed = handle.elapsed().as_secs();
                ui.add(egui::Spinner::new());
                ui.label(format!("Running… {elapsed} s"));
                if ui.button("■ Cancel").on_hover_text("Stop the phreeqc engine").clicked() {
                    self.cancel_run();
                }
            } else if ui.button("▶ Run").on_hover_text("Run the simulation (Ctrl+R or F5)").clicked() {
                self.run_simulation();
            }
            ui.separator();
            if self.engine.is_none() {
                ui.colored_label(theme::ERROR, "phreeqc engine not found");
                if ui.button("Locate phreeqc…").clicked() {
                    self.locate_engine();
                }
                return;
            }
            ui.label("Database:");
            let selected_name = self
                .database
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "none found".into());
            let mut chosen = None;
            let mut browse = false;
            let combo = egui::ComboBox::from_id_salt("database_picker")
                .selected_text(selected_name)
                .width(200.0)
                .show_ui(ui, |ui| {
                    for path in self.database_choices() {
                        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                        let is_current = self.database.as_ref() == Some(&path);
                        if ui
                            .selectable_label(is_current, name)
                            .on_hover_text(path.display().to_string())
                            .clicked()
                        {
                            chosen = Some(path);
                        }
                    }
                    ui.separator();
                    if ui.button("Browse…").clicked() {
                        browse = true;
                    }
                });
            if let Some(db) = &self.database {
                combo.response.on_hover_text(db.display().to_string());
            }
            if let Some(path) = chosen {
                self.choose_database(path);
            }
            if browse {
                self.browse_database();
            }
        });
    }

    /// QA hook: act on a queued action without the prompt or a context.
    pub(crate) fn resolve_pending_for_screenshot(&mut self) {
        if let Some(Pending::OpenExample(index)) = self.pending.take() {
            self.load_example(index.min(EXAMPLES.len() - 1));
        }
    }

    fn load_example(&mut self, index: usize) {
        let (name, text) = EXAMPLES[index];
        self.load_document(io::parse_document(text), None);
        self.set_status(format!("Example loaded: {name} (Save As to keep a copy)"), false);
    }

    pub(crate) fn set_find_query(&mut self, query: &str) {
        self.find_query = query.to_string();
    }

    fn set_output(&mut self, text: String) {
        self.output_text = text;
        self.output_generation += 1;
        self.find_current = 0;
    }

    fn save_output_as(&mut self) {
        let name = self
            .current_file
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| format!("{}.pqo", s.to_string_lossy()))
            .unwrap_or_else(|| "output.pqo".into());
        let mut dialog = rfd::FileDialog::new()
            .set_title("Save engine output")
            .add_filter("PHREEQC output", &["pqo", "txt"])
            .set_file_name(name);
        if let Some(dir) = self.dialog_dir() {
            dialog = dialog.set_directory(dir);
        }
        if let Some(path) = dialog.save_file() {
            match std::fs::write(&path, &self.output_text) {
                Ok(()) => self.set_status(format!("Output saved to {}", path.display()), false),
                Err(e) => self.set_status(format!("Could not save output: {e}"), true),
            }
        }
    }

    fn export_plot(&mut self) {
        let mut dialog = rfd::FileDialog::new()
            .set_title("Export plot")
            .add_filter("PNG image", &["png"])
            .add_filter("SVG drawing", &["svg"])
            .set_file_name("plot.png");
        if let Some(dir) = self.dialog_dir() {
            dialog = dialog.set_directory(dir);
        }
        let Some(mut path) = dialog.save_file() else {
            return;
        };
        if path.extension().is_none() {
            path.set_extension("png");
        }
        match plotting::export(&self.plot, &path) {
            Ok(()) => self.set_status(format!("Plot exported to {}", path.display()), false),
            Err(e) => self.set_status(format!("Could not export plot: {e}"), true),
        }
    }

    fn export_table(&mut self) {
        let Some(table) = &self.plot.table else {
            return;
        };
        let stem = self
            .current_file
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "results".into());
        let mut dialog = rfd::FileDialog::new()
            .set_title("Export results table")
            .add_filter("CSV (spreadsheet)", &["csv"])
            .set_file_name(format!("{stem}.csv"));
        if let Some(dir) = self.dialog_dir() {
            dialog = dialog.set_directory(dir);
        }
        let Some(mut path) = dialog.save_file() else {
            return;
        };
        if path.extension().is_none() {
            path.set_extension("csv");
        }
        let text = plotting::table_text(table, &self.plot.config.sim_names, ',');
        match std::fs::write(&path, text) {
            Ok(()) => self.set_status(format!("Table exported to {}", path.display()), false),
            Err(e) => self.set_status(format!("Could not export table: {e}"), true),
        }
    }

    /// Saves what the chart shows (X, Y columns, simulations shown) as an
    /// Excel workbook or CSV, for plotting it elsewhere.
    fn export_data(&mut self) {
        let Some(table) = &self.plot.table else {
            return;
        };
        if self.plot.y_cols.is_empty() {
            self.set_status("Tick one or more Y series first", true);
            return;
        }
        let data = plotting::plotted_data(table, self.plot.x_col, &self.plot.y_cols, &self.plot.config);
        let stem = self
            .current_file
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "results".into());
        let mut dialog = rfd::FileDialog::new()
            .set_title("Export plotted data")
            .add_filter("Excel workbook", &["xlsx"])
            .add_filter("CSV (any spreadsheet)", &["csv"])
            .set_file_name(format!("{stem}-plot-data.xlsx"));
        if let Some(dir) = self.dialog_dir() {
            dialog = dialog.set_directory(dir);
        }
        let Some(mut path) = dialog.save_file() else {
            return;
        };
        if path.extension().is_none() {
            path.set_extension("xlsx");
        }
        let csv = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("csv"));
        let result = if csv {
            std::fs::write(&path, plotting::plotted_csv(&data)).map_err(|e| e.into())
        } else {
            plotting::write_plotted_xlsx(&data, &path)
        };
        match result {
            Ok(()) => self.set_status(format!("Plotted data exported to {}", path.display()), false),
            Err(e) => self.set_status(format!("Could not export the plotted data: {e}"), true),
        }
    }

    fn output_view(&mut self, ui: &mut egui::Ui, height: f32) {
        let key = (self.output_generation, self.find_query.clone());
        if self.find_cache.key != key {
            self.find_cache = FindCache {
                matches: find_matches(&self.output_text, &self.find_query),
                key,
            };
            self.find_current = 0;
            self.find_scroll_pending = true;
        }
        let match_count = self.find_cache.matches.len();

        let mut toolbar_height = 0.0;
        let toolbar = ui.horizontal(|ui| {
            ui.label("Find:");
            let find = ui.add(
                egui::TextEdit::singleline(&mut self.find_query)
                    .desired_width(160.0)
                    .hint_text("text in output"),
            );
            let enter = find.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
            if !self.find_query.is_empty() {
                if match_count == 0 {
                    ui.colored_label(theme::TEXT_SECONDARY, "no matches");
                } else {
                    ui.label(format!("{} of {match_count}", self.find_current + 1));
                }
            }
            let has = match_count > 0;
            if ui.add_enabled(has, egui::Button::new("◀")).on_hover_text("Previous match").clicked() {
                self.find_current = (self.find_current + match_count - 1) % match_count;
                self.find_scroll_pending = true;
            }
            if ui.add_enabled(has, egui::Button::new("▶")).on_hover_text("Next match (Enter)").clicked()
                || (enter && has)
            {
                self.find_current = (self.find_current + 1) % match_count;
                self.find_scroll_pending = true;
                if enter {
                    find.request_focus();
                }
            }
            ui.separator();
            let has_output = !self.output_text.is_empty();
            if ui.add_enabled(has_output, egui::Button::new("Copy")).on_hover_text("Copy all output").clicked() {
                ui.ctx().copy_text(self.output_text.clone());
                self.set_status("Output copied to the clipboard", false);
            }
            if ui.add_enabled(has_output, egui::Button::new("Save output as…")).clicked() {
                self.save_output_as();
            }
        });
        toolbar_height += toolbar.response.rect.height() + ui.spacing().item_spacing.y;

        let row_height = ui.fonts_mut(|f| f.row_height(&egui::FontId::monospace(12.5)));
        let mut scroll = egui::ScrollArea::both()
            .id_salt("output_text_scroll")
            .max_height((height - toolbar_height).max(80.0))
            .auto_shrink([false, false]);
        if self.find_scroll_pending && match_count > 0 {
            let line = self.find_cache.matches[self.find_current].1;
            scroll = scroll.vertical_scroll_offset(((line as f32) - 3.0).max(0.0) * row_height);
            self.find_scroll_pending = false;
        }
        let view_height = (height - toolbar_height).max(80.0);
        scroll.show(ui, |ui| {
            let galley_key = GalleyKey {
                generation: self.output_generation,
                query: self.find_query.clone(),
                current: self.find_current,
            };
            let matches = &self.find_cache.matches;
            let len = self.find_query.len();
            let cache = &mut self.galley_cache;
            let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, _wrap: f32| {
                if let Some((k, g)) = cache.as_ref() {
                    if *k == galley_key {
                        return g.clone();
                    }
                }
                let job = highlighted_job(buf.as_str(), matches, len, galley_key.current);
                let galley = ui.fonts_mut(|f| f.layout_job(job));
                *cache = Some((galley_key.clone(), galley.clone()));
                galley
            };
            let mut text: &str = &self.output_text;
            ui.add(
                egui::TextEdit::multiline(&mut text)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .min_size(egui::vec2(ui.available_width(), view_height))
                    .layouter(&mut layouter),
            );
        });
    }

    fn output_panel(&mut self, ui: &mut egui::Ui) {
        self.run_bar(ui);
        if let Some(err) = &self.run_error {
            ui.colored_label(theme::ERROR, err);
        }
        if let Some(warning) = &self.run_warning {
            ui.colored_label(theme::WARNING, warning);
        }
        // `horizontal_top` shrink-wraps its children, so the row height is
        // captured here and passed down explicitly.
        let row_height = ui.available_height();
        ui.horizontal_top(|ui| {
            let col_width = ui.available_width();
            ui.allocate_ui_with_layout(
                egui::vec2(col_width * 0.55, row_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| self.output_view(ui, row_height),
            );
            ui.separator();
            let mut export = plotting::PlotAction::None;
            // Explicit top-down column: inside `horizontal_top` the plot
            // controls and chart would otherwise be laid out left to right.
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), row_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("plot_scroll")
                        .max_height(row_height)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            export = crate::plot_ui::panel(ui, &mut self.plot, row_height - 40.0);
                        });
                },
            );
            match export {
                plotting::PlotAction::ExportPlot => self.export_plot(),
                plotting::PlotAction::ExportTable => self.export_table(),
                plotting::PlotAction::ExportData => self.export_data(),
                plotting::PlotAction::None => {}
            }
        });
    }

    fn form_card(&mut self, ui: &mut egui::Ui) {
        let avail_height = ui.available_height();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if self.last_card_height < avail_height {
                    ui.add_space((avail_height - self.last_card_height) / 2.0);
                }
                let card = egui::Frame::new()
                    .fill(theme::BG_PANEL)
                    .stroke(egui::Stroke::new(1.0, theme::STROKE_SUBTLE))
                    .corner_radius(egui::CornerRadius::same(6))
                    .inner_margin(egui::Margin::same(16))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        if let Some(block) = self.doc.blocks.get_mut(self.selected) {
                            forms::block_ui(ui, block);
                        } else {
                            ui.label("No blocks yet. Add one from the Insert menu.");
                        }
                    });
                self.last_card_height = card.response.rect.height();
            });
    }

    fn unsaved_prompt(&mut self, ctx: &egui::Context) {
        let Some(action) = self.pending.clone() else {
            return;
        };
        let verb = match action {
            Pending::Quit => "closing",
            Pending::New => "starting a new document",
            Pending::Open | Pending::OpenPath(_) | Pending::OpenExample(_) => "opening another file",
        };
        let mut choice: Option<&str> = None;
        let modal = egui::Modal::new(egui::Id::unique("unsaved_changes")).show(ctx, |ui| {
            ui.set_width(380.0);
            ui.heading("Unsaved changes");
            ui.add_space(6.0);
            ui.label(format!("Save changes to “{}” before {verb}?", self.display_name()));
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    choice = Some("save");
                }
                if ui.button("Don't save").clicked() {
                    choice = Some("discard");
                }
                if ui.button("Cancel").clicked() {
                    choice = Some("cancel");
                }
            });
        });
        if choice.is_none() && modal.should_close() {
            choice = Some("cancel");
        }
        match choice {
            Some("save") => {
                if self.save() {
                    self.pending = None;
                    self.perform(ctx, action);
                }
            }
            Some("discard") => {
                self.pending = None;
                self.perform(ctx, action);
            }
            Some(_) => self.pending = None,
            None => {}
        }
    }

    fn sim_dialog_window(&mut self, ctx: &egui::Context) {
        let Some(dialog) = &mut self.sim_dialog else {
            return;
        };
        let mut chosen: Option<simulations::Plan> = None;
        let mut cancel = false;
        let modal = egui::Modal::new(egui::Id::unique("sim_combinations")).show(ctx, |ui| {
            ui.set_width(560.0);
            ui.heading("Simulations from combinations");
            ui.add_space(6.0);
            ui.label(
                "PHREEQC reacts one solution or mix per simulation. Tick the waters to run and \
                 the reactant blocks to apply: every ticked water is run with every combination \
                 of the ticked choices. Each run becomes a USE … END group at the end of the \
                 input, as ordinary blocks you can see and edit; using this dialog again \
                 replaces them.",
            );
            ui.add_space(12.0);
            egui::ScrollArea::vertical()
                .id_salt("sim_dialog_scroll")
                .max_height(ctx.content_rect().height() * 0.6)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Waters to run").strong());
                        if ui.small_button("All").clicked() {
                            dialog.waters.iter_mut().for_each(|w| w.2 = true);
                        }
                        if ui.small_button("None").clicked() {
                            dialog.waters.iter_mut().for_each(|w| w.2 = false);
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        for (_, label, ticked) in &mut dialog.waters {
                            ui.checkbox(ticked, label.as_str());
                        }
                    });
                    ui.add_space(12.0);
                    ui.label(egui::RichText::new("Applied to each water").strong());
                    if dialog.reactants.is_empty() {
                        ui.label(
                            "No reactant blocks are defined (temperature, pressure, equilibrium \
                             phases, reaction…), so each simulation only repeats its water's \
                             speciation.",
                        );
                    }
                    ui.label(
                        egui::RichText::new("Tick several choices to run each of them, e.g. none, CO2 and Calcite.")
                            .small()
                            .color(theme::TEXT_SECONDARY),
                    );
                    ui.add_space(4.0);
                    egui::Grid::new("sim_reactants")
                        .num_columns(2)
                        .spacing([16.0, 8.0])
                        .show(ui, |ui| {
                            for (entity, choices) in &mut dialog.reactants {
                                ui.label(entity.label());
                                ui.horizontal_wrapped(|ui| {
                                    for (_, label, ticked) in choices.iter_mut() {
                                        ui.checkbox(ticked, label.as_str());
                                    }
                                });
                                ui.end_row();
                            }
                        });
                });
            ui.add_space(14.0);
            let plan = dialog.plan();
            let count = if plan.runs.is_empty() { 0 } else { plan.simulation_count() };
            ui.horizontal(|ui| {
                let label = match count {
                    0 => "Remove generated simulations".to_string(),
                    1 => "Create 1 simulation".to_string(),
                    n => format!("Create {n} simulations"),
                };
                if ui.button(label).clicked() {
                    chosen = Some(plan);
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
        });
        if let Some(plan) = chosen {
            self.sim_dialog = None;
            self.apply_sim_plan(&plan);
        } else if cancel || modal.should_close() {
            self.sim_dialog = None;
        }
    }

    fn about_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_about;
        egui::Window::new("About")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.heading(phreeqc::APP_NAME);
                ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                ui.add_space(8.0);
                ui.label("A graphical interface for the USGS PHREEQC geochemical engine.");
                ui.label("Not affiliated with or endorsed by the U.S. Geological Survey.");
                ui.add_space(8.0);
                ui.label("All calculations are done by the unmodified USGS phreeqc engine,");
                ui.label("written by David L. Parkhurst and C.A.J. (Tony) Appelo.");
                ui.add_space(8.0);
                ui.label(format!("Engine: {}", path_or_missing(&self.engine)));
                ui.label(format!("Database: {}", path_or_missing(&self.database)));
            });
        self.show_about = open;
    }
}

/// Find matches in the output: byte offset of each match and its line number.
#[derive(Default)]
struct FindCache {
    key: (u64, String),
    matches: Vec<(usize, usize)>,
}

#[derive(PartialEq, Clone)]
struct GalleyKey {
    generation: u64,
    query: String,
    current: usize,
}

fn find_matches(text: &str, query: &str) -> Vec<(usize, usize)> {
    if query.is_empty() {
        return Vec::new();
    }
    // PHREEQC output is ASCII, so ASCII lowercasing keeps byte offsets intact.
    let hay = text.to_ascii_lowercase();
    let needle = query.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut line = 0;
    let mut scanned = 0;
    let mut from = 0;
    while let Some(pos) = hay[from..].find(&needle) {
        let start = from + pos;
        line += hay.as_bytes()[scanned..start].iter().filter(|&&b| b == b'\n').count();
        scanned = start;
        out.push((start, line));
        from = start + needle.len().max(1);
    }
    out
}

fn highlighted_job(text: &str, matches: &[(usize, usize)], len: usize, current: usize) -> egui::text::LayoutJob {
    let font = egui::FontId::monospace(12.5);
    let plain = egui::TextFormat::simple(font.clone(), theme::TEXT_PRIMARY);
    let mut job = egui::text::LayoutJob::default();
    let mut pos = 0;
    for (i, &(start, _)) in matches.iter().enumerate() {
        if start < pos || !text.is_char_boundary(start) || !text.is_char_boundary(start + len) {
            continue;
        }
        job.append(&text[pos..start], 0.0, plain.clone());
        let mut hit = plain.clone();
        hit.background = if i == current { theme::WARNING } else { theme::ACCENT_BASE };
        if i == current {
            hit.color = theme::BG_EXTREME;
        }
        job.append(&text[start..start + len], 0.0, hit);
        pos = start + len;
    }
    job.append(&text[pos..], 0.0, plain);
    job.wrap.max_width = f32::INFINITY;
    job
}

#[cfg(test)]
mod find_tests {
    use super::find_matches;

    #[test]
    fn finds_case_insensitive_with_line_numbers() {
        let text = "Ca 1\nmg 2\nCalcite  CA\n";
        assert_eq!(find_matches(text, "ca"), vec![(0, 0), (10, 2), (19, 2)]);
        assert!(find_matches(text, "").is_empty());
        assert!(find_matches(text, "xyz").is_empty());
    }
}

/// The file the first SELECTED_OUTPUT block writes, whether it opened as a
/// form or was kept as text. Without `-file`, PHREEQC writes `selected_<n>.out`.
fn selected_output_file(doc: &crate::model::Document) -> Option<String> {
    doc.blocks.iter().find_map(|b| match b {
        Block::SelectedOutput(so) if so.file.trim().is_empty() => Some(format!("selected_output_{}.sel", so.number)),
        Block::SelectedOutput(so) => Some(so.file.trim().to_string()),
        Block::Raw(r) => {
            let mut header = r.header_line.split_whitespace();
            if !header.next()?.eq_ignore_ascii_case("SELECTED_OUTPUT") {
                return None;
            }
            let number = header.next().and_then(|n| n.parse::<u32>().ok()).unwrap_or(1);
            let file = r.body.lines().find_map(|line| {
                let code = line.split('#').next().unwrap_or("");
                let mut tokens = code.split_whitespace();
                let identifier = tokens.next()?.trim_start_matches('-').to_lowercase();
                (identifier == "file" || identifier == "filename").then(|| tokens.next().map(str::to_string))?
            });
            Some(file.unwrap_or_else(|| format!("selected_output_{number}.sel")))
        }
        _ => None,
    })
}

#[cfg(test)]
mod selected_output_tests {
    use super::selected_output_file;
    use crate::io::parse_document;

    #[test]
    fn finds_file_in_form_and_text_blocks() {
        let form = parse_document(include_str!("../assets/examples/calcite-co2.pqi"));
        assert_eq!(selected_output_file(&form).as_deref(), Some("calcite-co2.sel"));
        let text = parse_document("SOLUTION 1\nSELECTED_OUTPUT 2\n    -file out.sel\n    -reset false\nEND\n");
        assert_eq!(selected_output_file(&text).as_deref(), Some("out.sel"));
        let default = parse_document("SOLUTION 1\nSELECTED_OUTPUT 3\n    -reset false\nEND\n");
        assert_eq!(selected_output_file(&default).as_deref(), Some("selected_output_3.sel"));
    }
}

#[cfg(test)]
mod insert_tests {
    use super::App;
    use crate::io::parse_document;
    use crate::model::{Block, SelectedOutput};
    use crate::simulations;

    #[test]
    fn definitions_inserted_in_generated_groups_go_to_the_definitions() {
        let mut app = App::default();
        app.doc = parse_document("SOLUTION 1\nSOLUTION 2\nREACTION_TEMPERATURE 1\n    25 50\nEND\n");
        let plan = simulations::current_plan(&app.doc.blocks);
        app.apply_sim_plan(&plan);
        app.selected = app.doc.blocks.len() - 1;
        app.insert_block(Block::SelectedOutput(SelectedOutput::default()));
        let sims = simulations::split(&app.doc.blocks);
        assert_eq!(sims.len(), 3);
        assert!(matches!(app.doc.blocks[app.selected], Block::SelectedOutput(_)));
        assert!(sims[0].contains(&app.selected));
        assert_eq!(simulations::current_plan(&app.doc.blocks), plan);
    }
}

fn path_or_missing(p: &Option<PathBuf>) -> String {
    p.as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "not found".into())
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        crate::maybe_take_screenshot(&ctx, self);
        if std::mem::take(&mut self.plot.styles_changed) {
            if let Err(e) = plotting::write_style_library(&self.plot.config.series) {
                self.set_status(format!("Could not save the plot styles: {e}"), true);
            }
        }
        self.poll_run(&ctx);
        self.handle_window_events(&ctx);
        self.handle_shortcuts(&ctx);

        egui::Panel::top("top_menu_panel")
            .resizable(false)
            .show(ui, |ui| self.menu_bar(ui, &ctx));

        egui::Panel::left("tree_panel")
            .resizable(true)
            .default_size(240.0)
            .show(ui, |ui| self.block_tree(ui));

        // Default the output panel to most of the space the selected form
        // card leaves free; keyed by block index so each block keeps its own
        // remembered size.
        let desired_output_height =
            (ui.available_height() - self.last_card_height - 32.0 - 60.0).clamp(240.0, 640.0);
        egui::Panel::bottom(format!("output_panel_{}", self.selected))
            .resizable(true)
            .default_size(desired_output_height)
            .show(ui, |ui| self.output_panel(ui));

        egui::CentralPanel::default().show(ui, |ui| self.form_card(ui));

        match crate::plot_ui::window(&ctx, &mut self.plot) {
            plotting::PlotAction::ExportPlot => self.export_plot(),
            plotting::PlotAction::ExportTable => self.export_table(),
            plotting::PlotAction::ExportData => self.export_data(),
            plotting::PlotAction::None => {}
        }
        self.about_window(&ctx);
        self.sim_dialog_window(&ctx);
        self.unsaved_prompt(&ctx);
    }

    fn on_exit(&mut self) {
        self.cancel_run();
        let _ = self.settings.save();
    }
}
