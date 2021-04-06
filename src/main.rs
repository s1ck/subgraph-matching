#![allow(dead_code)]
mod cli;
mod graph;

use eyre::Result;

fn main() -> Result<()> {
    let args = cli::main()?;

    // Load input graphs
    let _query_graph = graph::parse(args.query_graph)?;
    let _data_graph = graph::parse(args.data_graph)?;

    Ok(())
}
