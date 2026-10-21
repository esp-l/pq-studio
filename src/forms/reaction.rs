use crate::model::{Reactant, Reaction};

pub fn ui(ui: &mut egui::Ui, r: &mut Reaction) {
    ui.heading(format!("Reaction {}", r.number));
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label("Description");
        ui.text_edit_singleline(&mut r.description);
    });
    ui.add_space(8.0);

    ui.label(egui::RichText::new("Reactants").strong());
    let mut remove_idx = None;
    egui::Grid::new("reaction_reactants_grid")
        .num_columns(3)
        .spacing([8.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label("Phase/formula");
            ui.label("Relative stoichiometry");
            ui.label("");
            ui.end_row();
            for (i, reactant) in r.reactants.iter_mut().enumerate() {
                super::sized_text_edit(ui, &mut reactant.name, 140.0);
                super::numeric_text_edit(ui, &mut reactant.relative_stoichiometry, 100.0);
                if ui.button("✕").clicked() {
                    remove_idx = Some(i);
                }
                ui.end_row();
            }
        });
    if let Some(i) = remove_idx {
        r.reactants.remove(i);
    }
    if ui.button("+ Add reactant").clicked() {
        r.reactants.push(Reactant::default());
    }

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(egui::RichText::new("Reaction amount").strong());
    egui::Grid::new("reaction_amount_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Amount");
            super::numeric_field(ui, &mut r.amount);
            ui.end_row();
            ui.label("Units");
            egui::ComboBox::from_id_salt("reaction_units")
                .selected_text(r.units.clone())
                .show_ui(ui, |ui| {
                    for u in ["moles", "millimoles", "micromoles"] {
                        ui.selectable_value(&mut r.units, u.to_string(), u);
                    }
                });
            ui.end_row();
            ui.label("Steps");
            super::whole_number_field(ui, &mut r.steps);
            ui.end_row();
        });
}
