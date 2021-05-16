use crate::Graph;

/// The k-core of a graph is a maximal subgraph in which
/// each node has at least degree k. The coreness of a
/// node is the highest order of a k-core containing the
/// node, i.e., node `u` has coreness `c` if it belongs
/// to a `c-core` but not to any `(c + 1)-core`.
///
/// The implementation is based on the algorithm presented in
///
/// Vladimir Batagelj, Matjaz Zaversnik:
/// An O(m) Algorithm for Cores Decomposition of Networks.
pub fn coreness(graph: &Graph) -> Vec<usize> {
    let node_count = graph.node_count();
    let max_degree = graph.max_degree();

    let mut core_table = vec![0; node_count];

    // nodes sorted by degree
    let mut nodes = vec![0_usize; node_count];
    // position of nodes in nodes array
    let mut position = vec![0_usize; node_count];

    // degree histogram
    let mut degree_hist = vec![0; max_degree + 1];

    // compute histogram
    for (node, degree) in core_table.iter_mut().enumerate() {
        *degree = graph.degree(node);
        degree_hist[*degree] += 1;
    }

    // compute offsets from histogram
    let mut offset = 0;
    for count in degree_hist.iter_mut() {
        let temp = *count;
        *count = offset;
        offset += temp;
    }

    // sort nodes by degree (corrupts histogram)
    for node in 0..node_count {
        let degree = graph.degree(node);
        position[node] = degree_hist[degree];
        nodes[position[node]] = node;
        degree_hist[degree] += 1;
    }

    // correct histogram
    for degree in (1..=max_degree).rev() {
        degree_hist[degree] = degree_hist[degree - 1];
    }
    degree_hist[0] = 0;

    for i in 0..node_count {
        let u = nodes[i];
        for &v in graph.neighbors(u) {
            if core_table[v] > core_table[u] {
                // Get the first node with the same degree at the
                // beginning of the offset in the nodes array.
                let degree_v = core_table[v];
                let position_v = position[v];
                let position_w = degree_hist[degree_v];
                let w = nodes[position_w];

                if v != w {
                    // swap u and w
                    position[v] = position_w;
                    position[w] = position_v;
                    nodes[position_v] = w;
                    nodes[position_w] = v;
                }

                degree_hist[degree_v] += 1;
                core_table[v] -= 1;
            }
        }
    }

    core_table
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::GdlGraph;
    use trim_margin::MarginTrimmable;

    #[test]
    fn test_coreness() {
        // d(n0) = 1
        // d(n1) = 4
        // d(n2) = 3
        // d(n3) = 2
        // d(n4) = 4
        let graph = "
            |(n0:L0)
            |(n1:L0)
            |(n2:L0)
            |(n3:L0)
            |(n4:L0)
            |(n0)-->(n1)
            |(n1)-->(n2)
            |(n1)-->(n3)
            |(n2)-->(n4)
            |(n3)-->(n4)
            |(n4)-->(n1)
            |(n4)-->(n2)
            |"
        .trim_margin()
        .unwrap()
        .parse::<GdlGraph>()
        .unwrap();

        let core_table = coreness(&graph);

        assert_eq!(core_table, vec![1, 2, 2, 2, 2])
    }
}
