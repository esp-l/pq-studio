use crate::model::{EquilibriumPhases, PhaseAmount};

pub fn ui(ui: &mut egui::Ui, eq: &mut EquilibriumPhases) {
    ui.heading(format!("Equilibrium Phases {}", eq.number));
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label("Description");
        ui.text_edit_singleline(&mut eq.description);
    });
    ui.add_space(8.0);

    let mut remove_idx = None;
    egui::Grid::new("eq_phases_grid")
        .num_columns(4)
        .spacing([8.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label("Phase");
            ui.label("Target SI");
            ui.label("Amount (mol)");
            ui.label("");
            ui.end_row();
            for (i, p) in eq.phases.iter_mut().enumerate() {
                super::sized_text_edit(ui, &mut p.phase_name, 140.0);
                super::numeric_text_edit(ui, &mut p.saturation_index, 70.0);
                super::numeric_text_edit(ui, &mut p.amount, 80.0);
                if ui.button("✕").clicked() {
                    remove_idx = Some(i);
                }
                ui.end_row();
            }
        });
    if let Some(i) = remove_idx {
        eq.phases.remove(i);
    }
    ui.add_space(6.0);
    if ui.button("+ Add phase").clicked() {
        eq.phases.push(PhaseAmount::default());
    }
}
