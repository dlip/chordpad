use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    Result,
    model::{Action, Key},
};

#[derive(Clone, Debug)]
pub struct Corpus {
    pub characters: Vec<char>,
    pub sha256: String,
}

impl Corpus {
    pub fn new(text: &str) -> Result<Self> {
        // Normalize file line endings without discarding case, spaces, or punctuation.
        let normalized = text.replace("\r\n", "\n");
        if normalized.is_empty() {
            return Err("corpus must not be empty".into());
        }
        Ok(Self {
            characters: normalized.chars().collect(),
            sha256: sha256(text.as_bytes()),
        })
    }

    pub fn actions(&self, ascii: bool, include: &str) -> Vec<Action> {
        let mut actions: BTreeSet<Action> = self
            .characters
            .iter()
            .copied()
            .map(Action::Character)
            .collect();
        actions.extend(include.chars().map(Action::Character));
        if ascii {
            actions.extend((' '..='~').chain(['\n', '\t']).map(Action::Character));
        }
        actions.extend([Action::Key(Key::Backspace), Action::Key(Key::Escape)]);
        actions.into_iter().collect()
    }
}

pub fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[derive(Clone, Debug)]
pub struct Stats {
    pub actions: Vec<Action>,
    pub counts: Vec<u64>,
    pub pairs: Vec<(usize, usize, u64)>,
    pub skipgrams: Vec<(Vec<usize>, u64)>,
    pub first: usize,
    pub last: usize,
    pub total: usize,
}

impl Stats {
    pub fn new(corpus: &Corpus, actions: &[Action]) -> Result<Self> {
        let indexes: BTreeMap<_, _> = actions.iter().enumerate().map(|(i, a)| (a, i)).collect();
        let sequence = corpus
            .characters
            .iter()
            .map(|&c| {
                indexes.get(&Action::Character(c)).copied().ok_or_else(|| {
                    format!(
                        "layout does not cover corpus character {c:?}; use --include to require it"
                    )
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let mut counts = vec![0; actions.len()];
        let mut pairs = BTreeMap::new();
        for &index in &sequence {
            counts[index] += 1;
        }
        for pair in sequence.windows(2) {
            *pairs.entry((pair[0], pair[1])).or_insert(0) += 1;
        }
        let mut skipgrams = BTreeMap::new();
        for length in 3..=5 {
            for window in sequence.windows(length) {
                *skipgrams.entry(window.to_vec()).or_insert(0) += 1;
            }
        }
        Ok(Self {
            actions: actions.to_vec(),
            counts,
            pairs: pairs.into_iter().map(|((a, b), n)| (a, b, n)).collect(),
            skipgrams: skipgrams.into_iter().collect(),
            first: sequence[0],
            last: *sequence.last().unwrap(),
            total: sequence.len(),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct CorpusSummary {
    pub sha256: String,
    pub characters: usize,
    pub normalization: &'static str,
}

impl From<&Corpus> for CorpusSummary {
    fn from(corpus: &Corpus) -> Self {
        Self {
            sha256: corpus.sha256.clone(),
            characters: corpus.characters.len(),
            normalization: "CRLF to LF; otherwise unchanged",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpus_preserves_case_unicode_spaces_and_line_endings() {
        let corpus = Corpus::new("a Aé\r\n").unwrap();
        assert_eq!(corpus.characters, ['a', ' ', 'A', 'é', '\n']);
        assert_ne!(corpus.sha256, Corpus::new("a Aé\n").unwrap().sha256);
        let actions = corpus.actions(false, "");
        let stats = Stats::new(&corpus, &actions).unwrap();
        assert_eq!(stats.total, 5);
        assert_eq!(stats.pairs.iter().map(|(_, _, n)| n).sum::<u64>(), 4);
    }

    #[test]
    fn empty_text_and_unknown_evaluation_characters_fail() {
        assert!(Corpus::new("").is_err());
        let train = Corpus::new("aa").unwrap();
        assert!(Stats::new(&Corpus::new("ab").unwrap(), &train.actions(false, "")).is_err());
    }

    #[test]
    fn skipgrams_preserve_intervening_characters_and_count_repeated_windows() {
        for text in ["a", "ab", "abcde", "aaaaaa"] {
            let corpus = Corpus::new(text).unwrap();
            let actions = corpus.actions(false, "");
            let stats = Stats::new(&corpus, &actions).unwrap();
            for length in 3..=5 {
                assert_eq!(
                    stats
                        .skipgrams
                        .iter()
                        .filter(|(w, _)| w.len() == length)
                        .map(|(_, n)| n)
                        .sum::<u64>(),
                    text.len().saturating_sub(length - 1) as u64
                );
            }
            if text == "abcde" {
                let a = actions
                    .iter()
                    .position(|a| *a == Action::Character('a'))
                    .unwrap();
                let c = actions
                    .iter()
                    .position(|a| *a == Action::Character('c'))
                    .unwrap();
                assert!(
                    stats
                        .skipgrams
                        .iter()
                        .any(|(w, n)| w.len() == 3 && w[0] == a && w[2] == c && *n == 1)
                );
            }
            if text == "aaaaaa" {
                assert_eq!(stats.skipgrams.len(), 3);
            }
        }
    }
}
