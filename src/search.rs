use crate::{Result, corpus::Stats, model::*};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    pub input: Input,
    pub layer: bool,
}

#[derive(Clone, Copy)]
struct Output {
    group: usize,
    shifted: bool,
}

pub struct Problem {
    pub actions: Vec<Action>,
    pub slots: Vec<Slot>,
    pub strategy: Strategy,
    pub profile: Profile,
    pub stats: Stats,
    groups: Vec<Action>,
    outputs: Vec<Output>,
    valid: Vec<Vec<usize>>,
    pinned: BTreeMap<usize, usize>,
    movement: Movement,
}

impl Problem {
    pub fn new(
        config: &Config,
        actions: &[Action],
        stats: Stats,
        strategy: &Strategy,
    ) -> Result<Self> {
        if config
            .profile
            .disabled_buttons
            .iter()
            .any(|b| strategy.modifier_mask() & b.bit() != 0)
        {
            return Err("a modifier uses a disabled button".into());
        }
        let groups: Vec<_> = actions
            .iter()
            .map(|a| a.group(strategy.shift.is_some()).0)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let outputs = actions
            .iter()
            .map(|a| {
                let (group, shifted) = a.group(strategy.shift.is_some());
                Output {
                    group: groups.binary_search(&group).unwrap(),
                    shifted,
                }
            })
            .collect();
        let ordinary_exclusions =
            strategy.modifier_mask() | button_mask(&config.profile.modifier_only_buttons)?;
        let mut slots = vec![];
        for layer in [false, true]
            .into_iter()
            .take(if strategy.layer.is_some() { 2 } else { 1 })
        {
            for dpad in Dpad::ALL {
                for mask in 0..=u8::MAX {
                    let input = Input { dpad, mask };
                    if input.controls() != 0 && mask & ordinary_exclusions == 0 {
                        slots.push(Slot { input, layer });
                    }
                }
            }
        }
        let mut p = Self {
            actions: actions.to_vec(),
            slots,
            strategy: strategy.clone(),
            profile: config.profile.clone(),
            stats,
            groups,
            outputs,
            valid: vec![],
            pinned: BTreeMap::new(),
            movement: Movement::new(),
        };
        for group in 0..p.groups.len() {
            let valid = (0..p.slots.len())
                .filter(|&s| {
                    p.outputs.iter().filter(|o| o.group == group).all(|o| {
                        let slot = p.slots[s];
                        p.profile.allows(Input {
                            dpad: slot.input.dpad,
                            mask: slot.input.mask | p.held(slot, o.shifted),
                        })
                    })
                })
                .collect::<Vec<_>>();
            if valid.is_empty() {
                return Err(format!("no feasible chord for {}", p.groups[group].label()));
            }
            p.valid.push(valid);
        }
        let mut occupied = BTreeSet::new();
        for pin in &config.pinned_bindings {
            let action = actions
                .iter()
                .position(|a| a == &pin.action)
                .ok_or_else(|| format!("pinned action {} is not required", pin.action.label()))?;
            let output = p.outputs[action];
            let input = pin.chord.input()?;
            let held = button_mask(&pin.held)?;
            let slot = p
                .slots
                .iter()
                .position(|s| s.input == input && p.held(*s, output.shifted) == held)
                .ok_or_else(|| {
                    format!(
                        "pin for {} conflicts with modifier-only controls or strategy roles",
                        pin.action.label()
                    )
                })?;
            if p.valid[output.group].binary_search(&slot).is_err() {
                return Err(format!(
                    "pin for {} is forbidden by the profile",
                    pin.action.label()
                ));
            }
            if let Some(&previous) = p.pinned.get(&output.group) {
                if previous != slot {
                    return Err("Shift-related pins must share an ordinary chord and layer".into());
                }
            } else {
                if !occupied.insert(slot) {
                    return Err("two pinned groups occupy the same chord and layer".into());
                }
                p.pinned.insert(output.group, slot);
            }
        }
        Ok(p)
    }

    fn held(&self, slot: Slot, shifted: bool) -> u8 {
        (if shifted {
            self.strategy.shift.map_or(0, Button::bit)
        } else {
            0
        }) | (if slot.layer {
            self.strategy.layer.map_or(0, Button::bit)
        } else {
            0
        })
    }

    fn physical(&self, action: usize, assignment: &[usize]) -> (Input, u8) {
        let output = self.outputs[action];
        let slot = self.slots[assignment[output.group]];
        (slot.input, self.held(slot, output.shifted))
    }

    fn unary(&self, group: usize, slot: usize) -> Metrics {
        let slot = self.slots[slot];
        let mut m = Metrics::default();
        let n = self.stats.total as f64;
        for (action, o) in self
            .outputs
            .iter()
            .enumerate()
            .filter(|(_, o)| o.group == group)
        {
            let held = self.held(slot, o.shifted);
            let held_count = f64::from(held.count_ones());
            let effort = self.profile.costs.effort(slot.input, held);
            let presses = f64::from(slot.input.controls());
            let frequency = self.stats.counts[action] as f64 / n;
            m.effort += effort * frequency;
            m.activations += presses * frequency;
            m.coverage += (effort
                + presses
                + held_count
                + 2.0 * self.profile.costs.modifier_change * held_count)
                / self.actions.len() as f64;
            if action == self.stats.first {
                m.effort += self.profile.costs.modifier_change * held_count / n;
                m.activations += held_count / n;
            }
            if action == self.stats.last {
                m.effort += self.profile.costs.modifier_change * held_count / n;
            }
        }
        m
    }

    fn edge(&self, edge: usize, assignment: &[usize]) -> Metrics {
        let (a, b, count) = self.stats.pairs[edge];
        let (ia, ha) = self.physical(a, assignment);
        let (ib, hb) = self.physical(b, assignment);
        let frequency = count as f64 / self.stats.total as f64;
        Metrics {
            effort: f64::from((ha ^ hb).count_ones())
                * self.profile.costs.modifier_change
                * frequency,
            activations: f64::from((hb & !ha).count_ones()) * frequency,
            movement: self.movement.cost(ia, ib) * frequency,
            ..Metrics::default()
        }
    }

    pub fn metrics(&self, assignment: &[usize]) -> Metrics {
        let mut total = Metrics {
            learning: self.groups.len() as f64
                + 4.0 * f64::from(self.strategy.modifier_mask().count_ones()),
            ..Metrics::default()
        };
        for (group, &slot) in assignment.iter().enumerate() {
            add(&mut total, self.unary(group, slot));
        }
        for edge in 0..self.stats.pairs.len() {
            add(&mut total, self.edge(edge, assignment));
        }
        total
    }

    pub fn baseline(&self, weights: &Weights) -> Result<Vec<usize>> {
        let unit = Metrics {
            effort: 1.0,
            activations: 1.0,
            movement: 1.0,
            coverage: 1.0,
            learning: 1.0,
        };
        let mut orders = self.valid.clone();
        for (g, slots) in orders.iter_mut().enumerate() {
            let mut costs = vec![0.0; self.slots.len()];
            for &s in slots.iter() {
                costs[s] = self.unary(g, s).objective(unit, weights);
            }
            slots.sort_by(|&a, &b| costs[a].total_cmp(&costs[b]).then(a.cmp(&b)));
        }
        let mut order: Vec<_> = (0..self.groups.len()).collect();
        let frequency = |g| {
            self.outputs
                .iter()
                .enumerate()
                .filter(|(_, o)| o.group == g)
                .map(|(a, _)| self.stats.counts[a])
                .sum::<u64>()
        };
        order.sort_by_key(|&g| (std::cmp::Reverse(frequency(g)), g));
        let mut assignment = vec![usize::MAX; self.groups.len()];
        let mut used = vec![None; self.slots.len()];
        for (&g, &s) in &self.pinned {
            assignment[g] = s;
            used[s] = Some(g);
        }
        for g in order {
            if self.pinned.contains_key(&g) {
                continue;
            }
            if let Some(&s) = orders[g].iter().find(|&&s| used[s].is_none()) {
                assignment[g] = s;
                used[s] = Some(g);
            } else if !place(
                g,
                &orders,
                &self.pinned,
                &mut assignment,
                &mut used,
                &mut vec![false; self.slots.len()],
            ) {
                return Err(
                    "not enough distinct feasible chords for the required actions and pins".into(),
                );
            }
        }
        Ok(assignment)
    }

    pub fn layout(
        &self,
        assignment: &[usize],
        weights: &Weights,
        normalization: Metrics,
    ) -> Layout {
        let bindings = self
            .actions
            .iter()
            .enumerate()
            .map(|(a, value)| {
                let (input, held) = self.physical(a, assignment);
                Binding {
                    action: value.clone(),
                    chord: input.chord(),
                    held: Button::ALL
                        .into_iter()
                        .filter(|b| held & b.bit() != 0)
                        .collect(),
                }
            })
            .collect();
        Layout {
            schema_version: SCHEMA_VERSION,
            cost_model_version: COST_MODEL_VERSION,
            strategy: self.strategy.clone(),
            profile: self.profile.clone(),
            weights: weights.clone(),
            normalization,
            recognition: "release-all".into(),
            bindings,
        }
    }

    pub fn summary(
        &self,
        assignment: &[usize],
        weights: &Weights,
        normalization: Metrics,
    ) -> Score {
        let metrics = self.metrics(assignment);
        let mut max_controls = 0;
        let mut chords = BTreeSet::new();
        let mut layers = BTreeSet::new();
        for (g, &s) in assignment.iter().enumerate() {
            let slot = self.slots[s];
            chords.insert(slot.input);
            layers.insert(slot.layer);
            for o in self.outputs.iter().filter(|o| o.group == g) {
                max_controls = max_controls.max(
                    (slot.input.mask | self.held(slot, o.shifted)).count_ones()
                        + slot.input.dpad.controls(),
                );
            }
        }
        Score {
            objective: metrics.objective(normalization, weights),
            metrics,
            max_controls,
            distinct_chords: chords.len(),
            layers_used: layers.len(),
        }
    }
}

fn place(
    g: usize,
    orders: &[Vec<usize>],
    pinned: &BTreeMap<usize, usize>,
    assignment: &mut [usize],
    used: &mut [Option<usize>],
    seen: &mut [bool],
) -> bool {
    for &s in &orders[g] {
        if seen[s] {
            continue;
        }
        seen[s] = true;
        if let Some(other) = used[s]
            && (pinned.contains_key(&other)
                || !place(other, orders, pinned, assignment, used, seen))
        {
            continue;
        }
        assignment[g] = s;
        used[s] = Some(g);
        return true;
    }
    false
}

fn add(a: &mut Metrics, b: Metrics) {
    a.effort += b.effort;
    a.activations += b.activations;
    a.movement += b.movement;
    a.coverage += b.coverage;
    a.learning += b.learning;
}

struct Movement {
    dpad: [[f64; 9]; 9],
    face: [[f64; 16]; 16],
}

impl Movement {
    fn new() -> Self {
        let mut dpad = [[0.0; 9]; 9];
        for a in Dpad::ALL {
            for b in Dpad::ALL {
                let (ax, ay) = a.position();
                let (bx, by) = b.position();
                dpad[a as usize][b as usize] =
                    f64::from((ax - bx).pow(2) + (ay - by).pow(2)).sqrt();
            }
        }
        let centroid = |mask: u8| {
            let (mut x, mut y, mut count) = (0.0, 0.0, 0.0_f64);
            for (bit, px, py) in [(1, 0.0, 1.0), (2, 1.0, 0.0), (4, -1.0, 0.0), (8, 0.0, -1.0)] {
                if mask & bit != 0 {
                    x += px;
                    y += py;
                    count += 1.0;
                }
            }
            (x / count.max(1.0), y / count.max(1.0))
        };
        let mut face = [[0.0; 16]; 16];
        for a in 0..16 {
            for b in 0..16 {
                let (ax, ay) = centroid(a);
                let (bx, by) = centroid(b);
                face[a as usize][b as usize] = ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt();
            }
        }
        Self { dpad, face }
    }
    fn cost(&self, a: Input, b: Input) -> f64 {
        self.dpad[a.dpad as usize][b.dpad as usize]
            + self.face[(a.mask & 15) as usize][(b.mask & 15) as usize]
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Score {
    pub objective: f64,
    pub metrics: Metrics,
    pub max_controls: u32,
    pub distinct_chords: usize,
    pub layers_used: usize,
}

struct Evaluator<'a> {
    problem: &'a Problem,
    normalization: Metrics,
    weights: &'a Weights,
    unary: Vec<Vec<f64>>,
    preferred: Vec<Vec<usize>>,
    adjacent: Vec<Vec<usize>>,
}

impl<'a> Evaluator<'a> {
    fn new(p: &'a Problem, weights: &'a Weights, normalization: Metrics) -> Self {
        let unary: Vec<Vec<f64>> = p
            .valid
            .iter()
            .enumerate()
            .map(|(g, slots)| {
                let mut row = vec![f64::INFINITY; p.slots.len()];
                for &s in slots {
                    row[s] = p.unary(g, s).objective(normalization, weights);
                }
                row
            })
            .collect();
        let preferred = p
            .valid
            .iter()
            .enumerate()
            .map(|(g, slots)| {
                let mut slots = slots.clone();
                slots.sort_by(|&a, &b| unary[g][a].total_cmp(&unary[g][b]).then(a.cmp(&b)));
                slots.truncate((p.groups.len() * 2).max(16));
                slots
            })
            .collect();
        let mut adjacent = vec![vec![]; p.groups.len()];
        for (edge, &(a, b, _)) in p.stats.pairs.iter().enumerate() {
            let ga = p.outputs[a].group;
            let gb = p.outputs[b].group;
            adjacent[ga].push(edge);
            if ga != gb {
                adjacent[gb].push(edge);
            }
        }
        Self {
            problem: p,
            normalization,
            weights,
            unary,
            preferred,
            adjacent,
        }
    }
    fn affected(&self, assignment: &[usize], first: usize, second: Option<usize>) -> f64 {
        let mut score = self.unary[first][assignment[first]];
        for &edge in &self.adjacent[first] {
            score += self
                .problem
                .edge(edge, assignment)
                .objective(self.normalization, self.weights);
        }
        if let Some(second) = second {
            score += self.unary[second][assignment[second]];
            for &edge in &self.adjacent[second] {
                let (a, b, _) = self.problem.stats.pairs[edge];
                if self.problem.outputs[a].group != first && self.problem.outputs[b].group != first
                {
                    score += self
                        .problem
                        .edge(edge, assignment)
                        .objective(self.normalization, self.weights);
                }
            }
        }
        score
    }
}

pub fn optimize(
    p: &Problem,
    baseline: &[usize],
    weights: &Weights,
    normalization: Metrics,
    iterations: usize,
    seeds: &[u64],
) -> Vec<usize> {
    let movable: Vec<_> = (0..p.groups.len())
        .filter(|g| !p.pinned.contains_key(g))
        .collect();
    if movable.is_empty() {
        return baseline.to_vec();
    }
    let evaluator = Evaluator::new(p, weights, normalization);
    let mut best = baseline.to_vec();
    let mut best_score = p.metrics(&best).objective(normalization, weights);
    for &seed in seeds {
        let mut rng = fastrand::Rng::with_seed(seed);
        let mut assignment = baseline.to_vec();
        let mut used = vec![None; p.slots.len()];
        for (g, &s) in assignment.iter().enumerate() {
            used[s] = Some(g);
        }
        let mut score = p.metrics(&assignment).objective(normalization, weights);
        let initial_temperature = (score.abs() * 0.02).max(0.01);
        for step in 0..iterations {
            let first = movable[rng.usize(..movable.len())];
            let options = if rng.bool() {
                &p.valid[first]
            } else {
                &evaluator.preferred[first]
            };
            let next = options[rng.usize(..options.len())];
            let previous = assignment[first];
            if next == previous {
                continue;
            }
            let second = used[next];
            if second.is_some_and(|g| {
                p.pinned.contains_key(&g) || p.valid[g].binary_search(&previous).is_err()
            }) {
                continue;
            }
            let old = evaluator.affected(&assignment, first, second);
            assignment[first] = next;
            if let Some(g) = second {
                assignment[g] = previous;
            }
            let delta = evaluator.affected(&assignment, first, second) - old;
            let progress = step as f64 / iterations.max(1) as f64;
            let temperature = initial_temperature * 0.0001_f64.powf(progress);
            if delta <= 0.0 || rng.f64() < (-delta / temperature).exp() {
                score += delta;
                used[previous] = second;
                used[next] = Some(first);
                if score < best_score {
                    best = assignment.clone();
                    best_score = score;
                }
            } else {
                assignment[first] = previous;
                if let Some(g) = second {
                    assignment[g] = next;
                }
            }
        }
    }
    if p.metrics(&best).objective(normalization, weights)
        > p.metrics(baseline).objective(normalization, weights)
    {
        baseline.to_vec()
    } else {
        best
    }
}

pub fn load_layout(layout: &Layout, stats: Stats) -> Result<(Problem, Vec<usize>)> {
    if layout.schema_version != SCHEMA_VERSION || layout.cost_model_version != COST_MODEL_VERSION {
        return Err("unsupported layout schema or cost model version".into());
    }
    if layout.recognition != "release-all" {
        return Err("unsupported chord recognition policy".into());
    }
    let r = layout.normalization;
    if [r.effort, r.activations, r.movement, r.coverage, r.learning]
        .iter()
        .any(|v| !v.is_finite() || *v <= 0.0)
    {
        return Err("normalization values must be finite and positive".into());
    }
    let config = Config {
        profile: layout.profile.clone(),
        weights: layout.weights.clone(),
        strategies: vec![layout.strategy.clone()],
        pinned_bindings: layout.bindings.clone(),
    };
    config.validate()?;
    let actions: Vec<_> = layout.bindings.iter().map(|b| b.action.clone()).collect();
    if actions.is_empty() {
        return Err("layout must contain bindings".into());
    }
    let p = Problem::new(&config, &actions, stats, &layout.strategy)?;
    let assignment = p.baseline(&layout.weights)?;
    Ok((p, assignment))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus::Corpus;

    fn strategy(shift: Option<Button>, layer: Option<Button>) -> Strategy {
        Strategy {
            name: "test".into(),
            shift,
            layer,
        }
    }

    fn pin(c: char, button: Button, held: Vec<Button>) -> Binding {
        Binding {
            action: Action::Character(c),
            chord: Chord {
                dpad: Dpad::Neutral,
                buttons: vec![button],
            },
            held,
        }
    }

    fn problem(text: &str, strategy: Strategy, pins: Vec<Binding>) -> (Config, Problem) {
        let corpus = Corpus::new(text).unwrap();
        let actions = corpus.actions(false, "");
        let config = Config {
            strategies: vec![strategy.clone()],
            pinned_bindings: pins,
            ..Config::default()
        };
        config.validate().unwrap();
        let stats = Stats::new(&corpus, &actions).unwrap();
        let p = Problem::new(&config, &actions, stats, &strategy).unwrap();
        (config, p)
    }

    #[test]
    fn all_combinations_are_eligible_without_a_size_limit() {
        let (mut config, p) = problem("a", strategy(None, None), vec![]);
        config.profile.modifier_only_buttons.clear();
        let p = Problem::new(&config, &p.actions, p.stats.clone(), &p.strategy).unwrap();
        assert_eq!(p.slots.len(), 2303);
        assert_eq!(p.valid[0].len(), 2303);
        assert!(
            p.slots
                .iter()
                .any(|s| s.input.mask == 255 && s.input.dpad == Dpad::UpLeft)
        );
    }

    #[test]
    fn default_strategies_never_use_triggers_as_ordinary_chord_members() {
        let config = Config::default();
        let corpus = Corpus::new("Aa!").unwrap();
        let actions = corpus.actions(false, "");
        let trigger_mask = Button::L2.bit() | Button::R2.bit();
        for strategy in &config.strategies {
            let p = Problem::new(
                &config,
                &actions,
                Stats::new(&corpus, &actions).unwrap(),
                strategy,
            )
            .unwrap();
            assert!(p.slots.iter().all(|s| s.input.mask & trigger_mask == 0));
            let assignment = p.baseline(&config.weights).unwrap();
            let layout = p.layout(
                &assignment,
                &config.weights,
                p.metrics(&assignment).normalization(),
            );
            load_layout(&layout, p.stats.clone()).unwrap();
            if strategy.modifier_mask() == 0 {
                assert_eq!(p.slots.len(), 575);
            }
        }
    }

    #[test]
    fn ordinary_trigger_pins_require_explicit_opt_in() {
        let (mut config, p) = problem("a", strategy(None, None), vec![]);
        config.pinned_bindings = vec![pin('a', Button::L2, vec![])];
        assert!(Problem::new(&config, &p.actions, p.stats.clone(), &p.strategy).is_err());
        config.profile.modifier_only_buttons.clear();
        let p = Problem::new(&config, &p.actions, p.stats.clone(), &p.strategy).unwrap();
        let assignment = p.baseline(&config.weights).unwrap();
        let layout = p.layout(
            &assignment,
            &config.weights,
            p.metrics(&assignment).normalization(),
        );
        assert_eq!(
            layout
                .bindings
                .iter()
                .find(|b| b.action == Action::Character('a'))
                .unwrap()
                .chord
                .buttons,
            [Button::L2]
        );
    }

    #[test]
    fn held_layer_is_entered_once_for_a_run_and_cleaned_up_at_end() {
        let (config, p) = problem(
            "aaa",
            strategy(None, Some(Button::R2)),
            vec![pin('a', Button::A, vec![Button::R2])],
        );
        let assignment = p.baseline(&config.weights).unwrap();
        let metrics = p.metrics(&assignment);
        assert!((metrics.activations - 4.0 / 3.0).abs() < 1e-12);
        let stroke_effort = p.profile.costs.effort(
            Input {
                dpad: Dpad::Neutral,
                mask: Button::A.bit(),
            },
            Button::R2.bit(),
        );
        let expected = stroke_effort + 2.0 * p.profile.costs.modifier_change / 3.0;
        assert!((metrics.effort - expected).abs() < 1e-12);
    }

    #[test]
    fn leaving_and_reentering_a_layer_costs_another_activation() {
        let (config, p) = problem(
            "aba",
            strategy(None, Some(Button::R2)),
            vec![
                pin('a', Button::A, vec![Button::R2]),
                pin('b', Button::B, vec![]),
            ],
        );
        let assignment = p.baseline(&config.weights).unwrap();
        assert!((p.metrics(&assignment).activations - 5.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn shift_pairs_share_a_chord_and_shift_can_persist() {
        let (config, p) = problem(
            "AAAa",
            strategy(Some(Button::L2), None),
            vec![
                pin('a', Button::A, vec![]),
                pin('A', Button::A, vec![Button::L2]),
            ],
        );
        let assignment = p.baseline(&config.weights).unwrap();
        assert!((p.metrics(&assignment).activations - 5.0 / 4.0).abs() < 1e-12);
        let layout = p.layout(
            &assignment,
            &config.weights,
            p.metrics(&assignment).normalization(),
        );
        let a = layout
            .bindings
            .iter()
            .find(|b| b.action == Action::Character('a'))
            .unwrap();
        let upper = layout
            .bindings
            .iter()
            .find(|b| b.action == Action::Character('A'))
            .unwrap();
        assert_eq!(a.chord, upper.chord);
        assert_eq!(upper.held, [Button::L2]);
    }

    #[test]
    fn forbidden_combinations_include_held_modifiers_and_supersets() {
        let (mut config, p) = problem(
            "a",
            strategy(None, Some(Button::R2)),
            vec![pin('a', Button::A, vec![Button::R2])],
        );
        config.profile.forbidden_combinations = vec![vec![Button::A, Button::R2]];
        assert!(Problem::new(&config, &p.actions, p.stats.clone(), &p.strategy).is_err());
        assert!(!config.profile.allows(Input {
            dpad: Dpad::Up,
            mask: Button::A.bit() | Button::R2.bit() | Button::L1.bit()
        }));
    }

    #[test]
    fn a_size_limit_can_make_the_required_alphabet_infeasible() {
        let corpus = Corpus::new("a").unwrap();
        let actions = corpus.actions(true, "");
        let mut config = Config::default();
        config.profile.max_chord_size = Some(1);
        let p = Problem::new(
            &config,
            &actions,
            Stats::new(&corpus, &actions).unwrap(),
            &config.strategies[0],
        )
        .unwrap();
        assert!(p.baseline(&config.weights).is_err());
    }

    #[test]
    fn incremental_scores_match_full_sequence_scores_for_moves_and_swaps() {
        let (config, p) = problem(
            "AAAb abba! aab? BAba.",
            strategy(Some(Button::L2), Some(Button::R2)),
            vec![],
        );
        let mut assignment = p.baseline(&config.weights).unwrap();
        let reference = p.metrics(&assignment).normalization();
        let evaluator = Evaluator::new(&p, &config.weights, reference);
        let mut rng = fastrand::Rng::with_seed(12);
        for _ in 0..200 {
            let first = rng.usize(..assignment.len());
            let next = p.valid[first][rng.usize(..p.valid[first].len())];
            let previous = assignment[first];
            if next == previous {
                continue;
            }
            let second = assignment.iter().position(|&s| s == next);
            if second.is_some_and(|g| p.valid[g].binary_search(&previous).is_err()) {
                continue;
            }
            let old_full = p.metrics(&assignment).objective(reference, &config.weights);
            let old_part = evaluator.affected(&assignment, first, second);
            assignment[first] = next;
            if let Some(g) = second {
                assignment[g] = previous;
            }
            let full_delta =
                p.metrics(&assignment).objective(reference, &config.weights) - old_full;
            let part_delta = evaluator.affected(&assignment, first, second) - old_part;
            assert!((full_delta - part_delta).abs() < 1e-9);
        }
    }

    #[test]
    fn search_is_reproducible_preserves_pins_and_keeps_the_baseline() {
        let (config, p) = problem(
            "aababa abba AAB!",
            strategy(Some(Button::L2), Some(Button::R2)),
            vec![pin(' ', Button::A, vec![])],
        );
        let baseline = p.baseline(&config.weights).unwrap();
        let reference = p.metrics(&baseline).normalization();
        let a = optimize(&p, &baseline, &config.weights, reference, 1000, &[2, 3]);
        let b = optimize(&p, &baseline, &config.weights, reference, 1000, &[2, 3]);
        assert_eq!(a, b);
        assert!(
            p.metrics(&a).objective(reference, &config.weights)
                <= p.metrics(&baseline).objective(reference, &config.weights)
        );
        for (&g, &slot) in &p.pinned {
            assert_eq!(a[g], slot);
        }
        assert_eq!(a.iter().collect::<BTreeSet<_>>().len(), a.len());
        for (g, &slot) in a.iter().enumerate() {
            assert!(p.valid[g].contains(&slot));
        }
        let layout = p.layout(&a, &config.weights, reference);
        let (loaded, loaded_assignment) = load_layout(&layout, p.stats.clone()).unwrap();
        assert!(
            (loaded
                .metrics(&loaded_assignment)
                .objective(reference, &config.weights)
                - p.metrics(&a).objective(reference, &config.weights))
            .abs()
                < 1e-10
        );
    }

    #[test]
    fn unknown_versions_duplicate_bindings_and_empty_chords_are_rejected() {
        let (config, p) = problem("ab", strategy(None, None), vec![]);
        let assignment = p.baseline(&config.weights).unwrap();
        let layout = p.layout(
            &assignment,
            &config.weights,
            p.metrics(&assignment).normalization(),
        );
        let mut invalid = layout.clone();
        invalid.schema_version += 1;
        assert!(load_layout(&invalid, p.stats.clone()).is_err());
        let mut invalid = layout.clone();
        invalid.bindings.push(invalid.bindings[0].clone());
        assert!(load_layout(&invalid, p.stats.clone()).is_err());
        let mut invalid = layout;
        invalid.bindings[0].chord = Chord::default();
        assert!(load_layout(&invalid, p.stats.clone()).is_err());
    }
}
