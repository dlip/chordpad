use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;

use padtype::{
    Result,
    corpus::{Corpus, CorpusSummary, Stats},
    inspector,
    model::*,
    search::{self, Problem, Score},
};

#[derive(Parser)]
#[command(
    version,
    about = "Generate gamepad typing layouts, inspect chords, and type on Linux"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Search direct chords and held-modifier strategies.
    Generate(Generate),
    /// Evaluate an exported layout on another corpus.
    Score {
        #[arg(long)]
        layout: PathBuf,
        #[arg(long)]
        corpus: PathBuf,
    },
    /// Print the complete editable configuration, including all cost estimates.
    Defaults,
    /// List readable Linux gamepad event devices and their controls.
    Devices {
        #[arg(long)]
        json: bool,
    },
    /// Display normalized controls and recognized chords without typing.
    Inspect(inspector::InspectArgs),
    /// Measure analog trigger endpoints and save a new controller mapping.
    Calibrate(inspector::CalibrateArgs),
    /// Replay a recorded controller session without hardware or delays.
    Replay(inspector::ReplayArgs),
    /// Type with a generated layout using a Linux virtual keyboard (starts paused).
    Type(padtype::typing::TypeArgs),
    /// Export a searchable, printable visual reference for a generated layout.
    Render {
        #[arg(long)]
        layout: PathBuf,
        #[arg(short, long, default_value = "layout.html")]
        output: PathBuf,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Alphabet {
    Ascii,
    Corpus,
}

#[derive(clap::Args)]
struct Generate {
    /// Generate independent half strokes with L2 Shift, R2 numbers/symbols, and fixed editing keys.
    #[arg(long)]
    independent: bool,
    /// Add a held R2 + L1 navigation layer with arrows and page/home/end keys.
    #[arg(long)]
    navigation_layer: bool,
    /// Add a held R2 + R1 function layer using the number chords for F1–F10.
    #[arg(long)]
    function_layer: bool,
    /// Exclude A+Y and B+X, including their supersets and modifier contexts.
    #[arg(long)]
    exclude_opposite_face_pairs: bool,
    /// Estimated cost for a transition that reuses an occupied hand.
    #[arg(long)]
    same_hand_penalty: Option<f64>,
    /// UTF-8 training text. Defaults to the small bundled demonstration corpus.
    #[arg(long)]
    corpus: Option<PathBuf>,
    /// Evaluate on separate text, without using its frequencies during search.
    #[arg(long)]
    held_out: Option<PathBuf>,
    /// JSON profile, costs, weights, strategies, and pinned bindings.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Required characters: printable ASCII + Enter/Tab, or training characters only.
    /// Both choices also include observed Unicode characters, Backspace, and Escape.
    #[arg(long, value_enum, default_value = "ascii")]
    alphabet: Alphabet,
    /// Additional required characters, useful with --alphabet corpus.
    #[arg(long, default_value = "")]
    include: String,
    /// Restrict search to named strategies. May be repeated.
    #[arg(long)]
    strategy: Vec<String>,
    #[arg(long, default_value_t = 42)]
    seed: u64,
    /// Annealing proposals per restart, per strategy.
    #[arg(long, default_value_t = 1_000)]
    iterations: usize,
    #[arg(long, default_value_t = 3)]
    restarts: usize,
    #[arg(short, long, default_value = "layout.json")]
    output: PathBuf,
    /// Complete results, including every candidate layout. Defaults to OUTPUT.report.json.
    #[arg(long)]
    report: Option<PathBuf>,
}

#[derive(Serialize)]
struct Candidate {
    strategy: String,
    seeds: Vec<u64>,
    baseline: Score,
    training: Score,
    held_out: Option<Score>,
    layout: Layout,
}

#[derive(Serialize)]
struct Skipped {
    strategy: String,
    reason: String,
}

#[derive(Serialize)]
struct Report {
    schema_version: u32,
    generator_version: &'static str,
    dependency_lock_sha256: String,
    cost_model_version: u32,
    seed: u64,
    iterations_per_restart: usize,
    restarts_per_strategy: usize,
    training_corpus: CorpusSummary,
    held_out_corpus: Option<CorpusSummary>,
    config: Config,
    reference_strategy: String,
    normalization: Metrics,
    selection: &'static str,
    assumptions: Vec<&'static str>,
    candidates: Vec<Candidate>,
    skipped: Vec<Skipped>,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Defaults => {
            println!("{}", json(&Config::default())?);
            Ok(())
        }
        Command::Score { layout, corpus } => {
            let layout: Layout = read_json(&layout)?;
            let corpus = Corpus::new(&read_text(&corpus)?)?;
            let actions = layout.scoring_actions();
            let stats = Stats::new(&corpus, &actions)?;
            let (problem, assignment) = search::load_layout(&layout, stats)?;
            let score = problem.summary(&assignment, &layout.weights, layout.normalization);
            check_score(&score)?;
            println!("{}", json(&score)?);
            Ok(())
        }
        Command::Generate(args) => generate(args),
        Command::Replay(args) => inspector::replay(args),
        Command::Render { layout, output } => {
            if resolved_path(&layout)? == resolved_path(&output)? {
                return Err("render output must not overwrite the input layout".into());
            }
            let layout: Layout = read_json(&layout)?;
            let html = padtype::render::html(&layout)?;
            stage_file(&output, &html)?.persist(&output)
                .map_err(|e| format!("{}: {e}", output.display()))?;
            println!("Visual layout: {}", output.display());
            Ok(())
        }
        #[cfg(target_os="linux")]
        Command::Devices{json}=>padtype::linux::devices(json),
        #[cfg(target_os="linux")]
        Command::Inspect(args)=>padtype::linux::inspect(args),
        #[cfg(target_os="linux")]
        Command::Calibrate(args)=>padtype::linux::calibrate(args),
        #[cfg(target_os="linux")]
        Command::Type(args)=>padtype::linux::type_layout(args),
        #[cfg(not(target_os="linux"))]
        Command::Devices{..}|Command::Inspect(_)|Command::Calibrate(_)|Command::Type(_)=>Err("live controller access and typing require Linux; recorded sessions can be replayed on this platform".into()),
    }
}

fn generate(args: Generate) -> Result<()> {
    if args.iterations == 0 || args.restarts == 0 {
        return Err("iterations and restarts must be positive".into());
    }
    let report_path = args
        .report
        .clone()
        .unwrap_or_else(|| args.output.with_extension("report.json"));
    let output_paths = [resolved_path(&args.output)?, resolved_path(&report_path)?];
    if output_paths[0] == output_paths[1] {
        return Err("layout and report paths must be different".into());
    }
    for input in [
        args.corpus.as_deref(),
        args.held_out.as_deref(),
        args.config.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if output_paths.contains(&resolved_path(input)?) {
            return Err(format!(
                "output paths must not overwrite input file {}",
                input.display()
            ));
        }
    }
    let mut config: Config = args
        .config
        .as_ref()
        .map(|p| read_json(p))
        .transpose()?
        .unwrap_or_default();
    if args.independent {
        config.strategies = vec![Strategy {
            independent: true,
            name: "independent".into(),
            shift: Some(Button::L2),
            layer: Some(Button::R2),
        }];
    }
    if args.exclude_opposite_face_pairs {
        config.profile.exclude_opposite_face_pairs = true;
    }
    if args.navigation_layer {
        config.profile.navigation_layer = true;
    }
    if args.function_layer {
        config.profile.function_layer = true;
    }
    if let Some(cost) = args.same_hand_penalty {
        config.profile.costs.same_hand_transition = cost;
    }
    config.validate()?;
    if !args.strategy.is_empty() {
        for name in &args.strategy {
            if !config.strategies.iter().any(|s| &s.name == name) {
                return Err(format!("unknown strategy {name:?}"));
            }
        }
        config
            .strategies
            .retain(|s| args.strategy.contains(&s.name));
    }
    let text = match &args.corpus {
        Some(path) => read_text(path)?,
        None => {
            eprintln!(
                "Using the bundled demonstration corpus. Use --corpus for a representative layout."
            );
            include_str!("../examples/corpus.txt").into()
        }
    };
    let training = Corpus::new(&text)?;
    let held_out = args
        .held_out
        .as_ref()
        .map(|p| Corpus::new(&read_text(p)?))
        .transpose()?;
    let mut actions = training
        .actions(matches!(args.alphabet, Alphabet::Ascii), &args.include)
        .into_iter()
        .collect::<BTreeSet<_>>();
    actions.extend(config.pinned_bindings.iter().map(|b| b.action.clone()));
    if config.profile.function_layer {
        actions.extend(('0'..='9').map(Action::Character));
    }
    if config.strategies.iter().any(|s| s.independent) {
        actions.insert(Action::Character('\n'));
        actions.insert(Action::Character(' '));
        actions.insert(Action::Key(Key::Backspace));
    }
    let actions = actions.into_iter().collect::<Vec<_>>();
    let stats = Stats::new(&training, &actions)?;
    let held_stats = held_out
        .as_ref()
        .map(|c| Stats::new(c, &actions))
        .transpose()?;
    let mut prepared = vec![];
    let mut skipped = vec![];
    for strategy in &config.strategies {
        match Problem::new(&config, &actions, stats.clone(), strategy).and_then(|p| {
            let baseline = p.baseline(&config.weights)?;
            Ok((p, baseline))
        }) {
            Ok(preparation) => prepared.push(preparation),
            Err(reason) => skipped.push(Skipped {
                strategy: strategy.name.clone(),
                reason,
            }),
        }
    }
    let (first, first_baseline) = prepared.first().ok_or_else(|| {
        format!(
            "no feasible strategy: {}",
            skipped
                .iter()
                .map(|s| format!("{}: {}", s.strategy, s.reason))
                .collect::<Vec<_>>()
                .join("; ")
        )
    })?;
    let reference_strategy = first.strategy.name.clone();
    let normalization = first.metrics(first_baseline).normalization();
    let mut candidates = vec![];
    for (index, (problem, baseline)) in prepared.into_iter().enumerate() {
        eprintln!(
            "Searching {}: {} required actions, {} candidate slots",
            problem.strategy.name,
            actions.len(),
            problem.slots.len()
        );
        let seeds = (0..args.restarts)
            .map(|restart| {
                args.seed
                    .wrapping_add((index as u64).wrapping_mul(0x9e3779b97f4a7c15))
                    .wrapping_add(restart as u64)
            })
            .collect::<Vec<_>>();
        let baseline_score = problem.summary(&baseline, &config.weights, normalization);
        check_score(&baseline_score)?;
        let assignment = search::optimize(
            &problem,
            &baseline,
            &config.weights,
            normalization,
            args.iterations,
            &seeds,
        );
        let training_score = problem.summary(&assignment, &config.weights, normalization);
        check_score(&training_score)?;
        let held_score = held_stats
            .as_ref()
            .map(|stats| {
                let evaluation = Problem::new(&config, &actions, stats.clone(), &problem.strategy)?;
                let score = evaluation.summary(&assignment, &config.weights, normalization);
                check_score(&score)?;
                Ok::<_, String>(score)
            })
            .transpose()?;
        let layout = problem.layout(&assignment, &config.weights, normalization);
        search::load_layout(&layout, problem.stats.clone())?;
        eprintln!(
            "  objective {:.4} -> {:.4}; {:.3} activations/character",
            baseline_score.objective, training_score.objective, training_score.metrics.activations
        );
        candidates.push(Candidate {
            strategy: problem.strategy.name.clone(),
            seeds,
            baseline: baseline_score,
            training: training_score,
            held_out: held_score,
            layout,
        });
    }
    candidates.sort_by(|a, b| {
        a.training
            .objective
            .total_cmp(&b.training.objective)
            .then(a.strategy.cmp(&b.strategy))
    });
    let report = Report {
        schema_version: SCHEMA_VERSION,
        generator_version: env!("CARGO_PKG_VERSION"),
        dependency_lock_sha256: padtype::corpus::sha256(include_bytes!("../Cargo.lock")),
        cost_model_version: COST_MODEL_VERSION,
        seed: args.seed,
        iterations_per_restart: args.iterations,
        restarts_per_strategy: args.restarts,
        training_corpus: (&training).into(),
        held_out_corpus: held_out.as_ref().map(CorpusSummary::from),
        config,
        reference_strategy,
        normalization,
        selection: "lowest training objective; held-out text is evaluation only",
        assumptions: vec![
            "All ergonomic costs are estimates, not measured speed or error predictions.",
            "Each restart finishes its best annealed assignment with exhaustive best-improvement moves and swaps until none improves the objective beyond a relative 1e-12 numerical tolerance. These passes are additional to the annealing proposal count.",
            "Each corpus is one continuous task, with modifier entry and cleanup at its boundaries.",
            "Independent strategies release each half separately; legacy strategies require global ordinary release.",
            "Independent layouts use global held L2 Shift and R2 numbers/symbols, latched at stroke start; R1 Space and L1 Backspace work in all modifier states.",
            "Independent output follows stroke-start order; same-report starts are left-first.",
            "Text-corpus scores do not model thumbstick shortcuts; invariant Space/Backspace are scored at their base bindings.",
            "The same-hand transition penalty uses full hand occupancy, including held modifiers; it is an estimated preference, not measured timing.",
            "Optional same-finger scoring counts each finger reused by consecutive ordinary strokes. D-pad and face buttons use their respective thumbs; Start/Select use right/left thumbs. Held modifiers affect allocation but do not count as repeated presses.",
            "Each side uses its index for either shoulder alone. With both shoulders active, the index uses L1/R1 and the middle uses L2/R2. Optional grip-change effort charges each middle-finger recruitment and return, including task boundaries; same-side shoulder effort models sustained load.",
            "Shift uses standard US character pairs; uppercase shares lowercase chords. Independent letters stay on the base bank and numbers/symbols use R2.",
            "Movement uses D-pad positions and Xbox-style face-button centroids (A south, B east, X west, Y north).",
            "Optional skipgram movement weights score the next use of each thumb across one, two, or three intervening inputs that do not use that thumb; this adds to adjacent movement without charging extra key presses or modifier changes.",
            "Optional awkward-diagonal effort penalizes D-pad UpRight/DownLeft and face X+Y/A+B, mirrored inner-up/outer-down positions; held D-pad load scales this penalty like other held effort.",
            "Coverage gives unobserved required outputs a uniform access-cost penalty.",
            "Learning is a proxy: one unit per canonical character/key group and four per fixed modifier role.",
            "Backspace and Escape are required but have no observed task frequency in text corpora.",
            "Independent Enter/Escape use Start/Select unless explicitly pinned to chords; dedicated menu controls cost one activation and unit effort, with movement estimated from the neutral thumb positions.",
        ],
        candidates,
        skipped,
    };
    let winner = &report.candidates[0];
    let layout_json = json(&winner.layout)?;
    let report_json = json(&report)?;
    let staged_layout = stage_file(&args.output, &layout_json)?;
    let staged_report = stage_file(&report_path, &report_json)?;
    staged_report
        .persist(&report_path)
        .map_err(|e| format!("{}: {e}", report_path.display()))?;
    staged_layout
        .persist(&args.output)
        .map_err(|e| format!("{}: {e}", args.output.display()))?;
    println!(
        "{:<25} {:>10} {:>12} {:>10} {:>10}",
        "Strategy", "Objective", "Presses/char", "Max chord", "Held-out"
    );
    for candidate in &report.candidates {
        println!(
            "{:<25} {:>10.4} {:>12.3} {:>10} {:>10}",
            candidate.strategy,
            candidate.training.objective,
            candidate.training.metrics.activations,
            candidate.training.max_controls,
            candidate
                .held_out
                .as_ref()
                .map_or_else(|| "-".into(), |s| format!("{:.4}", s.objective))
        );
    }
    for skipped in &report.skipped {
        eprintln!("Skipped {}: {}", skipped.strategy, skipped.reason);
    }
    println!(
        "\nBest layout: {} ({})\nReport: {}",
        args.output.display(),
        winner.strategy,
        report_path.display()
    );
    Ok(())
}

fn check_score(score: &Score) -> Result<()> {
    let m = score.metrics;
    if [
        score.objective,
        m.effort,
        m.activations,
        m.movement,
        m.coverage,
        m.learning,
    ]
    .iter()
    .any(|v| !v.is_finite())
    {
        Err("cost model overflow; reduce the configured costs or weights".into())
    } else {
        Ok(())
    }
}

fn stage_file(path: &Path, contents: &str) -> Result<tempfile::NamedTempFile> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut file = tempfile::Builder::new()
        .prefix(".padtype-")
        .tempfile_in(parent)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    file.write_all(contents.as_bytes())
        .and_then(|()| file.as_file().sync_all())
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(file)
}

fn resolved_path(path: &Path) -> Result<PathBuf> {
    match fs::canonicalize(path) {
        Ok(resolved) => Ok(resolved),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let parent = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            let name = path
                .file_name()
                .ok_or_else(|| format!("{}: missing filename", path.display()))?;
            fs::canonicalize(parent)
                .map(|resolved| resolved.join(name))
                .map_err(|e| format!("{}: {e}", path.display()))
        }
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

fn read_text(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    serde_json::from_str(&read_text(path)?).map_err(|e| format!("{}: {e}", path.display()))
}

fn json<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_string_pretty(value)
        .map(|s| s + "\n")
        .map_err(|e| e.to_string())
}
