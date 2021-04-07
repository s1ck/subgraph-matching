#![allow(dead_code)]
mod cli;
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

    println!("------");
    println!("Generate a matching order...");

    let _order = order::gql_order(&data_graph, &query_graph, &candidates);

    println!("------");

    Ok(())
}
