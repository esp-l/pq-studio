use crate::model::{DATABASE_KEYWORDS, Raw};

/// Text editor for keyword blocks without a structured form: the database
/// definition keywords (PHASES, *_SPECIES, PITZER, SIT…) and anything else
/// preserved verbatim from an opened file. Written back exactly as shown.
pub fn ui(ui: &mut egui::Ui, r: &mut Raw) {
    if r.header_line.trim().eq_ignore_ascii_case("END") && r.body.trim().is_empty() {
        ui.heading("End of simulation (END)");
        ui.add_space(4.0);
        ui.label(
            "PHREEQC runs the blocks above this END as one simulation; the blocks after it form \
             the next one. Each simulation reacts one solution: the one named by USE solution, \
             or else the first SOLUTION defined in it.",
        );
        return;
    }
    let keyword = r.header_line.split_whitespace().next().unwrap_or("").to_uppercase();
    let db_label = DATABASE_KEYWORDS
        .iter()
        .find(|(_, kw, _)| *kw == keyword)
        .map(|(label, _, _)| *label);
    match db_label {
        Some(label) => {
            ui.heading(format!("{label} ({keyword})"));
            ui.add_space(4.0);
            ui.label(
                "Database definitions are edited as PHREEQC text and added to the database \
                 for this run. Lines starting with # are comments; the template shows the \
                 syntax. See the PHREEQC manual (Help menu) for every option.",
            );
        }
        None => {
            ui.heading(if keyword.is_empty() { "Other keyword".to_string() } else { keyword.clone() });
            ui.add_space(4.0);
            ui.label(
                "This keyword has no form in PQ Studio. Its text was kept exactly as read \
                 and is written back unchanged unless you edit it below.",
            );
        }
    }
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label("Keyword line");
        ui.text_edit_singleline(&mut r.header_line);
    });
    ui.add_space(6.0);
    ui.add(
        egui::TextEdit::multiline(&mut r.body)
            .font(egui::TextStyle::Monospace)
            .code_editor()
            .desired_width(f32::INFINITY)
            .desired_rows(16),
    );
}
