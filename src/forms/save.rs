use crate::model::{Entity, Save};

pub fn ui(ui: &mut egui::Ui, s: &mut Save) {
    ui.heading("Save");
    ui.add_space(8.0);
    egui::Grid::new("save_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Entity");
            egui::ComboBox::from_id_salt("save_entity")
                .selected_text(s.entity.label())
                .show_ui(ui, |ui| {
                    for &e in Entity::ALL {
                        ui.selectable_value(&mut s.entity, e, e.label());
                    }
                });
            ui.end_row();

            ui.label("Save as number");
            ui.text_edit_singleline(&mut s.number);
            ui.end_row();
        });
    ui.add_space(8.0);
    ui.label(
        "Stores the current end-of-batch-reaction composition of this entity under the \
         given number, for reuse by a later Solution/Equilibrium Phases/etc. block in the \
         same run.",
    );
}
