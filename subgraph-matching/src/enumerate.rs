use crate::{filter::Candidates, graph::Graph};

pub fn gql(
    data_graph: &Graph,
    query_graph: &Graph,
    candidates: &Candidates,
    order: &[usize],
) -> usize {
    gql_with(data_graph, query_graph, candidates, order, |_| {})
}

pub fn gql_with<F>(
    data_graph: &Graph,
    query_graph: &Graph,
    candidates: &Candidates,
    order: &[usize],
    mut action: F,
) -> usize
where
    F: FnMut(&[usize]),
{
    let mut embedding_count = 0;

    // Stores the neighbors for each query node that have already been visited
    // according to the defined order.
    let visited_neighbors = visited_neighbors(query_graph, order);

    // The root of the traversal.
    let start_node = order[0];
    let max_depth = query_graph.node_count();

    // TODO bit set?
    // Tracks which data node has already been visited during the traversal.
    let mut visited = vec![false; data_graph.node_count()];

    // Represents the valid next candidates out of the possible candidates for each depth.
    // For depth 0, this is equivalent to the candidates of query node at order[0].
    let mut valid_candidates = Vec::with_capacity(max_depth);
    // TODO: can we avoid copying from slice (this array is never updated)
    valid_candidates.push(Vec::from(candidates.candidates(start_node)));
    for u in order[1..].iter() {
        // We pre-allocate the vec with the number of candidates since we can't
        // know how many of them will be valid neighbors according to the query.
        valid_candidates.push(vec![0; candidates.candidate_count(*u)]);
    }

    // Idx tracks the currently processed candidate at each depth.
    let mut idx = vec![0_usize; max_depth];
    // Idx_count tracks the number of valid candidates at each depth.
    let mut idx_count = vec![0_usize; max_depth];
    // Stores the mapping between query and data nodes according to order.
    let mut embedding = vec![0_usize; max_depth];

    let mut cur_depth = 0;

    idx[cur_depth] = 0;
    idx_count[cur_depth] = candidates.candidate_count(start_node);

    loop {
        while idx[cur_depth] < idx_count[cur_depth] {
            let u = order[cur_depth];
            let v = valid_candidates[cur_depth][idx[cur_depth]];

            embedding[u] = v;
            visited[v] = true;
            idx[cur_depth] += 1;

            if cur_depth == max_depth - 1 {
                embedding_count += 1;
                visited[v] = false;
                action(&embedding);
                // TODO output limit
            } else {
                // Go down into the rabbit hole.
                cur_depth += 1;
                idx[cur_depth] = 0;

                generate_valid_candidates(
                    data_graph,
                    cur_depth,
                    &embedding,
                    &mut idx_count,
                    &mut valid_candidates,
                    &visited,
                    &visited_neighbors,
                    order,
                    candidates,
                );
            }
        }

        if cur_depth == 0 {
            break;
        }
        // backtrack
        cur_depth -= 1;
        visited[embedding[order[cur_depth]]] = false;
    }

    embedding_count
}

/// For each node in the query graph stores which
/// of their neighbors already have been visited
/// according to the matching order.
fn visited_neighbors(query_graph: &Graph, order: &[usize]) -> Vec<Vec<usize>> {
    let max_depth = query_graph.node_count();
    let start_node = order[0];

    let mut blacklist = vec![Vec::<usize>::with_capacity(max_depth); max_depth];
    let mut visited = vec![false; max_depth];
    visited[start_node] = true;

    for i in 1..max_depth {
        let cur_node = order[i];
        for neighbor in query_graph.neighbors(cur_node) {
            if visited[*neighbor] {
                blacklist[i].push(*neighbor);
            }
        }
        visited[cur_node] = true;
    }

    blacklist
}

fn generate_valid_candidates(
    data_graph: &Graph,
    depth: usize,
    embedding: &[usize],
    idx_count: &mut [usize],
    valid_candidates: &mut [Vec<usize>],
    visited: &[bool],
    visited_neighbors: &[Vec<usize>],
    order: &[usize],
    candidates: &Candidates,
) {
    let u = order[depth];

    idx_count[depth] = 0;

    for v in candidates.candidates(u) {
        if !visited[*v] {
            let mut valid = true;

            // Visited neighbors contains the adjacent query nodes that
            // we already evaluated and mapped to a data node. We need
            // to make sure that for each relationship to those neighbors
            // there exists a relationship in the data graph that points
            // to the candidate node v.
            for u_nbr in &visited_neighbors[depth][..] {
                let u_nbr_v = embedding[*u_nbr];

                if !data_graph.exists(*v, u_nbr_v) {
                    valid = false;
                    break;
                }
            }

            // We could successfully map each relationship from the query
            // graph to a relationship in the data graph that ends in v.
            // Therefore, v is a validate candidate for the current depth.
            if valid {
                valid_candidates[depth][idx_count[depth]] = *v;
                idx_count[depth] += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{filter, graph::GdlGraph, order};
    use trim_margin::MarginTrimmable;

    fn graph(gdl: &str) -> GdlGraph {
        gdl.trim_margin().unwrap().parse::<GdlGraph>().unwrap()
    }

    const TEST_GRAPH: &str = "
        |(n0:L0)
        |(n1:L1)
        |(n2:L2)
        |(n3:L1)
        |(n4:L2)
        |(n0)-->(n1)
        |(n0)-->(n2)
        |(n1)-->(n2)
        |(n1)-->(n3)
        |(n2)-->(n4)
        |(n3)-->(n4)
        |";

    #[test]
    fn test_visited_neighbors() {
        let graph = graph(TEST_GRAPH);

        let order = vec![2, 4, 0, 1, 3];

        let visited_neighbors = visited_neighbors(&graph, &order);

        assert_eq!(visited_neighbors[0], vec![]);
        assert_eq!(visited_neighbors[1], vec![2]);
        assert_eq!(visited_neighbors[2], vec![2]);
        assert_eq!(visited_neighbors[3], vec![0, 2]);
        assert_eq!(visited_neighbors[4], vec![1, 4]);
    }

    #[test]
    fn test_line_query() {
        let data_graph = graph(TEST_GRAPH);
        let query_graph = graph(
            "
            |(n0:L0),(n1:L1),(n2:L2)
            |(n0)-->(n1)
            |(n1)-->(n2)
            |",
        );

        let candidates = filter::ldf_filter(&data_graph, &query_graph).unwrap();
        assert_eq!(candidates.candidates(0), &[0]);
        assert_eq!(candidates.candidates(1), &[1, 3]);
        assert_eq!(candidates.candidates(2), &[2, 4]);
        let order = order::gql_order(&data_graph, &query_graph, &candidates);
        assert_eq!(order, &[0, 1, 2]);

        let embedding_count = gql_with(
            &data_graph,
            &query_graph,
            &candidates,
            &order,
            |embedding| assert_eq!(embedding, &[0, 1, 2]),
        );

        assert_eq!(embedding_count, 1);
    }

    #[test]
    fn test_diamond() {
        let data_graph = graph(TEST_GRAPH);
        let query_graph = graph(
            "
            |(n0:L1),(n1:L2),(n2:L1),(n3:L2)
            |(n0)-->(n1)
            |(n0)-->(n2)
            |(n1)-->(n3)
            |(n2)-->(n3)
            |",
        );

        let candidates = filter::ldf_filter(&data_graph, &query_graph).unwrap();
        assert_eq!(candidates.candidates(0), &[1, 3]);
        assert_eq!(candidates.candidates(1), &[2, 4]);
        assert_eq!(candidates.candidates(2), &[1, 3]);
        assert_eq!(candidates.candidates(3), &[2, 4]);

        let order = order::gql_order(&data_graph, &query_graph, &candidates);
        assert_eq!(order, &[0, 1, 2, 3]);

        let mut embeddings = Vec::with_capacity(2);

        let embedding_count = gql_with(
            &data_graph,
            &query_graph,
            &candidates,
            &order,
            |embedding| embeddings.push(Vec::from(embedding)),
        );

        assert_eq!(embedding_count, 2);
        assert_eq!(embeddings[0], vec![1, 2, 3, 4]);
        assert_eq!(embeddings[1], vec![3, 4, 1, 2]);
    }
}
