pub mod user;
pub mod world;

use typed_sled::sled;
pub fn test_db() -> sled::Db {
    let config = sled::Config::new().temporary(true);
    let db = config.open().unwrap();
    db
}
