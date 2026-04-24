use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Debug, PartialEq, Eq)]
pub enum BranchedListError {
    CursorOutOfBounds,
    EmptyList { message: String },
}

/// A shared reference to the head of a branch.
type BranchReference<'branch_keys, T> = Rc<RefCell<BranchHead<'branch_keys, T>>>;

/// A data structure that acts like a vec/list but can be branched at any index.
/// Branches are named deviations from one of the nodes that lead to a new list.
/// Branches are realized as a hashmap with string keys and values that are
/// references to the new list.
/// It should be noted that branching _on_ a node actually means the branching
/// happens after the node. It is also possible to branch before the first node
/// on a branch. In other words, branches are realized as gaps between nodes.
pub struct BranchedList<'branch_keys, T> {
    root: BranchReference<'branch_keys, T>,
}

impl<'branch_keys, T> BranchedList<'branch_keys, T> {
    /// Creates a new empty `BranchedList` with no nodes in the root branch and no
    /// branches diverging from root as well.
    pub fn empty() -> Self {
        Self {
            root: Rc::new(RefCell::new(BranchHead {
                contents: Vec::new(),
                head_branches: HashMap::new(),
            })),
        }
    }

    /// Creates a new `BranchedList` with a single node containing the given
    /// value in the root branch. The new list will have no branches diverging
    /// from root.
    pub fn with_value(value: T) -> Self {
        Self {
            root: Rc::new(RefCell::new(BranchHead {
                contents: vec![Node {
                    value,
                    branches: HashMap::new(),
                }],
                head_branches: HashMap::new(),
            })),
        }
    }

    /// Returns a copy of the value at the specified index in the root branch,
    /// or `None` if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<T>
    where
        T: Clone,
    {
        self.root
            .borrow()
            .contents
            .get(index)
            .map(|node| node.value.clone())
    }

    /// Appends a value to the end of the root branch.
    pub fn append(&mut self, value: T) {
        let new_node = Node {
            value,
            branches: HashMap::new(),
        };
        self.root.borrow_mut().contents.push(new_node);
    }

    /// Inserts a value at the specified index in the root branch.
    pub fn insert(&mut self, value: T, at_index: usize) -> Result<(), BranchedListError> {
        if at_index > self.root.borrow().contents.len() {
            return Err(BranchedListError::CursorOutOfBounds);
        }
        let new_node = Node {
            value,
            branches: HashMap::new(),
        };
        self.root.borrow_mut().contents.insert(at_index, new_node);
        Ok(())
    }

    /// Creates a new branch before the specified index in the root branch with the
    /// given key. If a branch with the same key already exists at that index,
    /// it will be reused instead of creating a new one. If the branch is newly
    /// created, it will start as an empty list. The method returns a new instance
    /// of `BranchedList` with the new branch as its root.
    ///
    /// `before_index` is interpreted as a *gap* (position between elements):
    /// - `0` branches before the first element
    /// - `n` branches between element `n-1` and `n`
    /// - `len` branches after the last element
    pub fn branch_before_index(
        &mut self,
        key: &'branch_keys str,
        before_index: usize,
    ) -> Result<Self, BranchedListError> {
        let root_branch_size = self.root.borrow().contents.len();
        if before_index > root_branch_size {
            return Err(BranchedListError::CursorOutOfBounds);
        }

        let next_root: BranchReference<'branch_keys, T> = {
            let mut head = self.root.borrow_mut();
            if before_index == 0 {
                if let Some(existing) = head.head_branches.get(key) {
                    Rc::clone(existing)
                } else {
                    let new_branch = Rc::new(RefCell::new(BranchHead {
                        contents: Vec::new(),
                        head_branches: HashMap::new(),
                    }));
                    head.head_branches.insert(key, Rc::clone(&new_branch));
                    new_branch
                }
            } else {
                let after_index = before_index - 1;
                let node = head
                    .contents
                    .get_mut(after_index)
                    .ok_or(BranchedListError::CursorOutOfBounds)?;

                if let Some(existing) = node.branches.get(key) {
                    Rc::clone(existing)
                } else {
                    let new_branch = Rc::new(RefCell::new(BranchHead {
                        contents: Vec::new(),
                        head_branches: HashMap::new(),
                    }));
                    node.branches.insert(key, Rc::clone(&new_branch));
                    new_branch
                }
            }
        };

        Ok(Self { root: next_root })
    }

    /// Convenience wrapper around `branch_before_index` that branches after the
    /// element at `after_index` (i.e. at gap `after_index + 1`).
    pub fn branch_after_index(
        &mut self,
        key: &'branch_keys str,
        after_index: usize,
    ) -> Result<Self, BranchedListError> {
        let root_branch_size = self.root.borrow().contents.len();
        if after_index >= root_branch_size {
            return Err(BranchedListError::CursorOutOfBounds);
        }
        self.branch_before_index(key, after_index + 1)
    }

    /// Clones the current list as a new instance.
    pub fn snapshot(&self) -> Self {
        Self {
            root: Rc::clone(&self.root),
        }
    }

    /// Returns the number of nodes in the root branch.
    /// Note that this does not consider any branches that diverge from the root.
    pub fn root_branch_size(&self) -> usize {
        self.root.borrow().contents.len()
    }

    /// Returns `true` if the root branch is empty, and `false` otherwise.
    /// Note that this does not consider any branches that diverge from the root.
    pub fn root_branch_empty(&self) -> bool {
        self.root.borrow().contents.is_empty()
    }
}

/// An instance of this type sits at the start of every branch. Its purpose is
/// to allow branching before the first node on the branch.
struct BranchHead<'branch_keys, T> {
    contents: Vec<Node<'branch_keys, T>>,
    head_branches: HashMap<&'branch_keys str, BranchReference<'branch_keys, T>>,
}

/// A wrapper for the actual elements inserted into the `BranchedList`. It contains
/// a hashmap of branches that diverge from (after) this node alongside the actual
/// element value.
struct Node<'branch_keys, T> {
    value: T,
    branches: HashMap<&'branch_keys str, BranchReference<'branch_keys, T>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_list_starts_empty() {
        let list: BranchedList<'static, i32> = BranchedList::empty();
        assert!(list.root_branch_empty());
        assert_eq!(list.root_branch_size(), 0);
        assert_eq!(list.get(0), None);
    }

    #[test]
    fn with_value_starts_with_single_element() {
        let list: BranchedList<'static, i32> = BranchedList::with_value(7);
        assert!(!list.root_branch_empty());
        assert_eq!(list.root_branch_size(), 1);
        assert_eq!(list.get(0), Some(7));
        assert_eq!(list.get(1), None);
    }

    #[test]
    fn append_and_insert_modify_root_branch() {
        let mut list: BranchedList<'static, i32> = BranchedList::empty();
        list.append(1);
        list.append(3);
        assert_eq!(list.root_branch_size(), 2);
        assert_eq!(list.get(0), Some(1));
        assert_eq!(list.get(1), Some(3));

        list.insert(2, 1).expect("insert at middle");
        assert_eq!(list.root_branch_size(), 3);
        assert_eq!(list.get(0), Some(1));
        assert_eq!(list.get(1), Some(2));
        assert_eq!(list.get(2), Some(3));

        list.insert(0, 0).expect("insert at start");
        assert_eq!(list.get(0), Some(0));
        assert_eq!(list.get(1), Some(1));

        list.insert(4, list.root_branch_size())
            .expect("insert at end");
        assert_eq!(list.get(list.root_branch_size() - 1), Some(4));
    }

    #[test]
    fn insert_out_of_bounds_errors() {
        let mut list: BranchedList<'static, i32> = BranchedList::with_value(1);
        match list.insert(2, 3) {
            Err(BranchedListError::CursorOutOfBounds) => {}
            other => panic!("expected CursorOutOfBounds, got: {:?}", other),
        }
    }

    #[test]
    fn snapshot_shares_root_branch_contents() {
        let mut list: BranchedList<'static, i32> = BranchedList::with_value(1);
        let mut snapshot = list.snapshot();

        snapshot.append(2);
        assert_eq!(list.root_branch_size(), 2);
        assert_eq!(list.get(1), Some(2));

        list.append(3);
        assert_eq!(snapshot.root_branch_size(), 3);
        assert_eq!(snapshot.get(2), Some(3));
    }

    #[test]
    fn branch_before_index_zero_works_on_empty_list_and_is_reused() {
        let mut list: BranchedList<'static, i32> = BranchedList::empty();

        let mut b1 = list
            .branch_before_index("h", 0)
            .expect("branch at gap 0");
        assert_eq!(b1.root_branch_size(), 0);
        b1.append(10);

        let b2 = list
            .branch_before_index("h", 0)
            .expect("reuse same branch");
        assert_eq!(b2.get(0), Some(10));
        assert_eq!(b2.root_branch_size(), 1);

        // Root branch remains unchanged.
        assert_eq!(list.root_branch_size(), 0);
    }

    #[test]
    fn branching_at_middle_and_end_gaps_creates_independent_paths() {
        let mut list: BranchedList<'static, i32> = BranchedList::empty();
        list.append(1);
        list.append(2);
        list.append(3);

        // gap 1 = between 1 and 2
        let mut middle = list
            .branch_before_index("m", 1)
            .expect("branch at middle gap");
        middle.append(100);

        // gap len = after last element
        let mut end = list
            .branch_before_index("e", list.root_branch_size())
            .expect("branch at end gap");
        end.append(200);

        assert_eq!(middle.get(0), Some(100));
        assert_eq!(end.get(0), Some(200));
        assert_eq!(list.get(0), Some(1));
        assert_eq!(list.get(1), Some(2));
        assert_eq!(list.get(2), Some(3));
    }

    #[test]
    fn branch_after_index_is_same_gap_as_branch_before_index_plus_one() {
        let mut list: BranchedList<'static, i32> = BranchedList::empty();
        list.append(1);
        list.append(2);

        let mut after_first = list
            .branch_after_index("x", 0)
            .expect("branch after index 0");
        after_first.append(9);

        // Same key and same gap via branch_before_index(1) should reuse.
        let same = list
            .branch_before_index("x", 1)
            .expect("branch before index 1");
        assert_eq!(same.get(0), Some(9));
    }

    #[test]
    fn branch_bounds_are_enforced() {
        let mut empty: BranchedList<'static, i32> = BranchedList::empty();
        match empty.branch_before_index("k", 1) {
            Err(BranchedListError::CursorOutOfBounds) => {}
            _ => panic!("expected CursorOutOfBounds"),
        }

        let mut list: BranchedList<'static, i32> = BranchedList::with_value(1);
        match list.branch_after_index("k", 1) {
            Err(BranchedListError::CursorOutOfBounds) => {}
            _ => panic!("expected CursorOutOfBounds"),
        }
    }

    #[test]
    fn branches_created_from_snapshot_are_visible_from_original() {
        let mut list: BranchedList<'static, i32> = BranchedList::with_value(1);
        let mut snap = list.snapshot();

        let mut branch = snap
            .branch_before_index("b", 0)
            .expect("branch from snapshot");
        branch.append(42);

        let branch_from_original = list
            .branch_before_index("b", 0)
            .expect("branch from original");
        assert_eq!(branch_from_original.get(0), Some(42));
    }
}
