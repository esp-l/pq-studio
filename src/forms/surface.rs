use crate::model::{Surface, SurfaceElectrostatics, SurfaceSite};

/// Diffuse-layer model only — no CD-MUSIC.
pub fn ui(ui: &mut egui::Ui, s: &mut Surface) {
    ui.heading(format!("Surface {}", s.number));
    ui.add_space(8.0);

    egui::Grid::new("surface_top_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Description");
            ui.text_edit_singleline(&mut s.description);
            ui.end_row();

            ui.label("Equilibrate with solution #");
            super::whole_number_field(ui, &mut s.equilibrate_with_solution);
            ui.end_row();
        });

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(egui::RichText::new("Surface sites").strong());
    let show_area = s.electrostatics == SurfaceElectrostatics::DiffuseLayer;
    if show_area {
        ui.label("Specific area/mass only needed on the first site of each distinct surface name.");
    }
    let mut remove_idx = None;
    egui::Grid::new("surface_sites_grid")
        .num_columns(if show_area { 5 } else { 3 })
        .spacing([8.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label("Surface formula (e.g. Hfo_wOH)");
            ui.label("Amount (mol)");
            if show_area {
                ui.label("Area (m²/g)");
                ui.label("Mass (g)");
            }
            ui.label("");
            ui.end_row();
            for (i, site) in s.sites.iter_mut().enumerate() {
                super::sized_text_edit(ui, &mut site.surface_formula, 160.0);
                super::numeric_text_edit(ui, &mut site.amount, 90.0);
                if show_area {
                    super::numeric_text_edit(ui, &mut site.specific_area_m2_g, 80.0);
                    super::numeric_text_edit(ui, &mut site.mass_g, 80.0);
                }
                if ui.button("✕").clicked() {
                    remove_idx = Some(i);
                }
                ui.end_row();
            }
        });
    if let Some(i) = remove_idx {
        s.sites.remove(i);
    }
    if ui.button("+ Add site").clicked() {
        s.sites.push(SurfaceSite::default());
    }

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(egui::RichText::new("Electrostatics").strong());
    ui.horizontal(|ui| {
        ui.radio_value(
            &mut s.electrostatics,
            SurfaceElectrostatics::None,
            "No electrostatics",
        );
        ui.radio_value(
            &mut s.electrostatics,
            SurfaceElectrostatics::DiffuseLayer,
            "Diffuse layer (Dzombak & Morel)",
        );
    });
    if s.electrostatics == SurfaceElectrostatics::DiffuseLayer {
        ui.horizontal(|ui| {
            ui.label("Thickness (m, blank = default)");
            ui.text_edit_singleline(&mut s.diffuse_layer_thickness);
        });
    }
}
