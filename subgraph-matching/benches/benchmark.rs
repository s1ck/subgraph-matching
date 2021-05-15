use std::path::PathBuf;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use subgraph_matching::{
    find,
    graph::{load, Graph, LoadConfig},
    Config, Enumeration, Filter, Order,
};

const CRATE_ROOT: &str = env!("CARGO_MANIFEST_DIR");
const HPRD_PATH: &[&str] = &[CRATE_ROOT, "resources", "data_graph", "HPRD.graph"];
const QUERY_PATH: &[&str] = &[
    CRATE_ROOT,
    "resources",
    "query_graph",
    "query_dense_16_2.graph",
];

fn graphs(load_config: LoadConfig) -> (Graph, Graph) {
    let data_graph = load(&HPRD_PATH.iter().collect::<PathBuf>(), load_config).unwrap();
    let query_graph = load(&QUERY_PATH.iter().collect::<PathBuf>(), load_config).unwrap();
    (data_graph, query_graph)
}

fn run_find(data_graph: &Graph, query_graph: &Graph, config: Config) -> usize {
    let embedding_count = find(data_graph, query_graph, config);
    black_box(embedding_count)
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let graphs = graphs(LoadConfig::with_neighbor_label_frequency());

    let mut group = c.benchmark_group("find");

    for filter in vec![Filter::Ldf, Filter::Gql, Filter::Nlf] {
        for order in vec![Order::Gql] {
            for enumeration in vec![Enumeration::Gql] {
                let config = Config {
                    filter,
                    order,
                    enumeration,
                };

                group.bench_with_input(
                    BenchmarkId::from_parameter(config),
                    &(&graphs, config),
                    |b, ((data_graph, query_graph), config)| {
                        b.iter(|| run_find(data_graph, query_graph, *config));
                    },
                );
            }
        }
    }
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
