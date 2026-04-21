use std::{cell::RefCell, collections::HashMap, rc::Rc};

pub enum BranchedListError {
    CursorOutOfBounds,
    EmptyList { message: String },
}

type BranchReference<'branch_keys, T> = Rc<RefCell<Vec<Node<'branch_keys, T>>>>;

/// A data structure that acts like a vec/list but can be branched at any index.
/// Branches are named deviations from one of the nodes that lead to a new list.
/// Branches are realized as a hashmap with string keys and values that are
/// references to the new list.
pub struct BranchedList<'branch_keys, T> {
    root: BranchReference<'branch_keys, T>,
}

impl<'branch_keys, T> BranchedList<'branch_keys, T> {
    pub fn empty() -> Self {
        Self {
            root: Rc::new(RefCell::new(vec![])),
        }
    }

    pub fn with_value(value: T) -> Self {
        Self {
            root: Rc::new(RefCell::new(vec![Node {
                value,
                branches: HashMap::new(),
            }])),
        }
    }

    /// Returns a copy of the value at the specified index in the root branch,
    /// or `None` if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<T>
    where
        T: Clone,
    {
        self.root.borrow().get(index).map(|node| node.value.clone())
    }

    /// Appends a value to the end of the root branch.
    pub fn append(&mut self, value: T) {
        let new_node = Node {
            value,
            branches: HashMap::new(),
        };
        self.root.borrow_mut().push(new_node);
    }

    /// Inserts a value at the specified index in the root branch.
    pub fn insert(&mut self, value: T, at_index: usize) -> Result<(), BranchedListError> {
        if at_index > self.root.borrow().len() {
            return Err(BranchedListError::CursorOutOfBounds);
        }
        let new_node = Node {
            value,
            branches: HashMap::new(),
        };
        self.root.borrow_mut().insert(at_index, new_node);
        Ok(())
    }

    /// Creates a new branch at the specified index in the root branch with the
    /// given key. If a branch with the same key already exists at that index,
    /// it will be reused instead of creating a new one. If the branch is newly
    /// created, it will start as an empty list. The method returns a new instance
    /// of `BranchedList` with the new branch as its root.
    pub fn branch(
        &mut self,
        key: &'branch_keys str,
        at_index: usize,
    ) -> Result<Self, BranchedListError> {
        let root_branch_size = self.root.borrow().len();
        if root_branch_size == 0 {
            return Err(BranchedListError::EmptyList {
                message: "Cannot branch an empty list".to_string(),
            });
        }
        if at_index >= root_branch_size {
            return Err(BranchedListError::CursorOutOfBounds);
        }
        let next_root: BranchReference<'branch_keys, T> = {
            let mut current_branch = self.root.borrow_mut();
            let current_node = current_branch
                .get_mut(at_index)
                .ok_or(BranchedListError::CursorOutOfBounds)?;

            if let Some(existing) = current_node.branches.get(key) {
                Rc::clone(existing)
            } else {
                let new_branch = Rc::new(RefCell::new(Vec::new()));
                current_node.branches.insert(key, new_branch.clone());
                new_branch
            }
        };
        Ok(Self { root: next_root })
    }

    /// Clones the current list as a new instance.
    pub fn snapshot(&self) -> Self {
        Self {
            root: Rc::clone(&self.root),
        }
    }
}

struct Node<'branch_keys, T> {
    value: T,
    branches: HashMap<&'branch_keys str, BranchReference<'branch_keys, T>>,
}
