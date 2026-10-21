use crate::model::Knobs;

pub fn ui(ui: &mut egui::Ui, k: &mut Knobs) {
    ui.heading("Knobs");
    ui.add_space(4.0);
    ui.label("Numerical-method tuning for the equation solver. Leave blank for PHREEQC defaults.");
    ui.add_space(8.0);
    egui::Grid::new("knobs_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Max iterations");
            ui.text_edit_singleline(&mut k.iterations);
            ui.end_row();

            ui.label("Convergence tolerance");
            ui.text_edit_singleline(&mut k.convergence_tolerance);
            ui.end_row();

            ui.label("Tolerance (optimization solver)");
            ui.text_edit_singleline(&mut k.tolerance);
            ui.end_row();

            ui.label("Step size");
            ui.text_edit_singleline(&mut k.step_size);
            ui.end_row();

            ui.label("pe step size");
            ui.text_edit_singleline(&mut k.pe_step_size);
            ui.end_row();

            ui.label("Diagonal scale");
            ui.checkbox(&mut k.diagonal_scale, "");
            ui.end_row();
        });
}
