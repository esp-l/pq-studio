use crate::model::Dump;

pub fn ui(ui: &mut egui::Ui, d: &mut Dump) {
    ui.heading("Dump");
    ui.add_space(8.0);
    egui::Grid::new("dump_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("File");
            ui.text_edit_singleline(&mut d.file);
            ui.end_row();

            ui.label("Append");
            ui.checkbox(&mut d.append, "");
            ui.end_row();
        });

    ui.add_space(8.0);
    ui.checkbox(&mut d.all, "Dump every defined entity of every type");

    if !d.all {
        ui.add_space(8.0);
        ui.label("Or select specific entity types (blank range = all instances of that type):");
        egui::Grid::new("dump_entities_grid")
            .num_columns(3)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                for e in &mut d.entities {
                    ui.checkbox(&mut e.enabled, e.entity.label());
                    if e.enabled {
                        ui.label("Ranges");
                        ui.text_edit_singleline(&mut e.ranges);
                    } else {
                        ui.label("");
                        ui.label("");
                    }
                    ui.end_row();
                }
            });
    }
}
