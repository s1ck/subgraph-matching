use crate::graph::Graph;

// label-and-degree filtering
pub(crate) fn ldf_filter(data_graph: &Graph, query_graph: &Graph) -> Option<Candidates> {
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

    pub fn candidates(&self, data_node: usize) -> &[usize] {
        self.candidates[data_node].as_slice()
    }

    pub fn candidate_count(&self, query_node: usize) -> usize {
        self.candidates[query_node].len()
    }

    pub fn sort(&mut self) {
        for c in self.candidates.iter_mut() {
            c.sort()
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use trim_margin::MarginTrimmable;

    const TEST_GRAPH: &str = "
        |t 5 6
        |v 0 0 2
        |v 1 1 3
        |v 2 2 3
        |v 3 1 2
        |v 4 4 2
        |e 0 1
        |e 0 2
        |e 1 2
        |e 1 3
        |e 2 4
        |e 3 4
        |";

    #[test]
    fn test_ldf_filter() {
        let query_graph = "
        |t 3 2
        |v 0 0 1
        |v 1 1 2
        |v 2 2 1
        |e 0 1
        |e 1 2
        |"
        .trim_margin()
        .unwrap();

        let data_graph = TEST_GRAPH.trim_margin().unwrap().parse::<Graph>().unwrap();
        let query_graph = query_graph.parse::<Graph>().unwrap();

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
        let query_graph = "
        |t 3 2
        |v 0 3 1
        |v 1 1 2
        |v 2 2 1
        |e 0 1
        |e 1 2
        |"
        .trim_margin()
        .unwrap();

        let data_graph = TEST_GRAPH.trim_margin().unwrap().parse::<Graph>().unwrap();
        let query_graph = query_graph.parse::<Graph>().unwrap();

        let candidates = ldf_filter(&data_graph, &query_graph);

        assert!(candidates.is_none())
    }

    #[test]
    fn test_ldf_filter_invalid_degree() {
        let query_graph = "
        |t 3 4
        |v 0 3 3
        |v 1 1 2
        |v 2 2 1
        |e 0 1
        |e 0 2
        |e 0 2
        |e 1 2
        |"
        .trim_margin()
        .unwrap();

        let data_graph = TEST_GRAPH.trim_margin().unwrap().parse::<Graph>().unwrap();
        let query_graph = query_graph.parse::<Graph>().unwrap();

        let candidates = ldf_filter(&data_graph, &query_graph);

        assert!(candidates.is_none())
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
