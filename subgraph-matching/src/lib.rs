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
    let mut candidates = filter::ldf_filter(&data_graph, &query_graph).unwrap_or_default();
    candidates.sort();
    let order = order::gql_order(&data_graph, &query_graph, &candidates);
    enumerate::gql(&data_graph, &query_graph, &candidates, &order)
}
