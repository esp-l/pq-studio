use crate::model::{GasComponent, GasPhase};

pub fn ui(ui: &mut egui::Ui, g: &mut GasPhase) {
    ui.heading(format!("Gas Phase {}", g.number));
    ui.add_space(8.0);

    egui::Grid::new("gas_phase_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Description");
            ui.text_edit_singleline(&mut g.description);
            ui.end_row();

            ui.label("Type");
            ui.horizontal(|ui| {
                ui.radio_value(&mut g.fixed_volume, false, "Fixed pressure (bubble)");
                ui.radio_value(&mut g.fixed_volume, true, "Fixed volume");
            });
            ui.end_row();

            ui.label("Pressure (atm)");
            super::numeric_field(ui, &mut g.pressure);
            ui.end_row();

            ui.label("Volume (L)");
            super::numeric_field(ui, &mut g.volume);
            ui.end_row();

            ui.label("Temperature (°C)");
            super::numeric_field(ui, &mut g.temperature_c);
            ui.end_row();
        });

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(egui::RichText::new("Gas components").strong());

    let mut remove_idx = None;
    egui::Grid::new("gas_phase_components_grid")
        .num_columns(3)
        .spacing([8.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label("Phase (e.g. CO2(g))");
            ui.label("Partial pressure (atm)");
            ui.label("");
            ui.end_row();
            for (i, c) in g.components.iter_mut().enumerate() {
                super::sized_text_edit(ui, &mut c.phase_name, 140.0);
                super::numeric_text_edit(ui, &mut c.partial_pressure, 100.0);
                if ui.button("✕").clicked() {
                    remove_idx = Some(i);
                }
                ui.end_row();
            }
        });
    if let Some(i) = remove_idx {
        g.components.remove(i);
    }
    if ui.button("+ Add component").clicked() {
        g.components.push(GasComponent::default());
    }
}
