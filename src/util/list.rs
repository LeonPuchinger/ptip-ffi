use std::{cell::RefCell, collections::HashMap, rc::Rc};

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
