use crate::model::*;

fn header(keyword: &str, number: u32, description: &str) -> String {
    let mut s = format!("{} {}", keyword, number);
    if !description.trim().is_empty() {
        s.push(' ');
        s.push_str(description.trim());
    }
    s.push('\n');
    s
}

fn steps_spec_lines(spec: &StepsSpec) -> String {
    match spec {
        StepsSpec::Explicit(values) => {
            let vals: Vec<&str> = values
                .iter()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();
            if vals.is_empty() {
                String::new()
            } else {
                format!("    {}\n", vals.join(" "))
            }
        }
        StepsSpec::Linear { start, end, steps } => {
            if start.trim().is_empty() || end.trim().is_empty() || steps.trim().is_empty() {
                String::new()
            } else {
                format!(
                    "    {} {} in {} steps\n",
                    start.trim(),
                    end.trim(),
                    steps.trim()
                )
            }
        }
    }
}

pub fn solution_to_pqi(sol: &Solution) -> String {
    let mut out = header("SOLUTION", sol.number, &sol.description);

    if !sol.temperature_c.trim().is_empty() {
        out.push_str(&format!("    temp      {}\n", sol.temperature_c.trim()));
    }
    if !sol.ph.trim().is_empty() {
        let charge_tag = if sol.charge_balance == "pH" {
            " charge"
        } else {
            ""
        };
        out.push_str(&format!("    pH        {}{}\n", sol.ph.trim(), charge_tag));
    }
    if !sol.pe.trim().is_empty() {
        out.push_str(&format!("    pe        {}\n", sol.pe.trim()));
    }
    if !sol.redox_couple.trim().is_empty() {
        out.push_str(&format!("    redox     {}\n", sol.redox_couple.trim()));
    }
    if !sol.density.trim().is_empty() {
        out.push_str(&format!("    density   {}\n", sol.density.trim()));
    }
    if !sol.default_units.trim().is_empty() {
        out.push_str(&format!("    units     {}\n", sol.default_units.trim()));
    }

    for el in &sol.elements {
        if el.element.trim().is_empty() {
            continue;
        }
        let mut line = format!("    {:<10} {}", el.element.trim(), el.concentration.trim());
        if !el.unit_override.trim().is_empty() {
            line.push(' ');
            line.push_str(el.unit_override.trim());
        }
        if !el.as_form.trim().is_empty() {
            line.push_str(" as ");
            line.push_str(el.as_form.trim());
        }
        if !el.redox_couple.trim().is_empty() {
            line.push(' ');
            line.push_str(el.redox_couple.trim());
        }
        if sol.charge_balance == el.element.trim() {
            line.push_str(" charge");
        }
        out.push_str(&line);
        out.push('\n');
    }

    out
}

pub fn equilibrium_phases_to_pqi(eq: &EquilibriumPhases) -> String {
    let mut out = header("EQUILIBRIUM_PHASES", eq.number, &eq.description);
    for p in &eq.phases {
        if p.phase_name.trim().is_empty() {
            continue;
        }
        let si = if p.saturation_index.trim().is_empty() {
            "0.0"
        } else {
            p.saturation_index.trim()
        };
        let mut line = format!("    {:<20} {}", p.phase_name.trim(), si);
        if !p.amount.trim().is_empty() {
            line.push(' ');
            line.push_str(p.amount.trim());
        }
        out.push_str(&line);
        out.push('\n');
    }
    out
}

pub fn reaction_to_pqi(r: &Reaction) -> String {
    let mut out = header("REACTION", r.number, &r.description);
    for reactant in &r.reactants {
        if reactant.name.trim().is_empty() {
            continue;
        }
        let coeff = if reactant.relative_stoichiometry.trim().is_empty() {
            "1.0".to_string()
        } else {
            reactant.relative_stoichiometry.trim().to_string()
        };
        out.push_str(&format!("    {} {}\n", reactant.name.trim(), coeff));
    }
    if !r.amount.trim().is_empty() {
        let units = if r.units.trim().is_empty() {
            "moles"
        } else {
            r.units.trim()
        };
        let steps = if r.steps.trim().is_empty() {
            "1"
        } else {
            r.steps.trim()
        };
        out.push_str(&format!(
            "    {} {} in {} steps\n",
            r.amount.trim(),
            units,
            steps
        ));
    }
    out
}

pub fn reaction_temperature_to_pqi(rt: &ReactionTemperature) -> String {
    let mut out = header("REACTION_TEMPERATURE", rt.number, &rt.description);
    out.push_str(&steps_spec_lines(&rt.values));
    out
}

pub fn reaction_pressure_to_pqi(rp: &ReactionPressure) -> String {
    let mut out = header("REACTION_PRESSURE", rp.number, &rp.description);
    out.push_str(&steps_spec_lines(&rp.values));
    out
}

pub fn gas_phase_to_pqi(g: &GasPhase) -> String {
    let mut out = header("GAS_PHASE", g.number, &g.description);
    if g.fixed_volume {
        out.push_str("    -fixed_volume\n");
    } else {
        out.push_str("    -fixed_pressure\n");
    }
    if !g.pressure.trim().is_empty() {
        out.push_str(&format!("    -pressure        {}\n", g.pressure.trim()));
    }
    if !g.volume.trim().is_empty() {
        out.push_str(&format!("    -volume          {}\n", g.volume.trim()));
    }
    if !g.temperature_c.trim().is_empty() {
        out.push_str(&format!(
            "    -temperature     {}\n",
            g.temperature_c.trim()
        ));
    }
    for c in &g.components {
        if c.phase_name.trim().is_empty() {
            continue;
        }
        let pp = if c.partial_pressure.trim().is_empty() {
            "0.0"
        } else {
            c.partial_pressure.trim()
        };
        out.push_str(&format!("    {:<16} {}\n", c.phase_name.trim(), pp));
    }
    out
}

pub fn exchange_to_pqi(ex: &Exchange) -> String {
    let mut out = header("EXCHANGE", ex.number, &ex.description);
    for c in &ex.components {
        if c.exchange_formula.trim().is_empty() {
            continue;
        }
        let amount = if c.amount.trim().is_empty() {
            "0.0"
        } else {
            c.amount.trim()
        };
        out.push_str(&format!(
            "    {:<16} {}\n",
            c.exchange_formula.trim(),
            amount
        ));
    }
    out
}

pub fn surface_to_pqi(s: &Surface) -> String {
    let mut out = header("SURFACE", s.number, &s.description);
    for site in &s.sites {
        if site.surface_formula.trim().is_empty() {
            continue;
        }
        let amount = if site.amount.trim().is_empty() {
            "0.0"
        } else {
            site.amount.trim()
        };
        let mut line = format!("    {:<16} {}", site.surface_formula.trim(), amount);
        if s.electrostatics == SurfaceElectrostatics::DiffuseLayer
            && !site.specific_area_m2_g.trim().is_empty()
            && !site.mass_g.trim().is_empty()
        {
            line.push(' ');
            line.push_str(site.specific_area_m2_g.trim());
            line.push(' ');
            line.push_str(site.mass_g.trim());
        }
        out.push_str(&line);
        out.push('\n');
    }
    if !s.equilibrate_with_solution.trim().is_empty() {
        out.push_str(&format!(
            "    -equilibrate {}\n",
            s.equilibrate_with_solution.trim()
        ));
    }
    match s.electrostatics {
        SurfaceElectrostatics::None => {
            out.push_str("    -no_edl\n");
        }
        SurfaceElectrostatics::DiffuseLayer => {
            if !s.diffuse_layer_thickness.trim().is_empty() {
                out.push_str(&format!(
                    "    -diffuse_layer {}\n",
                    s.diffuse_layer_thickness.trim()
                ));
            }
        }
    }
    out
}

pub fn kinetics_to_pqi(k: &Kinetics) -> String {
    let mut out = header("KINETICS", k.number, &k.description);
    for r in &k.reactants {
        if r.rate_name.trim().is_empty() {
            continue;
        }
        out.push_str(&format!("    {}\n", r.rate_name.trim()));
        if !r.m.trim().is_empty() {
            out.push_str(&format!("        -m    {}\n", r.m.trim()));
        }
        if !r.m0.trim().is_empty() {
            out.push_str(&format!("        -m0   {}\n", r.m0.trim()));
        }
        if !r.parms.trim().is_empty() {
            out.push_str(&format!("        -parms {}\n", r.parms.trim()));
        }
        if !r.tol.trim().is_empty() {
            out.push_str(&format!("        -tol  {}\n", r.tol.trim()));
        }
    }
    let steps_line = match &k.steps {
        StepsSpec::Explicit(values) => {
            let vals: Vec<&str> = values
                .iter()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();
            if vals.is_empty() {
                None
            } else {
                Some(format!(
                    "    -steps {} {}\n",
                    vals.join(" "),
                    k.step_units.trim()
                ))
            }
        }
        StepsSpec::Linear { start, end, steps } => {
            if start.trim().is_empty() || end.trim().is_empty() || steps.trim().is_empty() {
                None
            } else {
                // KINETICS doesn't support "in N steps" shorthand on -steps directly;
                // expand to N evenly spaced explicit values.
                let s0: f64 = start.trim().parse().unwrap_or(0.0);
                let s1: f64 = end.trim().parse().unwrap_or(0.0);
                let n: usize = steps.trim().parse().unwrap_or(1).max(1);
                let vals: Vec<String> = (1..=n)
                    .map(|i| {
                        let t = s0 + (s1 - s0) * (i as f64) / (n as f64);
                        format!("{t}")
                    })
                    .collect();
                Some(format!(
                    "    -steps {} {}\n",
                    vals.join(" "),
                    k.step_units.trim()
                ))
            }
        }
    };
    if let Some(line) = steps_line {
        out.push_str(&line);
    }
    out
}

pub fn selected_output_to_pqi(so: &SelectedOutput) -> String {
    let mut out = format!("SELECTED_OUTPUT {}\n", so.number);
    if !so.file.trim().is_empty() {
        out.push_str(&format!("    -file               {}\n", so.file.trim()));
    }
    if let Some(v) = so.reset {
        out.push_str(&format!("    -{:<18} {}\n", "reset", v));
    }
    for ((name, _), value) in SELECTED_OUTPUT_FLAGS.iter().zip(&so.flags) {
        if let Some(v) = value {
            out.push_str(&format!("    -{:<18} {}\n", name, v));
        }
    }
    for (name, list) in [
        ("totals", &so.totals),
        ("molalities", &so.molalities),
        ("activities", &so.activities),
        ("saturation_indices", &so.saturation_indices),
        ("equilibrium_phases", &so.equilibrium_phases),
        ("gases", &so.gases),
    ] {
        if !list.trim().is_empty() {
            out.push_str(&format!("    -{:<18} {}\n", name, list.split_whitespace().collect::<Vec<_>>().join(" ")));
        }
    }
    out
}

pub fn mix_to_pqi(m: &Mix) -> String {
    let mut out = header("MIX", m.number, &m.description);
    for c in &m.components {
        if c.solution_number.trim().is_empty() {
            continue;
        }
        let frac = if c.fraction.trim().is_empty() {
            "1.0"
        } else {
            c.fraction.trim()
        };
        out.push_str(&format!("    {:<8} {}\n", c.solution_number.trim(), frac));
    }
    out
}

pub fn advection_to_pqi(a: &Advection) -> String {
    let mut out = "ADVECTION\n".to_string();
    out.push_str(&format!(
        "    -cells        {}\n",
        if a.cells.trim().is_empty() {
            "1"
        } else {
            a.cells.trim()
        }
    ));
    out.push_str(&format!(
        "    -shifts       {}\n",
        if a.shifts.trim().is_empty() {
            "1"
        } else {
            a.shifts.trim()
        }
    ));
    if !a.time_step.trim().is_empty() {
        out.push_str(&format!(
            "    -time_step    {} {}\n",
            a.time_step.trim(),
            a.time_step_unit.trim()
        ));
    }
    if !a.initial_time.trim().is_empty() {
        out.push_str(&format!("    -initial_time {}\n", a.initial_time.trim()));
    }
    out.push_str(&format!("    -warnings     {}\n", a.warnings));
    out
}

pub fn transport_to_pqi(t: &Transport) -> String {
    let mut out = "TRANSPORT\n".to_string();
    out.push_str(&format!(
        "    -cells               {}\n",
        if t.cells.trim().is_empty() {
            "1"
        } else {
            t.cells.trim()
        }
    ));
    out.push_str(&format!(
        "    -shifts              {}\n",
        if t.shifts.trim().is_empty() {
            "1"
        } else {
            t.shifts.trim()
        }
    ));
    if !t.time_step.trim().is_empty() {
        out.push_str(&format!(
            "    -time_step           {} {}\n",
            t.time_step.trim(),
            t.time_step_unit.trim()
        ));
    }
    let dir = match t.flow_direction {
        FlowDirection::Forward => "forward",
        FlowDirection::Back => "back",
        FlowDirection::Diffusion => "diffusion_only",
    };
    out.push_str(&format!("    -flow_direction      {dir}\n"));
    out.push_str(&format!(
        "    -boundary_conditions {} {}\n",
        t.boundary_condition_top.trim(),
        t.boundary_condition_bottom.trim()
    ));
    if !t.uniform_length.trim().is_empty() {
        out.push_str(&format!(
            "    -lengths             {}\n",
            t.uniform_length.trim()
        ));
    }
    if !t.uniform_dispersivity.trim().is_empty() {
        out.push_str(&format!(
            "    -dispersivities      {}\n",
            t.uniform_dispersivity.trim()
        ));
    }
    if !t.diffusion_coefficient.trim().is_empty() {
        out.push_str(&format!(
            "    -diffusion_coefficient {}\n",
            t.diffusion_coefficient.trim()
        ));
    }
    out
}

pub fn incremental_reactions_to_pqi(ir: &IncrementalReactions) -> String {
    format!(
        "INCREMENTAL_REACTIONS {}\n",
        if ir.enabled { "true" } else { "false" }
    )
}

pub fn inverse_modeling_to_pqi(im: &InverseModeling) -> String {
    let mut out = header("INVERSE_MODELING", im.number, &im.description);
    if !im.solutions.trim().is_empty() {
        out.push_str(&format!("    -solutions   {}\n", im.solutions.trim()));
    }
    if !im.uncertainty.trim().is_empty() {
        out.push_str(&format!("    -uncertainty {}\n", im.uncertainty.trim()));
    }
    if !im.phases.is_empty() {
        out.push_str("    -phases\n");
        for p in &im.phases {
            if !p.trim().is_empty() {
                out.push_str(&format!("        {}\n", p.trim()));
            }
        }
    }
    if !im.balances.is_empty() {
        out.push_str("    -balances\n");
        for b in &im.balances {
            if b.element_or_ph.trim().is_empty() {
                continue;
            }
            out.push_str(&format!(
                "        {} {}\n",
                b.element_or_ph.trim(),
                b.uncertainty.trim()
            ));
        }
    }
    if im.range {
        out.push_str("    -range\n");
    }
    if im.minimal {
        out.push_str("    -minimal\n");
    }
    if !im.tolerance.trim().is_empty() {
        out.push_str(&format!("    -tolerance   {}\n", im.tolerance.trim()));
    }
    out
}

pub fn solid_solutions_to_pqi(ss: &SolidSolutions) -> String {
    let mut out = header("SOLID_SOLUTIONS", ss.number, &ss.description);
    for sol in &ss.solid_solutions {
        if sol.name.trim().is_empty() {
            continue;
        }
        out.push_str(&format!("    {}\n", sol.name.trim()));
        for comp in &sol.components {
            if comp.phase_name.trim().is_empty() {
                continue;
            }
            let moles = if comp.moles.trim().is_empty() {
                "0.0"
            } else {
                comp.moles.trim()
            };
            out.push_str(&format!(
                "        -comp {} {}\n",
                comp.phase_name.trim(),
                moles
            ));
        }
    }
    out
}

pub fn solution_spread_to_pqi(sp: &SolutionSpread) -> String {
    let mut out = "SOLUTION_SPREAD\n".to_string();
    if !sp.default_temperature_c.trim().is_empty() {
        out.push_str(&format!("    -temp {}\n", sp.default_temperature_c.trim()));
    }
    if !sp.default_ph.trim().is_empty() {
        out.push_str(&format!("    -pH {}\n", sp.default_ph.trim()));
    }
    if !sp.default_pe.trim().is_empty() {
        out.push_str(&format!("    -pe {}\n", sp.default_pe.trim()));
    }
    if !sp.default_units.trim().is_empty() {
        out.push_str(&format!("    -units {}\n", sp.default_units.trim()));
    }
    if !sp.default_density.trim().is_empty() {
        out.push_str(&format!("    -density {}\n", sp.default_density.trim()));
    }

    let mut header_cols = vec!["Number".to_string()];
    header_cols.extend(sp.columns.iter().cloned());
    header_cols.push("Description".to_string());
    out.push_str(&header_cols.join("\t"));
    out.push('\n');

    for row in &sp.rows {
        if row.number.trim().is_empty() {
            continue;
        }
        let mut cells = vec![row.number.trim().to_string()];
        for i in 0..sp.columns.len() {
            cells.push(row.values.get(i).cloned().unwrap_or_default());
        }
        cells.push(row.description.clone());
        out.push_str(&cells.join("\t"));
        out.push('\n');
    }
    out
}

pub fn raw_to_pqi(r: &Raw) -> String {
    let mut out = r.header_line.clone();
    out.push('\n');
    if !r.body.trim().is_empty() {
        out.push_str(&r.body);
        if !r.body.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

pub fn save_to_pqi(s: &Save) -> String {
    format!("SAVE {} {}\n", s.entity.keyword(), s.number.trim())
}

pub fn use_to_pqi(u: &Use) -> String {
    let number = if u.number.trim().is_empty() {
        "none"
    } else {
        u.number.trim()
    };
    format!("USE {} {}\n", u.entity.keyword(), number)
}

pub fn dump_to_pqi(d: &Dump) -> String {
    let mut out = "DUMP\n".to_string();
    if !d.file.trim().is_empty() {
        out.push_str(&format!("    -file {}\n", d.file.trim()));
    }
    if d.append {
        out.push_str("    -append true\n");
    }
    if d.all {
        out.push_str("    -all\n");
    } else {
        for e in &d.entities {
            if !e.enabled {
                continue;
            }
            out.push_str(&format!(
                "    -{} {}\n",
                e.entity.keyword(),
                e.ranges.trim()
            ));
        }
    }
    out
}

pub fn knobs_to_pqi(k: &Knobs) -> String {
    let mut out = "KNOBS\n".to_string();
    if !k.iterations.trim().is_empty() {
        out.push_str(&format!(
            "    -iterations            {}\n",
            k.iterations.trim()
        ));
    }
    if !k.convergence_tolerance.trim().is_empty() {
        out.push_str(&format!(
            "    -convergence_tolerance {}\n",
            k.convergence_tolerance.trim()
        ));
    }
    if !k.tolerance.trim().is_empty() {
        out.push_str(&format!(
            "    -tolerance             {}\n",
            k.tolerance.trim()
        ));
    }
    if !k.step_size.trim().is_empty() {
        out.push_str(&format!(
            "    -step_size             {}\n",
            k.step_size.trim()
        ));
    }
    if !k.pe_step_size.trim().is_empty() {
        out.push_str(&format!(
            "    -pe_step_size          {}\n",
            k.pe_step_size.trim()
        ));
    }
    if k.diagonal_scale {
        out.push_str("    -diagonal_scale        true\n");
    }
    out
}

pub fn print_to_pqi(p: &Print) -> String {
    let mut out = "PRINT\n".to_string();
    let flag = |name: &str, v: bool| format!("    -{:<20} {}\n", name, v);
    out.push_str(&flag("echo_input", p.echo_input));
    out.push_str(&flag("equilibrium_phases", p.equilibrium_phases));
    out.push_str(&flag("exchange", p.exchange));
    out.push_str(&flag("gas_phase", p.gas_phase));
    out.push_str(&flag("headings", p.headings));
    out.push_str(&flag("inverse_modeling", p.inverse_modeling));
    out.push_str(&flag("kinetics", p.kinetics));
    out.push_str(&flag("saturation_indices", p.saturation_indices));
    out.push_str(&flag("solid_solutions", p.solid_solutions));
    out.push_str(&flag("species", p.species));
    out.push_str(&flag("surface", p.surface));
    out.push_str(&flag("totals", p.totals));
    out.push_str(&flag("user_print", p.user_print));
    out.push_str(&flag("user_graph", p.user_graph));
    out.push_str(&flag("selected_output", p.selected_output));
    out.push_str(&flag("high_precision", p.high_precision));
    out
}

pub fn block_to_pqi(block: &Block) -> String {
    match block {
        Block::Solution(b) => solution_to_pqi(b),
        Block::EquilibriumPhases(b) => equilibrium_phases_to_pqi(b),
        Block::Reaction(b) => reaction_to_pqi(b),
        Block::ReactionTemperature(b) => reaction_temperature_to_pqi(b),
        Block::ReactionPressure(b) => reaction_pressure_to_pqi(b),
        Block::GasPhase(b) => gas_phase_to_pqi(b),
        Block::Exchange(b) => exchange_to_pqi(b),
        Block::Surface(b) => surface_to_pqi(b),
        Block::Kinetics(b) => kinetics_to_pqi(b),
        Block::SelectedOutput(b) => selected_output_to_pqi(b),
        Block::Mix(b) => mix_to_pqi(b),
        Block::Advection(b) => advection_to_pqi(b),
        Block::Transport(b) => transport_to_pqi(b),
        Block::IncrementalReactions(b) => incremental_reactions_to_pqi(b),
        Block::InverseModeling(b) => inverse_modeling_to_pqi(b),
        Block::SolidSolutions(b) => solid_solutions_to_pqi(b),
        Block::SolutionSpread(b) => solution_spread_to_pqi(b),
        Block::Save(b) => save_to_pqi(b),
        Block::Use(b) => use_to_pqi(b),
        Block::Dump(b) => dump_to_pqi(b),
        Block::Knobs(b) => knobs_to_pqi(b),
        Block::Print(b) => print_to_pqi(b),
        Block::Raw(b) => raw_to_pqi(b),
    }
}

pub fn document_to_pqi(doc: &Document) -> String {
    let mut out = String::new();
    if !doc.title.trim().is_empty() {
        out.push_str(&format!("TITLE {}\n", doc.title.trim()));
    }
    for (i, block) in doc.blocks.iter().enumerate() {
        out.push_str(&block_to_pqi(block));
        // A run of USE lines and its END read as one group.
        let next = doc.blocks.get(i + 1);
        let grouped = matches!(block, Block::Use(_))
            && next.is_some_and(|n| matches!(n, Block::Use(_)) || crate::simulations::is_end(n));
        if !grouped {
            out.push('\n');
        }
    }
    // A document read from a multi-simulation file already ends with its own
    // END boundary block.
    let ends_with_end = matches!(doc.blocks.last(), Some(Block::Raw(r)) if r.header_line.trim().eq_ignore_ascii_case("END"));
    if !ends_with_end {
        out.push_str("END\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_solution_serializes() {
        let mut sol = Solution::default();
        sol.description = "Test seawater".into();
        sol.elements.push(ElementConcentration {
            element: "Na".into(),
            concentration: "468".into(),
            unit_override: "mmol/kgw".into(),
            ..Default::default()
        });
        let text = solution_to_pqi(&sol);
        assert!(text.starts_with("SOLUTION 1 Test seawater\n"));
        assert!(text.contains("Na         468 mmol/kgw"));
    }

    #[test]
    fn equilibrium_phases_serializes() {
        let mut eq = EquilibriumPhases::default();
        eq.phases.push(PhaseAmount {
            phase_name: "Calcite".into(),
            saturation_index: "0.0".into(),
            amount: "10".into(),
        });
        let text = equilibrium_phases_to_pqi(&eq);
        assert!(text.contains("Calcite              0.0 10"));
    }

    #[test]
    fn reaction_serializes_with_steps() {
        let mut r = Reaction::default();
        r.reactants.push(Reactant {
            name: "NaCl".into(),
            relative_stoichiometry: "2.0".into(),
        });
        r.amount = "1.0".into();
        r.steps = "4".into();
        let text = reaction_to_pqi(&r);
        assert!(text.contains("NaCl 2.0"));
        assert!(text.contains("1.0 moles in 4 steps"));
    }

    #[test]
    fn selected_output_serializes() {
        let mut so = SelectedOutput::default();
        so.totals = "Ca Mg Na Cl".into();
        let text = selected_output_to_pqi(&so);
        assert!(text.starts_with("SELECTED_OUTPUT 1\n"));
        assert!(text.contains("-totals             Ca Mg Na Cl"));
    }

    /// Not a real test: writes a document covering every block type to a
    /// fixed scratch path so it can be fed to the real `phreeqc` binary
    /// outside cargo, to check the real engine accepts the generated syntax
    /// (unit tests alone can't catch real-parser rejections).
    #[test]
    fn write_full_coverage_fixture() {
        let mut sol = Solution::default();
        sol.description = "Seawater".into();
        sol.elements.push(ElementConcentration {
            element: "Na".into(),
            concentration: "468".into(),
            unit_override: "mmol/kgw".into(),
            ..Default::default()
        });
        sol.elements.push(ElementConcentration {
            element: "Cl".into(),
            concentration: "546".into(),
            unit_override: "mmol/kgw".into(),
            ..Default::default()
        });
        sol.charge_balance = "Cl".into();

        let mut eq = EquilibriumPhases::default();
        eq.phases.push(PhaseAmount {
            phase_name: "Calcite".into(),
            saturation_index: "0.0".into(),
            amount: "10".into(),
        });

        let mut r = Reaction::default();
        r.reactants.push(Reactant {
            name: "NaCl".into(),
            relative_stoichiometry: "1.0".into(),
        });
        r.amount = "1.0".into();
        r.units = "moles".into();
        r.steps = "4".into();

        let mut rt = ReactionTemperature::default();
        rt.values = StepsSpec::Linear {
            start: "15".into(),
            end: "35".into(),
            steps: "3".into(),
        };

        let mut rp = ReactionPressure::default();
        rp.values = StepsSpec::Explicit(vec!["1.0".into(), "250.5".into(), "500.0".into()]);

        let mut gp = GasPhase::default();
        gp.components.push(GasComponent {
            phase_name: "CO2(g)".into(),
            partial_pressure: "0.000316".into(),
        });
        gp.components.push(GasComponent {
            phase_name: "N2(g)".into(),
            partial_pressure: "0.78".into(),
        });

        let mut ex = Exchange::default();
        ex.components.push(ExchangeComponent {
            exchange_formula: "NaX".into(),
            amount: "0.5".into(),
        });

        let mut surf = Surface::default();
        surf.sites.push(SurfaceSite {
            surface_formula: "Hfo_wOH".into(),
            amount: "0.2".into(),
            specific_area_m2_g: "600".into(),
            mass_g: "1.0".into(),
        });
        surf.electrostatics = SurfaceElectrostatics::DiffuseLayer;

        let mut kin = Kinetics::default();
        kin.reactants.push(KineticReactant {
            rate_name: "Pyrite".into(),
            m: "1e-3".into(),
            m0: "1e-3".into(),
            parms: "3.0 0.67 .5 -0.11".into(),
            tol: "1e-8".into(),
        });
        kin.steps = StepsSpec::Explicit(vec!["100".into(), "200".into(), "300".into()]);
        kin.step_units = "day".into();

        let mut so = SelectedOutput::default();
        so.totals = "Ca Mg Na Cl".into();

        let mut mix = Mix::default();
        mix.components.push(MixComponent {
            solution_number: "1".into(),
            fraction: "0.5".into(),
        });

        let doc = Document {
            title: "Full coverage fixture".into(),
            blocks: vec![
                Block::Solution(sol),
                Block::EquilibriumPhases(eq),
                Block::Reaction(r),
                Block::ReactionTemperature(rt),
                Block::ReactionPressure(rp),
                Block::GasPhase(gp),
                Block::Exchange(ex),
                Block::Surface(surf),
                Block::Kinetics(kin),
                Block::SelectedOutput(so),
                Block::Mix(mix),
            ],
        };

        let text = document_to_pqi(&doc);
        let path = std::env::temp_dir().join("pq_studio_full_coverage_fixture.pqi");
        std::fs::write(&path, text).unwrap();
        eprintln!("wrote fixture to {}", path.display());
    }

    /// Not a real test: writes a second fixture covering Advection, Transport,
    /// Incremental Reactions, Inverse Modeling, Solid Solutions, and Solution
    /// Spread (added after write_full_coverage_fixture, kept separate rather
    /// than growing that one further) to a fixed scratch path, to be run
    /// through the real `phreeqc` binary outside cargo.
    #[test]
    fn write_second_batch_fixture() {
        // ADVECTION's default boundary condition ("flux") needs a SOLUTION 0
        // defined as the influent water for cells shifting in from the
        // column's edge — a real PHREEQC modeling requirement (confirmed via
        // the real engine: "Solution 0 is needed for advection, but is not
        // defined"), not something this app's serializer should silently
        // paper over; the UI's Advection form should hint at it.
        let mut sol0 = Solution::default();
        sol0.number = 0;
        sol0.description = "Influent water for advection".into();
        sol0.elements.push(ElementConcentration {
            element: "Ca".into(),
            concentration: "0.5".into(),
            unit_override: "mmol/kgw".into(),
            ..Default::default()
        });
        sol0.elements.push(ElementConcentration {
            element: "Cl".into(),
            concentration: "1".into(),
            unit_override: "mmol/kgw".into(),
            ..Default::default()
        });

        let mut sol1 = Solution::default();
        sol1.number = 1;
        sol1.description = "Dilute water".into();
        sol1.elements.push(ElementConcentration {
            element: "Ca".into(),
            concentration: "1".into(),
            unit_override: "mmol/kgw".into(),
            ..Default::default()
        });
        sol1.elements.push(ElementConcentration {
            element: "Cl".into(),
            concentration: "2".into(),
            unit_override: "mmol/kgw".into(),
            ..Default::default()
        });

        let mut sol2 = Solution::default();
        sol2.number = 2;
        sol2.description = "Slightly more concentrated".into();
        sol2.elements.push(ElementConcentration {
            element: "Ca".into(),
            concentration: "2".into(),
            unit_override: "mmol/kgw".into(),
            ..Default::default()
        });
        sol2.elements.push(ElementConcentration {
            element: "Cl".into(),
            concentration: "4".into(),
            unit_override: "mmol/kgw".into(),
            ..Default::default()
        });

        let ir = IncrementalReactions { enabled: true };

        let mut ss = SolidSolutions::default();
        ss.description = "Two-component ideal solid solution".into();
        ss.solid_solutions.push(SolidSolution {
            name: "CaSrBaSO4".into(),
            components: vec![
                SolidSolutionComponent {
                    phase_name: "Anhydrite".into(),
                    moles: "1.5".into(),
                },
                SolidSolutionComponent {
                    phase_name: "Celestite".into(),
                    moles: "0.05".into(),
                },
                SolidSolutionComponent {
                    phase_name: "Barite".into(),
                    moles: "0.05".into(),
                },
            ],
        });

        let mut im = InverseModeling::default();
        im.description = "Mix 1 and 2".into();
        im.solutions = "1 2".into();
        im.phases.push("Calcite".into());
        im.phases.push("CO2(g)".into());

        let mut adv = Advection::default();
        adv.cells = "2".into();
        adv.shifts = "2".into();

        let mut spread = SolutionSpread::default();
        spread.columns = vec!["Ca".into(), "Na".into()];
        spread.rows.push(SpreadRow {
            number: "3".into(),
            description: "spread soln 3".into(),
            values: vec!["1.5".into(), "0.5".into()],
        });
        spread.rows.push(SpreadRow {
            number: "4".into(),
            description: "spread soln 4".into(),
            values: vec!["2.5".into(), "1.0".into()],
        });

        let sol1_for_inverse = sol1.clone();
        let sol2_for_inverse = sol2.clone();

        let doc = Document {
            title: "Second batch fixture".into(),
            blocks: vec![
                Block::Solution(sol0),
                Block::Solution(sol1),
                Block::Solution(sol2),
                Block::IncrementalReactions(ir),
                Block::SolidSolutions(ss),
                Block::Advection(adv),
                Block::SolutionSpread(spread),
            ],
        };

        let text = document_to_pqi(&doc);
        let path = std::env::temp_dir().join("pq_studio_second_batch_fixture.pqi");
        std::fs::write(&path, text).unwrap();
        eprintln!("wrote fixture to {}", path.display());

        // INVERSE_MODELING kept as a separate fixture: the real engine treats
        // a failed inverse-model search ("no feasible model" for this
        // contrived toy chemistry — a legitimate outcome of the technique,
        // not a parser rejection) as fatal for the whole simulation,
        // regardless of block ordering within it, so it can't share a run
        // with blocks whose own successful computation needs checking.
        let inverse_doc = Document {
            title: "Inverse modeling fixture".into(),
            blocks: vec![
                Block::Solution(sol1_for_inverse),
                Block::Solution(sol2_for_inverse),
                Block::InverseModeling(im),
            ],
        };
        let inverse_text = document_to_pqi(&inverse_doc);
        let inverse_path = std::env::temp_dir().join("pq_studio_inverse_modeling_fixture.pqi");
        std::fs::write(&inverse_path, inverse_text).unwrap();
        eprintln!("wrote fixture to {}", inverse_path.display());
    }

    /// Not a real test: writes a third fixture covering Save, Use, Dump,
    /// Knobs, and Print, to check the real engine accepts their syntax.
    #[test]
    fn write_third_batch_fixture() {
        let mut sol1 = Solution::default();
        sol1.number = 1;
        sol1.elements.push(ElementConcentration {
            element: "Ca".into(),
            concentration: "1".into(),
            unit_override: "mmol/kgw".into(),
            ..Default::default()
        });
        sol1.elements.push(ElementConcentration {
            element: "Cl".into(),
            concentration: "2".into(),
            unit_override: "mmol/kgw".into(),
            ..Default::default()
        });

        let mut sol2 = sol1.clone();
        sol2.number = 2;

        let mut knobs = Knobs::default();
        knobs.iterations = "200".into();
        knobs.convergence_tolerance = "1e-10".into();

        let mut print = Print::default();
        print.high_precision = true;

        let save = Save {
            entity: Entity::Solution,
            number: "3".into(),
        };
        let use_ = Use {
            entity: Entity::Solution,
            number: "3".into(),
        };

        let mut dump = Dump::default();
        dump.file = std::env::temp_dir()
            .join("pq_studio_third_batch.dmp")
            .to_string_lossy()
            .into_owned();

        let doc = Document {
            title: "Third batch fixture".into(),
            blocks: vec![
                Block::Knobs(knobs),
                Block::Print(print),
                Block::Solution(sol1),
                Block::Solution(sol2),
                Block::Save(save),
                Block::Use(use_),
                Block::Dump(dump),
            ],
        };

        let text = document_to_pqi(&doc);
        let path = std::env::temp_dir().join("pq_studio_third_batch_fixture.pqi");
        std::fs::write(&path, text).unwrap();
        eprintln!("wrote fixture to {}", path.display());
    }

    #[test]
    fn document_assembles_title_and_end() {
        let doc = Document {
            title: "My sim".into(),
            blocks: vec![Block::Solution(Solution::default())],
        };
        let text = document_to_pqi(&doc);
        assert!(text.starts_with("TITLE My sim\n"));
        assert!(text.trim_end().ends_with("END"));
    }
}
