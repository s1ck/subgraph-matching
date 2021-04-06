use crate::graph::Graph;

// label-and-degree filtering
fn ldf_filter(data_graph: &Graph, query_graph: &Graph) -> Option<Candidates> {
    let mut candidates = Candidates::new(data_graph, query_graph);

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

struct Candidates {
    /// candidates for each query node
    candidates: Box<[Vec<usize>]>,
    // number of candidates for each query node
    candidates_count: Vec<usize>,
}

impl Candidates {
    fn new(data_graph: &Graph, query_graph: &Graph) -> Self {
        let query_node_count = query_graph.node_count();
        let max_candidates = data_graph.max_label_frequency();

        let mut candidates_count = Vec::with_capacity(query_node_count);
        candidates_count.resize(query_node_count, 0);

        let mut candidates = Vec::with_capacity(query_node_count);

        for _ in 0..query_node_count {
            candidates.push(Vec::<usize>::with_capacity(max_candidates));
        }

        Self {
            candidates: candidates.into_boxed_slice(),
            candidates_count,
        }
    }

    fn is_valid(&self) -> bool {
        let query_node_count = self.candidates_count.len();

        for i in 0..query_node_count {
            if self.candidates_count[i] == 0 {
                return false;
            }
        }
        return true;
    }

    fn add_candidate(&mut self, query_node: usize, data_node: usize) {
        self.candidates[query_node].push(data_node);
        self.candidates_count[query_node] += 1;
    }

    fn candidate_count(&self, query_node: usize) -> usize {
        self.candidates_count[query_node]
    }
}
