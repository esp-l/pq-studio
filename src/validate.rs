//! Light pre-run checks for fields that must be numbers. Only catches what
//! would otherwise produce a confusing engine error; PHREEQC itself remains
//! the authority on whether an input makes sense. Empty means "use PHREEQC's
//! default" and is always accepted.

use crate::model::{Block, Document};

pub fn is_number(text: &str) -> bool {
    let t = text.trim();
    t.is_empty() || t.parse::<f64>().is_ok_and(f64::is_finite)
}

pub fn is_whole_number(text: &str) -> bool {
    let t = text.trim();
    t.is_empty() || t.parse::<u64>().is_ok()
}

/// One message per offending field, naming the block and field.
pub fn problems(doc: &Document) -> Vec<String> {
    let mut out = Vec::new();
    for block in &doc.blocks {
        let name = block.label();
        let mut num = |label: &str, value: &str| {
            if !is_number(value) {
                out.push(format!("{name}: {label} \"{}\" is not a number", value.trim()));
            }
        };
        match block {
            Block::Solution(s) => {
                num("temperature", &s.temperature_c);
                // pH and pe may carry a phase after the value ("8.2 CO2(g) -3.5").
                num("pH", s.ph.split_whitespace().next().unwrap_or(""));
                num("pe", s.pe.split_whitespace().next().unwrap_or(""));
                num("density", &s.density);
                for e in &s.elements {
                    num(&format!("{} concentration", e.element.trim()), &e.concentration);
                }
            }
            Block::EquilibriumPhases(e) => {
                for p in &e.phases {
                    num(&format!("{} saturation index", p.phase_name.trim()), &p.saturation_index);
                    num(&format!("{} amount", p.phase_name.trim()), &p.amount);
                }
            }
            Block::Reaction(r) => {
                for x in &r.reactants {
                    num(&format!("{} coefficient", x.name.trim()), &x.relative_stoichiometry);
                }
                num("amount", &r.amount);
            }
            Block::GasPhase(g) => {
                num("pressure", &g.pressure);
                num("volume", &g.volume);
                num("temperature", &g.temperature_c);
                for c in &g.components {
                    num(&format!("{} partial pressure", c.phase_name.trim()), &c.partial_pressure);
                }
            }
            Block::Exchange(x) => {
                for c in &x.components {
                    num(&format!("{} amount", c.exchange_formula.trim()), &c.amount);
                }
            }
            Block::Surface(s) => {
                for site in &s.sites {
                    let f = site.surface_formula.trim();
                    num(&format!("{f} sites"), &site.amount);
                    num(&format!("{f} specific area"), &site.specific_area_m2_g);
                    num(&format!("{f} mass"), &site.mass_g);
                }
            }
            Block::Mix(m) => {
                for c in &m.components {
                    num(&format!("solution {} fraction", c.solution_number.trim()), &c.fraction);
                }
            }
            Block::Advection(a) => {
                num("time step", &a.time_step);
                num("initial time", &a.initial_time);
            }
            Block::Transport(t) => {
                num("time step", &t.time_step);
                num("cell length", &t.uniform_length);
                num("dispersivity", &t.uniform_dispersivity);
                num("diffusion coefficient", &t.diffusion_coefficient);
            }
            _ => {}
        }
        let mut whole = |label: &str, value: &str| {
            if !is_whole_number(value) {
                out.push(format!("{name}: {label} \"{}\" must be a whole number", value.trim()));
            }
        };
        match block {
            Block::Reaction(r) => whole("steps", &r.steps),
            Block::Surface(s) => whole("equilibrate with solution", &s.equilibrate_with_solution),
            Block::Mix(m) => {
                for c in &m.components {
                    whole("solution number", &c.solution_number);
                }
            }
            Block::Advection(a) => {
                whole("cells", &a.cells);
                whole("shifts", &a.shifts);
            }
            Block::Transport(t) => {
                whole("cells", &t.cells);
                whole("shifts", &t.shifts);
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ElementConcentration, Solution};

    #[test]
    fn accepts_numbers_and_blanks_only() {
        for ok in ["", "  ", "7", "-0.5", "1e-3", "2.5E+2", " 8.1 "] {
            assert!(is_number(ok), "{ok:?}");
        }
        for bad in ["7,5", "abc", "1..2", "nan", "inf"] {
            assert!(!is_number(bad), "{bad:?}");
        }
        assert!(is_whole_number("12") && is_whole_number(""));
        assert!(!is_whole_number("1.5") && !is_whole_number("-1"));
    }

    #[test]
    fn reports_bad_fields_by_block() {
        let mut sol = Solution::default();
        sol.ph = "7,5".into();
        sol.elements.push(ElementConcentration {
            element: "Ca".into(),
            concentration: "1".into(),
            ..Default::default()
        });
        let doc = Document {
            title: String::new(),
            blocks: vec![Block::Solution(sol)],
        };
        assert_eq!(problems(&doc), vec!["Solution 1: pH \"7,5\" is not a number"]);
    }
}
