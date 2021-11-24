use crate::gui::{host, login, RpcConn};
use crate::Error;
use futures::stream::{self, BoxStream};
use protocol::tarpc::context;
use protocol::Host;
use std::cell::Cell;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub enum Event {
    LoggedIn(RpcConn, Option<Host>),
    HostPage(host::Event),
    LoginPage(login::Event),
    StartHosting,
    HostAssigned,
    Server(protocol::Event),
    Error(Error),
    None,
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
            },
            move |mut state| async move {
                let event = await_event(&mut state.conn).await;
                Some((event, state))
            },
        ))
    }
}

async fn get_events(conn: &mut RpcConn) -> Result<protocol::Event, Error> {
    let server_events = conn
        .client
        .await_event(context::current(), conn.session)
        .await
        .map_err(|_| Error::NoMetaConn)??;
    Ok(server_events)
}

async fn await_event(conn: &mut RpcConn) -> Event {
    match get_events(conn).await {
        Err(err) => Event::Error(err),
        Ok(ev) => {
            let event = Event::from(ev);
            event
        }
    }
}
