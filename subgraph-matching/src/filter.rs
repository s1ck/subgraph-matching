use std::{fmt::Display, usize};

use crate::graph::Graph;

mod gql;
mod ldf;

pub use gql::gql_filter;
pub use ldf::ldf_filter;

const INVALID_NODE_ID: usize = usize::MAX;

#[derive(Debug, Default)]
pub struct Candidates {
    /// candidates for each query node
    candidates: Box<[Vec<usize>]>,
}

impl Candidates {
    pub fn new(candidates: Vec<Vec<usize>>) -> Self {
        Self {
            candidates: candidates.into_boxed_slice(),
        }
    }

    pub fn add_candidate(&mut self, query_node: usize, data_node: usize) {
        self.candidates[query_node].push(data_node);
    }

    pub fn set_candidate(&mut self, query_node: usize, idx: usize, data_node: usize) {
        self.candidates[query_node][idx] = data_node;
    }

    pub fn candidates(&self, data_node: usize) -> &[usize] {
        self.candidates[data_node].as_slice()
    }

    pub fn candidates_mut(&mut self, data_node: usize) -> &mut [usize] {
        self.candidates[data_node].as_mut_slice()
    }

    pub fn candidate_count(&self, query_node: usize) -> usize {
        self.candidates[query_node].len()
    }

    pub fn sort(&mut self) {
        for c in self.candidates.iter_mut() {
            c.sort_unstable()
        }
    }

    pub fn compact(&mut self) {
        for node_candidates in self.candidates.iter_mut() {
            let mut write_idx = 0;
            for idx in 0..node_candidates.len() {
                if node_candidates[idx] != INVALID_NODE_ID {
                    node_candidates[write_idx] = node_candidates[idx];
                    write_idx += 1;
                }
            }
            // Safely shorten the length of the vector
            node_candidates.drain(write_idx..node_candidates.len());
        }
    }

    pub fn is_valid(&self) -> bool {
        for node_candidates in self.candidates.iter() {
            if node_candidates.is_empty() {
                return false;
            }
        }
        true
    }
}

impl From<(&Graph, &Graph)> for Candidates {
    fn from((data_graph, query_graph): (&Graph, &Graph)) -> Self {
        let query_node_count = query_graph.node_count();
        let max_candidates = data_graph.max_label_frequency();

        let mut candidates = Vec::with_capacity(query_node_count);

        for _ in 0..query_node_count {
            candidates.push(Vec::<usize>::with_capacity(max_candidates));
        }

        Self::new(candidates)
    }
}

impl Display for Candidates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let counts = self
            .candidates
            .iter()
            .enumerate()
            .map(|(n, c)| format!("{}: {}", n, c.len()))
            .collect::<Vec<_>>();

        write!(f, "{{{}}}", counts.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candidates_sorting() {
        let input = vec![vec![4, 2], vec![1, 7, 3, 3], vec![0]];
        let mut candidates = Candidates::new(input);

        candidates.sort();

        assert_eq!(candidates.candidates(0), &[2, 4]);
        assert_eq!(candidates.candidates(1), &[1, 3, 3, 7]);
        assert_eq!(candidates.candidates(2), &[0]);
    }
}
