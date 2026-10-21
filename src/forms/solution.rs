use crate::model::{DEFAULT_UNITS_OPTIONS, ElementConcentration, Solution};

/// Renders the SOLUTION editor. Functional scope matches the real GUI's
/// General + Individual element input tabs (PhreeqcI dialog resources
/// 190/191); visual presentation is a modernized plain form rather than
/// a literal MSHFlexGrid ActiveX clone.
pub fn ui(ui: &mut egui::Ui, sol: &mut Solution) {
    ui.heading(format!("Solution {}", sol.number));
    ui.add_space(8.0);

    egui::Grid::new("solution_general_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Description");
            ui.text_edit_singleline(&mut sol.description);
            ui.end_row();

            ui.label("Temperature (°C)");
            super::numeric_field(ui, &mut sol.temperature_c);
            ui.end_row();

            ui.label("pH");
            super::leading_number_field(ui, &mut sol.ph);
            ui.end_row();

            ui.label("pe");
            super::leading_number_field(ui, &mut sol.pe);
            ui.end_row();

            ui.label("Redox couple");
            ui.text_edit_singleline(&mut sol.redox_couple);
            ui.end_row();

            ui.label("Density (kg/L)");
            super::numeric_field(ui, &mut sol.density);
            ui.end_row();

            ui.label("Default units");
            egui::ComboBox::from_id_salt("default_units")
                .selected_text(sol.default_units.clone())
                .show_ui(ui, |ui| {
                    for opt in DEFAULT_UNITS_OPTIONS {
                        ui.selectable_value(&mut sol.default_units, opt.to_string(), *opt);
                    }
                });
            ui.end_row();

            ui.label("Charge balance");
            let mut choices = vec!["None".to_string(), "pH".to_string()];
            for el in &sol.elements {
                if !el.element.trim().is_empty() {
                    choices.push(el.element.trim().to_string());
                }
            }
            egui::ComboBox::from_id_salt("charge_balance")
                .selected_text(sol.charge_balance.clone())
                .show_ui(ui, |ui| {
                    for c in &choices {
                        ui.selectable_value(&mut sol.charge_balance, c.clone(), c.clone());
                    }
                });
            ui.end_row();
        });

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(egui::RichText::new("Element concentrations").strong());
    ui.add_space(4.0);

    let mut remove_idx: Option<usize> = None;
    egui::Grid::new("solution_elements_grid")
        .num_columns(6)
        .spacing([8.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label("Element");
            ui.label("Conc.");
            ui.label("Units");
            ui.label("As");
            ui.label("Redox");
            ui.label("");
            ui.end_row();

            for (i, el) in sol.elements.iter_mut().enumerate() {
                super::sized_text_edit(ui, &mut el.element, 60.0);
                super::numeric_text_edit(ui, &mut el.concentration, 70.0);
                super::sized_text_edit(ui, &mut el.unit_override, 140.0);
                super::sized_text_edit(ui, &mut el.as_form, 60.0);
                super::sized_text_edit(ui, &mut el.redox_couple, 60.0);
                if ui.button("✕").clicked() {
                    remove_idx = Some(i);
                }
                ui.end_row();
            }
        });

    if let Some(i) = remove_idx {
        sol.elements.remove(i);
    }

    ui.add_space(6.0);
    if ui.button("+ Add element").clicked() {
        sol.elements.push(ElementConcentration::default());
    }
}
