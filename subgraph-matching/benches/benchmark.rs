use std::path::PathBuf;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use subgraph_matching::{
    find,
    graph::{parse, Graph},
    Filter,
};

const CRATE_ROOT: &str = env!("CARGO_MANIFEST_DIR");
const HPRD_PATH: &[&str] = &[CRATE_ROOT, "resources", "data_graph", "HPRD.graph"];
const QUERY_PATH: &[&str] = &[
    CRATE_ROOT,
    "resources",
    "query_graph",
    "query_dense_16_2.graph",
];

fn graphs() -> (Graph, Graph) {
    let data_graph = parse(&HPRD_PATH.iter().collect::<PathBuf>()).unwrap();
    let query_graph = parse(&QUERY_PATH.iter().collect::<PathBuf>()).unwrap();
    (data_graph, query_graph)
}

fn find_with_filter(data_graph: &Graph, query_graph: &Graph, filter: Filter) -> usize {
    let embedding_count = find(data_graph, query_graph, filter);
    black_box(embedding_count)
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let graphs = graphs();

    let mut group = c.benchmark_group("find_with_filter");
    for filter in &[Filter::LDF, Filter::GQL] {
        group.bench_with_input(
            BenchmarkId::from_parameter(filter),
            &(&graphs, filter),
            |b, ((data_graph, query_graph), &filter)| {
                b.iter(|| find_with_filter(data_graph, query_graph, filter));
            },
        );
    }
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
