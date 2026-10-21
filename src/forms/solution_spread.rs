use crate::model::{SolutionSpread, SpreadRow};

pub fn ui(ui: &mut egui::Ui, sp: &mut SolutionSpread) {
    ui.heading("Solution Spread");
    ui.add_space(8.0);

    ui.label(egui::RichText::new("Defaults (used when a cell is blank)").strong());
    egui::Grid::new("spread_defaults_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Temperature (°C)");
            ui.text_edit_singleline(&mut sp.default_temperature_c);
            ui.end_row();
            ui.label("pH");
            ui.text_edit_singleline(&mut sp.default_ph);
            ui.end_row();
            ui.label("pe");
            ui.text_edit_singleline(&mut sp.default_pe);
            ui.end_row();
            ui.label("Units");
            ui.text_edit_singleline(&mut sp.default_units);
            ui.end_row();
            ui.label("Density");
            ui.text_edit_singleline(&mut sp.default_density);
            ui.end_row();
        });

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(egui::RichText::new("Columns").strong());
    let mut remove_col = None;
    for (i, col) in sp.columns.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            super::sized_text_edit(ui, col, 140.0);
            if ui.button("✕").clicked() {
                remove_col = Some(i);
            }
        });
    }
    if let Some(i) = remove_col {
        sp.columns.remove(i);
        for row in &mut sp.rows {
            if i < row.values.len() {
                row.values.remove(i);
            }
        }
    }
    if ui.button("+ Add column (element/property name)").clicked() {
        sp.columns.push(String::new());
        for row in &mut sp.rows {
            row.values.push(String::new());
        }
    }

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(egui::RichText::new("Solutions (rows)").strong());

    let ncols = sp.columns.len();
    let mut remove_row = None;
    egui::ScrollArea::horizontal()
        .id_salt("spread_rows_scroll")
        .show(ui, |ui| {
            egui::Grid::new("spread_rows_grid")
                .num_columns(3 + ncols)
                .spacing([8.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Number");
                    for col in &sp.columns {
                        ui.label(if col.trim().is_empty() {
                            "(unnamed)"
                        } else {
                            col.as_str()
                        });
                    }
                    ui.label("Description");
                    ui.label("");
                    ui.end_row();

                    for (ri, row) in sp.rows.iter_mut().enumerate() {
                        super::sized_text_edit(ui, &mut row.number, 60.0);
                        while row.values.len() < ncols {
                            row.values.push(String::new());
                        }
                        for v in row.values.iter_mut().take(ncols) {
                            super::sized_text_edit(ui, v, 70.0);
                        }
                        super::sized_text_edit(ui, &mut row.description, 140.0);
                        if ui.button("✕").clicked() {
                            remove_row = Some(ri);
                        }
                        ui.end_row();
                    }
                });
        });
    if let Some(ri) = remove_row {
        sp.rows.remove(ri);
    }
    if ui.button("+ Add solution row").clicked() {
        sp.rows.push(SpreadRow {
            values: vec![String::new(); ncols],
            ..Default::default()
        });
    }
}
