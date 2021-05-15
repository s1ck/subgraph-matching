/*!
## Suma (Subgraph Matching)

A command-line utility for finding patterns in graphs.

This project is inspired by https://github.com/RapidsAtHKUST/SubgraphMatching, which is written in C++.
The corresponding [paper](https://dl.acm.org/doi/10.1145/3318464.3380581) was published at SIGMOD 2020.

### License

MIT
*/
#![allow(dead_code)]
use subgraph_matching::{
    enumerate, filter,
    graph::{self, LoadConfig},
    order, Filter,
};

use std::time::Instant;

use eyre::Result;

fn main() -> Result<()> {
    let args = cli::main()?;

    let load_config = if args.filter == Filter::Nlf {
        LoadConfig::with_neighbor_label_frequency()
    } else {
        LoadConfig::default()
    };

    let loading = Instant::now();
    let total = Instant::now();

    println!("------");
    let query_graph = measure("Load query graph", || {
        graph::load(&args.query_graph, load_config)
    })?;
    println!("------");
    let data_graph = measure("Load data graph", || {
        graph::load(&args.data_graph, load_config)
    })?;
    println!("------");

    let loading = loading.elapsed();
    let matching = Instant::now();

    println!("Query Graph Meta Information:\n{}", query_graph);
    println!("Data Graph Meta Information:\n{}", data_graph);
    println!("------");

    let candidates = measure("Filter candidates", || {
        let mut candidates = match args.filter {
            Filter::Ldf => filter::ldf_filter(&data_graph, &query_graph).unwrap_or_default(),
            Filter::Gql => filter::gql_filter(&data_graph, &query_graph).unwrap_or_default(),
            Filter::Nlf => filter::nlf_filter(&data_graph, &query_graph).unwrap_or_default(),
        };
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

    println!("Loading time = {:?}", loading);
    println!("Matching time = {:?}", matching.elapsed());
    println!("Total runtime = {:?}", total.elapsed());

    Ok(())
}

fn measure<R>(desc: &str, func: impl FnOnce() -> R) -> R {
    println!("Start :: {}", desc);
    let start = Instant::now();
    let result = func();
    println!("Finish :: {} took {:?}", desc, start.elapsed());
    result
}

mod cli {
    use pico_args::Arguments;
    use std::{ffi::OsStr, path::PathBuf, str::FromStr};
    use subgraph_matching::Filter;

    use crate::Result;

    #[derive(Debug)]
    pub(crate) struct AppArgs {
        pub(crate) query_graph: std::path::PathBuf,
        pub(crate) data_graph: std::path::PathBuf,
        pub(crate) filter: subgraph_matching::Filter,
    }

    pub(crate) fn main() -> Result<AppArgs> {
        let mut pargs = Arguments::from_env();

        fn as_path_buf(arg: &OsStr) -> Result<PathBuf> {
            Ok(arg.into())
        }

        let args = AppArgs {
            query_graph: pargs.value_from_os_str(["-q", "--query-graph"], as_path_buf)?,
            data_graph: pargs.value_from_os_str(["-d", "--data-graph"], as_path_buf)?,
            filter: pargs
                .opt_value_from_fn(["-f", "--filter"], FilterWrapper::from_str)?
                .unwrap_or(FilterWrapper(Filter::Ldf))
                .into(),
        };

        Ok(args)
    }

    struct FilterWrapper(Filter);

    impl From<FilterWrapper> for Filter {
        fn from(f: FilterWrapper) -> Self {
            f.0
        }
    }

    impl FromStr for FilterWrapper {
        type Err = eyre::Report;

        fn from_str(s: &str) -> Result<FilterWrapper> {
            match s {
                "LDF" | "ldf" => Ok(FilterWrapper(Filter::Ldf)),
                "GQL" | "gql" => Ok(FilterWrapper(Filter::Gql)),
                "NLF" | "nlf" => Ok(FilterWrapper(Filter::Nlf)),
                _ => Err(eyre::eyre!("Unsupported filter {}", s)),
            }
        }
    }
}
