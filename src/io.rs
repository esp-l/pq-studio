//! Save/Open for this app's own `.pqi` files.
//!
//! This is a round-trip parser for exactly the format `serialize::document_to_pqi`
//! emits, not a general PHREEQC input parser — field-level content inside a
//! *structured* block (one of the 17 types in model.rs's `Block` enum) is only
//! guaranteed to parse back correctly for this app's own output. Top-level
//! *structure*, though, is preserved for the full real PHREEQC keyword set: a
//! keyword this app doesn't model (ISOTOPES, USE, PITZER, PHASES, RATES, the
//! *_MASTER_SPECIES/*_SPECIES database-editing keywords, etc.) becomes a
//! `Block::Raw` carrying its header line and body verbatim, rather than being
//! silently dropped or merged into whatever structured block happened to be
//! open ("never destroy what it can't parse"). DATABASE is the one exception:
//! it's recognized and intentionally dropped rather than raw-preserved, since
//! this app manages its own database path separately (see phreeqc.rs) and
//! reintroducing a stale embedded DATABASE line is exactly the bug that was
//! found and guarded against there.

use crate::model::*;
use std::path::Path;

pub fn save(doc: &Document, path: &Path) -> std::io::Result<()> {
    std::fs::write(path, crate::serialize::document_to_pqi(doc))
}

pub fn load(path: &Path) -> std::io::Result<Document> {
    let text = std::fs::read_to_string(path)?;
    Ok(parse_document(&text))
}

fn split_first_token(line: &str) -> (&str, &str) {
    let line = line.trim();
    match line.find(char::is_whitespace) {
        Some(i) => (&line[..i], line[i..].trim_start()),
        None => (line, ""),
    }
}

pub fn parse_document(text: &str) -> Document {
    let mut doc = Document {
        title: String::new(),
        blocks: Vec::new(),
    };

    // Structured: this app parses these into typed Block variants.
    let structured_keywords = [
        "TITLE",
        "SOLUTION",
        "EQUILIBRIUM_PHASES",
        "REACTION_TEMPERATURE",
        "REACTION_PRESSURE",
        "REACTION",
        "GAS_PHASE",
        "EXCHANGE",
        "SURFACE",
        "KINETICS",
        "SELECTED_OUTPUT",
        "MIX",
        "ADVECTION",
        "TRANSPORT",
        "INCREMENTAL_REACTIONS",
        "INVERSE_MODELING",
        "SOLID_SOLUTIONS",
        "SOLUTION_SPREAD",
        "SAVE",
        "USE",
        "DUMP",
        "KNOBS",
        "PRINT",
    ];
    // The complete real PHREEQC top-level keyword set (from the real GUI's
    // tree-context menu, PhreeqcI menu resource 148, plus
    // DATABASE/END/TITLE) — used only to recognize where a new top-level
    // block starts, so unstructured keywords route to Block::Raw instead of
    // silently merging into whatever structured block was open. SOLID_SOLUTION
    // (singular) and EQUILIBRIUM/EQUILIBRIA/PURE_PHASES/PURE are documented
    // PHREEQC aliases for keywords above that this app doesn't match — an
    // input file using an alias spelling will misparse, a known limitation.
    let raw_eligible_keywords = [
        "ISOTOPES",
        "ISOTOPE_ALPHAS",
        "ISOTOPE_RATIOS",
        "COPY",
        "DELETE",
        "PITZER",
        "SIT",
        "PHASES",
        "RATES",
        "NAMED_EXPRESSIONS",
        "CALCULATE_VALUES",
        "RUN_CELLS",
        "USER_GRAPH",
        "USER_PRINT",
        "USER_PUNCH",
        "SOLUTION_MASTER_SPECIES",
        "SOLUTION_SPECIES",
        "EXCHANGE_MASTER_SPECIES",
        "EXCHANGE_SPECIES",
        "SURFACE_MASTER_SPECIES",
        "SURFACE_SPECIES",
        "LLNL_AQUEOUS_MODEL_PARAMETERS",
        "MEAN_GAMMAS",
        "MIX_EQUILIBRIUM_PHASE",
        "MIX_EXCHANGE",
        "MIX_GAS_PHASE",
        "MIX_KINETICS",
        "MIX_SOLID_SOLUTION",
        "MIX_SOLUTION",
        "MIX_SURFACE",
        "RATE_PARAMETERS_HERMANSKA",
        "RATE_PARAMETERS_PK",
        "RATE_PARAMETERS_SVD",
    ];
    let keywords: Vec<&str> = structured_keywords
        .iter()
        .chain(raw_eligible_keywords.iter())
        .chain(["DATABASE", "END"].iter())
        .copied()
        .collect();

    let mut current_kw: Option<&str> = None;
    let mut header_rest = String::new();
    let mut full_header_line = String::new();
    let mut body_lines: Vec<String> = Vec::new();

    let flush = |kw: Option<&str>,
                 header_rest: &str,
                 full_header_line: &str,
                 body: &[String],
                 doc: &mut Document| {
        let Some(kw) = kw else { return };
        if kw == "DATABASE" {
            return;
        }
        if kw == "END" {
            // Simulation boundary: kept so multi-simulation files stay
            // multi-simulation. A trailing END is implied by the serializer.
            doc.blocks.push(Block::Raw(Raw {
                header_line: "END".to_string(),
                body: String::new(),
            }));
            return;
        }
        // Structured parsers see data only: '#' starts a comment in PHREEQC
        // input, and a comment line would otherwise parse as a bogus row
        // (e.g. an element named "#"). Raw blocks below keep `body` verbatim.
        let data: Vec<String> = body
            .iter()
            .flat_map(|line| {
                line.split('#')
                    .next()
                    .unwrap_or("")
                    .split(';')
                    .map(|stmt| format!("    {}", stmt.trim()))
                    .collect::<Vec<_>>()
            })
            .filter(|line| !line.trim().is_empty())
            .collect();
        let body_verbatim = body;
        let body = &data[..];
        let (number_tok, description) = split_first_token(header_rest);
        let number: u32 = number_tok.parse().unwrap_or(1);
        let description = description.to_string();

        match kw {
            "TITLE" => {
                let mut title = header_rest.trim().to_string();
                for line in body_verbatim {
                    title.push('\n');
                    title.push_str(line.trim_end());
                }
                if doc.title.is_empty() && doc.blocks.is_empty() {
                    doc.title = title;
                } else {
                    // A later simulation's title: keep it where it was.
                    doc.blocks.push(Block::Raw(Raw {
                        header_line: full_header_line.to_string(),
                        body: body_verbatim.join("\n"),
                    }));
                }
            }
            "SOLUTION" => match parse_solution(number, description, body) {
                Some(sol) => doc.blocks.push(Block::Solution(sol)),
                None => doc.blocks.push(Block::Raw(Raw {
                    header_line: full_header_line.to_string(),
                    body: body_verbatim.join("\n"),
                })),
            },
            "EQUILIBRIUM_PHASES" => {
                doc.blocks
                    .push(Block::EquilibriumPhases(parse_equilibrium_phases(
                        number,
                        description,
                        body,
                    )))
            }
            "REACTION" => match parse_reaction(number, description, body) {
                Some(r) => doc.blocks.push(Block::Reaction(r)),
                None => doc.blocks.push(Block::Raw(Raw {
                    header_line: full_header_line.to_string(),
                    body: body_verbatim.join("\n"),
                })),
            },
            "REACTION_TEMPERATURE" => {
                doc.blocks
                    .push(Block::ReactionTemperature(parse_reaction_temperature(
                        number,
                        description,
                        body,
                    )))
            }
            "REACTION_PRESSURE" => {
                doc.blocks
                    .push(Block::ReactionPressure(parse_reaction_pressure(
                        number,
                        description,
                        body,
                    )))
            }
            "GAS_PHASE" => {
                doc.blocks
                    .push(Block::GasPhase(parse_gas_phase(number, description, body)))
            }
            "EXCHANGE" => match parse_exchange(number, description, body) {
                Some(x) => doc.blocks.push(Block::Exchange(x)),
                None => doc.blocks.push(Block::Raw(Raw {
                    header_line: full_header_line.to_string(),
                    body: body_verbatim.join("\n"),
                })),
            },
            "SURFACE" => doc
                .blocks
                .push(Block::Surface(parse_surface(number, description, body))),
            "KINETICS" => {
                doc.blocks
                    .push(Block::Kinetics(parse_kinetics(number, description, body)))
            }
            "SELECTED_OUTPUT" => doc
                .blocks
                .push(Block::SelectedOutput(parse_selected_output(number, body))),
            "MIX" => doc
                .blocks
                .push(Block::Mix(parse_mix(number, description, body))),
            "ADVECTION" => doc.blocks.push(Block::Advection(parse_advection(body))),
            "TRANSPORT" => doc.blocks.push(Block::Transport(parse_transport(body))),
            "INCREMENTAL_REACTIONS" => {
                doc.blocks
                    .push(Block::IncrementalReactions(parse_incremental_reactions(
                        header_rest,
                    )))
            }
            "INVERSE_MODELING" => doc
                .blocks
                .push(Block::InverseModeling(parse_inverse_modeling(
                    number,
                    description,
                    body,
                ))),
            "SOLID_SOLUTIONS" => doc.blocks.push(Block::SolidSolutions(parse_solid_solutions(
                number,
                description,
                body,
            ))),
            "SOLUTION_SPREAD" => doc
                .blocks
                .push(Block::SolutionSpread(parse_solution_spread(body))),
            "SAVE" => doc.blocks.push(Block::Save(parse_save(header_rest))),
            "USE" => doc.blocks.push(Block::Use(parse_use(header_rest))),
            "DUMP" => doc.blocks.push(Block::Dump(parse_dump(body))),
            "KNOBS" => doc.blocks.push(Block::Knobs(parse_knobs(body))),
            "PRINT" => doc.blocks.push(Block::Print(parse_print(body))),
            _ => {
                // Recognized real PHREEQC keyword this app doesn't structurally
                // model — preserve verbatim rather than drop (see module doc).
                doc.blocks.push(Block::Raw(Raw {
                    header_line: full_header_line.to_string(),
                    body: body_verbatim.join("\n"),
                }));
            }
        }

        // Keep a structured block only if writing it back preserves what the
        // file said; otherwise keep the user's text verbatim (see
        // `statements_preserved`).
        if kw != "TITLE" {
            if let Some(last) = doc.blocks.last() {
                if !matches!(last, Block::Raw(_))
                    && !statements_preserved(kw, full_header_line, body, last)
                {
                    doc.blocks.pop();
                    doc.blocks.push(Block::Raw(Raw {
                        header_line: full_header_line.to_string(),
                        body: body_verbatim.join("\n"),
                    }));
                }
            }
        }
    };

    // Blocks whose body is BASIC code, where ';' is left alone.
    const BASIC_BLOCKS: [&str; 5] = ["USER_PUNCH", "USER_PRINT", "USER_GRAPH", "RATES", "CALCULATE_VALUES"];
    let keyword_of = |segment: &str| -> Option<&str> {
        let (first_word, _) = split_first_token(segment.trim());
        let canonical = canonical_keyword(first_word)?;
        keywords.iter().find(|kw| **kw == canonical).copied()
    };

    for raw_line in text.lines() {
        // PHREEQC recognises a keyword as the first word of a logical line, in
        // any case and after any indentation, and ';' separates logical lines
        // (several blocks can share one physical line).
        // '#' comments out the rest of the physical line, including any ';'
        // after it, so only the code part is split.
        let (code, comment) = match raw_line.find('#') {
            Some(i) => (&raw_line[..i], Some(&raw_line[i..])),
            None => (raw_line, None),
        };
        let segments: Vec<&str> = code.split(';').collect();
        let in_basic = current_kw.is_some_and(|k| BASIC_BLOCKS.contains(&k));
        if segments.len() == 1 || (in_basic && keyword_of(segments[0]).is_none()) {
            if let Some(kw) = keyword_of(raw_line) {
                flush(current_kw, &header_rest, &full_header_line, &body_lines, &mut doc);
                let trimmed = raw_line.trim();
                current_kw = Some(kw);
                header_rest = split_first_token(trimmed).1.to_string();
                full_header_line = trimmed.to_string();
                body_lines.clear();
            } else if current_kw.is_some() && !raw_line.trim().is_empty() {
                body_lines.push(raw_line.to_string());
            }
            continue;
        }
        for segment in segments {
            let trimmed = segment.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(kw) = keyword_of(trimmed) {
                flush(current_kw, &header_rest, &full_header_line, &body_lines, &mut doc);
                current_kw = Some(kw);
                header_rest = split_first_token(trimmed).1.to_string();
                full_header_line = trimmed.to_string();
                body_lines.clear();
            } else if current_kw.is_some() {
                body_lines.push(format!("    {trimmed}"));
            }
        }
        if let (Some(comment), Some(_)) = (comment, current_kw) {
            body_lines.push(format!("    {comment}"));
        }
    }
    flush(
        current_kw,
        &header_rest,
        &full_header_line,
        &body_lines,
        &mut doc,
    );

    // The final END is implied (the serializer always writes one); only END
    // boundaries between simulations are kept as blocks.
    while matches!(doc.blocks.last(), Some(Block::Raw(r)) if r.header_line.eq_ignore_ascii_case("END") && r.body.trim().is_empty())
    {
        doc.blocks.pop();
    }
    if doc.blocks.is_empty() {
        doc.blocks.push(Block::Solution(Solution::default()));
    }
    doc
}

/// The canonical keyword for `word` (case-insensitive), resolving PHREEQC's
/// documented alternative spellings.
fn canonical_keyword(word: &str) -> Option<String> {
    if word.is_empty() || word.starts_with('-') {
        return None;
    }
    let upper = word.to_uppercase();
    Some(
        match upper.as_str() {
            "EQUILIBRIUM" | "EQUILIBRIA" | "PURE_PHASES" | "PURE" => "EQUILIBRIUM_PHASES",
            "SOLID_SOLUTION" => "SOLID_SOLUTIONS",
            "REACTION_TEMPERATURES" => "REACTION_TEMPERATURE",
            "REACTION_PRESSURES" => "REACTION_PRESSURE",
            "SELECTED_OUT" => "SELECTED_OUTPUT",
            other => other,
        }
        .to_string(),
    )
}

/// Tokens of one PHREEQC statement, compared loosely: case-insensitive, an
/// identifier's optional leading '-' ignored, comments removed.
fn normalized_statements(line: &str) -> Vec<Vec<String>> {
    line.split('#')
        .next()
        .unwrap_or("")
        .split(';')
        .map(|stmt| {
            stmt.split_whitespace()
                .map(|t| {
                    let t = t.to_lowercase();
                    let t = t.trim_start_matches('-');
                    match t {
                        "equilibrium" | "equilibria" | "pure_phases" | "pure" => "equilibrium_phases".to_string(),
                        "solid_solution" => "solid_solutions".to_string(),
                        "temperature" => "temp".to_string(),
                        "dens" => "density".to_string(),
                        "unit" => "units".to_string(),
                        other => other.to_string(),
                    }
                })
                .collect::<Vec<_>>()
        })
        .filter(|tokens| !tokens.is_empty())
        .collect()
}

fn is_subsequence(needle: &[String], hay: &[String]) -> bool {
    let mut it = hay.iter();
    needle.iter().all(|n| it.any(|h| h == n))
}

/// True when every statement of the original block (header and data lines)
/// reappears, in order and starting with the same keyword, in some line of
/// the block as this app would write it. Added defaults are fine; anything
/// dropped, rewritten or reordered means the structured form can't represent
/// the block faithfully.
/// Keywords whose options persist into later simulations: writing an option
/// the file didn't mention (even at its default) would override what an
/// earlier simulation set, so these must match exactly in both directions.
const STATEFUL_KEYWORDS: [&str; 5] = ["PRINT", "SELECTED_OUTPUT", "TRANSPORT", "ADVECTION", "KNOBS"];

/// SELECTED_OUTPUT's abbreviations and bare switches (`-si`, `-pH` meaning
/// `-pH true`) in the form this app writes them.
fn canonical_selected_output(mut stmt: Vec<String>) -> Vec<String> {
    let Some(first) = stmt.first_mut() else {
        return stmt;
    };
    let canonical = match first.as_str() {
        "si" | "saturation_index" => "saturation_indices",
        "gas" => "gases",
        "molality" => "molalities",
        "activity" => "activities",
        "total" => "totals",
        _ => "",
    };
    if !canonical.is_empty() {
        *first = canonical.to_string();
    }
    let is_switch = first == "reset"
        || first == "temp"
        || SELECTED_OUTPUT_FLAGS.iter().any(|(name, _)| name.eq_ignore_ascii_case(first));
    if is_switch {
        match stmt.get(1).map(String::as_str) {
            None => stmt.push("true".into()),
            Some(v) if v.starts_with('t') => stmt[1] = "true".into(),
            Some(v) if v.starts_with('f') => stmt[1] = "false".into(),
            _ => {}
        }
    }
    stmt
}

fn statements_preserved(keyword: &str, header_line: &str, data: &[String], block: &Block) -> bool {
    let written = crate::serialize::block_to_pqi(block);
    let mut written_lines = written.lines();
    let written_header: Vec<Vec<String>> = written_lines.next().map(normalized_statements).unwrap_or_default();
    let written_body: Vec<Vec<String>> = written_lines.flat_map(normalized_statements).collect();
    let header_ok = normalized_statements(header_line)
        .iter()
        .all(|stmt| written_header.iter().any(|w| is_subsequence(stmt, w)));
    let mut original: Vec<Vec<String>> = data.iter().flat_map(|l| normalized_statements(l)).collect();
    if keyword == "SELECTED_OUTPUT" {
        original = original.into_iter().map(canonical_selected_output).collect();
    }
    let preserved = header_ok
        && original.iter().all(|stmt| {
            written_body
                .iter()
                .any(|w| w.first() == stmt.first() && is_subsequence(&stmt, w))
        });
    if !STATEFUL_KEYWORDS.contains(&keyword) {
        return preserved;
    }
    let mut a = original;
    let mut b = written_body;
    a.sort();
    b.sort();
    preserved && a == b
}

/// None when the block uses something the Solution form doesn't model
/// (isotopes, -water, ...); the caller then keeps it as verbatim text.
fn parse_solution(number: u32, description: String, body: &[String]) -> Option<Solution> {
    let mut sol = Solution {
        number,
        description,
        ..Solution::default()
    };
    sol.elements.clear();
    for line in body {
        let (first, rest) = split_first_token(line);
        let identifier = first.trim_start_matches('-').to_lowercase();
        match identifier.as_str() {
            "temp" | "temperature" => sol.temperature_c = rest.to_string(),
            "ph" => {
                // "7 charge", or a value fixed by a phase ("8.2 CO2(g) -3.5").
                let (val, tail) = split_first_token(rest);
                if tail.trim() == "charge" {
                    sol.ph = val.to_string();
                    sol.charge_balance = "pH".to_string();
                } else {
                    sol.ph = rest.to_string();
                }
            }
            "pe" => sol.pe = rest.to_string(),
            "redox" => sol.redox_couple = rest.to_string(),
            "density" | "dens" => sol.density = rest.to_string(),
            "units" | "unit" => sol.default_units = rest.to_string(),
            "" => {}
            _ if first.starts_with('-') => return None,
            _ => {
                // Element concentration line: "Elem conc [units] [as form] [redox] [charge]"
                let tokens: Vec<&str> = rest.split_whitespace().collect();
                if tokens.is_empty() {
                    continue;
                }
                let mut el = ElementConcentration {
                    element: first.to_string(),
                    ..Default::default()
                };
                el.concentration = tokens[0].to_string();
                let mut i = 1;
                let mut charge = false;
                while i < tokens.len() {
                    match tokens[i] {
                        "as" if i + 1 < tokens.len() => {
                            el.as_form = tokens[i + 1].to_string();
                            i += 2;
                        }
                        "charge" => {
                            charge = true;
                            i += 1;
                        }
                        other => {
                            if el.unit_override.is_empty() {
                                el.unit_override = other.to_string();
                            } else if el.redox_couple.is_empty() {
                                el.redox_couple = other.to_string();
                            }
                            i += 1;
                        }
                    }
                }
                if charge {
                    sol.charge_balance = el.element.clone();
                }
                sol.elements.push(el);
            }
        }
    }
    Some(sol)
}

fn parse_equilibrium_phases(
    number: u32,
    description: String,
    body: &[String],
) -> EquilibriumPhases {
    let mut eq = EquilibriumPhases {
        number,
        description,
        phases: Vec::new(),
    };
    for line in body {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }
        eq.phases.push(PhaseAmount {
            phase_name: tokens[0].to_string(),
            saturation_index: tokens.get(1).unwrap_or(&"0.0").to_string(),
            amount: tokens.get(2).unwrap_or(&"").to_string(),
        });
    }
    eq
}

/// None for step lists ("0.1 0.2 0.5 moles", "3*0.1") the form can't hold.
fn parse_reaction(number: u32, description: String, body: &[String]) -> Option<Reaction> {
    let mut r = Reaction {
        number,
        description,
        ..Reaction::default()
    };
    r.reactants.clear();
    for line in body {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }
        let starts_with_number = tokens[0].parse::<f64>().is_ok();
        if starts_with_number {
            // "amount [units] [in N steps]"
            r.amount = tokens[0].to_string();
            let mut rest = &tokens[1..];
            if let Some(unit) = rest.first().filter(|t| **t != "in") {
                // Only PHREEQC's amount units; anything else is a step list
                // ("0 27*1", "0.1 0.2 0.5") the form can't represent.
                let u = unit.to_lowercase();
                let is_unit = ["mol", "mmol", "umol", "millimol", "micromol"]
                    .iter()
                    .any(|prefix| u.starts_with(prefix));
                if !is_unit {
                    return None;
                }
                r.units = unit.to_string();
                rest = &rest[1..];
            }
            match rest {
                [] => r.steps = "1".to_string(),
                ["in", n, "steps"] | ["in", n, "step"] | ["in", n] => r.steps = n.to_string(),
                _ => return None,
            }
        } else if tokens.len() >= 2 {
            r.reactants.push(Reactant {
                name: tokens[0].to_string(),
                relative_stoichiometry: tokens[1].to_string(),
            });
        } else {
            r.reactants.push(Reactant {
                name: tokens[0].to_string(),
                relative_stoichiometry: "1.0".to_string(),
            });
        }
    }
    Some(r)
}

fn parse_steps_spec_lines(body: &[String]) -> StepsSpec {
    if body.is_empty() {
        return StepsSpec::default();
    }
    let tokens: Vec<&str> = body[0].split_whitespace().collect();
    if tokens.len() >= 4 && tokens.contains(&"in") && tokens.contains(&"steps") {
        let pos = tokens.iter().position(|t| *t == "in").unwrap();
        StepsSpec::Linear {
            start: tokens[0].to_string(),
            end: tokens[1].to_string(),
            steps: tokens.get(pos + 1).unwrap_or(&"1").to_string(),
        }
    } else {
        StepsSpec::Explicit(tokens.iter().map(|s| s.to_string()).collect())
    }
}

fn parse_reaction_temperature(
    number: u32,
    description: String,
    body: &[String],
) -> ReactionTemperature {
    ReactionTemperature {
        number,
        description,
        values: parse_steps_spec_lines(body),
    }
}

fn parse_reaction_pressure(number: u32, description: String, body: &[String]) -> ReactionPressure {
    ReactionPressure {
        number,
        description,
        values: parse_steps_spec_lines(body),
    }
}

fn parse_gas_phase(number: u32, description: String, body: &[String]) -> GasPhase {
    let mut g = GasPhase {
        number,
        description,
        components: Vec::new(),
        ..GasPhase::default()
    };
    for line in body {
        let (first, rest) = split_first_token(line);
        match first {
            "-fixed_volume" => g.fixed_volume = true,
            "-fixed_pressure" => g.fixed_volume = false,
            "-pressure" => g.pressure = rest.to_string(),
            "-volume" => g.volume = rest.to_string(),
            "-temperature" => g.temperature_c = rest.to_string(),
            _ => {
                g.components.push(GasComponent {
                    phase_name: first.to_string(),
                    partial_pressure: rest.to_string(),
                });
            }
        }
    }
    g
}

/// None for options the Exchange form doesn't model (-equilibrate, …).
fn parse_exchange(number: u32, description: String, body: &[String]) -> Option<Exchange> {
    let mut ex = Exchange {
        number,
        description,
        components: Vec::new(),
    };
    for line in body {
        let (first, rest) = split_first_token(line);
        if first.is_empty() {
            continue;
        }
        if first.starts_with('-') {
            return None;
        }
        ex.components.push(ExchangeComponent {
            exchange_formula: first.to_string(),
            amount: rest.to_string(),
        });
    }
    Some(ex)
}

fn parse_surface(number: u32, description: String, body: &[String]) -> Surface {
    let mut s = Surface {
        number,
        description,
        sites: Vec::new(),
        ..Surface::default()
    };
    for line in body {
        let (first, rest) = split_first_token(line);
        match first {
            "-equilibrate" => s.equilibrate_with_solution = rest.to_string(),
            "-no_edl" => s.electrostatics = SurfaceElectrostatics::None,
            "-diffuse_layer" => {
                s.electrostatics = SurfaceElectrostatics::DiffuseLayer;
                s.diffuse_layer_thickness = rest.to_string();
            }
            "" => {}
            _ => {
                let tokens: Vec<&str> = rest.split_whitespace().collect();
                s.sites.push(SurfaceSite {
                    surface_formula: first.to_string(),
                    amount: tokens.first().unwrap_or(&"").to_string(),
                    specific_area_m2_g: tokens.get(1).unwrap_or(&"").to_string(),
                    mass_g: tokens.get(2).unwrap_or(&"").to_string(),
                });
            }
        }
    }
    s
}

fn parse_kinetics(number: u32, description: String, body: &[String]) -> Kinetics {
    let mut k = Kinetics {
        number,
        description,
        reactants: Vec::new(),
        ..Kinetics::default()
    };
    for line in body {
        let (first, rest) = split_first_token(line);
        match first {
            "-m" => {
                if let Some(last) = k.reactants.last_mut() {
                    last.m = rest.to_string();
                }
            }
            "-m0" => {
                if let Some(last) = k.reactants.last_mut() {
                    last.m0 = rest.to_string();
                }
            }
            "-parms" => {
                if let Some(last) = k.reactants.last_mut() {
                    last.parms = rest.to_string();
                }
            }
            "-tol" => {
                if let Some(last) = k.reactants.last_mut() {
                    last.tol = rest.to_string();
                }
            }
            "-steps" => {
                let tokens: Vec<&str> = rest.split_whitespace().collect();
                if let Some((unit, values)) = tokens.split_last() {
                    k.step_units = unit.to_string();
                    k.steps = StepsSpec::Explicit(values.iter().map(|s| s.to_string()).collect());
                }
            }
            "" => {}
            _ => k.reactants.push(KineticReactant {
                rate_name: first.to_string(),
                ..Default::default()
            }),
        }
    }
    k
}

fn parse_selected_output(number: u32, body: &[String]) -> SelectedOutput {
    let mut so = SelectedOutput::empty(number);
    for line in body {
        let (first, rest) = split_first_token(line);
        let identifier = first.trim_start_matches('-').to_lowercase();
        let identifier = match identifier.as_str() {
            "temp" => "temperature".to_string(),
            "si" | "saturation_index" => "saturation_indices".to_string(),
            "equilibrium" | "equilibria" | "pure_phases" | "pure" => "equilibrium_phases".to_string(),
            "gas" => "gases".to_string(),
            "molality" => "molalities".to_string(),
            "activity" => "activities".to_string(),
            "total" => "totals".to_string(),
            other => other.to_string(),
        };
        if let Some(i) = SELECTED_OUTPUT_FLAGS.iter().position(|(name, _)| name.eq_ignore_ascii_case(&identifier)) {
            so.flags[i] = Some(!rest.trim().to_ascii_lowercase().starts_with('f'));
            continue;
        }
        match identifier.as_str() {
            "reset" => so.reset = Some(!rest.trim().to_ascii_lowercase().starts_with('f')),
            "file" => so.file = rest.to_string(),
            "totals" => so.totals = rest.to_string(),
            "molalities" => so.molalities = rest.to_string(),
            "activities" => so.activities = rest.to_string(),
            "saturation_indices" => so.saturation_indices = rest.to_string(),
            "equilibrium_phases" => so.equilibrium_phases = rest.to_string(),
            "gases" => so.gases = rest.to_string(),
            _ => {}
        }
    }
    so
}

fn parse_mix(number: u32, description: String, body: &[String]) -> Mix {
    let mut m = Mix {
        number,
        description,
        components: Vec::new(),
    };
    for line in body {
        let (first, rest) = split_first_token(line);
        if first.is_empty() {
            continue;
        }
        m.components.push(MixComponent {
            solution_number: first.to_string(),
            fraction: rest.to_string(),
        });
    }
    m
}

fn entity_from_keyword(kw: &str) -> Entity {
    match kw.to_lowercase().as_str() {
        "equilibrium_phases" | "equilibrium" | "equilibria" | "pure_phases" | "pure" => {
            Entity::EquilibriumPhases
        }
        "exchange" => Entity::Exchange,
        "gas_phase" => Entity::GasPhase,
        "kinetics" => Entity::Kinetics,
        "mix" => Entity::Mix,
        "reaction" => Entity::Reaction,
        "reaction_pressure" => Entity::ReactionPressure,
        "reaction_temperature" => Entity::ReactionTemperature,
        "solid_solution" | "solid_solutions" => Entity::SolidSolution,
        "surface" => Entity::Surface,
        _ => Entity::Solution,
    }
}

fn parse_save(header_rest: &str) -> Save {
    let (kw, number) = split_first_token(header_rest);
    Save {
        entity: entity_from_keyword(kw),
        number: number.to_string(),
    }
}

fn parse_use(header_rest: &str) -> Use {
    let (kw, number) = split_first_token(header_rest);
    let number = if number.trim().eq_ignore_ascii_case("none") {
        String::new()
    } else {
        number.to_string()
    };
    Use {
        entity: entity_from_keyword(kw),
        number,
    }
}

fn parse_dump(body: &[String]) -> Dump {
    let mut d = Dump {
        entities: Vec::new(),
        all: false,
        ..Dump::default()
    };
    for e in Entity::ALL {
        d.entities.push(DumpEntitySelection {
            entity: *e,
            enabled: false,
            ranges: String::new(),
        });
    }
    for line in body {
        let (first, rest) = split_first_token(line);
        match first {
            "-file" => d.file = rest.to_string(),
            "-append" => {
                d.append = rest.trim().eq_ignore_ascii_case("true") || rest.trim().is_empty()
            }
            "-all" => d.all = true,
            _ if first.starts_with('-') => {
                let kw = &first[1..];
                if let Some(sel) = d.entities.iter_mut().find(|e| e.entity.keyword() == kw) {
                    sel.enabled = true;
                    sel.ranges = rest.to_string();
                }
            }
            _ => {}
        }
    }
    d
}

fn parse_knobs(body: &[String]) -> Knobs {
    let mut k = Knobs::default();
    for line in body {
        let (first, rest) = split_first_token(line);
        match first {
            "-iterations" => k.iterations = rest.to_string(),
            "-convergence_tolerance" => k.convergence_tolerance = rest.to_string(),
            "-tolerance" => k.tolerance = rest.to_string(),
            "-step_size" => k.step_size = rest.to_string(),
            "-pe_step_size" => k.pe_step_size = rest.to_string(),
            "-diagonal_scale" => {
                k.diagonal_scale =
                    rest.trim().eq_ignore_ascii_case("true") || rest.trim().is_empty()
            }
            _ => {}
        }
    }
    k
}

fn parse_print(body: &[String]) -> Print {
    let mut p = Print::default();
    for line in body {
        let (first, rest) = split_first_token(line);
        let flag = rest.trim().eq_ignore_ascii_case("true");
        match first {
            "-echo_input" => p.echo_input = flag,
            "-equilibrium_phases" => p.equilibrium_phases = flag,
            "-exchange" => p.exchange = flag,
            "-gas_phase" => p.gas_phase = flag,
            "-headings" => p.headings = flag,
            "-inverse_modeling" => p.inverse_modeling = flag,
            "-kinetics" => p.kinetics = flag,
            "-saturation_indices" => p.saturation_indices = flag,
            "-solid_solutions" => p.solid_solutions = flag,
            "-species" => p.species = flag,
            "-surface" => p.surface = flag,
            "-totals" => p.totals = flag,
            "-user_print" => p.user_print = flag,
            "-user_graph" => p.user_graph = flag,
            "-selected_output" => p.selected_output = flag,
            "-high_precision" => p.high_precision = flag,
            _ => {}
        }
    }
    p
}

fn parse_advection(body: &[String]) -> Advection {
    let mut a = Advection::default();
    for line in body {
        let (first, rest) = split_first_token(line);
        match first {
            "-cells" => a.cells = rest.to_string(),
            "-shifts" => a.shifts = rest.to_string(),
            "-time_step" => {
                let (v, unit) = split_first_token(rest);
                a.time_step = v.to_string();
                if !unit.is_empty() {
                    a.time_step_unit = unit.to_string();
                }
            }
            "-initial_time" => a.initial_time = rest.to_string(),
            "-warnings" => a.warnings = rest.trim().eq_ignore_ascii_case("true"),
            _ => {}
        }
    }
    a
}

fn parse_transport(body: &[String]) -> Transport {
    let mut t = Transport::default();
    for line in body {
        let (first, rest) = split_first_token(line);
        match first {
            "-cells" => t.cells = rest.to_string(),
            "-shifts" => t.shifts = rest.to_string(),
            "-time_step" => {
                let (v, unit) = split_first_token(rest);
                t.time_step = v.to_string();
                if !unit.is_empty() {
                    t.time_step_unit = unit.to_string();
                }
            }
            "-flow_direction" => {
                t.flow_direction = match rest.trim() {
                    "back" => FlowDirection::Back,
                    "diffusion_only" => FlowDirection::Diffusion,
                    _ => FlowDirection::Forward,
                };
            }
            "-boundary_conditions" => {
                let tokens: Vec<&str> = rest.split_whitespace().collect();
                if let Some(top) = tokens.first() {
                    t.boundary_condition_top = top.to_string();
                }
                if let Some(bottom) = tokens.get(1) {
                    t.boundary_condition_bottom = bottom.to_string();
                }
            }
            "-lengths" => t.uniform_length = rest.to_string(),
            "-dispersivities" => t.uniform_dispersivity = rest.to_string(),
            "-diffusion_coefficient" => t.diffusion_coefficient = rest.to_string(),
            _ => {}
        }
    }
    t
}

fn parse_incremental_reactions(header_rest: &str) -> IncrementalReactions {
    let v = header_rest.trim();
    IncrementalReactions {
        enabled: v.is_empty() || v.eq_ignore_ascii_case("true"),
    }
}

fn parse_inverse_modeling(number: u32, description: String, body: &[String]) -> InverseModeling {
    let mut im = InverseModeling {
        number,
        description,
        ..InverseModeling::default()
    };
    im.phases.clear();
    im.balances.clear();
    let mut section: Option<&str> = None;
    for line in body {
        let trimmed = line.trim();
        match trimmed {
            "-phases" => {
                section = Some("phases");
                continue;
            }
            "-balances" => {
                section = Some("balances");
                continue;
            }
            _ => {}
        }
        let (first, rest) = split_first_token(trimmed);
        match first {
            "-solutions" => {
                im.solutions = rest.to_string();
                section = None;
            }
            "-uncertainty" => {
                im.uncertainty = rest.to_string();
                section = None;
            }
            "-tolerance" => {
                im.tolerance = rest.to_string();
                section = None;
            }
            "-range" => {
                im.range = true;
                section = None;
            }
            "-minimal" => {
                im.minimal = true;
                section = None;
            }
            _ => match section {
                Some("phases") => {
                    if !first.is_empty() {
                        im.phases.push(trimmed.to_string());
                    }
                }
                Some("balances") => {
                    if !first.is_empty() {
                        im.balances.push(InverseBalance {
                            element_or_ph: first.to_string(),
                            uncertainty: rest.to_string(),
                        });
                    }
                }
                _ => {}
            },
        }
    }
    im
}

fn parse_solid_solutions(number: u32, description: String, body: &[String]) -> SolidSolutions {
    let mut ss = SolidSolutions {
        number,
        description,
        solid_solutions: Vec::new(),
    };
    for line in body {
        let trimmed = line.trim();
        let (first, rest) = split_first_token(trimmed);
        if first == "-comp" {
            if let Some(last) = ss.solid_solutions.last_mut() {
                let (phase, moles) = split_first_token(rest);
                last.components.push(SolidSolutionComponent {
                    phase_name: phase.to_string(),
                    moles: moles.to_string(),
                });
            }
        } else if !first.is_empty() {
            ss.solid_solutions.push(SolidSolution {
                name: trimmed.to_string(),
                components: Vec::new(),
            });
        }
    }
    ss
}

fn parse_solution_spread(body: &[String]) -> SolutionSpread {
    let mut sp = SolutionSpread::default();
    let mut header_cols: Option<Vec<String>> = None;
    for line in body {
        let trimmed = line.trim();
        if trimmed.starts_with('-') {
            let (first, rest) = split_first_token(trimmed);
            match first.to_lowercase().as_str() {
                "-temp" => sp.default_temperature_c = rest.to_string(),
                "-ph" => sp.default_ph = rest.to_string(),
                "-pe" => sp.default_pe = rest.to_string(),
                "-units" => sp.default_units = rest.to_string(),
                "-density" => sp.default_density = rest.to_string(),
                _ => {}
            }
            continue;
        }
        let cells: Vec<String> = line.split('\t').map(|c| c.trim().to_string()).collect();
        if header_cols.is_none() {
            // First non-default line is the header: Number, ...columns..., Description
            if cells.len() >= 2 {
                sp.columns = cells[1..cells.len() - 1].to_vec();
            }
            header_cols = Some(cells);
            continue;
        }
        if cells.is_empty() {
            continue;
        }
        let ncols = sp.columns.len();
        let number = cells.first().cloned().unwrap_or_default();
        let values: Vec<String> = cells.iter().skip(1).take(ncols).cloned().collect();
        let description = cells.get(1 + ncols).cloned().unwrap_or_default();
        sp.rows.push(SpreadRow {
            number,
            description,
            values,
        });
    }
    sp
}

#[cfg(test)]
mod tests {
    #[test]
    fn partial_print_block_is_kept_verbatim() {
        // Only some flags set: the rest carry over from earlier simulations,
        // so writing all flags (at defaults) would change the output.
        let text = "PRINT\n    -reset false\n    -echo_input true\nSOLUTION 1\nEND\n";
        let doc = parse_document(text);
        assert!(matches!(&doc.blocks[0], Block::Raw(r) if r.header_line == "PRINT"));
    }

    #[test]
    fn semicolons_after_a_comment_stay_commented() {
        let text = "SOLUTION 3\n  Hto 1e-6; -water 0.2\n # Cs 1; Na 2; -water 1.02\nEND\n";
        let doc = parse_document(text);
        let saved = crate::serialize::document_to_pqi(&doc);
        assert!(saved.contains("-water 0.2"), "{saved}");
        assert!(!saved.lines().any(|l| l.trim_start().starts_with("-water 1.02")), "{saved}");
    }

    #[test]
    fn several_blocks_on_one_line_and_lowercase_keywords() {
        let text = "MIX 1; 1 0.9; 22 0.1 ;MIX 2; 2 0.9; 23 0.1\nsolution 5\n  pH 8\nEND\n";
        let doc = parse_document(text);
        let mixes = doc.blocks.iter().filter(|b| matches!(b, Block::Mix(_))).count();
        assert_eq!(mixes, 2);
        assert!(doc.blocks.iter().any(|b| matches!(b, Block::Solution(s) if s.number == 5)));
    }

    #[test]
    fn comments_in_structured_blocks_are_not_parsed_as_data() {
        let text = "SOLUTION 1 test\n    # a full-line comment\n    pH 7.5 # inline\n    Ca 1.0\n\
                    EQUILIBRIUM_PHASES 1\n    # comment here\n    Calcite 0 10 # inline\n\
                    PHASES\n    # kept verbatim in text blocks\n    Fix_H+\nEND\n";
        let doc = parse_document(text);
        let Block::Solution(sol) = &doc.blocks[0] else { panic!("solution first") };
        assert_eq!(sol.ph, "7.5");
        assert_eq!(sol.elements.len(), 1);
        assert_eq!(sol.elements[0].element, "Ca");
        let Block::EquilibriumPhases(eq) = &doc.blocks[1] else { panic!("phases second") };
        assert_eq!(eq.phases.len(), 1);
        assert_eq!(eq.phases[0].phase_name, "Calcite");
        assert_eq!(eq.phases[0].amount, "10");
        let Block::Raw(raw) = &doc.blocks[2] else { panic!("raw third") };
        assert!(raw.body.contains("# kept verbatim in text blocks"));
        assert!(crate::validate::problems(&doc).is_empty());
    }

    use super::*;
    use crate::serialize::document_to_pqi;

    #[test]
    fn round_trips_a_mixed_document() {
        let mut sol = Solution::default();
        sol.description = "Seawater".into();
        sol.elements.push(ElementConcentration {
            element: "Na".into(),
            concentration: "468".into(),
            unit_override: "mmol/kgw".into(),
            ..Default::default()
        });
        sol.charge_balance = "pH".into();

        let mut eq = EquilibriumPhases::default();
        eq.phases.push(PhaseAmount {
            phase_name: "Calcite".into(),
            saturation_index: "0.0".into(),
            amount: "10".into(),
        });

        let mut r = Reaction::default();
        r.reactants.push(Reactant {
            name: "NaCl".into(),
            relative_stoichiometry: "2.0".into(),
        });
        r.amount = "1.0".into();
        r.steps = "4".into();

        let mut so = SelectedOutput::default();
        so.totals = "Ca Mg Na Cl".into();

        let doc = Document {
            title: "Round trip test".into(),
            blocks: vec![
                Block::Solution(sol),
                Block::EquilibriumPhases(eq),
                Block::Reaction(r),
                Block::SelectedOutput(so),
            ],
        };

        let text = document_to_pqi(&doc);
        let parsed = parse_document(&text);

        assert_eq!(parsed.title, "Round trip test");
        assert_eq!(parsed.blocks.len(), 4);

        let Block::Solution(p_sol) = &parsed.blocks[0] else {
            panic!("expected Solution")
        };
        assert_eq!(p_sol.description, "Seawater");
        assert_eq!(p_sol.charge_balance, "pH");
        assert_eq!(p_sol.elements[0].element, "Na");
        assert_eq!(p_sol.elements[0].concentration, "468");

        let Block::EquilibriumPhases(p_eq) = &parsed.blocks[1] else {
            panic!("expected EquilibriumPhases")
        };
        assert_eq!(p_eq.phases[0].phase_name, "Calcite");

        let Block::Reaction(p_r) = &parsed.blocks[2] else {
            panic!("expected Reaction")
        };
        assert_eq!(p_r.reactants[0].name, "NaCl");
        assert_eq!(p_r.steps, "4");

        let Block::SelectedOutput(p_so) = &parsed.blocks[3] else {
            panic!("expected SelectedOutput")
        };
        assert_eq!(p_so.totals, "Ca Mg Na Cl");
    }

    #[test]
    fn round_trips_second_batch_types_and_preserves_raw_content() {
        let mut adv = Advection::default();
        adv.cells = "3".into();
        adv.shifts = "5".into();
        adv.time_step = "1".into();
        adv.time_step_unit = "year".into();

        let mut t = Transport::default();
        t.cells = "4".into();
        t.flow_direction = FlowDirection::Back;
        t.boundary_condition_top = "constant".into();
        t.boundary_condition_bottom = "closed".into();

        let ir = IncrementalReactions { enabled: false };

        let mut im = InverseModeling::default();
        im.solutions = "1 2 3".into();
        im.phases.push("Calcite".into());
        im.balances.push(InverseBalance {
            element_or_ph: "pH".into(),
            uncertainty: "0.1".into(),
        });

        let mut ss = SolidSolutions::default();
        ss.solid_solutions.push(SolidSolution {
            name: "CaSrBaSO4".into(),
            components: vec![SolidSolutionComponent {
                phase_name: "Anhydrite".into(),
                moles: "1.5".into(),
            }],
        });

        let mut sp = SolutionSpread::default();
        sp.columns = vec!["Ca".into(), "Na".into()];
        sp.rows.push(SpreadRow {
            number: "3".into(),
            description: "test row".into(),
            values: vec!["1.0".into(), "2.0".into()],
        });

        let doc = Document {
            title: "Second batch round trip".into(),
            blocks: vec![
                Block::Advection(adv),
                Block::Transport(t),
                Block::IncrementalReactions(ir),
                Block::InverseModeling(im),
                Block::SolidSolutions(ss),
                Block::SolutionSpread(sp),
            ],
        };

        let text = document_to_pqi(&doc);
        let parsed = parse_document(&text);
        assert_eq!(parsed.blocks.len(), 6);

        let Block::Advection(p) = &parsed.blocks[0] else {
            panic!("expected Advection")
        };
        assert_eq!(p.cells, "3");
        assert_eq!(p.shifts, "5");
        assert_eq!(p.time_step_unit, "year");

        let Block::Transport(p) = &parsed.blocks[1] else {
            panic!("expected Transport")
        };
        assert_eq!(p.cells, "4");
        assert_eq!(p.flow_direction, FlowDirection::Back);
        assert_eq!(p.boundary_condition_top, "constant");
        assert_eq!(p.boundary_condition_bottom, "closed");

        let Block::IncrementalReactions(p) = &parsed.blocks[2] else {
            panic!("expected IncrementalReactions")
        };
        assert!(!p.enabled);

        let Block::InverseModeling(p) = &parsed.blocks[3] else {
            panic!("expected InverseModeling")
        };
        assert_eq!(p.solutions, "1 2 3");
        assert_eq!(p.phases, vec!["Calcite".to_string()]);
        assert_eq!(p.balances[0].element_or_ph, "pH");

        let Block::SolidSolutions(p) = &parsed.blocks[4] else {
            panic!("expected SolidSolutions")
        };
        assert_eq!(p.solid_solutions[0].name, "CaSrBaSO4");
        assert_eq!(p.solid_solutions[0].components[0].phase_name, "Anhydrite");

        let Block::SolutionSpread(p) = &parsed.blocks[5] else {
            panic!("expected SolutionSpread")
        };
        assert_eq!(p.columns, vec!["Ca".to_string(), "Na".to_string()]);
        assert_eq!(p.rows[0].number, "3");
        assert_eq!(p.rows[0].values, vec!["1.0".to_string(), "2.0".to_string()]);
        assert_eq!(p.rows[0].description, "test row");
    }

    #[test]
    fn preserves_unmodeled_keywords_as_raw_blocks_instead_of_dropping_them() {
        let text = "\
TITLE Raw preservation test
SOLUTION 1
    pH 7
ISOTOPES 1 custom isotope ratios
    U-234/U-238 1.1 0.05
    U-235/U-238 0.0072 0.001
PITZER
    -MACINNES true
END
";
        let parsed = parse_document(text);
        assert_eq!(parsed.blocks.len(), 3);
        assert!(matches!(parsed.blocks[0], Block::Solution(_)));

        let Block::Raw(isotopes) = &parsed.blocks[1] else {
            panic!("expected Raw for ISOTOPES")
        };
        assert_eq!(isotopes.header_line, "ISOTOPES 1 custom isotope ratios");
        assert!(isotopes.body.contains("U-234/U-238 1.1 0.05"));
        assert!(isotopes.body.contains("U-235/U-238 0.0072 0.001"));

        let Block::Raw(pitzer) = &parsed.blocks[2] else {
            panic!("expected Raw for PITZER")
        };
        assert_eq!(pitzer.header_line, "PITZER");
        assert!(pitzer.body.contains("-MACINNES true"));

        // Preserved content round-trips back out byte-identical in substance.
        let regenerated = document_to_pqi(&parsed);
        assert!(regenerated.contains("ISOTOPES 1 custom isotope ratios"));
        assert!(regenerated.contains("U-234/U-238 1.1 0.05"));
        assert!(regenerated.contains("PITZER"));
        assert!(regenerated.contains("-MACINNES true"));
    }

    #[test]
    fn round_trips_save_use_dump_knobs_print() {
        let save = Save {
            entity: Entity::Exchange,
            number: "2".into(),
        };
        let use_ = Use {
            entity: Entity::GasPhase,
            number: "3".into(),
        };
        let use_none = Use {
            entity: Entity::Kinetics,
            number: String::new(),
        };

        let mut dump = Dump {
            all: false,
            ..Dump::default()
        };
        dump.file = "myrun.dmp".into();
        dump.append = true;
        if let Some(sel) = dump
            .entities
            .iter_mut()
            .find(|e| e.entity == Entity::Solution)
        {
            sel.enabled = true;
            sel.ranges = "1-3".into();
        }

        let mut knobs = Knobs::default();
        knobs.iterations = "200".into();
        knobs.convergence_tolerance = "1e-10".into();

        let mut print = Print::default();
        print.high_precision = true;
        print.species = false;

        let doc = Document {
            title: String::new(),
            blocks: vec![
                Block::Save(save),
                Block::Use(use_),
                Block::Use(use_none),
                Block::Dump(dump),
                Block::Knobs(knobs),
                Block::Print(print),
            ],
        };

        let text = document_to_pqi(&doc);
        let parsed = parse_document(&text);
        assert_eq!(parsed.blocks.len(), 6);

        let Block::Save(p) = &parsed.blocks[0] else {
            panic!("expected Save")
        };
        assert_eq!(p.entity, Entity::Exchange);
        assert_eq!(p.number, "2");

        let Block::Use(p) = &parsed.blocks[1] else {
            panic!("expected Use")
        };
        assert_eq!(p.entity, Entity::GasPhase);
        assert_eq!(p.number, "3");

        let Block::Use(p) = &parsed.blocks[2] else {
            panic!("expected Use (none)")
        };
        assert_eq!(p.entity, Entity::Kinetics);
        assert_eq!(p.number, "");

        let Block::Dump(p) = &parsed.blocks[3] else {
            panic!("expected Dump")
        };
        assert_eq!(p.file, "myrun.dmp");
        assert!(p.append);
        assert!(!p.all);
        let sol_sel = p
            .entities
            .iter()
            .find(|e| e.entity == Entity::Solution)
            .unwrap();
        assert!(sol_sel.enabled);
        assert_eq!(sol_sel.ranges, "1-3");

        let Block::Knobs(p) = &parsed.blocks[4] else {
            panic!("expected Knobs")
        };
        assert_eq!(p.iterations, "200");
        assert_eq!(p.convergence_tolerance, "1e-10");

        let Block::Print(p) = &parsed.blocks[5] else {
            panic!("expected Print")
        };
        assert!(p.high_precision);
        assert!(!p.species);
    }

    #[test]
    fn drops_database_line_rather_than_preserving_it() {
        // Preserving an imported DATABASE line (as the Raw mechanism would
        // for any other unmodeled keyword) risks reintroducing the exact
        // stale-path engine hang documented in phreeqc.rs — this app manages
        // its own database path separately and must never round-trip one.
        let text = "DATABASE C:\\stale\\path\\phreeqc.dat\nSOLUTION 1\n    pH 7\nEND\n";
        let parsed = parse_document(text);
        assert_eq!(parsed.blocks.len(), 1);
        assert!(matches!(parsed.blocks[0], Block::Solution(_)));
    }

    #[test]
    fn hand_written_selected_output_opens_as_a_form_and_keeps_its_options() {
        let text = "SOLUTION 1\nSELECTED_OUTPUT 1\n    -file part2.sel\n    -temperature true\n    \
                    -totals Ca C(4)\n    -saturation_indices Calcite Anhydrite\n    \
                    -equilibrium_phases Calcite CO2(g)\nSELECTED_OUTPUT 2\n    -reset false\n    -pH\n    -si Barite\nEND\n";
        let doc = parse_document(text);
        let Block::SelectedOutput(so) = &doc.blocks[1] else {
            panic!("expected a form, got {:?}", doc.blocks[1]);
        };
        assert_eq!(so.saturation_indices, "Calcite Anhydrite");
        assert_eq!(so.equilibrium_phases, "Calcite CO2(g)");
        assert!(so.flag(0) && so.flag(9) && !so.flag(8));
        let Block::SelectedOutput(second) = &doc.blocks[2] else {
            panic!("expected a form, got {:?}", doc.blocks[2]);
        };
        assert!(second.flag(6) && !second.flag(0));
        let written = crate::serialize::document_to_pqi(&doc);
        assert!(!written.contains("-simulation"), "{written}");
        assert!(written.contains("-saturation_indices Barite"), "{written}");
        // Writing it out and reading it back is stable.
        assert_eq!(crate::serialize::document_to_pqi(&parse_document(&written)), written);
    }
}
