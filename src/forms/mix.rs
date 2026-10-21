use crate::model::{Mix, MixComponent};

pub fn ui(ui: &mut egui::Ui, m: &mut Mix) {
    ui.heading(format!("Mix {}", m.number));
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label("Description");
        ui.text_edit_singleline(&mut m.description);
    });
    ui.add_space(8.0);

    let mut remove_idx = None;
    egui::Grid::new("mix_grid")
        .num_columns(3)
        .spacing([8.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label("Solution #");
            ui.label("Mixing fraction");
            ui.label("");
            ui.end_row();
            for (i, c) in m.components.iter_mut().enumerate() {
                super::whole_number_text_edit(ui, &mut c.solution_number, 70.0);
                super::numeric_text_edit(ui, &mut c.fraction, 90.0);
                if ui.button("✕").clicked() {
                    remove_idx = Some(i);
                }
                ui.end_row();
            }
        });
    if let Some(i) = remove_idx {
        m.components.remove(i);
    }
    if ui.button("+ Add solution").clicked() {
        m.components.push(MixComponent::default());
    }
}
