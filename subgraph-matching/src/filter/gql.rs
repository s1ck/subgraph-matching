use crate::graph::Graph;

use super::Candidates;
use super::INVALID_NODE_ID;

// The C++ impl uses 100_000_000 :shrug:
const NOT_FOUND: usize = usize::MAX;

pub fn gql_filter(data_graph: &Graph, query_graph: &Graph) -> Option<Candidates> {
    // Local refinement
    let mut candidates = super::ldf_filter(data_graph, query_graph)?;

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

                match_cheap(
                    &bigraph_offsets,
                    &bigraph_targets,
                    &mut left_mapping,
                    &mut right_mapping,
                    left_partition_size,
                );

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

fn match_cheap(
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

// Checks if each element on the left side has a match on the right side
fn is_semi_perfect_mapping(mapping: &[usize], size: usize) -> bool {
    for i in 0..size {
        if mapping[i] == NOT_FOUND {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::GdlGraph;
    use trim_margin::MarginTrimmable;

    fn graph(gdl: &str) -> GdlGraph {
        gdl.trim_margin().unwrap().parse::<GdlGraph>().unwrap()
    }

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

        let candidates = gql_filter(&data_graph, &query_graph).unwrap();

        assert_eq!(candidates.candidates(0), &[0]);
        assert_eq!(candidates.candidates(1), &[4]);
        assert_eq!(candidates.candidates(2), &[3, 5]);
        assert_eq!(candidates.candidates(3), &[10, 12]);

        assert_eq!(candidates.candidate_count(0), 1);
        assert_eq!(candidates.candidate_count(1), 1);
        assert_eq!(candidates.candidate_count(2), 2);
        assert_eq!(candidates.candidate_count(3), 2);
    }
}
