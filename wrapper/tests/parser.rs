use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;
use time::Time;
use wrapper::parser::{parse, Coords, Level, Line, Message, Exception, Version};

#[test]
fn parse_loading() {
    let input = "[10:13:02] [Worker-Main-9/INFO]: Preparing spawn area: 68%";
    let msg = parse(input).unwrap();
    let correct = Line {
        time: Time::from_hms(10, 13, 2).unwrap(),
        source: "Worker-Main-9".to_owned(),
        level: Level::Info,
        msg: Message::Loading(68),
    };
    assert_eq!(msg, correct);
}

#[test]
fn parse_done() {
    let input = "[00:08:39] [Server thread/INFO]: Done (9.997s)! For help, type \"help\"";
    let msg = parse(input).unwrap();
    let correct = Line {
        time: Time::from_hms(0, 08, 39).unwrap(),
        source: "Server thread".to_owned(),
        level: Level::Info,
        msg: Message::DoneLoading(Duration::from_secs_f32(9.997)),
    };
    assert_eq!(msg, correct);
}

#[test]
fn parse_overloaded() {
    let input = "[00:08:39] [Server thread/WARN]: Can't keep up! Is the server overloaded? Running 100ms or 20 ticks behind";
    let msg = parse(input).unwrap();
    let correct = Line {
        time: Time::from_hms(0, 08, 39).unwrap(),
        source: "Server thread".to_owned(),
        level: Level::Warn,
        msg: Message::Overloaded(Duration::from_millis(100), 20),
    };
    assert_eq!(msg, correct);
}

#[test]
fn parse_eula() {
    let input = "[01:03:12] [main/WARN]: Failed to load eula.txt";
    let msg = parse(input).unwrap();
    let correct = Line {
        time: Time::from_hms(1, 3, 12).unwrap(),
        source: "main".to_owned(),
        level: Level::Warn,
        msg: Message::EulaUnaccepted,
    };
    assert_eq!(msg, correct);
}

#[test]
fn parse_login() {
    let input = "[01:08:43] [Server thread/INFO]: Test[/212.102.34.132:52788] logged in with entity id 179 at (238.3447269617549, 56.707824678197724, 335.3201202126774)";
    let msg = parse(input).unwrap();
    let correct = Line {
        time: Time::from_hms(1, 8, 43).unwrap(),
        source: "Server thread".to_owned(),
        level: Level::Info,
        msg: Message::Joined {
            user: "Test".to_owned(),
            address: SocketAddr::from_str("212.102.34.132:52788").unwrap(),
            entity_id: 179,
            coords: Coords {
                x: 238.3447269617549,
                y: 56.707824678197724,
                z: 335.3201202126774,
            },
        },
    };
    assert_eq!(msg, correct);
}

#[test]
fn parse_disconnected() {
    let input = "[20:32:38] [Server thread/INFO]: Test lost connection: Disconnected";
    let msg = parse(input).unwrap();
    let correct = Line {
        time: Time::from_hms(20, 32, 38).unwrap(),
        source: "Server thread".to_owned(),
        level: Level::Info,
        msg: Message::Left("Test".to_owned()),
    };
    assert_eq!(msg, correct);
}

#[test]
fn parse_kicked() {
    let input = "[20:32:22] [Server thread/INFO]: Kicked Test: Kicked by an operator";
    let msg = parse(input).unwrap();
    let correct = Line {
        time: Time::from_hms(20, 32, 22).unwrap(),
        source: "Server thread".to_owned(),
        level: Level::Info,
        msg: Message::Kicked("Test".to_owned()),
    };
    assert_eq!(msg, correct);
}

#[test]
fn parse_saved() {
    let input = "[20:32:22] [Server thread/INFO]: Saved the game";
    let msg = parse(input).unwrap();
    let correct = Line {
        time: Time::from_hms(20, 32, 22).unwrap(),
        source: "Server thread".to_owned(),
        level: Level::Info,
        msg: Message::Saved,
    };
    assert_eq!(msg, correct);
}

#[test]
fn parse_stopping() {
    let input = "[20:32:22] [Server thread/INFO]: Stopping server";
    let msg = parse(input).unwrap();
    let correct = Line {
        time: Time::from_hms(20, 32, 22).unwrap(),
        source: "Server thread".to_owned(),
        level: Level::Info,
        msg: Message::Stopping,
    };
    assert_eq!(msg, correct);
}

#[test]
fn parse_addr_in_use() {
    let input = r#"[21:14:50] [Server thread/WARN]: The exception was: io.netty.channel.unix.Errors$NativeIoException: bind(..) failed: Address already in use"#;
    let msg = parse(input).unwrap();
    let correct = Line {
        time: Time::from_hms(21, 14, 50).unwrap(),
        source: "Server thread".to_owned(),
        level: Level::Warn,
        msg: Message::Exception(Exception::AddressInUse),
    };
    assert_eq!(msg, correct);
}

#[test]
fn parse_full_release() {
    let input = r#"[21:14:50] [Server thread/INFO]: Starting minecraft server version 1.17.1"#;
    let line = parse(input).unwrap();
    let correct = Message::Version(Version::Full(1,17,1));
    assert_eq!(line.msg, correct);
}

#[test]
fn parse_pre_release() {
    let input = r#"[21:14:50] [Server thread/INFO]: Starting minecraft server version 1.14 Pre-Release 3"#;
    let line = parse(input).unwrap();
    let correct = Message::Version(Version::Pre(1,14,3));
    assert_eq!(line.msg, correct);
}

#[test]
fn parse_snapshot() {
    let input = r#"[21:14:50] [Server thread/INFO]: Starting minecraft server version 18w10d"#;
    let line = parse(input).unwrap();
    let correct = Message::Version(Version::Snapshot{
            year: 18,
            week: 10,
            revision: 'd'
        });
    assert_eq!(line.msg, correct);
}

#[test]
fn parse_experimental_snapshot() {
    let input = r#"[21:14:50] [Server thread/INFO]: Starting minecraft server version 1.18 Experimental Snapshot 1"#;
    let line = parse(input).unwrap();
    let correct = Message::Version(Version::ExpSnapshot(1,18,1));
    assert_eq!(line.msg, correct);
}

#[test]
fn parse_release_candidate() {
    let input = r#"[21:14:50] [Server thread/INFO]: Starting minecraft server version 1.16 Release Candidate 3"#;
    let line = parse(input).unwrap();
    let correct = Message::Version(Version::Rc(1,16,3));
    assert_eq!(line.msg, correct);
}

#[test]
fn parse_chat() {
    let input = r#"[17:44:44] [Server thread/INFO]: [Server] test"#;
    let line = parse(input).unwrap();
    let correct = Message::Chat{from: "Server".into(), msg: "test".into()};
    assert_eq!(line.msg, correct);
}
