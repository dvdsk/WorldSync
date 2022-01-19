use std::hash::{Hash, Hasher};
use std::pin::Pin;

use crate::{java_path, Event};
use futures::stream::{self, BoxStream, StreamExt};
use futures::{TryStreamExt, TryStream, Stream};

pub fn sub(count: usize) -> iced::Subscription<Event> {
    iced::Subscription::from_recipe(JavaSetup { count })
}

pub struct JavaSetup {
    count: usize,
}

use localjava::Version;
#[derive(Default)]
pub struct State {
    stream: Option<Pin<Box<dyn Stream<Item = Result<localjava::Progress, localjava::download::Error>>>>>,
    phase: Phase,
}

impl Default for Phase {
    fn default() -> Self {
        Self::Started
    }
}

pub enum Phase {
    Started,
    Download,
    Streaming,
    Stop,
}

impl<H, I> iced_native::subscription::Recipe<H, I> for JavaSetup
where
    H: Hasher,
{
    type Output = Event;

    fn hash(&self, state: &mut H) {
        struct Marker;
        self.count.hash(state);
        std::any::TypeId::of::<Marker>().hash(state);
    }

    fn stream(self: Box<Self>, _input: BoxStream<'static, I>) -> BoxStream<'static, Self::Output> {
        Box::pin(stream::unfold(State::default(), move |state| async move {
            match &state.phase {
                Started => {
                    let res = localjava::version(java_path());
                    match res {
                        Err(e) => Some((Event::Empty, state)),
                        Ok(v) if v == Version::new(17, 0, 1) => Some((Event::Empty, state)),
                        Ok(_) => Some((Event::Empty, state)),
                    }
                }
                Download => match localjava::download_stream(java_path().to_owned()).await {
                    Ok(stream) => {
                        let stream = Pin::new(Box::new(stream));
                        state.phase = Phase::Streaming;
                        state.stream = Some(stream);
                        Some((Event::Empty, state))
                    }
                    Err(error) => Some((Event::Empty, state)),
                },
                Streaming => match state.stream.unwrap().try_next().unwrap().await {
                    Some(Ok(e)) => Some((Event::Empty, state)),
                    Some(Err(e)) => Some((Event::Empty, state)),
                    None => Some((Event::Empty, state)),
                },
                Stop => todo!(),
            }
        }))
    }
}
