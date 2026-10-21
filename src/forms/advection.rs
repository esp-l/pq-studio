use crate::model::Advection;

pub fn ui(ui: &mut egui::Ui, a: &mut Advection) {
    ui.heading("Advection");
    ui.add_space(8.0);
    egui::Grid::new("advection_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Cells");
            super::whole_number_field(ui, &mut a.cells);
            ui.end_row();

            ui.label("Shifts");
            super::whole_number_field(ui, &mut a.shifts);
            ui.end_row();

            ui.label("Time step");
            ui.horizontal(|ui| {
                super::numeric_field(ui, &mut a.time_step);
                egui::ComboBox::from_id_salt("advection_time_unit")
                    .selected_text(a.time_step_unit.clone())
                    .show_ui(ui, |ui| {
                        for u in ["second", "minute", "hour", "day", "year"] {
                            ui.selectable_value(&mut a.time_step_unit, u.to_string(), u);
                        }
                    });
            });
            ui.end_row();

            ui.label("Initial time (s)");
            super::numeric_field(ui, &mut a.initial_time);
            ui.end_row();

            ui.label("Warnings");
            ui.checkbox(&mut a.warnings, "");
            ui.end_row();
        });
    ui.add_space(8.0);
    ui.label(
        "Tip: with the default flux boundary condition, PHREEQC needs a \
         Solution 0 defined as the influent water, or it will error with \
         \"Solution 0 is needed for advection\".",
    );
}
