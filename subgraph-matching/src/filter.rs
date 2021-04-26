use std::{fmt::Display, usize};

use crate::graph::Graph;

// The C++ impl uses 100_000_000 :shrug:
const INVALID_NODE_ID: usize = usize::MAX;
const NOT_FOUND: usize = usize::MAX;

// LDF: label-and-degree filtering
//
// C(u) = { v ∈ V(G) | L(v) = L(u) ∧ d(v) >= d(u) }
pub fn ldf_filter(data_graph: &Graph, query_graph: &Graph) -> Option<Candidates> {
    let mut candidates = Candidates::from((data_graph, query_graph));

    for query_node in 0..query_graph.node_count() {
        let label = query_graph.label(query_node);
        let degree = query_graph.degree(query_node);

        let nodes_by_label = data_graph.nodes_by_label(label);

        for data_node in nodes_by_label {
            if data_graph.degree(*data_node) >= degree {
                candidates.add_candidate(query_node, *data_node);
            }
        }

        // break early
        if candidates.candidate_count(query_node) == 0 {
            return None;
        }
    }

    Some(candidates)
}

pub fn gql_filter(data_graph: &Graph, query_graph: &Graph) -> Option<Candidates> {
    // Local refinement
    let mut candidates = ldf_filter(data_graph, query_graph)?;

    let query_node_count = query_graph.node_count();
    let data_node_count = data_graph.node_count();

    // Record valid candidate vertices for each query vertex
    // TODO: bitset
    let mut valid_candidates = Vec::with_capacity(query_node_count);
    for query_node in 0..query_node_count {
        let mut node_candidates = vec![false; data_node_count];
        for data_node in candidates.candidates(query_node) {
            node_candidates[*data_node] = true;
        }
        valid_candidates.push(node_candidates);
    }

    let query_graph_max_degree = query_graph.max_degree();
    let data_graph_max_degree = data_graph.max_degree();

    // CSR datastructures to represent the bi-partite graph
    let mut bigraph_offsets = vec![0_usize; query_graph_max_degree + 1];
    let mut bigraph_targets = vec![0_usize; query_graph_max_degree * data_graph_max_degree];
    let mut left_mapping = vec![0_usize; query_graph_max_degree];
    let mut right_mapping = vec![0_usize; data_graph_max_degree];
    // Buffers for BFS
    let mut match_visited = vec![0_usize; data_graph_max_degree + 1];
    let mut match_queue = vec![0_usize; query_node_count];
    let mut match_previous = vec![0_usize; data_graph_max_degree + 1];

    // Global refinement
    for _ in 0..2 {
        for query_node in 0..query_node_count {
            for data_node in candidates.candidates_mut(query_node) {
                if *data_node == INVALID_NODE_ID {
                    continue;
                }

                let query_node_neighbors = query_graph.neighbors(query_node);
                let data_node_neighbors = data_graph.neighbors(*data_node);

                let left_partition_size = query_node_neighbors.len();

                compute_bipartite_graph(
                    query_node_neighbors,
                    data_node_neighbors,
                    &valid_candidates,
                    &mut bigraph_offsets,
                    &mut bigraph_targets,
                );

                left_mapping.fill(NOT_FOUND);
                right_mapping.fill(NOT_FOUND);

                match_bfs(
                    &bigraph_offsets,
                    &bigraph_targets,
                    &mut left_mapping,
                    &mut right_mapping,
                    &mut match_visited,
                    &mut match_queue,
                    &mut match_previous,
                    left_partition_size,
                );

                if !is_semi_perfect_mapping(&left_mapping, left_partition_size) {
                    valid_candidates[query_node][*data_node] = false;
                    *data_node = INVALID_NODE_ID;
                }
            }
        }
    }

    candidates.compact();

    if candidates.is_valid() {
        Some(candidates)
    } else {
        None
    }
}

fn match_bfs(
    bigraph_offsets: &[usize],
    bigraph_targets: &[usize],
    left_mapping: &mut [usize],
    right_mapping: &mut [usize],
    visited: &mut [usize],
    queue: &mut [usize],
    predecessors: &mut [usize],
    left_size: usize,
) {
    old_cheap(
        bigraph_offsets,
        bigraph_targets,
        left_mapping,
        right_mapping,
        left_size,
    );

    visited.fill(0);

    let mut queue_ptr;
    let mut queue_size;
    let mut next;
    let mut target;
    let mut col;
    let mut temp;

    let mut next_augment_no = 1;

    for root in 0..left_size {
        if left_mapping[root] == NOT_FOUND && bigraph_offsets[root] != bigraph_offsets[root + 1] {
            queue[0] = root;
            queue_ptr = 0;
            queue_size = 1;

            while queue_ptr < queue_size {
                next = queue[queue_ptr];
                queue_ptr += 1;

                for offset in bigraph_offsets[next]..bigraph_offsets[next + 1] {
                    target = bigraph_targets[offset];
                    temp = visited[target];

                    if temp != next_augment_no && temp != NOT_FOUND {
                        predecessors[target] = next;
                        visited[target] = next_augment_no;

                        col = right_mapping[target];

                        if col == NOT_FOUND {
                            while target != NOT_FOUND {
                                col = predecessors[target];
                                temp = left_mapping[col];
                                left_mapping[col] = target;
                                right_mapping[target] = col;
                                target = temp;
                            }
                            next_augment_no += 1;
                            queue_size = 0;
                            break;
                        } else {
                            queue[queue_size] = col;
                            queue_size += 1;
                        }
                    }
                }
            }

            if left_mapping[root] == NOT_FOUND {
                for j in 1..queue_size {
                    visited[left_mapping[queue[j]]] = NOT_FOUND;
                }
            }
        }
    }
}

fn old_cheap(
    bigraph_offsets: &[usize],
    bigraph_targets: &[usize],
    left_mapping: &mut [usize],
    right_mapping: &mut [usize],
    left_size: usize,
) {
    for left in 0..left_size {
        for offset in bigraph_offsets[left]..bigraph_offsets[left + 1] {
            let right = bigraph_targets[offset];
            if right_mapping[right] == NOT_FOUND {
                left_mapping[left] = right;
                right_mapping[right] = left;
                break;
            }
        }
    }
}

// Constructs a bi-partite graph between the N(query_node) and N(data_node)
fn compute_bipartite_graph(
    query_node_neighbors: &[usize],
    data_node_neighbors: &[usize],
    valid_candidates: &[Vec<bool>],
    bigraph_offsets: &mut [usize],
    bigraph_targets: &mut [usize],
) {
    let mut rel_count: usize = 0;

    for (i, query_node_neighbor) in query_node_neighbors.iter().enumerate() {
        bigraph_offsets[i] = rel_count;

        for (j, data_node_neighbor) in data_node_neighbors.iter().enumerate() {
            if valid_candidates[*query_node_neighbor][*data_node_neighbor] {
                bigraph_targets[rel_count] = j;
                rel_count += 1;
            }
        }
    }

    bigraph_offsets[query_node_neighbors.len()] = rel_count;
}

// Checks if each element on the left side has a unique match on the right side
fn is_semi_perfect_mapping(mapping: &[usize], size: usize) -> bool {
    for i in 0..size {
        if mapping[i] == NOT_FOUND {
            return false;
        }
    }
    true
}

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
            c.sort()
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
    use crate::graph::GdlGraph;
    use trim_margin::MarginTrimmable;

    fn graph(gdl: &str) -> GdlGraph {
        gdl.trim_margin().unwrap().parse::<GdlGraph>().unwrap()
    }

    const DATA_GRAPH_1: &str = "
        |(n0:L0)
        |(n1:L1)
        |(n2:L2)
        |(n3:L1)
        |(n4:L4)
        |(n0)-->(n1)
        |(n0)-->(n2)
        |(n1)-->(n2)
        |(n1)-->(n3)
        |(n2)-->(n4)
        |(n3)-->(n4)
        |";

    // Figure 1b) in the paper
    // L0 -> A
    // L1 -> B
    // L2 -> C
    // L3 -> D
    const DATA_GRAPH_2: &str = "
        |(n0:L0)
        |(n1:L2)
        |(n2:L1)
        |(n3:L2)
        |(n4:L1)
        |(n5:L2)
        |(n6:L1)
        |(n7:L2)
        |(n8:L3)
        |(n9:L3)
        |(n10:L3)
        |(n11:L3)
        |(n12:L3)
        |(n13:L2)
        |(n14:L3)
        |(n0)-->(n1)
        |(n0)-->(n2)
        |(n0)-->(n3)
        |(n0)-->(n4)
        |(n0)-->(n5)
        |(n0)-->(n6)
        |(n0)-->(n7)
        |(n1)-->(n2)
        |(n1)-->(n8)
        |(n2)-->(n9)
        |(n2)-->(n10)
        |(n3)-->(n4)
        |(n3)-->(n10)
        |(n4)-->(n5)
        |(n4)-->(n10)
        |(n4)-->(n11)
        |(n4)-->(n12)
        |(n5)-->(n12)
        |(n6)-->(n12)
        |(n6)-->(n13)
        |(n7)-->(n14)
        |(n9)-->(n10)
        |";

    #[test]
    fn test_ldf_filter() {
        let data_graph = graph(DATA_GRAPH_1);
        let query_graph = graph("(n0:L0), (n1:L1), (n2:L2), (n0)-->(n1), (n1)-->(n2)");

        assert_eq!(data_graph.nodes_by_label(0), &[0]);
        assert_eq!(data_graph.nodes_by_label(1), &[1, 3]);
        assert_eq!(data_graph.nodes_by_label(2), &[2]);
        assert_eq!(data_graph.nodes_by_label(4), &[4]);

        let candidates = ldf_filter(&data_graph, &query_graph).unwrap();

        assert_eq!(candidates.candidates(0), &[0]);
        assert_eq!(candidates.candidates(1), &[1, 3]);
        assert_eq!(candidates.candidates(2), &[2]);

        assert_eq!(candidates.candidate_count(0), 1);
        assert_eq!(candidates.candidate_count(1), 2);
        assert_eq!(candidates.candidate_count(2), 1);
    }

    #[test]
    fn test_ldf_filter_invalid_label() {
        let data_graph = graph(DATA_GRAPH_1);
        let query_graph = graph("(n0:L3), (n1:L1), (n2:L2), (n0)-->(n1), (n1)-->(n2)");
        let candidates = ldf_filter(&data_graph, &query_graph);
        assert!(candidates.is_none())
    }

    #[test]
    fn test_ldf_filter_invalid_degree() {
        let data_graph = graph(DATA_GRAPH_1);
        let query_graph = graph(
            "
                |(n0:L3),(n1:L1),(n2:L2)
                |(n0)-->(n1)
                |(n0)-->(n2)
                |(n0)-->(n2)
                |(n1)-->(n2)
                |",
        );
        let candidates = ldf_filter(&data_graph, &query_graph);
        assert!(candidates.is_none())
    }

    #[test]
    fn test_gql_filter() {
        let data_graph = graph(DATA_GRAPH_2);
        let query_graph = graph(
            "
            |(n0:L0)
            |(n1:L1)
            |(n2:L2)
            |(n3:L3)
            |(n0)-->(n1)
            |(n0)-->(n2)
            |(n1)-->(n2)
            |(n1)-->(n3)
            |(n2)-->(n3)
            |",
        );

        let candidates = ldf_filter(&data_graph, &query_graph).unwrap();

        assert_eq!(candidates.candidates(0), &[0]);
        assert_eq!(candidates.candidates(1), &[2, 4, 6]);
        assert_eq!(candidates.candidates(2), &[1, 3, 5]);
        assert_eq!(candidates.candidates(3), &[9, 10, 12]);

        assert_eq!(candidates.candidate_count(0), 1);
        assert_eq!(candidates.candidate_count(1), 3);
        assert_eq!(candidates.candidate_count(2), 3);
        assert_eq!(candidates.candidate_count(3), 3);
    }

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
