use typed_sled::{Tree, sled};

#[derive(Clone)]
pub struct WorldDb {
    // index: HashMap<String, UserId>,
    // tree: Tree<UserId, UserEntry>,
    db: sled::Db,
}

impl WorldDb {
    pub fn from(db: sled::Db) -> Self {
        // let tree = Tree::init(&db, "worlddb");
        WorldDb { /*tree,*/ db }
    }
}
