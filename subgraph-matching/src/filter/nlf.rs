use cfg_if::cfg_if;

use crate::Graph;

use super::Candidates;

pub fn nlf_filter(data_graph: &Graph, query_graph: &Graph) -> Option<Candidates> {
    cfg_if! {
        if #[cfg(feature = "neighbor-label-frequency")] {
            nlf_filter_(data_graph, query_graph)
        } else {
            super::ldf_filter(data_graph, query_graph)
        }
    }
}

#[cfg(feature = "neighbor-label-frequency")]
fn nlf_filter_(data_graph: &Graph, query_graph: &Graph) -> Option<Candidates> {
    let mut candidates = Candidates::from((data_graph, query_graph));

    for query_node in 0..query_graph.node_count() {
        let label = query_graph.label(query_node);
        let degree = query_graph.degree(query_node);
        let query_nlf = query_graph.neighbor_label_frequency(query_node);

        for &data_node in data_graph.nodes_by_label(label) {
            if data_graph.degree(data_node) >= degree {
                let data_nlf = data_graph.neighbor_label_frequency(data_node);

                if data_nlf.len() >= query_nlf.len() {
                    let mut is_valid = true;

                    for (query_label, query_label_count) in query_nlf.iter() {
                        is_valid = match data_nlf.get(query_label) {
                            Some(data_label_count) if data_label_count >= query_label_count => true,
                            _ => false,
                        };
                    }

                    if is_valid {
                        candidates.add_candidate(query_node, data_node);
                    }
                }
            }
        }

        if candidates.candidate_count(query_node) == 0 {
            return None;
        }
    }

    Some(candidates)
}
