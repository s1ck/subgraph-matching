use core::panic;
use graph::input::dotgraph::DotGraph;
use graph::prelude::{Graph as OtherGraph, *};
use graph::UndirectedNodeLabeledCsrGraph;
use std::path::Path;
use std::{
    collections::HashMap, convert::TryFrom, fmt::Display, ops::Deref, str::FromStr, time::Instant,
};

use crate::{Config, Error, Filter};

use linereader::LineReader;

type CsrGraph = UndirectedNodeLabeledCsrGraph<usize, usize>;

pub struct Graph {
    graph: CsrGraph,
    neighbor_label_frequencies: Option<Box<[HashMap<usize, usize>]>>,
}

impl Graph {
    delegate::delegate! {
        to self.graph {
            pub fn node_count(&self) -> usize;
            pub fn edge_count(&self) -> usize;
            pub fn degree(&self, node: usize) -> usize;
            pub fn max_degree(&self) -> usize;
            pub fn label(&self, node: usize) -> usize;
            pub fn neighbors(&self, node: usize) -> &[usize];
            pub fn nodes_by_label(&self, label: usize) -> &[usize];
            pub fn label_count(&self) -> usize;
            pub fn max_label(&self) -> usize;
            pub fn max_label_frequency(&self) -> usize;
        }
    }

    pub fn exists(&self, source: usize, target: usize) -> bool {
        self.neighbors(source).binary_search(&target).is_ok()
    }

    pub fn neighbor_label_frequency(&self, node: usize) -> &HashMap<usize, usize> {
        match &self.neighbor_label_frequencies {
            Some(nlfs) => &nlfs[node],
            None => panic!("Neighbor label frequencies have not been loaded."),
        }
    }
}

impl Display for Graph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "|V|: {}, |E|: {}, |Σ|: {}\nMax Degree: {}, Max Label Frequency: {}",
            self.node_count(),
            self.edge_count(),
            self.label_count(),
            self.max_degree(),
            self.max_label_frequency()
        )
    }
}

impl FromStr for Graph {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Error> {
        let reader = LineReader::new(input.as_bytes());
        let dot_graph: DotGraph<usize, usize> = DotGraph::try_from(reader)?;
        let csr_graph: CsrGraph = CsrGraph::from((dot_graph, CsrLayout::Sorted));

        Ok(Graph::from((
            csr_graph,
            LoadConfig::with_neighbor_label_frequency(),
        )))
    }
}

impl From<(CsrGraph, LoadConfig)> for Graph {
    fn from((graph, load_config): (CsrGraph, LoadConfig)) -> Self {
        let neighbor_label_frequencies = if load_config.neighbor_label_frequency {
            Some(neighbor_label_frequencies(&graph).into_boxed_slice())
        } else {
            None
        };

        Self {
            graph,
            neighbor_label_frequencies,
        }
    }
}

fn neighbor_label_frequencies(graph: &CsrGraph) -> Vec<HashMap<usize, usize>> {
    let mut nlfs = Vec::with_capacity(graph.node_count());

    for node in 0..graph.node_count() {
        let mut nlf = HashMap::<usize, usize>::new();

        for &target in graph.neighbors(node) {
            let target_label = graph.label(target);
            let count = nlf.entry(target_label).or_insert(0);
            *count += 1;
        }

        nlfs.push(nlf);
    }

    nlfs
}

pub struct GdlGraph(Graph);

impl Deref for GdlGraph {
    type Target = Graph;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for GdlGraph {
    type Err = Error;

    fn from_str(gdl: &str) -> Result<Self, Error> {
        let csr_graph: CsrGraph = GraphBuilder::new().gdl_str::<usize, _>(gdl).build()?;
        let graph = Graph::from((csr_graph, LoadConfig::with_neighbor_label_frequency()));
        Ok(GdlGraph(graph))
    }
}

#[derive(Clone, Copy)]
pub struct LoadConfig {
    neighbor_label_frequency: bool,
}

impl LoadConfig {
    pub fn with_neighbor_label_frequency() -> Self {
        Self {
            neighbor_label_frequency: true,
        }
    }
}

impl Default for LoadConfig {
    fn default() -> Self {
        LoadConfig {
            neighbor_label_frequency: false,
        }
    }
}

impl From<Config> for LoadConfig {
    fn from(config: Config) -> Self {
        let neighbor_label_frequency = config.filter == Filter::Nlf;

        LoadConfig {
            neighbor_label_frequency,
        }
    }
}

pub fn load(path: &Path, load_config: LoadConfig) -> Result<Graph, Error> {
    println!("Reading from: {:?}", path);
    let start = Instant::now();
    println!("Preparing input: {:?}", start.elapsed());

    let start = Instant::now();
    let csr_graph: CsrGraph = GraphBuilder::new()
        .csr_layout(CsrLayout::Sorted)
        .file_format(graph::input::dotgraph::DotGraphInput::default())
        .path(path)
        .build()?;
    println!("Parsing graph: {:?}", start.elapsed());

    let start = Instant::now();
    let graph = Graph::from((csr_graph, load_config));
    println!("Building graph: {:?}", start.elapsed());

    Ok(graph)
}

#[cfg(test)]
mod tests {
    use super::*;
    use trim_margin::MarginTrimmable;

    #[test]
    fn read_from_slice() {
        let graph = "
        |t 5 6
        |v 0 0 2
        |v 1 1 3
        |v 2 2 3
        |v 3 1 2
        |v 4 2 2
        |e 0 1
        |e 0 2
        |e 1 2
        |e 1 3
        |e 2 4
        |e 3 4
        |"
        .trim_margin()
        .unwrap();

        let graph = graph.parse::<Graph>().unwrap();

        assert_eq!(graph.node_count(), 5);
        assert_eq!(graph.edge_count(), 6);
        assert_eq!(graph.label_count(), 3);

        assert_eq!(graph.max_label(), 2);
        assert_eq!(graph.max_degree(), 3);
        assert_eq!(graph.max_label_frequency(), 2);

        assert_eq!(graph.label(0), 0);
        assert_eq!(graph.label(1), 1);
        assert_eq!(graph.label(2), 2);
        assert_eq!(graph.label(3), 1);
        assert_eq!(graph.label(4), 2);

        assert_eq!(graph.degree(0), 2);
        assert_eq!(graph.degree(1), 3);
        assert_eq!(graph.degree(2), 3);
        assert_eq!(graph.degree(3), 2);
        assert_eq!(graph.degree(4), 2);

        assert_eq!(graph.neighbors(0), &[1, 2]);
        assert_eq!(graph.neighbors(1), &[0, 2, 3]);
        assert_eq!(graph.neighbors(2), &[0, 1, 4]);
        assert_eq!(graph.neighbors(3), &[1, 4]);
        assert_eq!(graph.neighbors(4), &[2, 3]);

        assert!(graph.exists(0, 1));
        assert!(graph.exists(0, 2));
        assert!(!graph.exists(0, 3));
        assert!(graph.exists(3, 4));
        assert!(!graph.exists(3, 2));

        assert_eq!(graph.nodes_by_label(0), &[0]);
        assert_eq!(graph.nodes_by_label(1), &[1, 3]);
        assert_eq!(graph.nodes_by_label(2), &[2, 4]);
    }

    #[test]
    fn read_from_gdl() {
        let graph = "
        |(n0:L0),
        |(n1:L1),
        |(n2:L2),
        |(n3:L1),
        |(n4:L2),
        |(n0)-->(n1),
        |(n0)-->(n2),
        |(n1)-->(n2),
        |(n1)-->(n3),
        |(n2)-->(n4),
        |(n3)-->(n4)
        |"
        .trim_margin()
        .unwrap()
        .parse::<GdlGraph>()
        .unwrap();

        assert_eq!(graph.node_count(), 5);
        assert_eq!(graph.edge_count(), 6);
        assert_eq!(graph.label_count(), 3);

        assert_eq!(graph.max_label(), 2);
        assert_eq!(graph.max_degree(), 3);
        assert_eq!(graph.max_label_frequency(), 2);

        assert_eq!(graph.label(0), 0);
        assert_eq!(graph.label(1), 1);
        assert_eq!(graph.label(2), 2);
        assert_eq!(graph.label(3), 1);
        assert_eq!(graph.label(4), 2);

        assert_eq!(graph.degree(0), 2);
        assert_eq!(graph.degree(1), 3);
        assert_eq!(graph.degree(2), 3);
        assert_eq!(graph.degree(3), 2);
        assert_eq!(graph.degree(4), 2);

        assert_eq!(graph.neighbors(0), &[1, 2]);
        assert_eq!(graph.neighbors(1), &[0, 2, 3]);
        assert_eq!(graph.neighbors(2), &[0, 1, 4]);
        assert_eq!(graph.neighbors(3), &[1, 4]);
        assert_eq!(graph.neighbors(4), &[2, 3]);

        assert!(graph.exists(0, 1));
        assert!(graph.exists(0, 2));
        assert!(!graph.exists(0, 3));
        assert!(graph.exists(3, 4));
        assert!(!graph.exists(3, 2));

        assert_eq!(graph.nodes_by_label(0), &[0]);
        assert_eq!(graph.nodes_by_label(1), &[1, 3]);
        assert_eq!(graph.nodes_by_label(2), &[2, 4]);
    }

    #[test]
    fn neighbor_label_frequencies() {
        let graph = "
        |(n0:L0),
        |(n1:L1),
        |(n2:L2),
        |(n3:L1),
        |(n4:L2),
        |(n0)-->(n1),
        |(n0)-->(n2),
        |(n0)-->(n4),
        |(n1)-->(n2),
        |(n1)-->(n3),
        |(n2)-->(n4),
        |(n3)-->(n4)
        |"
        .trim_margin()
        .unwrap()
        .parse::<GdlGraph>()
        .unwrap();

        assert_eq!(graph.neighbor_label_frequency(0).get(&0), None);
        assert_eq!(graph.neighbor_label_frequency(0).get(&1), Some(&1));
        assert_eq!(graph.neighbor_label_frequency(0).get(&2), Some(&2));
        assert_eq!(graph.neighbor_label_frequency(4).get(&2), Some(&1));
        assert_eq!(graph.neighbor_label_frequency(4).get(&1), Some(&1));
        assert_eq!(graph.neighbor_label_frequency(4).get(&4), None);
    }
}
