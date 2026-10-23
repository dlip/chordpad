use crate::{Result, corpus::Stats, model::*};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    pub input: Input,
    pub layer: bool,
    pub context: Input,
    pub menu: Option<MenuButton>,
}

fn menu_button(action: &Action) -> Option<MenuButton> {
    match action {
        Action::Character('\n') => Some(MenuButton::Start),
        Action::Key(Key::Escape) => Some(MenuButton::Select),
        _ => None,
    }
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
                    if input.controls() != 0
                        && mask & ordinary_exclusions == 0
                        && (!strategy.independent || input.typing_side().is_some())
                    {
                        slots.push(Slot {
                            input,
                            layer,
                            context: Input::default(),
                            menu: None,
                        });
                    }
                }
            }
        }
        if strategy.independent {
            for button in MenuButton::ALL {
                slots.push(Slot {
                    input: Input::default(),
                    layer: false,
                    context: Input::default(),
                    menu: Some(button),
                });
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
            let pinned = config
                .pinned_bindings
                .iter()
                .any(|b| b.action.group(strategy.shift.is_some()).0 == p.groups[group]);
            let valid = (0..p.slots.len())
                .filter(|&s| {
                    p.outputs
                        .iter()
                        .enumerate()
                        .filter(|(_, o)| o.group == group)
                        .all(|(action, o)| {
                            let slot = p.slots[s];
                            let menu = menu_button(&p.actions[action]);
                            let dedicated = strategy.independent && menu.is_some() && !pinned;
                            if dedicated || slot.menu.is_some() {
                                return dedicated && slot.menu == menu;
                            }
                            let context = p.context(slot, o.shifted);
                            if strategy.independent {
                                validate_independent_input(&p.profile, slot.input, context).is_ok()
                                    && independent_binding_allowed(
                                        &p.profile,
                                        &p.actions[action],
                                        slot.input,
                                        context,
                                    )
                            } else {
                                p.profile.allows(slot.input.with_context(context))
                            }
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
                .position(|s| {
                    s.input == input
                        && p.held(*s, output.shifted) == held
                        && s.context.dpad == pin.held_dpad
                })
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

    fn context(&self, slot: Slot, shifted: bool) -> Input {
        Input {
            mask: self.held(slot, shifted),
            dpad: slot.context.dpad,
        }
    }

    fn physical(&self, action: usize, assignment: &[usize]) -> (Input, Input) {
        let output = self.outputs[action];
        let slot = self.slots[assignment[output.group]];
        (slot.input, self.context(slot, output.shifted))
    }

    fn hands(&self, action: usize, assignment: &[usize]) -> u8 {
        let slot = self.slots[assignment[self.outputs[action].group]];
        match slot.menu {
            Some(MenuButton::Start) => 2,
            Some(MenuButton::Select) => 1,
            None => {
                let (input, held) = self.physical(action, assignment);
                input.with_context(held).hands()
            }
        }
    }

    fn fingers(&self, action: usize, assignment: &[usize]) -> u8 {
        let slot = self.slots[assignment[self.outputs[action].group]];
        match slot.menu {
            Some(MenuButton::Start) => Finger::RightThumb.bit(),
            Some(MenuButton::Select) => Finger::LeftThumb.bit(),
            None => {
                let (input, held) = self.physical(action, assignment);
                input.fingers(held)
            }
        }
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
            let held = self.context(slot, o.shifted);
            let held_count = f64::from(held.controls());
            let middle_count =
                f64::from(slot.input.with_context(held).middle_fingers().count_ones());
            let effort = if slot.menu.is_some() {
                1.0
            } else {
                self.profile.costs.effort_with_context(slot.input, held)
            };
            let presses = f64::from(slot.input.controls() + u32::from(slot.menu.is_some()));
            let frequency = self.stats.counts[action] as f64 / n;
            m.effort += effort * frequency;
            m.activations += presses * frequency;
            m.coverage += (effort
                + presses
                + held_count
                + 2.0 * self.profile.costs.modifier_change * held_count
                + 2.0 * self.profile.costs.grip_change * middle_count)
                / self.actions.len() as f64;
            if action == self.stats.first {
                m.effort += self.profile.costs.modifier_change * held_count / n;
                m.effort += self.profile.costs.grip_change * middle_count / n;
                m.activations += held_count / n;
            }
            if action == self.stats.last {
                m.effort += self.profile.costs.modifier_change * held_count / n;
                m.effort += self.profile.costs.grip_change * middle_count / n;
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
            effort: f64::from(
                (ha.mask ^ hb.mask).count_ones() + (ha.dpad.bits() ^ hb.dpad.bits()).count_ones(),
            ) * self.profile.costs.modifier_change
                * frequency
                + self.profile.costs.same_hand_transition
                    * f64::from(self.hands(a, assignment) & self.hands(b, assignment) != 0)
                    * frequency
                + self.profile.costs.same_finger_transition
                    * f64::from(
                        (self.fingers(a, assignment) & self.fingers(b, assignment)).count_ones(),
                    )
                    * frequency
                + self.profile.costs.grip_change
                    * f64::from(
                        (ia.with_context(ha).middle_fingers()
                            ^ ib.with_context(hb).middle_fingers())
                        .count_ones(),
                    )
                    * frequency,
            activations: f64::from(
                (hb.mask & !ha.mask).count_ones() + (hb.dpad.bits() & !ha.dpad.bits()).count_ones(),
            ) * frequency,
            movement: self.movement.cost(ia.with_context(ha), ib.with_context(hb)) * frequency,
            ..Metrics::default()
        }
    }

    fn skipgram(&self, edge: usize, assignment: &[usize]) -> Metrics {
        let (window, count) = &self.stats.skipgrams[edge];
        let weight = self.profile.costs.skipgram_weights[window.len() - 3];
        if weight == 0.0 {
            return Metrics::default();
        }
        let physical = |action| {
            let (input, held) = self.physical(action, assignment);
            input.with_context(held)
        };
        let first = physical(window[0]);
        let last = physical(*window.last().unwrap());
        let middle = &window[1..window.len() - 1];
        let mut movement = 0.0;
        if first.dpad != Dpad::Neutral
            && last.dpad != Dpad::Neutral
            && middle.iter().all(|&a| physical(a).dpad == Dpad::Neutral)
        {
            movement += self.movement.dpad[first.dpad as usize][last.dpad as usize];
        }
        if first.mask & 15 != 0
            && last.mask & 15 != 0
            && middle.iter().all(|&a| physical(a).mask & 15 == 0)
        {
            movement += self.movement.face[(first.mask & 15) as usize][(last.mask & 15) as usize];
        }
        Metrics {
            movement: movement * weight * *count as f64 / self.stats.total as f64,
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
        if self.profile.costs.skipgram_weights != [0.0; 3] {
            for edge in 0..self.stats.skipgrams.len() {
                add(&mut total, self.skipgram(edge, assignment));
            }
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
            .filter(|(a, _)| {
                self.slots[assignment[self.outputs[*a].group]]
                    .menu
                    .is_none()
            })
            .map(|(a, value)| {
                let (input, held) = self.physical(a, assignment);
                Binding {
                    held_dpad: held.dpad,
                    action: value.clone(),
                    chord: input.chord(),
                    held: Button::ALL
                        .into_iter()
                        .filter(|b| held.mask & b.bit() != 0)
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
            recognition: self.strategy.recognition().into(),
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
            if slot.menu.is_some() {
                max_controls = max_controls.max(1);
                continue;
            }
            chords.insert(slot.input);
            layers.insert(slot.layer);
            for o in self.outputs.iter().filter(|o| o.group == g) {
                max_controls = max_controls.max(
                    slot.input
                        .with_context(self.context(slot, o.shifted))
                        .controls(),
                );
            }
        }
        Score {
            objective: metrics.objective(normalization, weights),
            metrics,
            max_controls,
            distinct_chords: chords.len(),
            layers_used: layers.len(),
            same_hand_transition_rate: self
                .stats
                .pairs
                .iter()
                .map(|&(a, b, n)| {
                    if self.hands(a, assignment) & self.hands(b, assignment) != 0 {
                        n
                    } else {
                        0
                    }
                })
                .sum::<u64>() as f64
                / self.stats.total.saturating_sub(1).max(1) as f64,
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
    pub same_hand_transition_rate: f64,
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
    adjacent_skipgrams: Vec<Vec<usize>>,
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
        let mut adjacent_skipgrams = vec![vec![]; p.groups.len()];
        for (edge, (window, _)) in p.stats.skipgrams.iter().enumerate() {
            if p.profile.costs.skipgram_weights[window.len() - 3] == 0.0 {
                continue;
            }
            let groups: BTreeSet<_> = window.iter().map(|&a| p.outputs[a].group).collect();
            for group in groups {
                adjacent_skipgrams[group].push(edge);
            }
        }
        Self {
            problem: p,
            normalization,
            weights,
            unary,
            preferred,
            adjacent,
            adjacent_skipgrams,
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
        for &edge in &self.adjacent_skipgrams[first] {
            score += self
                .problem
                .skipgram(edge, assignment)
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
            for &edge in &self.adjacent_skipgrams[second] {
                let (window, _) = &self.problem.stats.skipgrams[edge];
                if window
                    .iter()
                    .all(|&a| self.problem.outputs[a].group != first)
                {
                    score += self
                        .problem
                        .skipgram(edge, assignment)
                        .objective(self.normalization, self.weights);
                }
            }
        }
        score
    }

    fn improve(&self, assignment: &mut [usize], movable: &[usize]) {
        let p = self.problem;
        let mut used = vec![None; p.slots.len()];
        for (group, &slot) in assignment.iter().enumerate() {
            used[slot] = Some(group);
        }
        let mut score = p
            .metrics(assignment)
            .objective(self.normalization, self.weights);
        loop {
            let epsilon = 1e-12 * score.abs().max(1.0);
            let mut best_delta = -epsilon;
            let mut best_change = None;
            for &first in movable {
                let previous = assignment[first];
                for &next in &p.valid[first] {
                    if next == previous {
                        continue;
                    }
                    let second = used[next];
                    if second.is_some_and(|g| {
                        g < first
                            || p.pinned.contains_key(&g)
                            || p.valid[g].binary_search(&previous).is_err()
                    }) {
                        continue;
                    }
                    let old = self.affected(assignment, first, second);
                    assignment[first] = next;
                    if let Some(group) = second {
                        assignment[group] = previous;
                    }
                    let delta = self.affected(assignment, first, second) - old;
                    assignment[first] = previous;
                    if let Some(group) = second {
                        assignment[group] = next;
                    }
                    if delta < best_delta {
                        best_delta = delta;
                        best_change = Some((first, next, second));
                    }
                }
            }
            let Some((first, next, second)) = best_change else {
                break;
            };
            let previous = assignment[first];
            assignment[first] = next;
            if let Some(group) = second {
                assignment[group] = previous;
            }
            let next_score = p
                .metrics(assignment)
                .objective(self.normalization, self.weights);
            if next_score >= score - epsilon {
                assignment[first] = previous;
                if let Some(group) = second {
                    assignment[group] = next;
                }
                break;
            }
            score = next_score;
            used[previous] = second;
            used[next] = Some(first);
        }
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
        let mut run_best = assignment.clone();
        let mut run_best_score = score;
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
                if score < run_best_score {
                    run_best = assignment.clone();
                    run_best_score = score;
                }
            } else {
                assignment[first] = previous;
                if let Some(g) = second {
                    assignment[g] = next;
                }
            }
        }
        evaluator.improve(&mut run_best, &movable);
        let score = p.metrics(&run_best).objective(normalization, weights);
        if score < best_score {
            best = run_best;
            best_score = score;
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
    layout.validate()?;
    let config = Config {
        profile: layout.profile.clone(),
        weights: layout.weights.clone(),
        strategies: vec![layout.strategy.clone()],
        pinned_bindings: layout.bindings.clone(),
    };
    config.validate()?;
    let actions = stats.actions.clone();
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
            independent: false,
            name: "test".into(),
            shift,
            layer,
        }
    }

    fn pin(c: char, button: Button, held: Vec<Button>) -> Binding {
        Binding {
            held_dpad: Dpad::Neutral,
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
        for independent in [false, true] {
            let mut mode = strategy(Some(Button::L2), Some(Button::R2));
            if independent {
                mode.independent = true;
                mode.shift = Some(Button::L2);
                mode.layer = Some(Button::R2);
            }
            let (mut config, p) = problem("AAAb abba! aab? BAba.", mode, vec![]);
            config.profile.costs.same_hand_transition = 1.0;
            config.profile.costs.same_finger_transition = 1.0;
            config.profile.costs.grip_change = 0.7;
            config.profile.costs.skipgram_weights = [0.5, 0.25, 0.125];
            config.profile.costs.awkward_diagonal_extra = 0.6;
            let p = Problem::new(&config, &p.actions, p.stats.clone(), &p.strategy).unwrap();
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
    }

    #[test]
    fn skipgram_movement_tracks_the_next_thumb_use_with_decaying_weights() {
        for (text, expected) in [
            ("abc", 1.0 / 3.0),
            ("abbc", 0.5 / 4.0),
            ("abbbc", 0.25 / 5.0),
            ("abbbbc", 0.0),
            ("ababc", 1.0 / 5.0),
        ] {
            for mirrored in [false, true] {
                let input = |c| match (c, mirrored) {
                    ('a', false) | ('b', true) => Input {
                        dpad: Dpad::Up,
                        mask: 0,
                    },
                    ('c', false) => Input {
                        dpad: Dpad::Down,
                        mask: 0,
                    },
                    ('c', true) => Input {
                        dpad: Dpad::Neutral,
                        mask: Button::Y.bit(),
                    },
                    _ => Input {
                        dpad: Dpad::Neutral,
                        mask: Button::A.bit(),
                    },
                };
                let pins = ['a', 'b', 'c']
                    .map(|c| Binding {
                        action: Action::Character(c),
                        chord: input(c).chord(),
                        held: vec![],
                        held_dpad: Dpad::Neutral,
                    })
                    .into();
                let (config, mut p) = problem(text, strategy(None, None), pins);
                let assignment = p.baseline(&config.weights).unwrap();
                let adjacent_only = p.metrics(&assignment);
                p.profile.costs.skipgram_weights = [0.5, 0.25, 0.125];
                let with_skipgrams = p.metrics(&assignment);
                assert!(
                    (with_skipgrams.movement - adjacent_only.movement - expected).abs() < 1e-12,
                    "{text}, mirrored={mirrored}"
                );
                assert_eq!(with_skipgrams.effort, adjacent_only.effort);
                assert_eq!(with_skipgrams.activations, adjacent_only.activations);
            }
        }
    }

    #[test]
    fn skipgram_cache_tracks_middle_character_moves_and_shift_layers() {
        let (config, mut p) = problem(
            "aBcaBc",
            strategy(Some(Button::L2), Some(Button::R2)),
            vec![],
        );
        p.profile.costs.skipgram_weights = [0.5, 0.25, 0.125];
        let mut assignment = p.baseline(&config.weights).unwrap();
        let group = |c| {
            p.groups
                .iter()
                .position(|a| *a == Action::Character(c))
                .unwrap()
        };
        let slot = |input, layer| {
            p.slots
                .iter()
                .position(|s| s.input == input && s.layer == layer)
                .unwrap()
        };
        assignment[group('a')] = slot(
            Input {
                dpad: Dpad::Up,
                mask: 0,
            },
            false,
        );
        assignment[group('b')] = slot(
            Input {
                dpad: Dpad::Neutral,
                mask: Button::A.bit(),
            },
            true,
        );
        assignment[group('c')] = slot(
            Input {
                dpad: Dpad::Down,
                mask: 0,
            },
            false,
        );
        let reference = p.metrics(&assignment).normalization();
        let evaluator = Evaluator::new(&p, &config.weights, reference);
        let first = group('b');
        let old_full = p.metrics(&assignment).objective(reference, &config.weights);
        let old_part = evaluator.affected(&assignment, first, None);
        assert!(
            p.stats
                .skipgrams
                .iter()
                .enumerate()
                .any(|(i, _)| p.skipgram(i, &assignment).movement > 0.0)
        );
        assignment[first] = slot(
            Input {
                dpad: Dpad::Left,
                mask: 0,
            },
            true,
        );
        assert!(
            p.stats
                .skipgrams
                .iter()
                .enumerate()
                .all(|(i, _)| p.skipgram(i, &assignment).movement == 0.0)
        );
        let full_delta = p.metrics(&assignment).objective(reference, &config.weights) - old_full;
        let part_delta = evaluator.affected(&assignment, first, None) - old_part;
        assert!((full_delta - part_delta).abs() < 1e-9);
    }

    #[test]
    fn finger_reuse_distinguishes_thumb_index_and_held_modifiers() {
        let input = |mask, dpad| Input { mask, dpad };
        for (a, b, shifted, reused) in [
            (
                input(Button::A.bit(), Dpad::Neutral),
                input(Button::R1.bit(), Dpad::Neutral),
                false,
                0,
            ),
            (
                input(Button::A.bit(), Dpad::Neutral),
                input(Button::B.bit(), Dpad::Neutral),
                false,
                1,
            ),
            (
                input(Button::R1.bit(), Dpad::Neutral),
                input(Button::R2.bit(), Dpad::Neutral),
                false,
                1,
            ),
            (
                input(Button::R1.bit() | Button::A.bit(), Dpad::Neutral),
                input(Button::R1.bit() | Button::B.bit(), Dpad::Neutral),
                false,
                2,
            ),
            (
                input(0, Dpad::Up),
                input(Button::L1.bit(), Dpad::Neutral),
                false,
                0,
            ),
            (
                input(0, Dpad::Up),
                input(Button::A.bit(), Dpad::Neutral),
                true,
                0,
            ),
            (
                input(Button::L1.bit(), Dpad::Up),
                input(Button::A.bit(), Dpad::Neutral),
                true,
                0,
            ),
        ] {
            let corpus = Corpus::new(if shifted { "AB" } else { "ab" }).unwrap();
            let actions = corpus.actions(false, "");
            let mode = strategy(shifted.then_some(Button::L2), None);
            let mut config = Config::default();
            config.profile.modifier_only_buttons.clear();
            config.profile.costs.same_finger_transition = 1.0;
            config.pinned_bindings = corpus
                .characters
                .iter()
                .zip([a, b])
                .map(|(&c, input)| Binding {
                    action: Action::Character(c),
                    chord: input.chord(),
                    held: if shifted { vec![Button::L2] } else { vec![] },
                    held_dpad: Dpad::Neutral,
                })
                .collect();
            let p = Problem::new(
                &config,
                &actions,
                Stats::new(&corpus, &actions).unwrap(),
                &mode,
            )
            .unwrap();
            let assignment = p.baseline(&config.weights).unwrap();
            let edge = p.stats.pairs.iter().position(|&(_, _, n)| n == 1).unwrap();
            assert!(
                (p.edge(edge, &assignment).effort - f64::from(reused) / 2.0).abs() < 1e-12,
                "{a:?} → {b:?}, shifted={shifted}"
            );
        }
    }

    #[test]
    fn middle_finger_grip_changes_include_entry_exit_and_persistent_runs() {
        for (text, both_sides, changes) in [
            ("aaaa", false, 2),
            ("abba", false, 4),
            ("ab", false, 2),
            ("ba", false, 2),
            ("aaaa", true, 4),
        ] {
            let corpus = Corpus::new(text).unwrap();
            let actions = corpus.actions(false, "");
            let mut config = Config::default();
            config.profile.modifier_only_buttons.clear();
            config.pinned_bindings = actions
                .iter()
                .filter(|a| matches!(a, Action::Character(_)))
                .map(|action| Binding {
                    action: action.clone(),
                    chord: Input {
                        dpad: Dpad::Neutral,
                        mask: if *action == Action::Character('a') {
                            Button::L1.bit()
                                | Button::L2.bit()
                                | if both_sides {
                                    Button::R1.bit() | Button::R2.bit()
                                } else {
                                    0
                                }
                        } else {
                            Button::A.bit()
                        },
                    }
                    .chord(),
                    held: vec![],
                    held_dpad: Dpad::Neutral,
                })
                .collect();
            let mut p = Problem::new(
                &config,
                &actions,
                Stats::new(&corpus, &actions).unwrap(),
                &strategy(None, None),
            )
            .unwrap();
            let assignment = p.baseline(&config.weights).unwrap();
            let before = p.metrics(&assignment);
            p.profile.costs.grip_change = 0.7;
            let after = p.metrics(&assignment);
            assert!(
                (after.effort - before.effort - 0.7 * f64::from(changes) / text.len() as f64).abs()
                    < 1e-12,
                "{text}, both_sides={both_sides}"
            );
            assert_eq!(after.activations, before.activations);
        }
    }

    #[test]
    fn held_shift_recruits_middle_only_for_strokes_using_the_same_side_bumper() {
        let (config, mut p) = problem(
            "AABA",
            strategy(Some(Button::L2), None),
            vec![
                Binding {
                    action: Action::Character('A'),
                    chord: Chord {
                        dpad: Dpad::Up,
                        buttons: vec![Button::L1],
                    },
                    held: vec![Button::L2],
                    held_dpad: Dpad::Neutral,
                },
                pin('B', Button::A, vec![Button::L2]),
            ],
        );
        let assignment = p.baseline(&config.weights).unwrap();
        let before = p.metrics(&assignment);
        p.profile.costs.grip_change = 0.7;
        let after = p.metrics(&assignment);
        assert!((after.effort - before.effort - 0.7).abs() < 1e-12);
        assert_eq!(after.activations, before.activations);
    }

    #[test]
    fn finishing_pass_chooses_the_best_legal_replacement_or_swap_without_annealing() {
        for spare in [false, true] {
            let corpus = Corpus::new("aaaaab").unwrap();
            let actions = corpus.actions(false, "");
            let mut config = Config::default();
            config.profile.diagonals = false;
            config.profile.max_chord_size = Some(1);
            config.profile.disabled_buttons = if spare {
                vec![Button::Y]
            } else {
                vec![Button::X, Button::Y]
            };
            config.profile.costs.buttons.insert(Button::A, 10.0);
            config.weights = Weights {
                effort: 1.0,
                activations: 0.0,
                movement: 0.0,
                coverage: 0.0,
                learning: 0.0,
            };
            for (key, button) in [(Key::Backspace, Button::L1), (Key::Escape, Button::R1)] {
                config.pinned_bindings.push(Binding {
                    action: Action::Key(key),
                    chord: Input {
                        dpad: Dpad::Neutral,
                        mask: button.bit(),
                    }
                    .chord(),
                    held: vec![],
                    held_dpad: Dpad::Neutral,
                });
            }
            // Restrict this problem to shoulders and face buttons so all remaining
            // choices are occupied in the swap-only case.
            config.profile.forbidden_chords.extend(
                Dpad::ALL
                    .into_iter()
                    .filter(|d| *d != Dpad::Neutral)
                    .map(|dpad| Chord {
                        dpad,
                        buttons: vec![],
                    }),
            );
            let p = Problem::new(
                &config,
                &actions,
                Stats::new(&corpus, &actions).unwrap(),
                &strategy(None, None),
            )
            .unwrap();
            let mut baseline = p.baseline(&config.weights).unwrap();
            let group = |c| {
                p.groups
                    .iter()
                    .position(|a| *a == Action::Character(c))
                    .unwrap()
            };
            let slot = |button: Button| {
                p.slots
                    .iter()
                    .position(|s| {
                        s.input
                            == Input {
                                dpad: Dpad::Neutral,
                                mask: button.bit(),
                            }
                    })
                    .unwrap()
            };
            baseline[group('a')] = slot(Button::A);
            baseline[group('b')] = slot(Button::B);
            let reference = p.metrics(&baseline).normalization();
            let improved = optimize(&p, &baseline, &config.weights, reference, 0, &[42]);
            assert!(
                p.metrics(&improved).objective(reference, &config.weights)
                    < p.metrics(&baseline).objective(reference, &config.weights)
            );
            assert_eq!(
                improved[group('a')],
                slot(if spare { Button::X } else { Button::B })
            );
            assert_eq!(
                improved[group('b')],
                slot(if spare { Button::B } else { Button::A })
            );
            for (&group, &slot) in &p.pinned {
                assert_eq!(improved[group], slot);
            }
            assert_eq!(
                optimize(&p, &improved, &config.weights, reference, 0, &[42]),
                improved
            );
        }
    }

    #[test]
    fn finished_layouts_have_no_legal_improving_move_or_swap_with_all_costs_enabled() {
        for independent in [false, true] {
            let mut mode = strategy(Some(Button::L2), Some(Button::R2));
            mode.independent = independent;
            let (mut config, p) =
                problem("aA bbb!?ab \n", mode, vec![pin(' ', Button::R1, vec![])]);
            config.profile.exclude_opposite_face_pairs = true;
            config.profile.costs.same_finger_transition = 1.0;
            config.profile.costs.grip_change = 0.7;
            config.profile.costs.skipgram_weights = [0.5, 0.25, 0.125];
            config.profile.costs.awkward_diagonal_extra = 0.6;
            let p = Problem::new(&config, &p.actions, p.stats.clone(), &p.strategy).unwrap();
            let baseline = p.baseline(&config.weights).unwrap();
            let reference = p.metrics(&baseline).normalization();
            let result = optimize(&p, &baseline, &config.weights, reference, 5, &[42, 43]);
            let score = p.metrics(&result).objective(reference, &config.weights);
            assert_eq!(result.iter().collect::<BTreeSet<_>>().len(), result.len());
            for (first, &previous) in result.iter().enumerate() {
                assert!(p.valid[first].contains(&previous));
                if let Some(&pin) = p.pinned.get(&first) {
                    assert_eq!(previous, pin);
                    continue;
                }
                for &next in &p.valid[first] {
                    if next == previous {
                        continue;
                    }
                    let second = result.iter().position(|&slot| slot == next);
                    if second.is_some_and(|g| {
                        p.pinned.contains_key(&g) || !p.valid[g].contains(&previous)
                    }) {
                        continue;
                    }
                    let mut neighbor = result.clone();
                    neighbor[first] = next;
                    if let Some(group) = second {
                        neighbor[group] = previous;
                    }
                    assert!(
                        p.metrics(&neighbor).objective(reference, &config.weights) >= score - 1e-10,
                        "independent={independent}, first={first}, next={next}"
                    );
                }
            }
            assert_eq!(
                optimize(&p, &result, &config.weights, reference, 0, &[42]),
                result
            );
        }
    }

    #[test]
    fn search_is_reproducible_preserves_pins_and_keeps_the_baseline() {
        let (mut config, p) = problem(
            "aababa abba AAB!",
            strategy(Some(Button::L2), Some(Button::R2)),
            vec![pin(' ', Button::A, vec![])],
        );
        config.profile.costs.same_finger_transition = 1.0;
        config.profile.costs.grip_change = 0.7;
        config.profile.costs.skipgram_weights = [0.5, 0.25, 0.125];
        config.profile.costs.awkward_diagonal_extra = 0.6;
        let p = Problem::new(&config, &p.actions, p.stats.clone(), &p.strategy).unwrap();
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
    fn independent_penalty_counts_hands_occupied_by_global_modifiers() {
        let mut mode = strategy(Some(Button::L2), Some(Button::R2));
        mode.independent = true;
        let left = Binding {
            action: Action::Character('a'),
            chord: Chord {
                dpad: Dpad::Up,
                buttons: vec![],
            },
            held: vec![],
            held_dpad: Dpad::Neutral,
        };
        let (mut config, p) = problem(
            "abab",
            mode.clone(),
            vec![left.clone(), pin('b', Button::A, vec![])],
        );
        config.profile.costs.same_hand_transition = 2.0;
        let p = Problem::new(&config, &p.actions, p.stats.clone(), &p.strategy).unwrap();
        let assignment = p.baseline(&config.weights).unwrap();
        let reference = p.metrics(&assignment).normalization();
        assert_eq!(
            p.summary(&assignment, &config.weights, reference)
                .same_hand_transition_rate,
            0.0
        );
        let (mut config, p) = problem(
            "a2a2",
            mode,
            vec![
                pin('a', Button::A, vec![]),
                Binding {
                    action: Action::Character('2'),
                    held: vec![Button::R2],
                    ..left
                },
            ],
        );
        config.profile.costs.same_hand_transition = 2.0;
        let mut p = Problem::new(&config, &p.actions, p.stats.clone(), &p.strategy).unwrap();
        let assignment = p.baseline(&config.weights).unwrap();
        assert_eq!(
            p.summary(&assignment, &config.weights, reference)
                .same_hand_transition_rate,
            1.0
        );
        let layout = p.layout(&assignment, &config.weights, reference);
        let (loaded, assignment) = load_layout(&layout, p.stats.clone()).unwrap();
        let charged = loaded.metrics(&assignment).effort;
        p.profile.costs.same_hand_transition = 0.0;
        assert!((charged - p.metrics(&assignment).effort - 1.5).abs() < 1e-12);
    }

    #[test]
    fn independent_menu_outputs_do_not_consume_chords_and_round_trip_in_scores() {
        let mode = Strategy {
            name: "independent".into(),
            independent: true,
            shift: Some(Button::L2),
            layer: Some(Button::R2),
        };
        for pins in [vec![], vec![pin('\n', Button::A, vec![Button::R2])]] {
            let pinned_enter = !pins.is_empty();
            let (config, p) = problem("a\nb\n", mode.clone(), pins);
            let assignment = p.baseline(&config.weights).unwrap();
            let expected = p.metrics(&assignment);
            let layout = p.layout(&assignment, &config.weights, expected.normalization());
            assert_eq!(
                layout
                    .bindings
                    .iter()
                    .any(|b| b.action == Action::Character('\n')),
                pinned_enter
            );
            assert!(
                !layout
                    .bindings
                    .iter()
                    .any(|b| b.action == Action::Key(Key::Escape))
            );
            let corpus = Corpus::new("a\nb\n").unwrap();
            let stats = Stats::new(&corpus, &layout.scoring_actions()).unwrap();
            let (loaded, loaded_assignment) = load_layout(&layout, stats).unwrap();
            let actual = loaded.metrics(&loaded_assignment);
            assert!((actual.effort - expected.effort).abs() < 1e-12);
            assert!((actual.activations - expected.activations).abs() < 1e-12);
            assert!((actual.movement - expected.movement).abs() < 1e-12);
            assert!((actual.coverage - expected.coverage).abs() < 1e-12);
            if !pinned_enter {
                assert_eq!(actual.activations, 1.0);
            }
        }
    }

    #[test]
    fn version_one_legacy_layouts_keep_their_original_scores() {
        let (config, p) = problem("aAa", strategy(Some(Button::L2), None), vec![]);
        let assignment = p.baseline(&config.weights).unwrap();
        let mut layout = p.layout(
            &assignment,
            &config.weights,
            p.metrics(&assignment).normalization(),
        );
        let expected = p.metrics(&assignment);
        layout.schema_version = 1;
        layout.cost_model_version = 1;
        let legacy_json = serde_json::to_string(&layout).unwrap();
        assert!(!legacy_json.contains("independent"));
        assert!(!legacy_json.contains("held_dpad"));
        let legacy: Layout = serde_json::from_str(&legacy_json).unwrap();
        let (loaded, assignment) = load_layout(&legacy, p.stats).unwrap();
        let actual = loaded.metrics(&assignment);
        assert_eq!(expected.effort, actual.effort);
        assert_eq!(expected.activations, actual.activations);
        assert_eq!(expected.movement, actual.movement);
        assert_eq!(expected.coverage, actual.coverage);
        assert_eq!(expected.learning, actual.learning);
        let (config, single) = problem("a", strategy(None, None), vec![]);
        let assignment = single.baseline(&config.weights).unwrap();
        assert_eq!(
            single
                .summary(
                    &assignment,
                    &config.weights,
                    single.metrics(&assignment).normalization()
                )
                .same_hand_transition_rate,
            0.0
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
