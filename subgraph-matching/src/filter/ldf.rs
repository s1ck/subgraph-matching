use crate::graph::Graph;

use super::Candidates;

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
}
