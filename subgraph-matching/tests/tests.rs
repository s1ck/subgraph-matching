use std::{collections::HashMap, path::PathBuf};
use subgraph_matching::{
    find,
    graph::{parse, Graph},
    Config, Enumeration, Filter, Order,
};

const CRATE_ROOT: &str = env!("CARGO_MANIFEST_DIR");
const HPRD_PATH: &[&str] = &[CRATE_ROOT, "resources", "data_graph", "HPRD.graph"];
const QUERY_PATH: &[&str] = &[CRATE_ROOT, "resources", "query_graph"];
const EXPECTED_COUNTS: &[&str] = &[CRATE_ROOT, "resources", "expected_output.res"];

fn data_graph() -> Graph {
    parse(&HPRD_PATH.iter().collect::<PathBuf>()).unwrap()
}

fn query_graphs() -> impl Iterator<Item = (String, Graph)> {
    let path = QUERY_PATH.iter().collect::<PathBuf>();
    std::fs::read_dir(path)
        .unwrap()
        .map(|path| path.unwrap())
        .map(|path| {
            (
                path.file_name()
                    .into_string()
                    .unwrap()
                    .split(".graph")
                    .next()
                    .unwrap()
                    .to_string(),
                parse(&path.path()).unwrap(),
            )
        })
}

fn expected_counts() -> HashMap<String, usize> {
    let path = EXPECTED_COUNTS.iter().collect::<PathBuf>();
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| line.split(":"))
        .map(|mut split| {
            (
                split.next().unwrap().to_string(),
                split.next().unwrap().parse::<usize>().unwrap(),
            )
        })
        .collect::<HashMap<_, _>>()
}

#[test]
fn filter_ldf_order_gql_enumeration_gql() {
    assert_expected_counts(Config::new(Filter::Ldf, Order::Gql, Enumeration::Gql))
}

#[test]
fn filter_gql_order_gql_enumeration_gql() {
    assert_expected_counts(Config::new(Filter::Gql, Order::Gql, Enumeration::Gql))
}
#[test]
fn filter_nlf_order_gql_enumeration_gql() {
    assert_expected_counts(Config::new(Filter::Nlf, Order::Gql, Enumeration::Gql))
}

fn assert_expected_counts(config: Config) {
    let data_graph = data_graph();
    let expected_counts = expected_counts();

    assert_eq!(data_graph.node_count(), 9460);
    assert_eq!(data_graph.relationship_count(), 34998);

    for (query_file, query_graph) in query_graphs() {
        let actual_count = find(&data_graph, &query_graph, config);
        let expected_count = expected_counts.get(&query_file).unwrap();
        assert_eq!(actual_count, *expected_count)
    }
}
