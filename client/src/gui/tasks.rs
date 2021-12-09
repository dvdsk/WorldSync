use super::{RpcConn, State};

#[derive(Default, Clone)]
pub struct SubStatus {
    id: usize,
    active: bool,
}

impl SubStatus {
    #[allow(dead_code)]
    pub fn stop(&mut self) {
        assert!(self.active, "subscription was not active");
        self.active = false;
    }
    pub fn start(&mut self) {
        assert!(!self.active, "subscription was already active");
        self.active = true;
        self.id += 1;
    }
    pub fn active(&self) -> Option<usize> {
        match self.active {
            true => Some(self.id),
            false => None,
        }
    }
}

impl State {
    pub fn unwrap_rpc(&self) -> RpcConn {
        self.rpc.as_ref().unwrap().clone()
    }
}
