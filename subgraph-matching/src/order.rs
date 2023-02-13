use crate::{filter::Candidates, graph::Graph};

/// Builds a matching order by starting with the node with the minimum
/// number of candidates and iteratively selecting nodes that are adjacent
/// to already selected nodes and having the minimum number of candidates.
pub fn gql_order(data_graph: &Graph, query_graph: &Graph, candidates: &Candidates) -> Vec<usize> {
    let node_count = query_graph.node_count();

    let mut visited = vec![false; node_count];
    let mut adjacent = vec![false; node_count];
    let mut order = Vec::<usize>::with_capacity(node_count);

    let start = gql_start_node(query_graph, candidates);
    order.push(start);

    update_valid_vertices(query_graph, start, &mut visited, &mut adjacent);

    for _ in 1..node_count {
        let mut next_node = usize::MAX;
        let mut min_value = data_graph.node_count() + 1;

        for curr_node in 0..node_count {
            if !visited[curr_node] && adjacent[curr_node] {
                let num_candidates = candidates.candidate_count(curr_node);

                if num_candidates < min_value {
                    min_value = num_candidates;
                    next_node = curr_node;
                } else if num_candidates == min_value
                    && query_graph.degree(curr_node) > query_graph.degree(next_node)
                {
                    next_node = curr_node;
                }
            }
        }
        update_valid_vertices(query_graph, next_node, &mut visited, &mut adjacent);
        order.push(next_node);
    }

    order
}

/// Selects the node with the minimum number of candidates as start node.
///
/// Ties are handles by picking the node with a higher degree.
fn gql_start_node(query_graph: &Graph, candidates: &Candidates) -> usize {
    let mut start = 0;

    for node in 1..query_graph.node_count() {
        let num_node_candidates = candidates.candidate_count(node);
        let num_start_candidates = candidates.candidate_count(start);

        if num_node_candidates < num_start_candidates
            || (num_node_candidates == num_start_candidates
                && query_graph.degree(node) > query_graph.degree(start))
        {
            start = node;
        }
    }

    start
}

fn update_valid_vertices(
    query_graph: &Graph,
    query_node: usize,
    visited: &mut [bool],
    adjacent: &mut [bool],
) {
    visited[query_node] = true;
    for neighbor in query_graph.neighbors(query_node) {
        adjacent[*neighbor] = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{filter::ldf_filter, graph::GdlGraph};
    use trim_margin::MarginTrimmable;

    fn graph(gdl: &str) -> GdlGraph {
        gdl.trim_margin().unwrap().parse::<GdlGraph>().unwrap()
    }

    const TEST_GRAPH: &str = "
        |(n0 { label: 0 })
        |(n1 { label: 1 })
        |(n2 { label: 2 })
        |(n3 { label: 1 })
        |(n4 { label: 4 })
        |(n0)-->(n1)
        |(n0)-->(n2)
        |(n1)-->(n2)
        |(n1)-->(n3)
        |(n2)-->(n4)
        |(n3)-->(n4)
        |";

    #[test]
    fn test_gql_order() {
        let data_graph = graph(TEST_GRAPH);
        let query_graph = graph(
            "
            |(n0 { label: 0 }),(n1 { label: 1 }),(n2 { label: 2 })
            |(n0)-->(n1)
            |(n0)-->(n2)
            |(n1)-->(n2)
            |",
        );

        let candidates = ldf_filter(&data_graph, &query_graph).unwrap();

        assert_eq!(candidates.candidates(0), &[0]);
        assert_eq!(candidates.candidates(1), &[1, 3]);
        assert_eq!(candidates.candidates(2), &[2]);

        let order = gql_order(&data_graph, &query_graph, &candidates);

        assert_eq!(order, vec![0, 2, 1]);
    }

    #[test]
    fn test_gql_order_same_graph() {
        let data_graph = graph(TEST_GRAPH);
        let query_graph = graph(TEST_GRAPH);
        let candidates = ldf_filter(&data_graph, &query_graph).unwrap();

        assert_eq!(candidates.candidates(0), &[0]);
        assert_eq!(candidates.candidates(1), &[1]);
        assert_eq!(candidates.candidates(2), &[2]);
        assert_eq!(candidates.candidates(3), &[1, 3]);
        assert_eq!(candidates.candidates(4), &[4]);

        let order = gql_order(&data_graph, &query_graph, &candidates);

        assert_eq!(order, vec![1, 2, 0, 4, 3]);
    }
}
