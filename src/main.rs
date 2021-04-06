#![allow(dead_code)]
mod cli;
mod filter;
mod graph;

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

    Ok(())
}
