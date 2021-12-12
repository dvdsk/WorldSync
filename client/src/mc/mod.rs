use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

use futures::stream::{self, BoxStream};
use iced::Command;
use shared::tarpc::context::Context;
use tracing::info;
use wrapper::Instance;

use crate::gui::RpcConn;
use crate::world_dl::SERVER_PATH;
use crate::Event;

// pub mod server;
pub fn sub() -> iced::Subscription<Event> {
    iced::Subscription::from_recipe(McServer {})
}

pub struct McServer {}

enum Phase {
    Start,
    Running,
    Error,
}

struct State {
    phase: Phase,
    instance: Option<Instance>,
}

impl<H, I> iced_native::subscription::Recipe<H, I> for McServer
where
    H: Hasher,
{
    type Output = Event;

    fn hash(&self, state: &mut H) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);
    }

    fn stream(self: Box<Self>, _input: BoxStream<'static, I>) -> BoxStream<'static, Self::Output> {
        Box::pin(stream::unfold(
            State {
                phase: Phase::Start,
                instance: None,
            },
            move |state| async move { state_machine(state).await },
        ))
    }
}

async fn start(mut state: State) -> (Event, State) {
    info!("starting minecraft server");
    match Instance::start(Path::new(SERVER_PATH), 2).await {
        Err(e) => {
            use crate::gui::host::Event as hEvent;
            let event = Event::HostPage(hEvent::Error(e.into()));
            state.phase = Phase::Error;
            (event, state)
        }
        Ok((instance, handle)) => {
            state.instance = Some(instance);
            state.phase = Phase::Running;
            let handle = Arc::new(handle);
            use crate::gui::hosting::Event as hEvent;
            let event = Event::HostingPage(hEvent::Handle(handle));
            (event, state)
        }
    }
}

async fn forward_events(mut state: State) -> (Event, State) {
    let res = state.instance.as_mut().unwrap().next_event().await;
    (Event::Mc(res), state)
}

async fn state_machine(state: State) -> Option<(Event, State)> {
    match state.phase {
        Phase::Start => Some(start(state).await),
        Phase::Running => Some(forward_events(state).await),
        Phase::Error => None,
    }
}

async fn send(line: wrapper::parser::Line, rpc: RpcConn) -> crate::Event {
    use crate::gui::hosting::Event as hEvent;
    use crate::gui::hosting::Error as hError;
    let res = rpc
        .client
        .pub_mc_line(Context::current(), rpc.session, line)
        .await
        .expect("rpc failure");
    match res {
        Ok(_) => Event::Empty,
        Err(protocol::Error::NotHost) => Event::HostingPage(hEvent::Error(hError::NotHost)),
        e_ => panic!("unexpected error: {:?}", e_),
    }
}

pub fn send_line(line: wrapper::parser::Line, rpc: RpcConn) -> Command<Event> {
    let send = send(line, rpc);
    Command::perform(send, |msg| msg)
}
