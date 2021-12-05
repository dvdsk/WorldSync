use std::time::{Duration, SystemTime};

pub use tarpc;
use tarpc::context::Context;

pub fn context(seconds: u64) -> Context {
    let mut context = Context::current();
    context.deadline = SystemTime::now() + Duration::from_secs(seconds);
    context
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
