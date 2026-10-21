use crate::forms::steps_spec_ui;
use crate::model::ReactionPressure;

pub fn ui(ui: &mut egui::Ui, rp: &mut ReactionPressure) {
    ui.heading(format!("Reaction Pressure {}", rp.number));
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label("Description");
        ui.text_edit_singleline(&mut rp.description);
    });
    ui.add_space(8.0);
    steps_spec_ui(ui, "reaction_pressure", "atm", &mut rp.values);
}
