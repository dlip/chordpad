use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io};

#[derive(Deserialize)]
struct Slot {
    bank: u8,
    thumb: Option<u8>,
    position: [f64; 2],
}

#[derive(Deserialize)]
struct Problem {
    counts: Vec<f64>,
    unary: Vec<f64>,
    edge_costs: Vec<Vec<Vec<f64>>>,
    edges: Vec<(usize, usize, usize, f64)>,
    skips: Vec<(Vec<usize>, f64)>,
    slots: Vec<Slot>,
    groups: Vec<Vec<usize>>,
    available: Vec<Vec<usize>>,
    pins: BTreeMap<usize, usize>,
    index_offset: f64,
    balance_weights: [f64; 2],
    travel_weight: f64,
    seeds: Vec<u64>,
    iterations: usize,
}

enum Factor {
    Edge(usize, usize, usize, f64),
    Skip(Vec<usize>, f64),
}

impl Factor {
    fn actions(&self) -> Vec<usize> {
        match self {
            Self::Edge(a, b, _, _) => vec![*a, *b],
            Self::Skip(window, _) => window.clone(),
        }
    }

    fn cost(&self, p: &Problem, assignment: &[usize]) -> f64 {
        match self {
            Self::Edge(a, b, kind, f) => f * p.edge_costs[*kind][assignment[*a]][assignment[*b]],
            Self::Skip(window, f) => {
                let first = &p.slots[assignment[window[0]]];
                let last = &p.slots[assignment[*window.last().unwrap()]];
                if first.thumb.is_none()
                    || first.thumb != last.thumb
                    || window[1..window.len() - 1]
                        .iter()
                        .any(|a| p.slots[assignment[*a]].thumb == first.thumb)
                {
                    return 0.0;
                }
                f * p.travel_weight
                    * (first.position[0] - last.position[0])
                        .hypot(first.position[1] - last.position[1])
            }
        }
    }
}

struct Search<'a> {
    problem: &'a Problem,
    factors: Vec<Factor>,
    swaps: Vec<(usize, usize, Vec<usize>)>,
}

impl<'a> Search<'a> {
    fn new(problem: &'a Problem) -> Self {
        let factors: Vec<_> = problem
            .edges
            .iter()
            .map(|&(a, b, kind, f)| Factor::Edge(a, b, kind, f))
            .chain(
                problem
                    .skips
                    .iter()
                    .map(|(window, f)| Factor::Skip(window.clone(), *f)),
            )
            .collect();
        let mut by_action = vec![vec![]; problem.counts.len()];
        for (i, factor) in factors.iter().enumerate() {
            for action in factor.actions() {
                by_action[action].push(i);
            }
        }
        let mut swaps = vec![];
        for group in &problem.groups {
            let free: Vec<_> = group
                .iter()
                .filter(|a| !problem.pins.contains_key(a))
                .copied()
                .collect();
            for (i, &a) in free.iter().enumerate() {
                for &b in &free[i + 1..] {
                    let mut affected = by_action[a].clone();
                    affected.extend_from_slice(&by_action[b]);
                    affected.sort_unstable();
                    affected.dedup();
                    swaps.push((a, b, affected));
                }
            }
        }
        Self {
            problem,
            factors,
            swaps,
        }
    }

    fn balance(&self, assignment: &[usize]) -> f64 {
        let p = self.problem;
        let mut thumbs: f64 = 0.0;
        let mut indices = p.index_offset;
        for (action, slot) in assignment.iter().enumerate() {
            let slot = &p.slots[*slot];
            let f = p.counts[action];
            if let Some(thumb) = slot.thumb {
                thumbs += f * if thumb == 0 { 1.0 } else { -1.0 };
            }
            indices += f
                * (f64::from(u8::from(slot.bank & 1 != 0))
                    - f64::from(u8::from(slot.bank & 6 != 0)));
        }
        p.balance_weights[0] * thumbs.powi(2) + p.balance_weights[1] * indices.powi(2)
    }

    fn objective(&self, assignment: &[usize]) -> f64 {
        let p = self.problem;
        assignment
            .iter()
            .enumerate()
            .map(|(a, s)| p.counts[a] * p.unary[*s])
            .sum::<f64>()
            + self
                .factors
                .iter()
                .map(|factor| factor.cost(p, assignment))
                .sum::<f64>()
            + self.balance(assignment)
    }

    fn delta(&self, swap: usize, assignment: &mut [usize]) -> f64 {
        let (a, b, affected) = &self.swaps[swap];
        let p = self.problem;
        let cost = |assignment: &[usize]| {
            affected
                .iter()
                .map(|i| self.factors[*i].cost(p, assignment))
                .sum::<f64>()
                + self.balance(assignment)
                + p.counts[*a] * p.unary[assignment[*a]]
                + p.counts[*b] * p.unary[assignment[*b]]
        };
        let before = cost(assignment);
        assignment.swap(*a, *b);
        let after = cost(assignment);
        assignment.swap(*a, *b);
        after - before
    }

    fn optimize(&self, seed: u64) -> Vec<usize> {
        let p = self.problem;
        let mut rng = fastrand::Rng::with_seed(seed);
        let mut assignment: Vec<_> = (0..p.counts.len()).collect();
        for (&a, &s) in &p.pins {
            assignment[a] = s;
        }
        for (group, available) in p.groups.iter().zip(&p.available) {
            let free: Vec<_> = group
                .iter()
                .filter(|a| !p.pins.contains_key(a))
                .copied()
                .collect();
            let mut slots: Vec<_> = available
                .iter()
                .filter(|s| !p.pins.values().any(|pin| pin == *s))
                .copied()
                .collect();
            rng.shuffle(&mut slots);
            for (a, s) in free.into_iter().zip(slots) {
                assignment[a] = s;
            }
        }
        let mut current = self.objective(&assignment);
        let mut best = (current, assignment.clone());
        for step in 0..p.iterations {
            let i = rng.usize(..self.swaps.len());
            let change = self.delta(i, &mut assignment);
            let temperature =
                0.025 * (0.00002_f64 / 0.025).powf(step as f64 / p.iterations.max(2) as f64);
            if change < 0.0 || rng.f64() < (-change / temperature).exp() {
                let (a, b, _) = self.swaps[i];
                assignment.swap(a, b);
                current += change;
                if current < best.0 {
                    best = (current, assignment.clone());
                }
            }
        }
        assignment = best.1;
        loop {
            let (change, i) = (0..self.swaps.len())
                .map(|i| (self.delta(i, &mut assignment), i))
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .unwrap();
            if change >= -1e-12 {
                break;
            }
            let (a, b, _) = self.swaps[i];
            assignment.swap(a, b);
        }
        assignment
    }
}

#[derive(Serialize)]
struct Run {
    seed: u64,
    objective: f64,
    assignment: Vec<usize>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let problem: Problem = serde_json::from_reader(io::stdin())?;
    let search = Search::new(&problem);
    let runs: Vec<_> = std::thread::scope(|scope| {
        let search = &search;
        let handles: Vec<_> = problem
            .seeds
            .iter()
            .map(|&seed| {
                scope.spawn(move || {
                    let assignment = search.optimize(seed);
                    let objective = search.objective(&assignment);
                    eprintln!("seed {seed}: training objective {objective:.8}");
                    Run {
                        seed,
                        objective,
                        assignment,
                    }
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("search worker panicked"))
            .collect()
    });
    serde_json::to_writer(io::stdout(), &runs)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem() -> Problem {
        Problem {
            counts: vec![0.2, 0.3, 0.5],
            unary: vec![1.0, 1.2, 1.4],
            edge_costs: vec![vec![vec![0.0; 3]; 3]],
            edges: vec![(0, 1, 0, 0.3)],
            skips: vec![(vec![0, 1, 2], 0.5)],
            slots: vec![
                Slot {
                    bank: 0,
                    thumb: Some(0),
                    position: [0.0, -1.0],
                },
                Slot {
                    bank: 1,
                    thumb: Some(1),
                    position: [0.0, 1.0],
                },
                Slot {
                    bank: 2,
                    thumb: Some(0),
                    position: [0.0, 1.0],
                },
            ],
            groups: vec![vec![0, 1, 2]],
            available: vec![vec![0, 1, 2]],
            pins: BTreeMap::new(),
            index_offset: -0.1,
            balance_weights: [2.0, 0.5],
            travel_weight: 0.4,
            seeds: vec![42],
            iterations: 100,
        }
    }

    #[test]
    fn skip_cost_counts_only_the_next_use_of_a_thumb() {
        let p = problem();
        assert_eq!(Factor::Skip(vec![0, 1, 2], 0.5).cost(&p, &[0, 1, 2]), 0.4);
        assert_eq!(Factor::Skip(vec![0, 2, 0], 0.5).cost(&p, &[0, 1, 2]), 0.0);
    }

    #[test]
    fn incremental_changes_equal_full_rescoring() {
        let p = problem();
        let search = Search::new(&p);
        let mut assignment = vec![0, 1, 2];
        for i in 0..search.swaps.len() {
            let before = search.objective(&assignment);
            let delta = search.delta(i, &mut assignment);
            let (a, b, _) = search.swaps[i];
            assignment.swap(a, b);
            assert!((search.objective(&assignment) - before - delta).abs() < 1e-12);
            assignment.swap(a, b);
        }
    }
}
