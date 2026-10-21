use crate::model::{Entity, Use};

/// Named `use_block` (not `use`, a Rust keyword).
pub fn ui(ui: &mut egui::Ui, u: &mut Use) {
    ui.heading("Use");
    ui.add_space(8.0);
    egui::Grid::new("use_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Entity");
            egui::ComboBox::from_id_salt("use_entity")
                .selected_text(u.entity.label())
                .show_ui(ui, |ui| {
                    for &e in Entity::ALL {
                        ui.selectable_value(&mut u.entity, e, e.label());
                    }
                });
            ui.end_row();

            ui.label("Number (blank = none)");
            ui.text_edit_singleline(&mut u.number);
            ui.end_row();
        });
    ui.add_space(8.0);
    ui.label(
        "Explicitly selects which previously defined composition/reaction of this \
         entity type to use in the next batch-reaction calculation, overriding the \
         default (the first of that type defined in the simulation). Leave the number \
         blank to use none of this entity type.",
    );
}
