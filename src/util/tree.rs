use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusteredTreeError {
    InvalidIndex,
}

/// A tree-like structure that can be described in two ways:
/// 1: A tree of items, where direct successor nodes are stored in a vec ("cluster").
/// Non-direct successors are stored in different clusters that are linked
/// to the previous cluster via hashmaps ("branches") in the nodes.
/// 2: An index-based array that can be branched off at any item. The branches
/// recursively reference other arrays of the same type.
pub struct ClusteredTree<'a, T> {
    root: Cluster<'a, T>,
}

impl<'a, T> ClusteredTree<'a, T> {
    pub fn new() -> Self {
        Self {
            root: Cluster {
                previous_items: 0,
                items: Vec::new(),
            },
        }
    }

    /// Walk the tree using `directions` starting from `node` and return the node
    /// at the end of the path along with the number of items passed on the path
    /// to that node (including the node itself).
    fn node_at_from_directions<'b>(
        mut node: &'b Node<'a, T>,
        mut items_until_node: usize,
        directions: &[IndexDirection<'a>],
    ) -> Result<(&'b Node<'a, T>, usize), ClusteredTreeError> {
        for direction in directions {
            let cluster = node
                .branches
                .get(direction.branch)
                .ok_or(ClusteredTreeError::InvalidIndex)?;

            // In the linearization model used for counting, the branch cluster starts right after the current node.
            items_until_node = items_until_node
                .checked_add(1)
                .and_then(|v| v.checked_add(direction.cluster_index))
                .ok_or(ClusteredTreeError::InvalidIndex)?;

            node = cluster
                .items
                .get(direction.cluster_index)
                .ok_or(ClusteredTreeError::InvalidIndex)?;
        }

        Ok((node, items_until_node))
    }

    /// Similar to `node_at_from_directions`, but
    /// returns a mutable reference to the node.
    fn node_at_from_directions_mut<'b>(
        node: &'b mut Node<'a, T>,
        items_until_node: usize,
        directions: &[IndexDirection<'a>],
    ) -> Result<(&'b mut Node<'a, T>, usize), ClusteredTreeError> {
        if let Some((first, rest)) = directions.split_first() {
            let items_until_next = items_until_node
                .checked_add(1)
                .and_then(|v| v.checked_add(first.cluster_index))
                .ok_or(ClusteredTreeError::InvalidIndex)?;

            let cluster = node
                .branches
                .get_mut(first.branch)
                .ok_or(ClusteredTreeError::InvalidIndex)?;
            let next_node = cluster
                .items
                .get_mut(first.cluster_index)
                .ok_or(ClusteredTreeError::InvalidIndex)?;
            return Self::node_at_from_directions_mut(next_node, items_until_next, rest);
        }

        Ok((node, items_until_node))
    }

    /// Walk the tree using `index` and return the node at that index along with
    /// the number of nodes passed on the path to that node (including the node itself).
    fn node_at(
        &self,
        index: &ClusteredTreeIndex<'a>,
    ) -> Result<(&Node<'a, T>, usize), ClusteredTreeError> {
        let root_node = self
            .root
            .items
            .get(index.root_index)
            .ok_or(ClusteredTreeError::InvalidIndex)?;
        Self::node_at_from_directions(root_node, index.root_index, &index.directions)
    }

    /// Similar to `node_at`, but returns a mutable reference to the node.
    fn node_at_mut(
        &mut self,
        index: &ClusteredTreeIndex<'a>,
    ) -> Result<(&mut Node<'a, T>, usize), ClusteredTreeError> {
        let root_node = self
            .root
            .items
            .get_mut(index.root_index)
            .ok_or(ClusteredTreeError::InvalidIndex)?;
        Self::node_at_from_directions_mut(root_node, index.root_index, &index.directions)
    }

    /// Create a new empty branch at `index`.
    pub fn branch(
        &mut self,
        index: &ClusteredTreeIndex<'a>,
        name: &'a str,
    ) -> Result<(), ClusteredTreeError> {
        let (node, items_until_node) = self.node_at_mut(index)?;

        if node.branches.contains_key(name) {
            return Err(ClusteredTreeError::InvalidIndex);
        }

        let new_cluster = Cluster {
            previous_items: items_until_node
                .checked_add(1)
                .ok_or(ClusteredTreeError::InvalidIndex)?,
            items: Vec::new(),
        };

        node.branches.insert(name, new_cluster);
        Ok(())
    }

    /// The clustered tree starts at `index` now.
    pub fn reroot(&mut self, index: &ClusteredTreeIndex<'a>) -> Result<(), ClusteredTreeError> {
        fn shift_descendants_previous_items<'a, T>(
            cluster: &mut Cluster<'a, T>,
            shift: usize,
        ) -> Result<(), ClusteredTreeError> {
            for node in &mut cluster.items {
                for child in node.branches.values_mut() {
                    if child.previous_items < shift {
                        return Err(ClusteredTreeError::InvalidIndex);
                    }
                    child.previous_items -= shift;
                    shift_descendants_previous_items(child, shift)?;
                }
            }
            Ok(())
        }

        let shift = self.node_at(index)?.1;

        if index.directions.is_empty() {
            let local_index = index.root_index;
            if local_index >= self.root.items.len() {
                return Err(ClusteredTreeError::InvalidIndex);
            }

            let new_root_items = self.root.items.split_off(local_index);
            self.root = Cluster {
                previous_items: 0,
                items: new_root_items,
            };

            shift_descendants_previous_items(&mut self.root, shift)?;
            return Ok(());
        }

        let (prefix, last) = index.directions.split_at(index.directions.len() - 1);

        let root_node = self
            .root
            .items
            .get_mut(index.root_index)
            .ok_or(ClusteredTreeError::InvalidIndex)?;
        let (parent, _) = Self::node_at_from_directions_mut(root_node, index.root_index, prefix)?;

        let mut new_root_cluster = parent
            .branches
            .remove(last[0].branch)
            .ok_or(ClusteredTreeError::InvalidIndex)?;

        if last[0].cluster_index >= new_root_cluster.items.len() {
            return Err(ClusteredTreeError::InvalidIndex);
        }

        let new_root_items = new_root_cluster.items.split_off(last[0].cluster_index);
        self.root = Cluster {
            previous_items: 0,
            items: new_root_items,
        };

        shift_descendants_previous_items(&mut self.root, shift)?;
        Ok(())
    }
}

impl<'a, T: Clone> ClusteredTree<'a, T> {
    /// Walk the tree using `index` and return the item at that index.
    pub fn at(&self, index: &ClusteredTreeIndex<'a>) -> Result<T, ClusteredTreeError> {
        let (node, _) = self.node_at(index)?;
        Ok(node.item.clone())
    }
}

/// A cluster is essentially just a vector of nodes, but it also keeps track of
/// the number of items in the tree that come before the first item in the cluster.
struct Cluster<'a, T> {
    previous_items: usize,
    items: Vec<Node<'a, T>>,
}

/// A node contains an item and a hashmap of branches to other clusters. Each branch
/// represents a divergence in the tree structure, where the cluster at the end
/// of the branch contains items that are not direct successors of the current node.
struct Node<'a, T> {
    item: T,
    branches: HashMap<&'a str, Cluster<'a, T>>,
}

/// Used to access a specific item in the tree by its index.
/// The tree is walked by following a sequence of directions, each of which specifies
/// a branch to take and an index to access in the cluster at that branch.
/// Accessing the tree via this index is efficient because accessing a cluster
/// via a numeric index as well as accessing a branch via a hashmap key is efficient.
pub struct ClusteredTreeIndex<'a> {
    root_index: usize,
    directions: Vec<IndexDirection<'a>>,
    items_until_index: usize,
}

struct IndexDirection<'a> {
    branch: &'a str,
    cluster_index: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idx<'a>(
        root_index: usize,
        items_until_index: usize,
        directions: Vec<(&'a str, usize)>,
    ) -> ClusteredTreeIndex<'a> {
        ClusteredTreeIndex {
            root_index,
            directions: directions
                .into_iter()
                .map(|(branch, cluster_index)| IndexDirection {
                    branch,
                    cluster_index,
                })
                .collect(),
            items_until_index,
        }
    }

    fn node<'a>(item: &'a str) -> Node<'a, String> {
        Node {
            item: item.to_string(),
            branches: HashMap::new(),
        }
    }

    #[test]
    fn at_returns_item_from_root_cluster() {
        let tree = ClusteredTree {
            root: Cluster {
                previous_items: 0,
                items: vec![node("a"), node("b"), node("c")],
            },
        };

        assert_eq!(tree.at(&idx(1, 0, vec![])).unwrap(), "b");
    }

    #[test]
    fn branch_creates_empty_cluster_at_index() {
        let mut tree = ClusteredTree {
            root: Cluster {
                previous_items: 0,
                items: vec![node("a")],
            },
        };

        // items_until_index is ignored for current methods; only traversal determines the count.
        tree.branch(&idx(0, 999, vec![]), "x").unwrap();

        let n0 = &tree.root.items[0];
        let child = n0.branches.get("x").unwrap();
        assert_eq!(child.previous_items, 1);
        assert!(child.items.is_empty());
    }

    #[test]
    fn reroot_moves_root_to_target_node() {
        let mut tree: ClusteredTree<'static, String> = ClusteredTree {
            root: Cluster {
                previous_items: 0,
                items: vec![node("a")],
            },
        };

        // From root node "a", branch "x" to a new cluster containing ["b", "c"].
        tree.root.items[0].branches.insert(
            "x",
            Cluster {
                previous_items: 1,
                items: vec![node("b"), node("c")],
            },
        );

        // Index semantics: start at root_index; then for each direction: follow `branch`, then pick `cluster_index`.
        // Reroot to "c" (follow branch "x", then pick index 1).
        // items_until_index is ignored for current methods; reroot computes the shift while traversing.
        tree.reroot(&idx(0, 999, vec![("x", 1)])).unwrap();

        assert_eq!(tree.at(&idx(0, 0, vec![])).unwrap(), "c");
    }
}
