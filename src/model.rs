//! Document model for the PHREEQC input blocks this app edits. Field sets are
//! drawn from the PHREEQC 3 keyword manual (Parkhurst and Appelo, 2013,
//! TM 6-A43) and, where available, PhreeqcI's dialog resource layouts. Scope
//! is deliberately limited: SURFACE has no CD-MUSIC, and KINETICS references
//! rate names rather than hosting a full custom RATES/BASIC editor.

/// A list of numeric values entered either explicitly (one per line in the
/// real syntax) or as a PHREEQC "start, end, in N steps" linear shorthand.
#[derive(Debug, Clone)]
pub enum StepsSpec {
    Explicit(Vec<String>),
    Linear {
        start: String,
        end: String,
        steps: String,
    },
}

impl Default for StepsSpec {
    fn default() -> Self {
        StepsSpec::Explicit(vec![String::new()])
    }
}

#[derive(Debug, Clone, Default)]
pub struct ElementConcentration {
    pub element: String,
    pub concentration: String,
    pub unit_override: String,
    pub as_form: String,
    pub redox_couple: String,
}

#[derive(Debug, Clone)]
pub struct Solution {
    pub number: u32,
    pub description: String,
    pub temperature_c: String,
    pub ph: String,
    pub pe: String,
    pub redox_couple: String,
    pub density: String,
    pub default_units: String,
    pub charge_balance: String,
    pub elements: Vec<ElementConcentration>,
}

impl Default for Solution {
    fn default() -> Self {
        Self {
            number: 1,
            description: String::new(),
            temperature_c: "25".to_string(),
            ph: "7".to_string(),
            pe: "4".to_string(),
            redox_couple: String::new(),
            density: "1".to_string(),
            default_units: "mmol/kgw".to_string(),
            charge_balance: "None".to_string(),
            elements: Vec::new(),
        }
    }
}

pub const DEFAULT_UNITS_OPTIONS: &[&str] = &[
    "mmol/kgw", "mol/kgw", "mg/kgw", "mg/L", "mmol/L", "mol/L", "ppm", "eq/kgw", "meq/kgw",
    "umol/kgw", "ug/kgw", "ug/L",
];

#[derive(Debug, Clone, Default)]
pub struct PhaseAmount {
    pub phase_name: String,
    pub saturation_index: String,
    pub amount: String,
}

#[derive(Debug, Clone)]
pub struct EquilibriumPhases {
    pub number: u32,
    pub description: String,
    pub phases: Vec<PhaseAmount>,
}

impl Default for EquilibriumPhases {
    fn default() -> Self {
        Self {
            number: 1,
            description: String::new(),
            phases: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Reactant {
    pub name: String,
    pub relative_stoichiometry: String,
}

#[derive(Debug, Clone)]
pub struct Reaction {
    pub number: u32,
    pub description: String,
    pub reactants: Vec<Reactant>,
    /// Total reaction amount, divided into `steps` equal steps (Example data
    /// block 2 syntax: "reaction_amount units in steps steps").
    pub amount: String,
    pub units: String,
    pub steps: String,
}

impl Default for Reaction {
    fn default() -> Self {
        Self {
            number: 1,
            description: String::new(),
            reactants: Vec::new(),
            amount: "1.0".to_string(),
            units: "moles".to_string(),
            steps: "1".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReactionTemperature {
    pub number: u32,
    pub description: String,
    pub values: StepsSpec,
}

impl Default for ReactionTemperature {
    fn default() -> Self {
        Self {
            number: 1,
            description: String::new(),
            values: StepsSpec::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReactionPressure {
    pub number: u32,
    pub description: String,
    pub values: StepsSpec,
}

impl Default for ReactionPressure {
    fn default() -> Self {
        Self {
            number: 1,
            description: String::new(),
            values: StepsSpec::default(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct GasComponent {
    pub phase_name: String,
    pub partial_pressure: String,
}

#[derive(Debug, Clone)]
pub struct GasPhase {
    pub number: u32,
    pub description: String,
    pub fixed_volume: bool,
    pub pressure: String,
    pub volume: String,
    pub temperature_c: String,
    pub components: Vec<GasComponent>,
}

impl Default for GasPhase {
    fn default() -> Self {
        Self {
            number: 1,
            description: String::new(),
            fixed_volume: false,
            pressure: "1.0".to_string(),
            volume: "1.0".to_string(),
            temperature_c: "25.0".to_string(),
            components: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ExchangeComponent {
    pub exchange_formula: String,
    pub amount: String,
}

#[derive(Debug, Clone)]
pub struct Exchange {
    pub number: u32,
    pub description: String,
    pub components: Vec<ExchangeComponent>,
}

impl Default for Exchange {
    fn default() -> Self {
        Self {
            number: 1,
            description: String::new(),
            components: Vec::new(),
        }
    }
}

/// Simplified to the diffuse-layer model only (no CD-MUSIC).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceElectrostatics {
    None,
    DiffuseLayer,
}

#[derive(Debug, Clone, Default)]
pub struct SurfaceSite {
    pub surface_formula: String,
    pub amount: String,
    /// Specific area (m^2/g) and mass (g) — required by PHREEQC on the first
    /// site line of a distinct surface name when using the diffuse-layer
    /// model; later sites sharing that surface name can leave these blank.
    pub specific_area_m2_g: String,
    pub mass_g: String,
}

#[derive(Debug, Clone)]
pub struct Surface {
    pub number: u32,
    pub description: String,
    pub sites: Vec<SurfaceSite>,
    pub equilibrate_with_solution: String,
    pub electrostatics: SurfaceElectrostatics,
    pub diffuse_layer_thickness: String,
}

impl Default for Surface {
    fn default() -> Self {
        Self {
            number: 1,
            description: String::new(),
            sites: Vec::new(),
            equilibrate_with_solution: String::new(),
            electrostatics: SurfaceElectrostatics::None,
            diffuse_layer_thickness: String::new(),
        }
    }
}

/// A kinetic reactant referencing a RATES-defined rate name. Rate-law
/// authoring (custom BASIC code) is out of scope.
#[derive(Debug, Clone, Default)]
pub struct KineticReactant {
    pub rate_name: String,
    pub m: String,
    pub m0: String,
    pub parms: String,
    pub tol: String,
}

#[derive(Debug, Clone)]
pub struct Kinetics {
    pub number: u32,
    pub description: String,
    pub reactants: Vec<KineticReactant>,
    pub steps: StepsSpec,
    pub step_units: String,
}

impl Default for Kinetics {
    fn default() -> Self {
        Self {
            number: 1,
            description: String::new(),
            reactants: Vec::new(),
            steps: StepsSpec::default(),
            step_units: "seconds".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SelectedOutput {
    pub number: u32,
    /// Empty = not written; PHREEQC then writes `selected_output_n.sel`.
    pub file: String,
    /// `-reset`: Some(false) turns every switch's default off.
    pub reset: Option<bool>,
    /// The standard column switches, in `SELECTED_OUTPUT_FLAGS` order.
    /// None = not written, so PHREEQC's default (or an earlier simulation's
    /// setting) applies; a file read in keeps exactly the switches it had.
    pub flags: [Option<bool>; SELECTED_OUTPUT_FLAGS.len()],
    /// Space-separated element/valence-state names, e.g. "Ca Mg Na Cl".
    pub totals: String,
    pub molalities: String,
    pub activities: String,
    /// Phase names whose saturation index is written (`si_` columns).
    pub saturation_indices: String,
    /// Equilibrium-phase names whose amount and change are written
    /// (`Calcite` and `d_Calcite` columns).
    pub equilibrium_phases: String,
    /// Gas-phase component names (`g_` columns).
    pub gases: String,
}

/// (identifier, PHREEQC's default) for each standard column switch.
pub const SELECTED_OUTPUT_FLAGS: [(&str, bool); 16] = [
    ("simulation", true),
    ("state", true),
    ("solution", true),
    ("distance", true),
    ("time", true),
    ("step", true),
    ("pH", true),
    ("pe", true),
    ("reaction", false),
    ("temperature", false),
    ("alkalinity", false),
    ("ionic_strength", false),
    ("water", false),
    ("charge_balance", false),
    ("percent_error", false),
    ("high_precision", false),
];

impl SelectedOutput {
    /// A block as read from a file: nothing written beyond what it states.
    pub fn empty(number: u32) -> Self {
        Self {
            number,
            file: String::new(),
            reset: None,
            flags: [None; SELECTED_OUTPUT_FLAGS.len()],
            totals: String::new(),
            molalities: String::new(),
            activities: String::new(),
            saturation_indices: String::new(),
            equilibrium_phases: String::new(),
            gases: String::new(),
        }
    }

    /// Whether a column is written, counting PHREEQC's default.
    pub fn flag(&self, index: usize) -> bool {
        let (name, default) = SELECTED_OUTPUT_FLAGS[index];
        let default = default && (self.reset != Some(false) || name == "high_precision");
        self.flags[index].unwrap_or(default)
    }
}

impl Default for SelectedOutput {
    /// A new block writes every switch, with temperature on since most
    /// runs that are plotted vary it.
    fn default() -> Self {
        let mut so = Self::empty(1);
        so.file = "selected.out".to_string();
        for (i, (name, default)) in SELECTED_OUTPUT_FLAGS.iter().enumerate() {
            so.flags[i] = Some(*default || *name == "temperature");
        }
        so
    }
}

#[derive(Debug, Clone, Default)]
pub struct MixComponent {
    pub solution_number: String,
    pub fraction: String,
}

#[derive(Debug, Clone)]
pub struct Mix {
    pub number: u32,
    pub description: String,
    pub components: Vec<MixComponent>,
}

impl Default for Mix {
    fn default() -> Self {
        Self {
            number: 1,
            description: String::new(),
            components: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Advection {
    pub cells: String,
    pub shifts: String,
    pub time_step: String,
    pub time_step_unit: String,
    pub initial_time: String,
    pub warnings: bool,
}

impl Default for Advection {
    fn default() -> Self {
        Self {
            cells: "1".to_string(),
            shifts: "1".to_string(),
            time_step: String::new(),
            time_step_unit: "second".to_string(),
            initial_time: String::new(),
            warnings: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowDirection {
    Forward,
    Back,
    Diffusion,
}

/// Simplified to uniform lengths/dispersivities (a single value applied to
/// every cell) rather than PHREEQC's full per-cell list syntax (e.g.
/// `4*1.0 2.0`) — covers the common case; -stagnant/-multi_D/-interlayer_D
/// (multi-region and multicomponent diffusion) are out of scope for now,
/// same kind of scope line already drawn for SURFACE's CD-MUSIC.
#[derive(Debug, Clone)]
pub struct Transport {
    pub cells: String,
    pub shifts: String,
    pub time_step: String,
    pub time_step_unit: String,
    pub flow_direction: FlowDirection,
    pub boundary_condition_top: String,
    pub boundary_condition_bottom: String,
    pub uniform_length: String,
    pub uniform_dispersivity: String,
    pub diffusion_coefficient: String,
}

impl Default for Transport {
    fn default() -> Self {
        Self {
            cells: "1".to_string(),
            shifts: "1".to_string(),
            time_step: String::new(),
            time_step_unit: "second".to_string(),
            flow_direction: FlowDirection::Forward,
            boundary_condition_top: "flux".to_string(),
            boundary_condition_bottom: "flux".to_string(),
            uniform_length: "1.0".to_string(),
            uniform_dispersivity: "0.0".to_string(),
            diffusion_coefficient: String::new(),
        }
    }
}

/// A single true/false switch affecting REACTION step and KINETICS time-step
/// interpretation app-wide, not a numbered block — modeled as a block anyway
/// so it fits the same tree/editor pattern as everything else.
#[derive(Debug, Clone)]
pub struct IncrementalReactions {
    pub enabled: bool,
}

impl Default for IncrementalReactions {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, Default)]
pub struct InverseBalance {
    pub element_or_ph: String,
    pub uncertainty: String,
}

#[derive(Debug, Clone)]
pub struct InverseModeling {
    pub number: u32,
    pub description: String,
    /// Solution numbers in order; the last is the "final" solution, all
    /// others are "initial" solutions mixed to form it.
    pub solutions: String,
    pub uncertainty: String,
    /// One phase name per line (the force/precipitate-only/isotope
    /// constraint columns PHREEQC also allows per phase are out of scope).
    pub phases: Vec<String>,
    pub balances: Vec<InverseBalance>,
    pub range: bool,
    pub minimal: bool,
    pub tolerance: String,
}

impl Default for InverseModeling {
    fn default() -> Self {
        Self {
            number: 1,
            description: String::new(),
            solutions: String::new(),
            uncertainty: "0.05".to_string(),
            phases: Vec::new(),
            balances: Vec::new(),
            range: false,
            minimal: false,
            tolerance: String::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SolidSolutionComponent {
    pub phase_name: String,
    pub moles: String,
}

#[derive(Debug, Clone, Default)]
pub struct SolidSolution {
    pub name: String,
    /// Ideal solid solution: one or more `-comp` entries. Nonideal binary
    /// (`-comp1`/`-comp2` plus Guggenheim/Margules excess-free-energy
    /// parameters) is out of scope for now — same kind of scope line as
    /// SURFACE's CD-MUSIC and Kinetics' custom RATES.
    pub components: Vec<SolidSolutionComponent>,
}

#[derive(Debug, Clone)]
pub struct SolidSolutions {
    pub number: u32,
    pub description: String,
    pub solid_solutions: Vec<SolidSolution>,
}

impl Default for SolidSolutions {
    fn default() -> Self {
        Self {
            number: 1,
            description: String::new(),
            solid_solutions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SpreadRow {
    pub number: String,
    pub description: String,
    /// Values in the same order as `SolutionSpread::columns`.
    pub values: Vec<String>,
}

/// Tabular multi-solution definition, matching PHREEQC's spreadsheet-style
/// SOLUTION_SPREAD format: default properties for any blank cell, then a
/// header row of column names, then one row per solution. Simplified: no
/// per-column units row (put units in the column name itself, e.g.
/// "Alkalinity as HCO3") and no -isotope support.
#[derive(Debug, Clone)]
pub struct SolutionSpread {
    pub default_temperature_c: String,
    pub default_ph: String,
    pub default_pe: String,
    pub default_units: String,
    pub default_density: String,
    pub columns: Vec<String>,
    pub rows: Vec<SpreadRow>,
}

impl Default for SolutionSpread {
    fn default() -> Self {
        Self {
            default_temperature_c: "25".to_string(),
            default_ph: "7".to_string(),
            default_pe: "4".to_string(),
            default_units: "mmol/kgw".to_string(),
            default_density: "1".to_string(),
            columns: Vec::new(),
            rows: Vec::new(),
        }
    }
}

/// The real PHREEQC entity types that SAVE/USE/DUMP operate on (the exact
/// set varies slightly per keyword in the manual; this is their union).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entity {
    EquilibriumPhases,
    Exchange,
    GasPhase,
    Kinetics,
    Mix,
    Reaction,
    ReactionPressure,
    ReactionTemperature,
    SolidSolution,
    Solution,
    Surface,
}

impl Entity {
    pub const ALL: &'static [Entity] = &[
        Entity::EquilibriumPhases,
        Entity::Exchange,
        Entity::GasPhase,
        Entity::Kinetics,
        Entity::Mix,
        Entity::Reaction,
        Entity::ReactionPressure,
        Entity::ReactionTemperature,
        Entity::SolidSolution,
        Entity::Solution,
        Entity::Surface,
    ];

    pub fn keyword(self) -> &'static str {
        match self {
            Entity::EquilibriumPhases => "equilibrium_phases",
            Entity::Exchange => "exchange",
            Entity::GasPhase => "gas_phase",
            Entity::Kinetics => "kinetics",
            Entity::Mix => "mix",
            Entity::Reaction => "reaction",
            Entity::ReactionPressure => "reaction_pressure",
            Entity::ReactionTemperature => "reaction_temperature",
            Entity::SolidSolution => "solid_solution",
            Entity::Solution => "solution",
            Entity::Surface => "surface",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Entity::EquilibriumPhases => "Equilibrium Phases",
            Entity::Exchange => "Exchange",
            Entity::GasPhase => "Gas Phase",
            Entity::Kinetics => "Kinetics",
            Entity::Mix => "Mix",
            Entity::Reaction => "Reaction",
            Entity::ReactionPressure => "Reaction Pressure",
            Entity::ReactionTemperature => "Reaction Temperature",
            Entity::SolidSolution => "Solid Solution",
            Entity::Solution => "Solution",
            Entity::Surface => "Surface",
        }
    }
}

impl Default for Entity {
    fn default() -> Self {
        Entity::Solution
    }
}

/// `SAVE keyword number` — takes no block number of its own (confirmed in
/// TM 6-A43: the number is the *target* composition slot, not a block
/// index), so this is numberless like Advection/Transport.
#[derive(Debug, Clone)]
pub struct Save {
    pub entity: Entity,
    pub number: String,
}

impl Default for Save {
    fn default() -> Self {
        Self {
            entity: Entity::Solution,
            number: "1".to_string(),
        }
    }
}

/// `USE keyword (number|none)` — same numberless shape as Save. SAVE's
/// entity list is a strict subset of USE's (USE additionally allows
/// Kinetics, Mix, Reaction, ReactionPressure, ReactionTemperature as
/// targets) — reusing the same `Entity` enum for both since PHREEQC accepts
/// an entity on USE that SAVE doesn't (not vice versa), so nothing invalid
/// is representable by relaxing SAVE's own choices to the same set.
#[derive(Debug, Clone)]
pub struct Use {
    pub entity: Entity,
    /// Empty means "none" (explicitly use no reactant of this type).
    pub number: String,
}

impl Default for Use {
    fn default() -> Self {
        Self {
            entity: Entity::Solution,
            number: "1".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DumpEntitySelection {
    pub entity: Entity,
    pub enabled: bool,
    /// Empty = dump every defined instance of this entity type.
    pub ranges: String,
}

/// `DUMP` — numberless, "No other data are input on the keyword line."
#[derive(Debug, Clone)]
pub struct Dump {
    pub file: String,
    pub append: bool,
    /// `-all`: dump every entity of every type, ignoring the per-entity list.
    pub all: bool,
    pub entities: Vec<DumpEntitySelection>,
}

impl Default for Dump {
    fn default() -> Self {
        Self {
            file: "dump.out".to_string(),
            append: false,
            all: true,
            entities: Entity::ALL
                .iter()
                .map(|&entity| DumpEntitySelection {
                    entity,
                    enabled: false,
                    ranges: String::new(),
                })
                .collect(),
        }
    }
}

/// `KNOBS` — numerical-method tuning, numberless. The `-debug_*` developer
/// flags are out of scope (not useful to an end user tuning convergence).
#[derive(Debug, Clone)]
pub struct Knobs {
    pub iterations: String,
    pub convergence_tolerance: String,
    pub tolerance: String,
    pub step_size: String,
    pub pe_step_size: String,
    pub diagonal_scale: bool,
}

impl Default for Knobs {
    fn default() -> Self {
        Self {
            iterations: String::new(),
            convergence_tolerance: String::new(),
            tolerance: String::new(),
            step_size: String::new(),
            pe_step_size: String::new(),
            diagonal_scale: false,
        }
    }
}

/// `PRINT` — toggles for what's written to the main output file, numberless.
/// Covers the identifiers most relevant to a typical session; `-status`,
/// `-warnings` (a count, not a bool), and the isotope/debug-oriented flags
/// are out of scope.
#[derive(Debug, Clone)]
pub struct Print {
    pub echo_input: bool,
    pub equilibrium_phases: bool,
    pub exchange: bool,
    pub gas_phase: bool,
    pub headings: bool,
    pub inverse_modeling: bool,
    pub kinetics: bool,
    pub saturation_indices: bool,
    pub solid_solutions: bool,
    pub species: bool,
    pub surface: bool,
    pub totals: bool,
    pub user_print: bool,
    pub user_graph: bool,
    pub selected_output: bool,
    pub high_precision: bool,
}

impl Default for Print {
    fn default() -> Self {
        Self {
            echo_input: true,
            equilibrium_phases: true,
            exchange: true,
            gas_phase: true,
            headings: true,
            inverse_modeling: true,
            kinetics: true,
            saturation_indices: true,
            solid_solutions: true,
            species: true,
            surface: true,
            totals: true,
            user_print: true,
            user_graph: true,
            selected_output: true,
            high_precision: false,
        }
    }
}

/// Verbatim passthrough for any top-level PHREEQC keyword block this app
/// doesn't structurally model (ISOTOPES, USE, KNOBS, PRINT, DUMP, PITZER,
/// SIT, PHASES, RATES, *_MASTER_SPECIES, *_SPECIES, USER_GRAPH, USER_PUNCH,
/// USER_PRINT, SAVE, COPY, DELETE, RUN_CELLS, NAMED_EXPRESSIONS,
/// CALCULATE_VALUES, and anything else not in BLOCK_KINDS). Preserves
/// imported content this app can't parse rather than silently dropping it
/// ("never destroy what it can't parse"). Not creatable from the Insert menu
/// (there's nothing meaningful to author into it); only produced by Open.
#[derive(Debug, Clone, Default)]
pub struct Raw {
    /// The exact top-level line as encountered, e.g. "ISOTOPES 1 my comment".
    pub header_line: String,
    /// The block's body lines, verbatim, newline-joined, no reformatting.
    pub body: String,
}

#[derive(Debug, Clone)]
pub enum Block {
    Solution(Solution),
    EquilibriumPhases(EquilibriumPhases),
    Reaction(Reaction),
    ReactionTemperature(ReactionTemperature),
    ReactionPressure(ReactionPressure),
    GasPhase(GasPhase),
    Exchange(Exchange),
    Surface(Surface),
    Kinetics(Kinetics),
    SelectedOutput(SelectedOutput),
    Mix(Mix),
    Advection(Advection),
    Transport(Transport),
    IncrementalReactions(IncrementalReactions),
    InverseModeling(InverseModeling),
    SolidSolutions(SolidSolutions),
    SolutionSpread(SolutionSpread),
    Save(Save),
    Use(Use),
    Dump(Dump),
    Knobs(Knobs),
    Print(Print),
    Raw(Raw),
}

/// Database-definition keywords, edited as PHREEQC text (a `Raw` block)
/// rather than structured forms. Each starts from a fully commented template
/// so inserting one never changes a model until the user writes definitions.
/// (menu label, keyword, template body)
pub const DATABASE_KEYWORDS: &[(&str, &str, &str)] = &[
    (
        "Phases",
        "PHASES",
        "# Define or redefine a mineral or gas. Illustrative example:\n\
         # Gypsum\n\
         #     CaSO4:2H2O = Ca+2 + SO4-2 + 2 H2O\n\
         #     log_k     -4.58\n\
         #     delta_h   -0.109 kcal\n\
         #     -analytic  68.2401  0.0  -3221.51  -25.0627  0.0\n",
    ),
    (
        "Solution Master Species",
        "SOLUTION_MASTER_SPECIES",
        "# element   master species   alkalinity   gfw or formula   gfw of element\n\
         # Ca        Ca+2             0.0          Ca               40.08\n",
    ),
    (
        "Solution Species",
        "SOLUTION_SPECIES",
        "# Association reaction followed by its constants. Illustrative example:\n\
         # Ca+2 + SO4-2 = CaSO4\n\
         #     log_k    2.3\n\
         #     delta_h  1.65 kcal\n",
    ),
    (
        "Exchange Master Species",
        "EXCHANGE_MASTER_SPECIES",
        "# exchange name   exchange master species\n\
         # X               X-\n",
    ),
    (
        "Exchange Species",
        "EXCHANGE_SPECIES",
        "# Half-reactions for exchange sites. Illustrative example:\n\
         # X- = X-\n\
         #     log_k  0.0\n\
         # Na+ + X- = NaX\n\
         #     log_k  0.0\n\
         #     -gamma 4.0 0.075\n",
    ),
    (
        "Surface Master Species",
        "SURFACE_MASTER_SPECIES",
        "# binding-site name   surface master species\n\
         # Hfo_w               Hfo_wOH\n",
    ),
    (
        "Surface Species",
        "SURFACE_SPECIES",
        "# Surface complexation reactions. Illustrative example:\n\
         # Hfo_wOH = Hfo_wOH\n\
         #     log_k  0.0\n\
         # Hfo_wOH + H+ = Hfo_wOH2+\n\
         #     log_k  7.29\n",
    ),
    (
        "Pitzer",
        "PITZER",
        "# Pitzer interaction parameters; use with a Pitzer database such as pitzer.dat.\n\
         # PITZER and SIT blocks cannot be used in the same run.\n\
         # Illustrative values only:\n\
         # -B0\n\
         #     Na+  Cl-  0.0765\n\
         # -B1\n\
         #     Na+  Cl-  0.2664\n\
         # -C0\n\
         #     Na+  Cl-  0.00127\n\
         # -THETA\n\
         #     K+   Na+  -0.012\n\
         # -PSI\n\
         #     Na+  K+  Cl-  -0.0018\n",
    ),
    (
        "SIT",
        "SIT",
        "# SIT ion-interaction coefficients; use with sit.dat.\n\
         # PITZER and SIT blocks cannot be used in the same run. Illustrative value:\n\
         # -epsilon\n\
         #     Na+  Cl-  0.03\n",
    ),
];

/// All block kinds this app can create, in the order the real GUI's "New X…"
/// tree-context menu groups them (PhreeqcI menu resource 148), restricted to
/// the 11 types used day to day.
pub const BLOCK_KINDS: &[&str] = &[
    "Solution",
    "Equilibrium Phases",
    "Reaction",
    "Reaction Temperature",
    "Reaction Pressure",
    "Gas Phase",
    "Exchange",
    "Surface",
    "Kinetics",
    "Selected Output",
    "Mix",
    "Advection",
    "Transport",
    "Incremental Reactions",
    "Inverse Modeling",
    "Solid Solutions",
    "Solution Spread",
    "Save",
    "Use",
    "Dump",
    "Knobs",
    "Print",
];

impl Block {
    pub fn new_default(kind: &str, number: u32) -> Option<Block> {
        Some(match kind {
            "Solution" => Block::Solution(Solution {
                number,
                ..Default::default()
            }),
            "Equilibrium Phases" => Block::EquilibriumPhases(EquilibriumPhases {
                number,
                ..Default::default()
            }),
            "Reaction" => Block::Reaction(Reaction {
                number,
                ..Default::default()
            }),
            "Reaction Temperature" => Block::ReactionTemperature(ReactionTemperature {
                number,
                ..Default::default()
            }),
            "Reaction Pressure" => Block::ReactionPressure(ReactionPressure {
                number,
                ..Default::default()
            }),
            "Gas Phase" => Block::GasPhase(GasPhase {
                number,
                ..Default::default()
            }),
            "Exchange" => Block::Exchange(Exchange {
                number,
                ..Default::default()
            }),
            "Surface" => Block::Surface(Surface {
                number,
                ..Default::default()
            }),
            "Kinetics" => Block::Kinetics(Kinetics {
                number,
                ..Default::default()
            }),
            "Selected Output" => Block::SelectedOutput(SelectedOutput {
                number,
                ..Default::default()
            }),
            "Mix" => Block::Mix(Mix {
                number,
                ..Default::default()
            }),
            "Advection" => Block::Advection(Advection::default()),
            "Transport" => Block::Transport(Transport::default()),
            "Incremental Reactions" => Block::IncrementalReactions(IncrementalReactions::default()),
            "Inverse Modeling" => Block::InverseModeling(InverseModeling {
                number,
                ..Default::default()
            }),
            "Solid Solutions" => Block::SolidSolutions(SolidSolutions {
                number,
                ..Default::default()
            }),
            "Solution Spread" => Block::SolutionSpread(SolutionSpread::default()),
            "Save" => Block::Save(Save::default()),
            "Use" => Block::Use(Use::default()),
            "Dump" => Block::Dump(Dump::default()),
            "Knobs" => Block::Knobs(Knobs::default()),
            "Print" => Block::Print(Print::default()),
            _ => return None,
        })
    }

    pub fn kind_name(&self) -> &'static str {
        match self {
            Block::Solution(_) => "Solution",
            Block::EquilibriumPhases(_) => "Equilibrium Phases",
            Block::Reaction(_) => "Reaction",
            Block::ReactionTemperature(_) => "Reaction Temperature",
            Block::ReactionPressure(_) => "Reaction Pressure",
            Block::GasPhase(_) => "Gas Phase",
            Block::Exchange(_) => "Exchange",
            Block::Surface(_) => "Surface",
            Block::Kinetics(_) => "Kinetics",
            Block::SelectedOutput(_) => "Selected Output",
            Block::Mix(_) => "Mix",
            Block::Advection(_) => "Advection",
            Block::Transport(_) => "Transport",
            Block::IncrementalReactions(_) => "Incremental Reactions",
            Block::InverseModeling(_) => "Inverse Modeling",
            Block::SolidSolutions(_) => "Solid Solutions",
            Block::SolutionSpread(_) => "Solution Spread",
            Block::Save(_) => "Save",
            Block::Use(_) => "Use",
            Block::Dump(_) => "Dump",
            Block::Knobs(_) => "Knobs",
            Block::Print(_) => "Print",
            Block::Raw(_) => "Other",
        }
    }

    /// True for the real PHREEQC keywords that take no number at all
    /// (ADVECTION, TRANSPORT, INCREMENTAL_REACTIONS, SOLUTION_SPREAD, SAVE,
    /// USE, DUMP, KNOBS, PRINT) — `number()` returns a fixed placeholder for
    /// these and must not be serialized into their header line.
    pub fn is_numberless(&self) -> bool {
        matches!(
            self,
            Block::Advection(_)
                | Block::Transport(_)
                | Block::IncrementalReactions(_)
                | Block::SolutionSpread(_)
                | Block::Save(_)
                | Block::Use(_)
                | Block::Dump(_)
                | Block::Knobs(_)
                | Block::Print(_)
                | Block::Raw(_)
        )
    }

    pub fn number(&self) -> u32 {
        match self {
            Block::Solution(b) => b.number,
            Block::EquilibriumPhases(b) => b.number,
            Block::Reaction(b) => b.number,
            Block::ReactionTemperature(b) => b.number,
            Block::ReactionPressure(b) => b.number,
            Block::GasPhase(b) => b.number,
            Block::Exchange(b) => b.number,
            Block::Surface(b) => b.number,
            Block::Kinetics(b) => b.number,
            Block::SelectedOutput(b) => b.number,
            Block::Mix(b) => b.number,
            Block::InverseModeling(b) => b.number,
            Block::SolidSolutions(b) => b.number,
            Block::Advection(_)
            | Block::Transport(_)
            | Block::IncrementalReactions(_)
            | Block::SolutionSpread(_)
            | Block::Save(_)
            | Block::Use(_)
            | Block::Dump(_)
            | Block::Knobs(_)
            | Block::Print(_)
            | Block::Raw(_) => 1,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Block::Solution(b) => &b.description,
            Block::EquilibriumPhases(b) => &b.description,
            Block::Reaction(b) => &b.description,
            Block::ReactionTemperature(b) => &b.description,
            Block::ReactionPressure(b) => &b.description,
            Block::GasPhase(b) => &b.description,
            Block::Exchange(b) => &b.description,
            Block::Surface(b) => &b.description,
            Block::Kinetics(b) => &b.description,
            Block::SelectedOutput(_) => "",
            Block::Mix(b) => &b.description,
            Block::InverseModeling(b) => &b.description,
            Block::SolidSolutions(b) => &b.description,
            Block::Advection(_)
            | Block::Transport(_)
            | Block::IncrementalReactions(_)
            | Block::SolutionSpread(_)
            | Block::Save(_)
            | Block::Use(_)
            | Block::Dump(_)
            | Block::Knobs(_)
            | Block::Print(_) => "",
            Block::Raw(b) => &b.header_line,
        }
    }

    /// Richer tree label for Save/Use, since their meaning is "which entity
    /// and which number" rather than a free-text description.
    pub fn save_use_label(&self) -> Option<String> {
        match self {
            Block::Save(s) => Some(format!("Save {} {}", s.entity.label(), s.number)),
            Block::Use(u) => Some(format!(
                "Use {} {}",
                u.entity.label(),
                if u.number.trim().is_empty() {
                    "none"
                } else {
                    &u.number
                }
            )),
            _ => None,
        }
    }

    pub fn label(&self) -> String {
        if let Block::Raw(r) = self {
            return r.header_line.clone();
        }
        if let Some(label) = self.save_use_label() {
            return label;
        }
        if self.is_numberless() {
            return self.kind_name().to_string();
        }
        let desc = self.description();
        if desc.trim().is_empty() {
            format!("{} {}", self.kind_name(), self.number())
        } else {
            format!("{} {} — {}", self.kind_name(), self.number(), desc.trim())
        }
    }
}

#[derive(Debug, Clone)]
pub struct Document {
    pub title: String,
    pub blocks: Vec<Block>,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            title: String::new(),
            blocks: vec![Block::Solution(Solution::default())],
        }
    }
}

impl Document {
    pub fn next_number_for(&self, kind: &str) -> u32 {
        self.blocks
            .iter()
            .filter(|b| b.kind_name() == kind)
            .map(|b| b.number())
            .max()
            .map(|n| n + 1)
            .unwrap_or(1)
    }
}
