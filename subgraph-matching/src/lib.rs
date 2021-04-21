/*!
## Subgraph Matching

A library for finding patterns in graphs.

This is work in progress and unstable.

This project is inspired by https://github.com/RapidsAtHKUST/SubgraphMatching, which is written in C++.
The corresponding [paper](https://dl.acm.org/doi/10.1145/3318464.3380581) was published at SIGMOD 2020.

### License

MIT
*/
#![allow(dead_code)]
pub mod enumerate;
pub mod filter;
pub mod graph;
pub mod order;

use std::io;

use graph::Graph;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("error while parsing graph file")]
    ParseGraph {
        #[from]
        source: io::Error,
    },
    #[error("error while parsing GDL graph")]
    ParseGdlGraph {
        #[from]
        source: gdl::graph::GraphHandlerError,
    },
}

pub fn find(data_graph: &Graph, query_graph: &Graph) -> usize {
    find_with(data_graph, query_graph, |_| {})
}

pub fn find_with<F>(data_graph: &Graph, query_graph: &Graph, action: F) -> usize
where
    F: FnMut(&[usize]),
{
    let mut candidates = filter::ldf_filter(&data_graph, &query_graph).unwrap_or_default();
    candidates.sort();
    let order = order::gql_order(&data_graph, &query_graph, &candidates);
    enumerate::gql_with(&data_graph, &query_graph, &candidates, &order, action)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::GdlGraph;
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
    fn test_find() {
        let data_graph = graph(TEST_GRAPH);
        let query_graph = graph(
            "
            |(n0:L2),(n1:L1),(n2:L1)
            |(n0)-->(n1)
            |(n1)-->(n2)
            |",
        );

        assert_eq!(find(&data_graph, &query_graph), 2)
    }

    #[test]
    fn test_find_with() {
        let data_graph = graph(TEST_GRAPH);
        let query_graph = graph(
            "
            |(n0:L2),(n1:L1),(n2:L1)
            |(n0)-->(n1)
            |(n1)-->(n2)
            |",
        );

        let mut embeddings = Vec::new();
        let count = find_with(&data_graph, &query_graph, |embedding| {
            embeddings.push(Vec::from(embedding))
        });

        assert_eq!(count, 2);
        assert_eq!(embeddings[0], vec![2, 1, 3]);
        assert_eq!(embeddings[1], vec![4, 3, 1])
    }
}
