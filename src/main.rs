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

use eyre::Result;

fn main() -> Result<()> {
    let args = cli::main()?;

    // Load input graphs

    println!("Load graphs...");

    let query_graph = graph::parse(args.query_graph)?;
    let data_graph = graph::parse(args.data_graph)?;

    println!("Query Graph Meta Information:\n{}", query_graph);
    println!("Data Graph Meta Information:\n{}", data_graph);

    println!("------");
    println!("Filter candidates...");

    let mut candidates = filter::ldf_filter(&data_graph, &query_graph).unwrap_or_default();
    // sorting candidates to support set intersection
    candidates.sort();
    println!("candidate counts: {} ", candidates);

    println!("------");
    println!("Generate a matching order...");

    let order = order::gql_order(&data_graph, &query_graph, &candidates);
    println!("matching order: {:?}", order);

    println!("------");
    println!("Enumerate");

    let embedding_count = enumerate::gql(&data_graph, &query_graph, &candidates, &order);

    println!("Embedding count = {}", embedding_count);

    Ok(())
}
