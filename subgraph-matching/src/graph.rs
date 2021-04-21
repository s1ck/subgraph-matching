use atoi::FromRadix10;
use std::{
    collections::HashMap, convert::TryFrom, fmt::Display, fs::File, io::Read, ops::Deref,
    str::FromStr, time::Instant,
};
use std::{fmt::Write, path::Path};

use crate::Error;

use linereader::LineReader;

pub struct Graph {
    node_count: usize,
    relationship_count: usize,
    label_count: usize,
    labels: Box<[usize]>,
    offsets: Box<[usize]>,
    neighbors: Box<[usize]>,
    reverse_index: Box<[usize]>,
    reverse_index_offsets: Box<[usize]>,
    max_degree: usize,
    max_label: usize,
    max_label_frequency: usize,
    label_frequency: HashMap<usize, usize>,
}

impl Graph {
    pub fn node_count(&self) -> usize {
        self.node_count
    }

    pub fn relationship_count(&self) -> usize {
        self.relationship_count
    }

    pub fn degree(&self, node: usize) -> usize {
        self.offsets[node + 1] - self.offsets[node]
    }

    pub fn label(&self, node: usize) -> usize {
        self.labels[node]
    }

    pub fn neighbors(&self, node: usize) -> &[usize] {
        let from = self.offsets[node];
        let to = self.offsets[node + 1];
        &self.neighbors[from..to]
    }

    pub fn exists(&self, source: usize, target: usize) -> bool {
        self.neighbors(source).binary_search(&target).is_ok()
    }

    pub fn nodes_by_label(&self, label: usize) -> &[usize] {
        let from = self.reverse_index_offsets[label];
        let to = self.reverse_index_offsets[label + 1];
        &self.reverse_index[from..to]
    }

    pub fn label_count(&self) -> usize {
        self.label_count
    }

    pub fn max_degree(&self) -> usize {
        self.max_degree
    }

    pub fn max_label(&self) -> usize {
        self.max_label
    }

    pub fn max_label_frequency(&self) -> usize {
        self.max_label_frequency
    }
}

impl Display for Graph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "|V|: {}, |E|: {}, |Σ|: {}\nMax Degree: {}, Max Label Frequency: {}",
            self.node_count,
            self.relationship_count,
            self.label_count,
            self.max_degree,
            self.max_label_frequency
        )
    }
}

impl FromStr for Graph {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Error> {
        let reader = LineReader::new(input.as_bytes());
        let parse_graph = ParseGraph::try_from(reader)?;
        Ok(Graph::from(parse_graph))
    }
}

struct ParseGraph {
    node_count: usize,
    relationship_count: usize,
    labels: Vec<usize>,
    offsets: Vec<usize>,
    neighbors: Vec<usize>,
    max_degree: usize,
    max_label: usize,
    label_frequency: HashMap<usize, usize>,
}

impl<R> TryFrom<LineReader<R>> for ParseGraph
where
    R: Read,
{
    type Error = Error;

    fn try_from(mut lines: LineReader<R>) -> Result<Self, Error> {
        let mut header = lines.next_line().expect("missing header line")?;

        // skip "t" char and white space
        header = &header[2..];
        let (node_count, used) = usize::from_radix_10(header);
        header = &header[used + 1..];
        let (relationship_count, _) = usize::from_radix_10(&header);

        let mut labels = Vec::<usize>::with_capacity(node_count);
        let mut offsets = Vec::<usize>::with_capacity(node_count + 1);
        // undirected
        let mut neighbors = vec![0; relationship_count * 2];

        offsets.push(0);

        let mut max_degree = 0;
        let mut max_label = 0;
        let mut label_frequency = HashMap::<usize, usize>::new();

        let mut batch = lines.next_batch().expect("missing data")?;

        // read nodes
        //
        // Unlike the C++ impl, this assumes the
        // input to be sorted by node id
        while offsets.len() <= node_count {
            if batch.len() == 0 {
                batch = lines.next_batch().expect("missing data")?;
            }

            // skip "v" char and white space
            batch = &batch[2..];
            // skip node id since input is always sorted by node id
            let (_, used) = usize::from_radix_10(batch);
            batch = &batch[used + 1..];
            let (label, used) = usize::from_radix_10(batch);
            batch = &batch[used + 1..];
            let (degree, used) = usize::from_radix_10(batch);
            batch = &batch[used + 1..];

            labels.push(label);
            offsets.push(offsets[offsets.len() - 1] + degree);

            if degree > max_degree {
                max_degree = degree;
            }

            let frequency = label_frequency.entry(label).or_insert_with(|| {
                if label > max_label {
                    max_label = label;
                }
                0
            });
            *frequency += 1;
        }

        // stores the next offset to insert for each node
        let mut next_offset = vec![0; node_count];

        // read (undirected) relationships
        //
        // Unlike the C++ impl this assumes the
        // input to be sorted by source and target
        for _ in 0..relationship_count {
            if batch.len() == 0 {
                batch = lines.next_batch().expect("missing data")?;
            }
            // skip "e" char and white space
            batch = &batch[2..];
            let (source, used) = usize::from_radix_10(batch);
            batch = &batch[used + 1..];
            let (target, used) = usize::from_radix_10(batch);
            batch = &batch[used + 1..];

            // add as outgoing to source adjacency list
            let offset = offsets[source] + next_offset[source];
            neighbors[offset] = target;

            // add as incoming to target adjacency list
            let offset = offsets[target] + next_offset[target];
            neighbors[offset] = source;

            next_offset[source] += 1;
            next_offset[target] += 1;
        }

        Ok(Self {
            node_count,
            relationship_count,
            labels,
            offsets,
            neighbors,
            max_degree,
            max_label,
            label_frequency,
        })
    }
}

impl From<ParseGraph> for Graph {
    fn from(
        ParseGraph {
            node_count,
            relationship_count,
            labels,
            offsets,
            mut neighbors,
            max_degree,
            max_label,
            label_frequency,
        }: ParseGraph,
    ) -> Self {
        // sort adjacency lists
        for node in 0..node_count {
            let from = offsets[node];
            let to = offsets[node + 1];
            neighbors[from..to].sort();
        }

        let label_count = if label_frequency.len() > max_label + 1 {
            label_frequency.len()
        } else {
            max_label + 1
        };

        let max_label_frequency = label_frequency.values().max().unwrap_or(&0).clone();

        // reverse label index
        let mut reverse_index = vec![0; node_count];
        let mut reverse_index_offsets = Vec::<usize>::with_capacity(label_count + 1);
        reverse_index_offsets.push(0);

        let mut total = 0;

        for label in 0..label_count {
            reverse_index_offsets.push(total);
            total += label_frequency.get(&label).unwrap_or(&0);
        }

        for node in 0..node_count {
            let label = labels[node];
            let offset = reverse_index_offsets[label + 1];
            reverse_index[offset] = node;
            reverse_index_offsets[label + 1] += 1;
        }

        Self {
            node_count,
            relationship_count,
            label_count,
            labels: labels.into_boxed_slice(),
            offsets: offsets.into_boxed_slice(),
            neighbors: neighbors.into_boxed_slice(),
            reverse_index: reverse_index.into_boxed_slice(),
            reverse_index_offsets: reverse_index_offsets.into_boxed_slice(),
            max_degree,
            max_label,
            max_label_frequency,
            label_frequency,
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
        fn degree(gdl_graph: &gdl::Graph, node: &gdl::graph::Node) -> usize {
            let mut degree = 0;

            for rel in gdl_graph.relationships() {
                if rel.source() == node.variable() {
                    degree += 1;
                }
                if rel.target() == node.variable() {
                    degree += 1;
                }
            }
            degree
        }

        let gdl_graph = gdl.parse::<gdl::Graph>()?;

        let header = format!(
            "t {} {}",
            gdl_graph.node_count(),
            gdl_graph.relationship_count()
        );

        let mut nodes_string = String::from("");

        let mut sorted_nodes = gdl_graph.nodes().collect::<Vec<_>>();
        sorted_nodes.sort_by_key(|node| node.id());

        for node in sorted_nodes {
            let id = node.id();
            let label = node.labels().next().expect("Single label expected");
            let degree = degree(&gdl_graph, node);
            let _ = write!(nodes_string, "v {} {} {}\n", id, &label[1..], degree);
        }

        let mut rels_string = String::from("");

        let mut sorted_rels = gdl_graph.relationships().collect::<Vec<_>>();
        sorted_rels.sort_by_key(|rel| (rel.source(), rel.target()));

        for rel in sorted_rels {
            let source_id = gdl_graph
                .get_node(rel.source())
                .expect("Source expected")
                .id();
            let target_id = gdl_graph
                .get_node(rel.target())
                .expect("Target expected")
                .id();
            let _ = write!(rels_string, "e {} {}\n", source_id, target_id);
        }

        let graph = format!("{}\n{}{}", header, nodes_string, rels_string)
            .parse::<Graph>()
            .unwrap();

        Ok(GdlGraph(graph))
    }
}

pub fn parse(path: &Path) -> Result<Graph, Error> {
    println!("Reading from: {:?}", path);
    let start = Instant::now();
    let file = File::open(path)?;
    println!("Preparing input: {:?}", start.elapsed());
    let start = Instant::now();
    let parse_graph = ParseGraph::try_from(LineReader::new(file))?;
    println!("Parsing graph: {:?}", start.elapsed());
    let start = Instant::now();
    let graph = Graph::from(parse_graph);
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
        assert_eq!(graph.relationship_count(), 6);
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
        assert_eq!(graph.relationship_count(), 6);
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
}