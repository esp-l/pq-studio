//! Simulation structure of a document: PHREEQC splits input into
//! simulations at each END, and in each simulation reacts at most one
//! solution or mix (the one named by USE, else a MIX defined there, else the
//! first SOLUTION defined there). Running several waters through several
//! reactant set-ups therefore takes one `USE … / END` group per combination.
//!
//! The "Simulations from combinations" helper writes those groups as
//! ordinary USE and END blocks at the end of the document, so the saved file
//! is exactly what runs and reopening it shows the same blocks. The groups
//! are recognised again from the file's content alone (trailing simulations
//! made only of USE blocks naming a solution or mix), with no hidden markers.

use crate::model::{Block, Entity, Raw, Use};
use std::collections::BTreeSet;
use std::ops::Range;

/// Entities a batch-reaction simulation can apply to its water, in the
/// order the dialog lists them.
pub const REACTANTS: &[Entity] = &[
    Entity::ReactionTemperature,
    Entity::ReactionPressure,
    Entity::EquilibriumPhases,
    Entity::Reaction,
    Entity::GasPhase,
    Entity::Exchange,
    Entity::Surface,
    Entity::SolidSolution,
    Entity::Kinetics,
];

pub fn end_block() -> Block {
    Block::Raw(Raw {
        header_line: "END".into(),
        body: String::new(),
    })
}

pub fn is_end(block: &Block) -> bool {
    matches!(block, Block::Raw(r) if r.header_line.trim().eq_ignore_ascii_case("END"))
}

/// Index ranges of each simulation; an END belongs to the simulation it
/// closes. Always at least one (possibly empty) simulation.
pub fn split(blocks: &[Block]) -> Vec<Range<usize>> {
    let mut sims = Vec::new();
    let mut start = 0;
    for (i, block) in blocks.iter().enumerate() {
        if is_end(block) {
            sims.push(start..i + 1);
            start = i + 1;
        }
    }
    if start < blocks.len() || sims.is_empty() {
        sims.push(start..blocks.len());
    }
    sims
}

/// Keyword, number and description of a text block's header, e.g.
/// `SOLUTION 3 Brine`. None for ranges (`SOLUTION 1-5`).
fn raw_header(r: &Raw) -> Option<(String, u32, String)> {
    let line = r.header_line.trim();
    let (keyword, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    let rest = rest.trim();
    let (number, description) = match rest.split_once(char::is_whitespace) {
        Some((n, d)) => (n, d.trim()),
        None => (rest, ""),
    };
    let number = if number.is_empty() { 1 } else { number.parse().ok()? };
    Some((keyword.to_ascii_uppercase(), number, description.to_string()))
}

fn entity_for_keyword(keyword: &str) -> Option<Entity> {
    match keyword {
        "SOLID_SOLUTIONS" => return Some(Entity::SolidSolution),
        "EQUILIBRIUM" | "EQUILIBRIA" | "PURE_PHASES" | "PURE" => return Some(Entity::EquilibriumPhases),
        _ => {}
    }
    Entity::ALL
        .iter()
        .copied()
        .find(|e| e.keyword().eq_ignore_ascii_case(keyword))
}

/// Which entity (and number) a block defines, for form and text blocks.
pub fn defines(block: &Block) -> Option<(Entity, u32)> {
    let entity = match block {
        Block::Solution(_) => Entity::Solution,
        Block::EquilibriumPhases(_) => Entity::EquilibriumPhases,
        Block::Reaction(_) => Entity::Reaction,
        Block::ReactionTemperature(_) => Entity::ReactionTemperature,
        Block::ReactionPressure(_) => Entity::ReactionPressure,
        Block::GasPhase(_) => Entity::GasPhase,
        Block::Exchange(_) => Entity::Exchange,
        Block::Surface(_) => Entity::Surface,
        Block::Kinetics(_) => Entity::Kinetics,
        Block::Mix(_) => Entity::Mix,
        Block::SolidSolutions(_) => Entity::SolidSolution,
        Block::Raw(r) => {
            let (keyword, number, _) = raw_header(r)?;
            return entity_for_keyword(&keyword).map(|e| (e, number));
        }
        _ => return None,
    };
    Some((entity, block.number()))
}

/// Every number defined for an entity, sorted and unique.
pub fn defined_numbers(blocks: &[Block], entity: Entity) -> Vec<u32> {
    blocks
        .iter()
        .filter_map(defines)
        .filter(|&(e, _)| e == entity)
        .map(|(_, n)| n)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// A short name for a defined entity: its description, else "Solution 3".
pub fn entity_name(blocks: &[Block], entity: Entity, number: u32) -> String {
    let description = blocks.iter().find_map(|b| {
        if defines(b) != Some((entity, number)) {
            return None;
        }
        let d = match b {
            Block::Raw(r) => raw_header(r).map(|(_, _, d)| d).unwrap_or_default(),
            other => other.description().trim().to_string(),
        };
        (!d.is_empty()).then_some(d)
    });
    description.unwrap_or_else(|| format!("{} {number}", entity.label()))
}

/// What a USE block in this simulation says about an entity: Some(Some(n))
/// for `USE x n`, Some(None) for `USE x none`, None when there is none.
fn used(blocks: &[Block], entity: Entity) -> Option<Option<u32>> {
    blocks.iter().find_map(|b| match b {
        Block::Use(u) if u.entity == entity => Some(u.number.trim().parse().ok()),
        _ => None,
    })
}

fn defined_here(blocks: &[Block], entity: Entity) -> Option<u32> {
    blocks.iter().filter_map(defines).find(|&(e, _)| e == entity).map(|(_, n)| n)
}

fn defines_reactant(blocks: &[Block]) -> bool {
    blocks
        .iter()
        .any(|b| defines(b).is_some_and(|(e, _)| REACTANTS.contains(&e)))
}

/// The water a simulation reacts when it names one with USE solution/mix.
pub fn simulation_subject(blocks: &[Block]) -> Option<(Entity, u32)> {
    [Entity::Mix, Entity::Solution]
        .into_iter()
        .find_map(|e| used(blocks, e).flatten().map(|n| (e, n)))
}

/// What a simulation reacts and with which reactants, following PHREEQC's
/// rules: USE first, else what is defined in the simulation. None when the
/// simulation runs no reaction.
fn simulation_setup(sim: &[Block]) -> Option<((Entity, u32), Vec<Option<u32>>)> {
    let reactants: Vec<Option<u32>> = REACTANTS
        .iter()
        .map(|&e| match used(sim, e) {
            Some(choice) => choice,
            None => defined_here(sim, e),
        })
        .collect();
    let subject = match simulation_subject(sim) {
        Some(s) => s,
        None => {
            let mix = match used(sim, Entity::Mix) {
                Some(_) => None,
                None => defined_here(sim, Entity::Mix),
            };
            if let Some(n) = mix {
                (Entity::Mix, n)
            } else if used(sim, Entity::Solution).is_some() || !reactants.iter().any(Option::is_some) {
                return None;
            } else {
                (Entity::Solution, defined_here(sim, Entity::Solution)?)
            }
        }
    };
    Some((subject, reactants))
}

/// A readable name per simulation ("NaCl · Calcite and CO2"), None for
/// simulations that react nothing. Reactants are named only where they
/// differ between simulations, so a temperature series shared by every
/// simulation doesn't repeat in every name.
pub fn simulation_names(blocks: &[Block]) -> Vec<Option<String>> {
    let setups: Vec<_> = split(blocks).into_iter().map(|r| simulation_setup(&blocks[r])).collect();
    let present: Vec<_> = setups.iter().flatten().collect();
    let varies = |i: usize| present.iter().any(|(_, r)| r[i] != present[0].1[i]);
    setups
        .iter()
        .map(|setup| {
            let ((entity, n), reactants) = setup.as_ref()?;
            let mut parts = vec![entity_name(blocks, *entity, *n)];
            for (i, &e) in REACTANTS.iter().enumerate() {
                if let (true, Some(m)) = (varies(i), reactants[i]) {
                    parts.push(entity_name(blocks, e, m));
                }
            }
            Some(parts.join(" · "))
        })
        .collect()
}

/// Simulations from combinations: every listed water with every
/// combination of the listed reactant choices.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    /// What each simulation reacts: a solution or a mix.
    pub runs: Vec<(Entity, u32)>,
    /// Per reactant type, the choices to combine; None = not used. An empty
    /// list counts as "not used".
    pub reactants: Vec<(Entity, Vec<Option<u32>>)>,
}

impl Plan {
    /// Each simulation's USE lines after the solution/mix, in order: water
    /// outermost, then the reactant types in `REACTANTS` order.
    fn combinations(&self) -> Vec<Vec<(Entity, u32)>> {
        let mut combos: Vec<Vec<(Entity, u32)>> = vec![Vec::new()];
        for (entity, choices) in &self.reactants {
            let choices: Vec<Option<u32>> = if choices.is_empty() { vec![None] } else { choices.clone() };
            combos = combos
                .into_iter()
                .flat_map(|combo| {
                    choices.iter().map(move |choice| {
                        let mut next = combo.clone();
                        next.extend(choice.map(|n| (*entity, n)));
                        next
                    })
                })
                .collect();
        }
        combos
    }

    pub fn simulation_count(&self) -> usize {
        self.runs.len() * self.combinations().len()
    }
}

/// Start of the trailing simulations that consist only of USE blocks naming
/// a solution or mix — the groups this helper writes.
pub fn generated_start(blocks: &[Block]) -> usize {
    let mut start = blocks.len();
    for sim in split(blocks).into_iter().rev() {
        let body = &blocks[sim.clone()];
        let only_use = body.iter().all(|b| matches!(b, Block::Use(_)) || is_end(b));
        if body.is_empty() || !only_use || simulation_subject(body).is_none() {
            break;
        }
        start = sim.start;
    }
    start
}

/// The plan the document currently expresses, read back from its trailing
/// USE groups; otherwise every solution and mix with each reactant type set
/// to its lowest number.
pub fn current_plan(blocks: &[Block]) -> Plan {
    let start = generated_start(blocks);
    let generated = &blocks[start..];
    if generated.is_empty() {
        let runs = [Entity::Solution, Entity::Mix]
            .into_iter()
            .flat_map(|e| defined_numbers(blocks, e).into_iter().map(move |n| (e, n)))
            .collect();
        let reactants = REACTANTS
            .iter()
            .map(|&e| (e, vec![defined_numbers(blocks, e).first().copied()]))
            .collect();
        return Plan { runs, reactants };
    }
    let sims: Vec<&[Block]> = split(generated).into_iter().map(|r| &generated[r]).collect();
    let mut runs = Vec::new();
    for sim in &sims {
        if let Some(subject) = simulation_subject(sim) {
            if !runs.contains(&subject) {
                runs.push(subject);
            }
        }
    }
    let reactants = REACTANTS
        .iter()
        .map(|&e| {
            let mut choices = Vec::new();
            for sim in &sims {
                let choice = used(sim, e).flatten();
                if !choices.contains(&choice) {
                    choices.push(choice);
                }
            }
            choices.sort();
            (e, choices)
        })
        .collect();
    Plan { runs, reactants }
}

fn is_use_none(block: &Block, entity: Entity) -> bool {
    matches!(block, Block::Use(u) if u.entity == entity && u.number.trim().is_empty())
}

/// What `apply` did besides writing the groups.
#[derive(Debug, Default, PartialEq)]
pub struct Applied {
    /// Index of the first new block.
    pub first_new: usize,
    /// `USE solution n` / `USE mix n` lines in the definitions simulation
    /// that were changed to `none`, e.g. "USE solution 1".
    pub neutralised: Vec<String>,
}

/// Replaces the trailing USE groups with one simulation per combination in
/// `plan`. The simulation before them is made to react nothing: a `USE
/// solution n` / `USE mix n` there (Insert → Use defaults to solution 1)
/// becomes `none`, since that water would otherwise run once more on top of
/// the generated groups, and `none` lines are added where needed.
pub fn apply(blocks: &mut Vec<Block>, plan: &Plan) -> Applied {
    blocks.truncate(generated_start(blocks));
    if plan.runs.is_empty() {
        // Undo: drop the closing END and the `none` lines added with it.
        if blocks.last().is_some_and(is_end) {
            blocks.pop();
        }
        let last = split(blocks).pop().unwrap_or(0..0);
        for entity in [Entity::Mix, Entity::Solution] {
            if let Some(i) = last.clone().rev().find(|&i| is_use_none(&blocks[i], entity)) {
                blocks.remove(i);
            }
        }
        return Applied {
            first_new: blocks.len(),
            neutralised: Vec::new(),
        };
    }

    let mut neutralised = Vec::new();
    if !blocks.is_empty() {
        if !blocks.last().is_some_and(is_end) {
            blocks.push(end_block());
        }
        let last = split(blocks).pop().unwrap_or(0..0);
        // One `none` per water type, at the first USE of it; later ones go.
        let mut seen = Vec::new();
        let mut duplicates = Vec::new();
        for i in last.clone() {
            if let Block::Use(u) = &mut blocks[i] {
                if !matches!(u.entity, Entity::Solution | Entity::Mix) {
                    continue;
                }
                if !u.number.trim().is_empty() {
                    neutralised.push(format!("USE {} {}", u.entity.keyword(), u.number.trim()));
                    u.number.clear();
                }
                if seen.contains(&u.entity) {
                    duplicates.push(i);
                } else {
                    seen.push(u.entity);
                }
            }
        }
        for i in duplicates.into_iter().rev() {
            blocks.remove(i);
        }
        let last = split(blocks).pop().unwrap_or(0..0);
        let sim = &blocks[last.clone()];
        let defines_mix = defined_here(sim, Entity::Mix).is_some();
        let mut nones = Vec::new();
        if (defines_reactant(sim) || defines_mix) && used(sim, Entity::Solution).is_none() {
            nones.push(Entity::Solution);
        }
        if defines_mix && used(sim, Entity::Mix).is_none() {
            nones.push(Entity::Mix);
        }
        for (k, entity) in nones.into_iter().enumerate() {
            blocks.insert(
                last.end - 1 + k,
                Block::Use(Use {
                    entity,
                    number: String::new(),
                }),
            );
        }
    }

    let first_new = blocks.len();
    let combos = plan.combinations();
    let mut first = true;
    for &(entity, n) in &plan.runs {
        for combo in &combos {
            if !first {
                blocks.push(end_block());
            }
            first = false;
            blocks.push(Block::Use(Use {
                entity,
                number: n.to_string(),
            }));
            for &(e, m) in combo {
                blocks.push(Block::Use(Use {
                    entity: e,
                    number: m.to_string(),
                }));
            }
        }
    }
    // The final END is written by the serializer, as for any document.
    Applied { first_new, neutralised }
}

/// Solutions that are defined but never reacted, worth a warning when some
/// simulation reacts without naming its water and so falls back on
/// PHREEQC's default (an input that names its waters everywhere is taken as
/// a deliberate choice). Transport, advection and inverse modelling use
/// solutions by number on their own, so they turn this check off.
pub fn unreacted_solutions(blocks: &[Block]) -> Vec<u32> {
    let uses_solutions_itself = blocks.iter().any(|b| match b {
        Block::Transport(_) | Block::Advection(_) | Block::InverseModeling(_) => true,
        Block::Raw(r) => {
            let header = r.header_line.trim().to_ascii_uppercase();
            ["TRANSPORT", "ADVECTION", "INVERSE_MODELING"].iter().any(|k| header.starts_with(k))
        }
        _ => false,
    });
    let defined = defined_numbers(blocks, Entity::Solution);
    if uses_solutions_itself || defined.len() < 2 {
        return Vec::new();
    }
    let mut reacted = BTreeSet::new();
    let mut relies_on_default = false;
    for sim in split(blocks) {
        let sim = &blocks[sim];
        let Some(((entity, n), _)) = simulation_setup(sim) else {
            continue;
        };
        match entity {
            Entity::Mix => {
                // A mix reacts every solution it takes part of.
                for b in blocks {
                    if let Block::Mix(m) = b {
                        if m.number == n {
                            reacted.extend(m.components.iter().filter_map(|c| c.solution_number.trim().parse::<u32>().ok()));
                        }
                    }
                }
            }
            _ => {
                reacted.insert(n);
            }
        }
        if simulation_subject(sim).is_none() {
            relies_on_default = true;
        }
    }
    if !relies_on_default {
        return Vec::new();
    }
    defined.into_iter().filter(|n| !reacted.contains(n)).collect()
}

/// "2", "2 and 5", "2, 3 and 5".
pub fn list_numbers(numbers: &[u32]) -> String {
    let words: Vec<String> = numbers.iter().map(u32::to_string).collect();
    match words.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::parse_document;
    use crate::serialize::document_to_pqi;

    const BRINES: &str = "SOLUTION 1\n    Na 100\nSOLUTION 2\n    Na 200\nSOLUTION 3\n    Na 300\n\
        SOLUTION 4\n    Na 400\nSOLUTION 5\n    Na 500\n\
        REACTION_TEMPERATURE 1\n    25 40 60 80 100 120\nREACTION_PRESSURE 1\n    10\nEND\n";

    fn plan_for(solutions: &[u32]) -> Plan {
        let mut plan = current_plan(&parse_document(BRINES).blocks);
        plan.runs = solutions.iter().map(|&n| (Entity::Solution, n)).collect();
        plan
    }

    fn tail(text: &str, n: usize) -> Vec<String> {
        let lines: Vec<String> = text.lines().filter(|l| !l.trim().is_empty()).map(str::to_string).collect();
        lines[lines.len() - n..].to_vec()
    }

    #[test]
    fn default_plan_runs_every_solution_with_defined_reactants() {
        let plan = current_plan(&parse_document(BRINES).blocks);
        assert_eq!(plan.runs, (1..=5).map(|n| (Entity::Solution, n)).collect::<Vec<_>>());
        let chosen: Vec<_> = plan.reactants.iter().filter(|(_, c)| c.iter().any(Option::is_some)).collect();
        assert_eq!(
            chosen,
            vec![
                &(Entity::ReactionTemperature, vec![Some(1)]),
                &(Entity::ReactionPressure, vec![Some(1)])
            ]
        );
        assert_eq!(plan.simulation_count(), 5);
    }

    #[test]
    fn writes_use_groups_and_round_trips() {
        let mut doc = parse_document(BRINES);
        apply(&mut doc.blocks, &plan_for(&[2, 5]));
        let text = document_to_pqi(&doc);
        assert_eq!(
            tail(&text, 10),
            [
                "USE solution none", "END",
                "USE solution 2", "USE reaction_temperature 1", "USE reaction_pressure 1", "END",
                "USE solution 5", "USE reaction_temperature 1", "USE reaction_pressure 1", "END",
            ],
            "{text}"
        );
        assert_eq!(split(&doc.blocks).len(), 3);

        // Reopening reads the same plan back, and applying it again is a no-op.
        let mut reopened = parse_document(&text);
        let plan = current_plan(&reopened.blocks);
        assert_eq!(plan.runs, vec![(Entity::Solution, 2), (Entity::Solution, 5)]);
        apply(&mut reopened.blocks, &plan);
        assert_eq!(document_to_pqi(&reopened), text);
    }

    #[test]
    fn reapplying_replaces_instead_of_appending() {
        let mut doc = parse_document(BRINES);
        apply(&mut doc.blocks, &plan_for(&[1, 2, 3, 4, 5]));
        apply(&mut doc.blocks, &plan_for(&[3]));
        let text = document_to_pqi(&doc);
        assert_eq!(text.matches("USE solution none").count(), 1);
        assert_eq!(text.matches("USE solution 3").count(), 1);
        assert_eq!(text.matches("USE solution 1\n").count(), 0);

        // Ticking nothing restores the original input.
        apply(&mut doc.blocks, &plan_for(&[]));
        assert_eq!(document_to_pqi(&doc), document_to_pqi(&parse_document(BRINES)));
    }

    const CARBONATE: &str = "SOLUTION 1 DW\nSOLUTION 2 NaCl\n    Na 657; Cl 657\n\
        EQUILIBRIUM_PHASES 1 CO2\n    CO2(g) -3.5 10\nEQUILIBRIUM_PHASES 2 Calcite\n    Calcite 0 10\n\
        REACTION_TEMPERATURE 1\n    25 50 75\nEND\n";

    #[test]
    fn combines_every_water_with_every_choice_and_reads_it_back() {
        let mut doc = parse_document(CARBONATE);
        let mut plan = current_plan(&doc.blocks);
        for (e, choices) in &mut plan.reactants {
            if *e == Entity::EquilibriumPhases {
                *choices = vec![None, Some(1), Some(2)];
            }
        }
        assert_eq!(plan.simulation_count(), 6);
        apply(&mut doc.blocks, &plan);
        let text = document_to_pqi(&doc);
        assert_eq!(
            tail(&text, 4),
            ["USE solution 2", "USE reaction_temperature 1", "USE equilibrium_phases 2", "END"],
            "{text}"
        );
        assert_eq!(text.matches("USE solution 1\n").count(), 3);
        assert_eq!(text.matches("USE equilibrium_phases 1").count(), 2);
        assert_eq!(current_plan(&parse_document(&text).blocks), plan);

        let names = simulation_names(&doc.blocks);
        let names: Vec<&str> = names.iter().flatten().map(String::as_str).collect();
        assert_eq!(names, ["DW", "DW · CO2", "DW · Calcite", "NaCl", "NaCl · CO2", "NaCl · Calcite"]);
    }

    const MIXES: &str = "SOLUTION 1 SW\nSOLUTION 2 FW\nMIX 1 FW:SW 50:50\n    2 0.5\n    1 0.5\n\
        REACTION_TEMPERATURE 1\n    25 50\nEND\n";

    #[test]
    fn mixes_are_waters_too_and_the_definitions_react_nothing() {
        let mut doc = parse_document(MIXES);
        let plan = current_plan(&doc.blocks);
        assert_eq!(plan.runs, vec![(Entity::Solution, 1), (Entity::Solution, 2), (Entity::Mix, 1)]);
        apply(&mut doc.blocks, &plan);
        let text = document_to_pqi(&doc);
        assert!(text.contains("USE solution none\nUSE mix none\nEND\n"), "{text}");
        assert!(text.contains("USE mix 1\nUSE reaction_temperature 1\n"), "{text}");
        let names = simulation_names(&doc.blocks);
        assert_eq!(names[0], None);
        assert_eq!(names[3].as_deref(), Some("FW:SW 50:50"));
        assert_eq!(current_plan(&parse_document(&text).blocks).runs, plan.runs);
    }

    #[test]
    fn warns_about_solutions_never_reacted() {
        let doc = parse_document(BRINES);
        assert_eq!(unreacted_solutions(&doc.blocks), vec![2, 3, 4, 5]);
        let mut fixed = doc.clone();
        apply(&mut fixed.blocks, &plan_for(&[1, 2, 3, 4, 5]));
        assert!(unreacted_solutions(&fixed.blocks).is_empty());
        // Leaving some out on purpose is not warned about.
        let mut chosen = doc.clone();
        apply(&mut chosen.blocks, &plan_for(&[2, 5]));
        assert!(unreacted_solutions(&chosen.blocks).is_empty());
        // Speciation only: nothing is reacted, nothing to warn about.
        let speciation = parse_document("SOLUTION 1\nSOLUTION 2\nEND\n");
        assert!(unreacted_solutions(&speciation.blocks).is_empty());
    }

    const FIVE: &str = "SOLUTION 1 DW\nSOLUTION 2 NaCl\nSOLUTION 3 NaCl+Ca\nSOLUTION 4 NaCl+Mg\nSOLUTION 5 SW\n\
        REACTION_TEMPERATURE 1\n    25 50 75 100 125 150\nREACTION_PRESSURE 1\n    10\nEND\n";

    /// The lines of the first simulation and the number of simulations.
    fn shape(text: &str) -> (Vec<String>, usize) {
        let doc = parse_document(text);
        let sims = split(&doc.blocks);
        let first = document_to_pqi(&crate::model::Document {
            title: String::new(),
            blocks: doc.blocks[sims[0].clone()].to_vec(),
        });
        (first.lines().filter(|l| l.starts_with("USE")).map(str::to_string).collect(), sims.len())
    }

    #[test]
    fn one_water_then_all_five_replaces_and_keeps_definitions_inert() {
        let mut doc = parse_document(FIVE);
        let mut plan = current_plan(&doc.blocks);
        plan.runs = vec![(Entity::Solution, 1)];
        apply(&mut doc.blocks, &plan);
        let mut plan = current_plan(&doc.blocks);
        plan.runs = (1..=5).map(|n| (Entity::Solution, n)).collect();
        let applied = apply(&mut doc.blocks, &plan);
        assert!(applied.neutralised.is_empty());
        let text = document_to_pqi(&doc);
        assert_eq!(shape(&text), (vec!["USE solution none".to_string()], 6), "{text}");
        assert_eq!(text.matches("USE solution 1\n").count(), 1, "{text}");
    }

    #[test]
    fn a_use_solution_left_in_the_definitions_becomes_none() {
        for extra in ["USE solution 1\n", "USE solution 1\nUSE solution none\n", "USE solution none\nUSE solution 3\n"] {
            let mut doc = parse_document(&FIVE.replace("    10\nEND", &format!("    10\n{extra}END")));
            let mut plan = current_plan(&doc.blocks);
            plan.runs = (1..=5).map(|n| (Entity::Solution, n)).collect();
            let applied = apply(&mut doc.blocks, &plan);
            assert_eq!(applied.neutralised.len(), 1, "{extra}");
            let text = document_to_pqi(&doc);
            assert_eq!(shape(&text), (vec!["USE solution none".to_string()], 6), "{text}");
            assert_eq!(text.matches("USE solution 1\n").count(), 1, "{text}");
        }
        // A stale `USE mix 2` in the definitions is neutralised too.
        let mut doc = parse_document(MIXES.replace("    25 50\nEND", "    25 50\nUSE mix 1\nEND").as_str());
        let plan = current_plan(&doc.blocks);
        let applied = apply(&mut doc.blocks, &plan);
        assert_eq!(applied.neutralised, vec!["USE mix 1".to_string()]);
        assert!(document_to_pqi(&doc).contains("USE mix none\nUSE solution none\nEND\n"));
    }

    #[test]
    fn lists_numbers_in_prose() {
        assert_eq!(list_numbers(&[2]), "2");
        assert_eq!(list_numbers(&[2, 5]), "2 and 5");
        assert_eq!(list_numbers(&[2, 3, 5]), "2, 3 and 5");
    }
}

