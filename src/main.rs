// No console window behind the app on Windows.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod forms;
mod io;
mod model;
mod phreeqc;
mod plot_ui;
mod plotting;
mod runner;
mod serialize;
mod settings;
mod simulations;
mod theme;
mod validate;

use app::App;

fn window_icon() -> Option<egui::IconData> {
    let img = image::load_from_memory_with_format(
        include_bytes!("../assets/icon/pq-studio-256.png"),
        image::ImageFormat::Png,
    )
    .ok()?
    .into_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}

fn main() -> eframe::Result<()> {
    let settings = settings::Settings::load();
    let size = settings
        .window_size
        .filter(|[w, h]| *w >= 900.0 && *h >= 600.0)
        .unwrap_or([1320.0, 880.0]);
    let mut viewport = egui::ViewportBuilder::default()
        .with_title(phreeqc::APP_NAME)
        .with_app_id(phreeqc::APP_ID)
        .with_inner_size(size)
        .with_min_inner_size([900.0, 600.0]);
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        phreeqc::APP_NAME,
        options,
        Box::new(|cc| {
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(App::new(settings)))
        }),
    )
}

/// Headless visual verification, triggered only via env vars that are never
/// set in normal use. Captures egui's own framebuffer after a few frames,
/// saves a PNG and exits. Run it under `xvfb-run` with `WAYLAND_DISPLAY`
/// unset, or the window opens on the real desktop.
pub(crate) fn maybe_take_screenshot(ctx: &egui::Context, app: &mut App) {
    let Ok(path) = std::env::var("PQ_STUDIO_SCREENSHOT_PATH") else {
        return;
    };
    if app.screenshot_requested {
        return;
    }
    if !app.screenshot_seeded {
        app.screenshot_seeded = true;
        if std::env::var("PQ_STUDIO_SCREENSHOT_SEED_DEMO").is_ok() {
            seed_demo_state(app);
        }
        if let Ok(index) = std::env::var("PQ_STUDIO_SCREENSHOT_EXAMPLE") {
            app.pending = Some(app::Pending::OpenExample(index.parse().unwrap_or(0)));
            app.resolve_pending_for_screenshot();
        }
        if let Ok(file) = std::env::var("PQ_STUDIO_SCREENSHOT_OPEN") {
            app.open_path(std::path::Path::new(&file));
            if let Some(i) = std::env::var("PQ_STUDIO_SCREENSHOT_SELECT_INDEX").ok().and_then(|s| s.parse().ok()) {
                app.selected = i;
            }
        }
        if let Ok(list) = std::env::var("PQ_STUDIO_SCREENSHOT_SIM_PLAN") {
            // "1,2,m3" = solutions 1 and 2 and mix 3; PQ_STUDIO_SCREENSHOT_SIM_EQ
            // = "none,1,2" ticks those equilibrium-phases choices.
            let mut plan = simulations::current_plan(&app.doc.blocks);
            plan.runs = list
                .split(',')
                .filter_map(|n| {
                    let n = n.trim();
                    match n.strip_prefix('m') {
                        Some(m) => m.parse().ok().map(|m| (model::Entity::Mix, m)),
                        None => n.parse().ok().map(|n| (model::Entity::Solution, n)),
                    }
                })
                .collect();
            if let Ok(eq) = std::env::var("PQ_STUDIO_SCREENSHOT_SIM_EQ") {
                for (entity, choices) in &mut plan.reactants {
                    if *entity == model::Entity::EquilibriumPhases {
                        *choices = eq.split(',').map(|c| c.trim().parse().ok()).collect();
                    }
                }
            }
            app.apply_sim_plan(&plan);
        }
        if let Ok(kind) = std::env::var("PQ_STUDIO_SCREENSHOT_INSERT") {
            app.selected = app.doc.blocks.len().saturating_sub(1);
            let number = app.doc.next_number_for(&kind);
            if let Some(block) = model::Block::new_default(&kind, number) {
                app.insert_block(block);
            }
        }
        if std::env::var("PQ_STUDIO_SCREENSHOT_SIM_DIALOG").is_ok() {
            app.open_sim_dialog();
        }
        if std::env::var("PQ_STUDIO_SCREENSHOT_BAD_VALUES").is_ok() {
            if let Some(model::Block::Solution(sol)) = app.doc.blocks.first_mut() {
                sol.ph = "7,5".into();
                if let Some(el) = sol.elements.first_mut() {
                    el.concentration = "lots".into();
                }
            }
        }
        if let Some(index) = std::env::var("PQ_STUDIO_SCREENSHOT_INSERT_DB").ok().and_then(|s| s.parse::<usize>().ok()) {
            let (_, keyword, template) = model::DATABASE_KEYWORDS[index.min(model::DATABASE_KEYWORDS.len() - 1)];
            app.doc.blocks.push(model::Block::Raw(model::Raw {
                header_line: keyword.into(),
                body: template.into(),
            }));
            app.selected = app.doc.blocks.len() - 1;
        }
        if std::env::var("PQ_STUDIO_SCREENSHOT_DIRTY").is_ok() {
            app.doc.title.push_str(" (edited)");
        }
        if std::env::var("PQ_STUDIO_SCREENSHOT_PROMPT").is_ok() {
            app.pending = Some(app::Pending::Quit);
        }
        if std::env::var("PQ_STUDIO_SCREENSHOT_AUTORUN").is_ok() {
            app.run_simulation();
        }
    }
    // An auto-started run finishes before frames are counted, unless the
    // running state itself is what is being captured.
    if app.is_running() && std::env::var("PQ_STUDIO_SCREENSHOT_WHILE_RUNNING").is_err() {
        ctx.request_repaint();
        return;
    }
    if app.screenshot_frame_count == 0 {
        apply_plot_hooks(app);
    }
    if let Ok(query) = std::env::var("PQ_STUDIO_SCREENSHOT_FIND") {
        app.set_find_query(&query);
    }
    app.screenshot_frame_count += 1;
    let wait_frames: u32 = std::env::var("PQ_STUDIO_SCREENSHOT_WAIT_FRAMES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);
    if app.screenshot_frame_count < wait_frames {
        ctx.request_repaint();
        return;
    }
    app.screenshot_requested = true;
    ctx.request_screenshot(move |image| {
        let width = image.size[0] as u32;
        let height = image.size[1] as u32;
        let rgba: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
        if let Some(buf) = image::RgbaImage::from_raw(width, height, rgba) {
            if let Err(e) = buf.save(&path) {
                eprintln!("screenshot save failed: {e}");
            } else {
                eprintln!("screenshot saved to {path}");
            }
        }
        std::process::exit(0);
    });
}

/// QA hooks on the finished run's plot: `_Y=pH,si_Calcite` picks Y
/// columns, `_HIDE_SIMS=2,3` hides simulations, `_EXPORT_TABLE`/`_EXPORT_PLOT`
/// write the table (CSV) and chart (PNG/SVG) to the given paths.
fn apply_plot_hooks(app: &mut App) {
    let Some(table) = app.plot.table.clone() else {
        return;
    };
    if let Ok(list) = std::env::var("PQ_STUDIO_SCREENSHOT_Y") {
        app.plot.y_cols = list
            .split(',')
            .filter_map(|name| table.headers.iter().position(|h| h == name.trim()))
            .collect();
    }
    if let Ok(list) = std::env::var("PQ_STUDIO_SCREENSHOT_HIDE_SIMS") {
        app.plot.config.hidden_sims = list.split(',').filter_map(|n| n.trim().parse().ok()).collect();
    }
    // The plot window opens by itself after a run; `_PLOT_WINDOW=0` keeps
    // it closed. `_LEGEND=top-left` (any position name or "outside") places
    // the legend, `_EXPORT_SCALE=n` sets the PNG resolution.
    if std::env::var("PQ_STUDIO_SCREENSHOT_PLOT_WINDOW").is_ok_and(|v| v == "0") {
        app.plot.window_open = false;
    }
    if let Some(position) = std::env::var("PQ_STUDIO_SCREENSHOT_LEGEND").ok().and_then(|p| plotting::LegendPosition::parse(&p)) {
        app.plot.config.legend.position = position;
    }
    if let Some(scale) = std::env::var("PQ_STUDIO_SCREENSHOT_EXPORT_SCALE").ok().and_then(|s| s.parse().ok()) {
        app.plot.config.export_scale = scale;
    }
    app.plot.assign_styles();
    app.plot.mark_dirty();
    if let Ok(path) = std::env::var("PQ_STUDIO_SCREENSHOT_EXPORT_TABLE") {
        let _ = std::fs::write(path, plotting::table_text(&table, &app.plot.config.sim_names, ','));
    }
    if let Ok(path) = std::env::var("PQ_STUDIO_SCREENSHOT_EXPORT_DATA") {
        let data = plotting::plotted_data(&table, app.plot.x_col, &app.plot.y_cols, &app.plot.config);
        let result = if path.ends_with(".csv") {
            std::fs::write(&path, plotting::plotted_csv(&data)).map_err(|e| e.into())
        } else {
            plotting::write_plotted_xlsx(&data, std::path::Path::new(&path))
        };
        if let Err(e) = result {
            eprintln!("data export failed: {e}");
        }
    }
    if let Ok(path) = std::env::var("PQ_STUDIO_SCREENSHOT_EXPORT_PLOT") {
        if let Err(e) = plotting::export(&app.plot, std::path::Path::new(&path)) {
            eprintln!("plot export failed: {e}");
        }
    }
}

/// Visual-QA-only demo state (never runs without both env vars set) so a
/// screenshot check can look at a busier, more realistic state: multiple
/// blocks in the tree, one selected with element rows filled in, rather than
/// just the empty default.
fn seed_demo_state(app: &mut App) {
    use model::*;

    app.doc.title = "Example seawater scenario".into();

    let mut sol = Solution {
        number: 1,
        ..Default::default()
    };
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

    let mut eq = EquilibriumPhases {
        number: 1,
        ..Default::default()
    };
    eq.phases.push(PhaseAmount {
        phase_name: "Calcite".into(),
        saturation_index: "0.0".into(),
        amount: "10".into(),
    });

    let mut so = SelectedOutput {
        number: 1,
        ..Default::default()
    };
    so.totals = "Ca Mg Na Cl".into();

    let mut surf = Surface {
        number: 1,
        ..Default::default()
    };
    surf.sites.push(SurfaceSite {
        surface_formula: "Hfo_wOH".into(),
        amount: "0.2".into(),
        specific_area_m2_g: "600".into(),
        mass_g: "1.0".into(),
    });
    surf.electrostatics = SurfaceElectrostatics::DiffuseLayer;
    surf.equilibrate_with_solution = "1".into();

    app.doc.blocks = vec![
        Block::Solution(sol),
        Block::EquilibriumPhases(eq),
        Block::SelectedOutput(so),
        Block::Mix(Mix {
            components: vec![MixComponent {
                solution_number: "1".into(),
                fraction: "1.0".into(),
            }],
            ..Default::default()
        }),
        Block::Surface(surf),
        Block::Save(Save {
            entity: Entity::Solution,
            number: "2".into(),
        }),
        Block::Use(Use {
            entity: Entity::Exchange,
            number: String::new(),
        }),
        Block::Dump(Dump::default()),
        Block::Knobs(Knobs::default()),
        Block::Print(Print::default()),
    ];
    if std::env::var("PQ_STUDIO_SCREENSHOT_ALL_KINDS").is_ok() {
        app.doc.blocks = BLOCK_KINDS
            .iter()
            .filter_map(|kind| Block::new_default(kind, 1))
            .collect();
    }
    app.selected = std::env::var("PQ_STUDIO_SCREENSHOT_SELECT_INDEX")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if std::env::var("PQ_STUDIO_SCREENSHOT_DEMO_STATUS").is_ok() {
        app.status = Some(("Saved to ~/Documents/seawater.pqi".into(), false));
        app.run_error = Some("ERROR: Solution 0 is needed for advection, but is not defined.".into());
    }
    if std::env::var("PQ_STUDIO_SCREENSHOT_DEMO_ABOUT").is_ok() {
        app.show_about = true;
    }
}
