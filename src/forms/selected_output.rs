use crate::model::{SELECTED_OUTPUT_FLAGS, SelectedOutput};

pub fn ui(ui: &mut egui::Ui, so: &mut SelectedOutput) {
    ui.heading(format!("Selected Output {}", so.number));
    ui.add_space(4.0);
    ui.label(
        "The table PQ Studio plots after each run: one row per calculation step, with the \
         columns chosen below. Export it for a spreadsheet with “Export table…” under the plot.",
    );
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        ui.label("Output file");
        ui.add(egui::TextEdit::singleline(&mut so.file).hint_text(format!("selected_output_{}.sel", so.number)));
    });

    ui.add_space(12.0);
    ui.label(egui::RichText::new("Standard columns").strong());
    egui::Grid::new("selected_output_flags_grid")
        .num_columns(4)
        .spacing([16.0, 4.0])
        .show(ui, |ui| {
            for i in 0..SELECTED_OUTPUT_FLAGS.len() {
                let mut on = so.flag(i);
                if ui.checkbox(&mut on, SELECTED_OUTPUT_FLAGS[i].0).changed() {
                    so.flags[i] = Some(on);
                }
                if i % 4 == 3 {
                    ui.end_row();
                }
            }
        });

    ui.add_space(12.0);
    ui.label(egui::RichText::new("Extra columns (names separated by spaces)").strong());
    ui.add_space(4.0);
    egui::Grid::new("selected_output_lists_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            let rows: [(&str, &mut String, &str, &str); 6] = [
                (
                    "Saturation indices",
                    &mut so.saturation_indices,
                    "Calcite Anhydrite Barite",
                    "Phases from the database; writes si_Calcite …. Above 0 = supersaturated, may precipitate.",
                ),
                (
                    "Equilibrium phases",
                    &mut so.equilibrium_phases,
                    "Calcite CO2(g)",
                    "Phases of an Equilibrium Phases block; writes the moles present (Calcite) and \
                     the change d_Calcite: negative = dissolved, positive = precipitated.",
                ),
                (
                    "Totals",
                    &mut so.totals,
                    "Ca Mg C(4)",
                    "Total dissolved element (mol/kgw), e.g. Ca, or a valence state, e.g. C(4).",
                ),
                (
                    "Molalities",
                    &mut so.molalities,
                    "HCO3- CO3-2 CaCO3",
                    "Species molalities (mol/kgw); writes m_HCO3- ….",
                ),
                (
                    "Activities",
                    &mut so.activities,
                    "H+ Ca+2",
                    "log10 of species activities; writes la_H+ ….",
                ),
                (
                    "Gases",
                    &mut so.gases,
                    "CO2(g)",
                    "Gas-phase components of a Gas Phase block (moles); writes g_CO2(g) ….",
                ),
            ];
            for (label, value, hint, help) in rows {
                ui.label(label).on_hover_text(help);
                ui.add(
                    egui::TextEdit::singleline(value)
                        .hint_text(format!("e.g. {hint}"))
                        .desired_width(f32::INFINITY),
                )
                .on_hover_text(help);
                ui.end_row();
            }
        });
}
