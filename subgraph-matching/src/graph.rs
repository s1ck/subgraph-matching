use core::panic;
use graph_builder::prelude::dotgraph::{
    LabelStats, NeighborLabelFrequencies, NeighborLabelFrequency, NodeLabelIndex,
};
use graph_builder::prelude::{Graph as OtherGraph, *};
use std::path::Path;
use std::{convert::TryFrom, fmt::Display, ops::Deref, str::FromStr, time::Instant};

use crate::{Config, Error, Filter};

use linereader::LineReader;

type Id = usize;
type Label = usize;

type CsrGraph = UndirectedCsrGraph<Id, Label>;

pub struct Graph {
    inner: CsrGraph,
    label_count: usize,
    max_degree: Id,
    max_label: Label,
    max_label_frequency: usize,
    neighbor_label_frequencies: Option<NeighborLabelFrequencies<Label, Id>>,
    node_label_index: Option<NodeLabelIndex<Label, Id>>,
}

impl Graph {
    delegate::delegate! {
        to self.inner {
            pub fn node_count(&self) -> Id;
            pub fn edge_count(&self) -> Id;
            pub fn degree(&self, node: Id) -> Id;
        }
    }

    pub fn neighbors(&self, node: Id) -> &[Id] {
        self.inner.neighbors(node).as_slice()
    }

    pub fn label(&self, node: Id) -> Label {
        *self.inner.node_value(node)
    }

    pub fn max_degree(&self) -> Id {
        self.max_degree
    }

    pub fn label_count(&self) -> usize {
        self.label_count
    }

    pub fn max_label(&self) -> Label {
        self.max_label
    }

    pub fn max_label_frequency(&self) -> usize {
        self.max_label_frequency
    }

    pub fn exists(&self, source: usize, target: usize) -> bool {
        self.neighbors(source).binary_search(&target).is_ok()
    }

    pub fn neighbor_label_frequency(&self, node: Id) -> NeighborLabelFrequency<Label> {
        match &self.neighbor_label_frequencies {
            Some(nlfs) => nlfs.neighbor_frequency(node),
            None => panic!("Neighbor label frequencies have not been loaded."),
        }
    }

    pub fn nodes_by_label(&self, label: usize) -> &[Id] {
        match &self.node_label_index {
            Some(index) => index.nodes(label),
            None => panic!("Node label index has not been loaded."),
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
        let dot_graph: DotGraph<Id, Label> = DotGraph::try_from(reader)?;

        let node_count = dot_graph.node_count();
        let label_count = dot_graph.label_count();
        let max_label_frequency = dot_graph.max_label_frequency();

        let DotGraph {
            labels,
            edge_list,
            max_degree,
            max_label,
            label_frequency,
        } = dot_graph;

        let label_stats = LabelStats {
            max_degree,
            label_count,
            max_label,
            max_label_frequency,
            label_frequency,
        };

        let label_index = NodeLabelIndex::from_stats(node_count, label_stats, |node| labels[node]);
        let node_values = graph_builder::graph::csr::NodeValues::new(labels);
        let graph = UndirectedCsrGraph::from((node_values, edge_list, CsrLayout::Sorted));
        let frequencies = NeighborLabelFrequencies::from_graph(&graph);

        let graph = Graph {
            inner: graph,
            label_count,
            max_degree,
            max_label,
            max_label_frequency,
            neighbor_label_frequencies: Some(frequencies),
            node_label_index: Some(label_index),
        };

        Ok(graph)
    }
}

impl From<(CsrGraph, LoadConfig)> for Graph {
    fn from((inner, load_config): (CsrGraph, LoadConfig)) -> Self {
        let neighbor_label_frequencies = if load_config.neighbor_label_frequency {
            Some(NeighborLabelFrequencies::from_graph(&inner))
        } else {
            None
        };

        let label_stats @ LabelStats {
            max_degree,
            label_count,
            max_label,
            max_label_frequency,
            ..
        } = LabelStats::from_graph(&inner);

        let node_count = inner.node_count();
        let label_func = |node| *inner.node_value(node);
        let node_label_index = NodeLabelIndex::from_stats(node_count, label_stats, label_func);

        Self {
            inner,
            label_count,
            max_degree,
            max_label,
            max_label_frequency,
            neighbor_label_frequencies,
            node_label_index: Some(node_label_index),
        }
    }
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
        let csr_graph: CsrGraph = GraphBuilder::new()
            .csr_layout(CsrLayout::Sorted)
            .gdl_str::<Id, _>(gdl)
            .build()?;

        let graph = Graph::from((csr_graph, LoadConfig::with_neighbor_label_frequency()));

        Ok(GdlGraph(graph))
    }
}

#[derive(Clone, Copy, Default)]
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
        .file_format(dotgraph::DotGraphInput::default())
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
        |(n0 { label: 0 }),
        |(n1 { label: 1 }),
        |(n2 { label: 2 }),
        |(n3 { label: 1 }),
        |(n4 { label: 2 }),
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
        |(n0 { label: 0 }),
        |(n1 { label: 1 }),
        |(n2 { label: 2 }),
        |(n3 { label: 1 }),
        |(n4 { label: 2 }),
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

        assert_eq!(graph.neighbor_label_frequency(0).get(0), None);
        assert_eq!(graph.neighbor_label_frequency(0).get(1), Some(1));
        assert_eq!(graph.neighbor_label_frequency(0).get(2), Some(2));
        assert_eq!(graph.neighbor_label_frequency(4).get(2), Some(1));
        assert_eq!(graph.neighbor_label_frequency(4).get(1), Some(1));
        assert_eq!(graph.neighbor_label_frequency(4).get(4), None);
    }
}
