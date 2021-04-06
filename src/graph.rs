use atoi::FromRadix10;
use std::{
    collections::HashMap, convert::TryFrom, fs::File, io::Read, path::PathBuf, time::Instant,
};

use crate::Result;

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
    type Error = eyre::Report;

    fn try_from(mut lines: LineReader<R>) -> Result<Self> {
        let mut header = lines.next_line().expect("missing header line")?;

        // skip "t" char and white space
        header = &header[2..];
        let (node_count, used) = usize::from_radix_10(header);
        header = &header[used + 1..];
        let (relationship_count, _) = usize::from_radix_10(&header);

        let mut labels = Vec::<usize>::with_capacity(node_count);
        let mut offsets = Vec::<usize>::with_capacity(node_count + 1);
        // undirected
        let mut neighbors = Vec::<usize>::with_capacity(relationship_count * 2);
        neighbors.resize(relationship_count * 2, 0);

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
        let mut next_offset = Vec::<usize>::with_capacity(node_count);
        next_offset.resize(node_count, 0);

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
    fn from(parse_graph: ParseGraph) -> Self {
        let ParseGraph {
            node_count,
            relationship_count,
            labels,
            offsets,
            mut neighbors,
            max_degree,
            max_label,
            label_frequency,
        } = parse_graph;

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
        let mut reverse_index = Vec::<usize>::with_capacity(node_count);
        reverse_index.resize(node_count, 0);
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

pub fn parse(path: PathBuf) -> Result<Graph> {
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
        let str = "
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

        let reader = LineReader::new(str.as_str().as_bytes());

        let graph = Graph::from(ParseGraph::try_from(reader).unwrap());

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

        assert_eq!(graph.nodes_by_label(0), &[0]);
        assert_eq!(graph.nodes_by_label(1), &[1, 3]);
        assert_eq!(graph.nodes_by_label(2), &[2, 4]);
    }
}
