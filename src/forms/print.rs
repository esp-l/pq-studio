use crate::model::Print;

pub fn ui(ui: &mut egui::Ui, p: &mut Print) {
    ui.heading("Print");
    ui.add_space(4.0);
    ui.label("Controls which result sections are written to the main output file.");
    ui.add_space(8.0);
    egui::Grid::new("print_grid")
        .num_columns(4)
        .spacing([16.0, 6.0])
        .show(ui, |ui| {
            ui.checkbox(&mut p.echo_input, "echo_input");
            ui.checkbox(&mut p.headings, "headings");
            ui.checkbox(&mut p.totals, "totals");
            ui.checkbox(&mut p.species, "species");
            ui.end_row();

            ui.checkbox(&mut p.equilibrium_phases, "equilibrium_phases");
            ui.checkbox(&mut p.exchange, "exchange");
            ui.checkbox(&mut p.gas_phase, "gas_phase");
            ui.checkbox(&mut p.surface, "surface");
            ui.end_row();

            ui.checkbox(&mut p.solid_solutions, "solid_solutions");
            ui.checkbox(&mut p.kinetics, "kinetics");
            ui.checkbox(&mut p.saturation_indices, "saturation_indices");
            ui.checkbox(&mut p.inverse_modeling, "inverse_modeling");
            ui.end_row();

            ui.checkbox(&mut p.user_print, "user_print");
            ui.checkbox(&mut p.user_graph, "user_graph");
            ui.checkbox(&mut p.selected_output, "selected_output");
            ui.checkbox(&mut p.high_precision, "high_precision");
            ui.end_row();
        });
}
