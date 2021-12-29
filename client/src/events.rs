use crate::gui::{host, hosting, login, RpcConn};
use crate::Error;
use futures::stream::{self, BoxStream};
use protocol::{HostState, AWAIT_EVENT_TIMEOUT};
use shared::tarpc::client::RpcError;
use shared::tarpc::context::Context;
use std::cell::Cell;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tracing::instrument;

#[derive(Debug, Clone)]
pub enum Event {
    LoggedIn(RpcConn, HostState),
    WorldUpdated,
    HostPage(host::Event),
    LoginPage(login::Event),
    HostingPage(hosting::Event),
    Server(protocol::Event),
    Mc(Result<wrapper::parser::Line, wrapper::Error>),
    McHandle(Arc<wrapper::Handle>),
    Error(Error),
    ClipHost,
    Empty,
}

impl From<protocol::Event> for Event {
    fn from(event: protocol::Event) -> Self {
        Event::Server(event)
    }
}

pub fn sub_to_server(conn: RpcConn) -> iced::Subscription<Event> {
    iced::Subscription::from_recipe(ServerSub {
        conn: Cell::new(Some(conn)),
    })
}

pub struct ServerSub {
    conn: Cell<Option<RpcConn>>,
}

struct State {
    conn: RpcConn,
    err: bool,
}

impl<H, I> iced_native::subscription::Recipe<H, I> for ServerSub
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
                conn: self.conn.replace(None).unwrap(),
                err: false,
            },
            move |mut state| async move {
                if state.err {
                    None
                } else {
                    let event = match get_events(&mut state.conn).await {
                        Err(err) => {
                            state.err = true;
                            Event::Error(err)
                        }
                        Ok(ev) => Event::from(ev),
                    };
                    Some((event, state))
                }
            },
        ))
    }
}

#[instrument(err)]
async fn get_events(conn: &mut RpcConn) -> Result<protocol::Event, Error> {
    let mut context = Context::current();
    loop {
        context.deadline = SystemTime::now() + AWAIT_EVENT_TIMEOUT + Duration::from_secs(2);
        let res = conn.client.await_event(context, conn.session).await;

        match res {
            Ok(event) => match event.unwrap() {
                protocol::Event::AwaitTimeout => continue,
                _event => return Ok(_event),
            },
            Err(RpcError::DeadlineExceeded) => {
                panic!("did not recieve event on time: {:?}", res);
            }
            Err(e) => return Err(Error::NoMetaConn(e)),
        }
    }
}
