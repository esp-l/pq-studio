use crate::model::{InverseBalance, InverseModeling};

pub fn ui(ui: &mut egui::Ui, im: &mut InverseModeling) {
    ui.heading(format!("Inverse Modeling {}", im.number));
    ui.add_space(8.0);
    egui::Grid::new("inverse_top_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Description");
            ui.text_edit_singleline(&mut im.description);
            ui.end_row();

            ui.label("Solutions (initial... final)");
            ui.text_edit_singleline(&mut im.solutions);
            ui.end_row();

            ui.label("Default uncertainty");
            ui.text_edit_singleline(&mut im.uncertainty);
            ui.end_row();

            ui.label("Tolerance");
            ui.text_edit_singleline(&mut im.tolerance);
            ui.end_row();
        });
    ui.horizontal(|ui| {
        ui.checkbox(&mut im.range, "Range (find minimal + maximal models)");
        ui.checkbox(&mut im.minimal, "Minimal (fewest phases only)");
    });

    ui.add_space(12.0);
    ui.separator();
    ui.label(egui::RichText::new("Candidate phases/exchangers").strong());
    let mut remove_idx = None;
    for (i, p) in im.phases.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            super::sized_text_edit(ui, p, 160.0);
            if ui.button("✕").clicked() {
                remove_idx = Some(i);
            }
        });
    }
    if let Some(i) = remove_idx {
        im.phases.remove(i);
    }
    if ui.button("+ Add phase").clicked() {
        im.phases.push(String::new());
    }

    ui.add_space(12.0);
    ui.separator();
    ui.label(egui::RichText::new("Element/pH uncertainty overrides").strong());
    let mut remove_idx = None;
    egui::Grid::new("inverse_balances_grid")
        .num_columns(3)
        .spacing([8.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label("Element or pH");
            ui.label("Uncertainty");
            ui.label("");
            ui.end_row();
            for (i, b) in im.balances.iter_mut().enumerate() {
                super::sized_text_edit(ui, &mut b.element_or_ph, 100.0);
                super::sized_text_edit(ui, &mut b.uncertainty, 80.0);
                if ui.button("✕").clicked() {
                    remove_idx = Some(i);
                }
                ui.end_row();
            }
        });
    if let Some(i) = remove_idx {
        im.balances.remove(i);
    }
    if ui.button("+ Add override").clicked() {
        im.balances.push(InverseBalance::default());
    }
}
