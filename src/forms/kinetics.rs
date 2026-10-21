use crate::forms::steps_spec_ui;
use crate::model::{KineticReactant, Kinetics};

/// Rate name is a reference into RATES-defined expressions; this app does
/// not host a custom RATES/BASIC-code editor.
pub fn ui(ui: &mut egui::Ui, k: &mut Kinetics) {
    ui.heading(format!("Kinetics {}", k.number));
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label("Description");
        ui.text_edit_singleline(&mut k.description);
    });
    ui.add_space(8.0);

    ui.label(egui::RichText::new("Kinetic reactants").strong());
    let mut remove_idx = None;
    egui::Grid::new("kinetics_reactants_grid")
        .num_columns(6)
        .spacing([8.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label("Rate name");
            ui.label("m (mol)");
            ui.label("m0 (mol)");
            ui.label("parms");
            ui.label("tol");
            ui.label("");
            ui.end_row();
            for (i, r) in k.reactants.iter_mut().enumerate() {
                super::sized_text_edit(ui, &mut r.rate_name, 100.0);
                super::sized_text_edit(ui, &mut r.m, 60.0);
                super::sized_text_edit(ui, &mut r.m0, 60.0);
                super::sized_text_edit(ui, &mut r.parms, 90.0);
                super::sized_text_edit(ui, &mut r.tol, 60.0);
                if ui.button("✕").clicked() {
                    remove_idx = Some(i);
                }
                ui.end_row();
            }
        });
    if let Some(i) = remove_idx {
        k.reactants.remove(i);
    }
    if ui.button("+ Add reactant").clicked() {
        k.reactants.push(KineticReactant::default());
    }

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(egui::RichText::new("Time steps").strong());
    ui.horizontal(|ui| {
        ui.label("Units");
        egui::ComboBox::from_id_salt("kinetics_step_units")
            .selected_text(k.step_units.clone())
            .show_ui(ui, |ui| {
                for u in ["seconds", "minutes", "hours", "days", "years"] {
                    ui.selectable_value(&mut k.step_units, u.to_string(), u);
                }
            });
    });
    steps_spec_ui(ui, "kinetics_steps", &k.step_units.clone(), &mut k.steps);
}
