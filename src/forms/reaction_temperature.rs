use crate::forms::steps_spec_ui;
use crate::model::ReactionTemperature;

pub fn ui(ui: &mut egui::Ui, rt: &mut ReactionTemperature) {
    ui.heading(format!("Reaction Temperature {}", rt.number));
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label("Description");
        ui.text_edit_singleline(&mut rt.description);
    });
    ui.add_space(8.0);
    steps_spec_ui(ui, "reaction_temperature", "°C", &mut rt.values);
}
