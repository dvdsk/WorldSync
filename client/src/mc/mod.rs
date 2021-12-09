use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

use futures::stream::{self, BoxStream};
use tracing::info;
use wrapper::Instance;

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
    use crate::gui::hosting::Error as hError;
    use crate::gui::hosting::Event as hEvent;
    let res = state.instance.as_mut().unwrap().next_event().await;
    let event = match res {
        Ok(event) => hEvent::Mc(event),
        Err(err) => hEvent::Error(hError::Mc(err)),
    };
    (Event::HostingPage(event), state)
}

async fn state_machine(state: State) -> Option<(Event, State)> {
    match state.phase {
        Phase::Start => Some(start(state).await),
        Phase::Running => Some(forward_events(state).await),
        Phase::Error => None,
    }
}
