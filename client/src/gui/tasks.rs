use super::State;

impl State {
    pub fn is_us(&self, host: &protocol::Host) -> bool {
        let our_id = self.rpc.as_ref().map(|c| c.session);
        our_id == Some(host.id)
    }
}
