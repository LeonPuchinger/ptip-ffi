use std::collections::HashMap;

/// A tree-like structure that can be described in two ways:
/// 1: A tree of items, where direct successor nodes are stored in a vec ("cluster").
/// Non-direct successors are stored in different clusters that are linked
/// to the previous cluster via hashmaps ("branches") in the nodes.
/// 2: An index-based array that can be branched off at any item. The branches
/// recursively reference other arrays of the same type.
pub struct ClusteredTree<'a, T> {
    root: Cluster<'a, T>,
}

impl ClusteredTree<'_, String> {
    pub fn new() -> Self {
        Self {
            root: Cluster {
                previous_items: 0,
                items: Vec::new(),
            },
        }
    }
}

struct Cluster<'a, T> {
    previous_items: usize,
    items: Vec<Node<'a, T>>,
}

enum Node<'a, T> {
    WithBranches {
        item: T,
        branches: HashMap<&'a str, Cluster<'a, T>>,
    },
    OnlyItem(T),
}

/// Used to access a specific item in the tree by its index.
/// The tree is walked by following a sequence of directions, each of which specifies
/// a branch to take and an index to access in the cluster at that branch.
/// Accessing the tree via this index is efficient because accessing a cluster
/// via a numeric index as well as accessing a branch via a hashmap key is efficient.
pub struct ClusteredTreeIndex<'a> {
    directions: Vec<IndexDirection<'a>>,
    items_until_index: usize,
}

struct IndexDirection<'a> {
    branch: &'a str,
    cluster_index: usize,
}
