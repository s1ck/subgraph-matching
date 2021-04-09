use crate::{filter::Candidates, graph::Graph, order::MatchingOrder};

pub fn gql(
    data_graph: &Graph,
    query_graph: &Graph,
    candidates: &Candidates,
    order: &MatchingOrder,
) -> usize {
    gql_with(data_graph, query_graph, candidates, order, |_| {})
}

pub fn gql_with<F>(
    data_graph: &Graph,
    query_graph: &Graph,
    candidates: &Candidates,
    order: &MatchingOrder,
    mut action: F,
) -> usize
where
    F: FnMut(&[usize]),
{
    let mut embedding_count = 0;

    let visited_neighbors = visited_neighbors(query_graph, order);

    let start_node = order[0];
    let mut cur_depth = 0;
    let max_depth = query_graph.node_count();

    // TODO bit set?
    let mut visited = vec![false; data_graph.node_count()];

    let mut valid_candidates = Vec::with_capacity(max_depth);
    // TODO: avoid copy from slice (this array is never updated)
    valid_candidates.push(Vec::from(candidates.candidates(start_node)));
    for u in order[1..].iter() {
        valid_candidates.push(vec![0; candidates.candidate_count(*u)]);
    }

    let mut idx = vec![0 as usize; max_depth];
    let mut idx_count = vec![0 as usize; max_depth];
    let mut embedding = vec![0 as usize; max_depth];

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
                action(&embedding);
                visited[v] = false;
                // TODO output limit
            } else {
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
fn visited_neighbors(query_graph: &Graph, order: &Vec<usize>) -> Vec<Vec<usize>> {
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

            for u_nbr in &visited_neighbors[depth][..] {
                let u_nbr_v = embedding[*u_nbr];

                if !data_graph.exists(*v, u_nbr_v) {
                    valid = false;
                    break;
                }
            }

            if valid {
                valid_candidates[depth][idx_count[depth]] = *v;
                idx_count[depth] += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{filter, order};

    use super::*;
    use trim_margin::MarginTrimmable;

    const TEST_GRAPH: &str = "
        |t 5 6
        |v 0 0 2
        |v 1 1 3
        |v 2 2 3
        |v 3 1 2
        |v 4 2 2
        |e 0 1
        |e 0 2
        |e 1 2
        |e 1 3
        |e 2 4
        |e 3 4
        |";

    #[test]
    fn test_visited_neighbors() {
        let graph = TEST_GRAPH.trim_margin().unwrap().parse::<Graph>().unwrap();

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
        let data_graph = TEST_GRAPH.trim_margin().unwrap().parse::<Graph>().unwrap();

        let query_graph = "
            |t 3 2
            |v 0 0 1
            |v 1 1 2
            |v 2 2 1
            |e 0 1
            |e 1 2
            |"
        .trim_margin()
        .unwrap()
        .parse::<Graph>()
        .unwrap();

        let candidates = filter::ldf_filter(&data_graph, &query_graph).unwrap();
        assert_eq!(candidates.candidates(0), &[0]);
        assert_eq!(candidates.candidates(1), &[1, 3]);
        assert_eq!(candidates.candidates(2), &[2, 4]);
        let order = order::gql_order(&&data_graph, &&query_graph, &candidates);
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
        let data_graph = TEST_GRAPH.trim_margin().unwrap().parse::<Graph>().unwrap();
        let query_graph = "
            |t 4 4
            |v 0 1 2
            |v 1 2 2
            |v 2 1 2
            |v 3 2 2
            |e 0 1
            |e 0 2
            |e 1 3
            |e 2 3
            |"
        .trim_margin()
        .unwrap()
        .parse::<Graph>()
        .unwrap();

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
            &&query_graph,
            &candidates,
            &order,
            |embedding| embeddings.push(Vec::from(embedding)),
        );

        assert_eq!(embedding_count, 2);
        assert_eq!(embeddings[0], vec![1, 2, 3, 4]);
        assert_eq!(embeddings[1], vec![3, 4, 1, 2]);
    }
}
