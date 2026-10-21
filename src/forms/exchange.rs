use crate::model::{Exchange, ExchangeComponent};

pub fn ui(ui: &mut egui::Ui, ex: &mut Exchange) {
    ui.heading(format!("Exchange {}", ex.number));
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label("Description");
        ui.text_edit_singleline(&mut ex.description);
    });
    ui.add_space(8.0);

    let mut remove_idx = None;
    egui::Grid::new("exchange_grid")
        .num_columns(3)
        .spacing([8.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label("Exchange formula (e.g. NaX)");
            ui.label("Amount (mol)");
            ui.label("");
            ui.end_row();
            for (i, c) in ex.components.iter_mut().enumerate() {
                super::sized_text_edit(ui, &mut c.exchange_formula, 140.0);
                super::numeric_text_edit(ui, &mut c.amount, 90.0);
                if ui.button("✕").clicked() {
                    remove_idx = Some(i);
                }
                ui.end_row();
            }
        });
    if let Some(i) = remove_idx {
        ex.components.remove(i);
    }
    if ui.button("+ Add exchanger").clicked() {
        ex.components.push(ExchangeComponent::default());
    }
}
