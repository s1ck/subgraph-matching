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
mod cli;
mod enumerate;
mod filter;
mod graph;
mod order;

use std::time::Instant;

use eyre::Result;

fn main() -> Result<()> {
    let args = cli::main()?;

    println!("------");
    let query_graph = measure("Load query graph", || graph::parse(&args.query_graph))?;
    println!("------");
    let data_graph = measure("Load data graph", || graph::parse(&args.data_graph))?;
    println!("------");

    println!("Query Graph Meta Information:\n{}", query_graph);
    println!("Data Graph Meta Information:\n{}", data_graph);
    println!("------");

    let candidates = measure("Filter candidates", || {
        let mut candidates = filter::ldf_filter(&data_graph, &query_graph).unwrap_or_default();
        // sorting candidates to support set intersection
        candidates.sort();
        candidates
    });
    println!("Candidate counts: {} ", candidates);
    println!("------");

    let order = measure("Generate matching order", || {
        order::gql_order(&data_graph, &query_graph, &candidates)
    });
    println!("Matching order: {:?}", order);
    println!("------");

    let embedding_count = measure("Enumerate", || {
        enumerate::gql(&data_graph, &query_graph, &candidates, &order)
    });
    println!("Embedding count = {}", embedding_count);
    println!("------");

    Ok(())
}
fn measure<R>(desc: &str, func: impl FnOnce() -> R) -> R {
    println!("Start :: {}", desc);
    let start = Instant::now();
    let result = func();
    println!("Finish :: {} took {:?}", desc, start.elapsed());
    result
}
