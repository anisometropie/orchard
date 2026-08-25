use std::sync::{Arc, Mutex};

use crate::hexagon::models::Tree;
use crate::hexagon::ports::TreeRepository;

pub struct InMemoryTreeRepository {
    saved_trees: Arc<Mutex<Vec<Tree>>>,
}

pub struct InMemoryTreeRepositoryObserver {
    saved_trees: Arc<Mutex<Vec<Tree>>>,
}

impl InMemoryTreeRepository {
    pub fn new() -> (Self, InMemoryTreeRepositoryObserver) {
        let saved_trees = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                saved_trees: Arc::clone(&saved_trees),
            },
            InMemoryTreeRepositoryObserver { saved_trees },
        )
    }
}

impl TreeRepository for InMemoryTreeRepository {
    fn save(&mut self, tree: Tree) {
        self.saved_trees.lock().unwrap().push(tree);
    }
}

impl InMemoryTreeRepositoryObserver {
    pub fn saved_trees(&self) -> Vec<Tree> {
        self.saved_trees.lock().unwrap().clone()
    }
}
