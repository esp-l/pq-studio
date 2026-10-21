use crate::model::IncrementalReactions;

pub fn ui(ui: &mut egui::Ui, ir: &mut IncrementalReactions) {
    ui.heading("Incremental Reactions");
    ui.add_space(8.0);
    ui.checkbox(
        &mut ir.enabled,
        "Treat REACTION/KINETICS steps as incremental",
    );
    ui.add_space(4.0);
    ui.label(
        "When on, each listed step adds to the previous step's result rather than \
         restarting from the initial solution each time.",
    );
}
