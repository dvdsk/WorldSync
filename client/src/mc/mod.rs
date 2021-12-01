use std::hash::{Hash, Hasher};
use std::sync::Arc;

use futures::stream::{self, BoxStream};
use wrapper::Instance;

use crate::Event;

// pub mod server;
pub fn sub() -> iced::Subscription<Event> {
    iced::Subscription::from_recipe(McServer {} )

}

pub struct McServer {}

enum Phase {
    Start,
    Running,
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
            State { phase: Phase::Start, instance: None },
            move |state| async move {
                work_on(state).await
            },
        ))
    }
}

use crate::gui::hosting::Event as hEvent;
async fn work_on(mut state: State) -> Option<(Event, State)> {
    match state.phase {
        Phase::Start => {
            let (instance, handle) = Instance::start("tests/data", 2)
                .await.unwrap();
            state.instance = Some(instance);
            state.phase = Phase::Running;
            let handle = Arc::new(handle);
            let event = Event::HostingPage(hEvent::Handle(handle));
            Some((event, state))
        }
        Phase::Running => todo!(),
    }
}

