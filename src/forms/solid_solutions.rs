use crate::model::{SolidSolution, SolidSolutionComponent, SolidSolutions};

/// Ideal solid solutions only (one or more `-comp` entries per named solid
/// solution). Nonideal binary solid solutions are out of scope — see model.rs.
pub fn ui(ui: &mut egui::Ui, ss: &mut SolidSolutions) {
    ui.heading(format!("Solid Solutions {}", ss.number));
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label("Description");
        ui.text_edit_singleline(&mut ss.description);
    });
    ui.add_space(12.0);

    let mut remove_ss_idx = None;
    for (si, sol) in ss.solid_solutions.iter_mut().enumerate() {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut sol.name);
                if ui.button("✕ Remove solid solution").clicked() {
                    remove_ss_idx = Some(si);
                }
            });

            let mut remove_comp_idx = None;
            egui::Grid::new(format!("solid_solution_{si}_components"))
                .num_columns(3)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Component phase");
                    ui.label("Moles");
                    ui.label("");
                    ui.end_row();
                    for (ci, comp) in sol.components.iter_mut().enumerate() {
                        super::sized_text_edit(ui, &mut comp.phase_name, 140.0);
                        super::sized_text_edit(ui, &mut comp.moles, 80.0);
                        if ui.button("✕").clicked() {
                            remove_comp_idx = Some(ci);
                        }
                        ui.end_row();
                    }
                });
            if let Some(ci) = remove_comp_idx {
                sol.components.remove(ci);
            }
            if ui.button("+ Add component").clicked() {
                sol.components.push(SolidSolutionComponent::default());
            }
        });
        ui.add_space(6.0);
    }
    if let Some(si) = remove_ss_idx {
        ss.solid_solutions.remove(si);
    }
    if ui.button("+ Add solid solution").clicked() {
        ss.solid_solutions.push(SolidSolution::default());
    }
}
