use crate::model::{FlowDirection, Transport};

/// Simplified to uniform length/dispersivity (same value for every cell) —
/// see model.rs for the full scope note (stagnant zones, multicomponent
/// diffusion not covered).
pub fn ui(ui: &mut egui::Ui, t: &mut Transport) {
    ui.heading("Transport");
    ui.add_space(8.0);
    egui::Grid::new("transport_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Cells");
            super::whole_number_field(ui, &mut t.cells);
            ui.end_row();

            ui.label("Shifts");
            super::whole_number_field(ui, &mut t.shifts);
            ui.end_row();

            ui.label("Time step");
            ui.horizontal(|ui| {
                super::numeric_field(ui, &mut t.time_step);
                egui::ComboBox::from_id_salt("transport_time_unit")
                    .selected_text(t.time_step_unit.clone())
                    .show_ui(ui, |ui| {
                        for u in ["second", "minute", "hour", "day", "year"] {
                            ui.selectable_value(&mut t.time_step_unit, u.to_string(), u);
                        }
                    });
            });
            ui.end_row();

            ui.label("Flow direction");
            ui.horizontal(|ui| {
                ui.radio_value(&mut t.flow_direction, FlowDirection::Forward, "Forward");
                ui.radio_value(&mut t.flow_direction, FlowDirection::Back, "Back");
                ui.radio_value(
                    &mut t.flow_direction,
                    FlowDirection::Diffusion,
                    "Diffusion only",
                );
            });
            ui.end_row();

            ui.label("Boundary condition (top)");
            boundary_combo(ui, "transport_bc_top", &mut t.boundary_condition_top);
            ui.end_row();

            ui.label("Boundary condition (bottom)");
            boundary_combo(ui, "transport_bc_bottom", &mut t.boundary_condition_bottom);
            ui.end_row();

            ui.label("Cell length (m, uniform)");
            super::numeric_field(ui, &mut t.uniform_length);
            ui.end_row();

            ui.label("Dispersivity (m, uniform)");
            super::numeric_field(ui, &mut t.uniform_dispersivity);
            ui.end_row();

            ui.label("Diffusion coefficient (m²/s)");
            super::numeric_field(ui, &mut t.diffusion_coefficient);
            ui.end_row();
        });
}

fn boundary_combo(ui: &mut egui::Ui, id: &str, value: &mut String) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(value.clone())
        .show_ui(ui, |ui| {
            for v in ["flux", "constant", "closed"] {
                ui.selectable_value(value, v.to_string(), v);
            }
        });
}
