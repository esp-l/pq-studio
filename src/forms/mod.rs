pub mod advection;
pub mod dump;
pub mod equilibrium_phases;
pub mod exchange;
pub mod gas_phase;
pub mod incremental_reactions;
pub mod inverse_modeling;
pub mod kinetics;
pub mod knobs;
pub mod mix;
pub mod print;
pub mod raw;
pub mod reaction;
pub mod reaction_pressure;
pub mod reaction_temperature;
pub mod save;
pub mod selected_output;
pub mod solid_solutions;
pub mod solution;
pub mod solution_spread;
pub mod surface;
pub mod transport;
pub mod use_block;

use crate::model::{Block, StepsSpec};

/// A fixed-width single-line text field. `egui::TextEdit::desired_width()` is
/// only a layout *hint* that `egui::Grid` does not reliably honor (confirmed
/// empirically: a Grid-cell TextEdit using `.desired_width()` alone rendered
/// at a much narrower width than requested, clipping real content like
/// "mmol/kgw" down to "mm" — `add_sized`, which allocates the outer rect
/// directly, does not have this problem). Use this instead of
/// `TextEdit::singleline(..).desired_width(..)` for every field inside a
/// Grid/table row across these forms.
pub fn sized_text_edit(ui: &mut egui::Ui, value: &mut String, width: f32) -> egui::Response {
    let h = ui.spacing().interact_size.y;
    ui.add_sized([width, h], egui::TextEdit::singleline(value))
}

/// Outline a field in the error colour, with a tooltip, when its text fails
/// `valid` (see `crate::validate`).
fn mark_invalid(ui: &egui::Ui, response: egui::Response, valid: bool, hint: &str) -> egui::Response {
    if valid {
        return response;
    }
    ui.painter().rect_stroke(
        response.rect.expand(1.0),
        egui::CornerRadius::same(3),
        egui::Stroke::new(1.5, crate::theme::ERROR),
        egui::StrokeKind::Outside,
    );
    response.on_hover_text(hint)
}

/// A free-width numeric field: blank (engine default) or a number.
pub fn numeric_field(ui: &mut egui::Ui, value: &mut String) -> egui::Response {
    let response = ui.text_edit_singleline(value);
    mark_invalid(ui, response, crate::validate::is_number(value), "Must be a number, e.g. 7.5 or 1e-3")
}

/// A numeric field that may be followed by a qualifier, e.g. pH "8.2 CO2(g) -3.5".
pub fn leading_number_field(ui: &mut egui::Ui, value: &mut String) -> egui::Response {
    let response = ui.text_edit_singleline(value);
    let lead = value.split_whitespace().next().unwrap_or("");
    mark_invalid(ui, response, crate::validate::is_number(lead), "Must start with a number, e.g. 8.2 or 8.2 CO2(g) -3.5")
}

/// A fixed-width numeric field for Grid/table cells (see `sized_text_edit`).
pub fn numeric_text_edit(ui: &mut egui::Ui, value: &mut String, width: f32) -> egui::Response {
    let response = sized_text_edit(ui, value, width);
    mark_invalid(ui, response, crate::validate::is_number(value), "Must be a number, e.g. 7.5 or 1e-3")
}

/// A free-width whole-number field (cells, shifts, steps, solution numbers).
pub fn whole_number_field(ui: &mut egui::Ui, value: &mut String) -> egui::Response {
    let response = ui.text_edit_singleline(value);
    mark_invalid(ui, response, crate::validate::is_whole_number(value), "Must be a whole number")
}

/// A fixed-width whole-number field for Grid/table cells.
pub fn whole_number_text_edit(ui: &mut egui::Ui, value: &mut String, width: f32) -> egui::Response {
    let response = sized_text_edit(ui, value, width);
    mark_invalid(ui, response, crate::validate::is_whole_number(value), "Must be a whole number")
}

pub fn block_ui(ui: &mut egui::Ui, block: &mut Block) {
    match block {
        Block::Solution(b) => solution::ui(ui, b),
        Block::EquilibriumPhases(b) => equilibrium_phases::ui(ui, b),
        Block::Reaction(b) => reaction::ui(ui, b),
        Block::ReactionTemperature(b) => reaction_temperature::ui(ui, b),
        Block::ReactionPressure(b) => reaction_pressure::ui(ui, b),
        Block::GasPhase(b) => gas_phase::ui(ui, b),
        Block::Exchange(b) => exchange::ui(ui, b),
        Block::Surface(b) => surface::ui(ui, b),
        Block::Kinetics(b) => kinetics::ui(ui, b),
        Block::SelectedOutput(b) => selected_output::ui(ui, b),
        Block::Mix(b) => mix::ui(ui, b),
        Block::Advection(b) => advection::ui(ui, b),
        Block::Transport(b) => transport::ui(ui, b),
        Block::IncrementalReactions(b) => incremental_reactions::ui(ui, b),
        Block::InverseModeling(b) => inverse_modeling::ui(ui, b),
        Block::SolidSolutions(b) => solid_solutions::ui(ui, b),
        Block::SolutionSpread(b) => solution_spread::ui(ui, b),
        Block::Save(b) => save::ui(ui, b),
        Block::Use(b) => use_block::ui(ui, b),
        Block::Dump(b) => dump::ui(ui, b),
        Block::Knobs(b) => knobs::ui(ui, b),
        Block::Print(b) => print::ui(ui, b),
        Block::Raw(b) => raw::ui(ui, b),
    }
}

/// Shared editor for the REACTION_TEMPERATURE/REACTION_PRESSURE/KINETICS
/// "explicit list of values" vs. "start, end, in N steps" choice.
pub fn steps_spec_ui(ui: &mut egui::Ui, id_salt: &str, unit_label: &str, spec: &mut StepsSpec) {
    let mut is_linear = matches!(spec, StepsSpec::Linear { .. });
    ui.horizontal(|ui| {
        if ui.radio(!is_linear, "Explicit list").clicked() {
            is_linear = false;
        }
        if ui
            .radio(is_linear, "Linear (start, end, N steps)")
            .clicked()
        {
            is_linear = true;
        }
    });

    match (is_linear, &mut *spec) {
        (false, StepsSpec::Explicit(_)) => {}
        (false, _) => *spec = StepsSpec::Explicit(vec![String::new()]),
        (true, StepsSpec::Linear { .. }) => {}
        (true, _) => {
            *spec = StepsSpec::Linear {
                start: String::new(),
                end: String::new(),
                steps: "1".to_string(),
            }
        }
    }

    match spec {
        StepsSpec::Explicit(values) => {
            let mut remove_idx = None;
            for (i, v) in values.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    sized_text_edit(ui, v, 100.0);
                    ui.label(unit_label);
                    if ui.button("✕").clicked() {
                        remove_idx = Some(i);
                    }
                });
            }
            if let Some(i) = remove_idx {
                values.remove(i);
            }
            if ui.button("+ Add value").clicked() {
                values.push(String::new());
            }
        }
        StepsSpec::Linear { start, end, steps } => {
            egui::Grid::new(format!("{id_salt}_linear_grid"))
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label(format!("Start ({unit_label})"));
                    ui.text_edit_singleline(start);
                    ui.end_row();
                    ui.label(format!("End ({unit_label})"));
                    ui.text_edit_singleline(end);
                    ui.end_row();
                    ui.label("Steps");
                    ui.text_edit_singleline(steps);
                    ui.end_row();
                });
        }
    }
}
