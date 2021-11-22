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

enum State {
    Starting { conn: RpcConn, backlog: Vec<Event> },
    Backlog { conn: RpcConn, backlog: Vec<Event> },
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
            State::Starting {
                conn: self.conn.replace(None).unwrap(),
                backlog: Vec::new(),
            },
            move |state| async move {
                match state {
                    State::Starting { mut conn, mut backlog } => {
                        let (event, extra) = await_events(&mut conn).await;
                        backlog.extend_from_slice(&extra);
                        Some((event, State::Backlog { conn, backlog }))
                    }
                    State::Backlog { mut conn, mut backlog } => match backlog.pop() {
                        Some(event) => Some((event, State::Backlog { conn, backlog })),
                        None => {
                            let (event, extra) = await_events(&mut conn).await;
                            backlog.extend_from_slice(&extra);
                            Some((event, State::Backlog { conn, backlog }))
                        }
                    },
                }
            },
        ))
    }
}

async fn get_events(conn: &mut RpcConn) -> Result<Vec<protocol::Event>, Error> {
    let server_events = conn
        .client
        .await_events(context::current(), conn.session)
        .await
        .map_err(|_| Error::NoMetaConn)??;
    Ok(server_events)
}

async fn await_events(conn: &mut RpcConn) -> (Event, Vec<Event>) {
    match get_events(conn).await {
        Err(err) => (Event::Error(err), Vec::new()),
        Ok(ev) => {
            let mut events = ev.into_iter().map(|server_event| Event::from(server_event));
            let event = events.next().expect("server only replies if it has events");
            let backlog = events.collect();
            (event, backlog)
        }
    }
}
