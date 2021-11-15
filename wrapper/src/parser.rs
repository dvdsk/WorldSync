use std::net::{SocketAddr, SocketAddrV4, Ipv4Addr};
use std::time::Duration;
use std::str::FromStr;
use time::Time;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Could not parse minecraft server output")]
    ParsingError,
}

#[derive(Debug, PartialEq)]
pub struct Line {
    time: Time,
    source: String,
    level: Level,
    msg: Message,
}

#[derive(Debug, PartialEq)]
pub enum Level {
    Info,
    Warn,
    Error,
}

#[derive(Debug, PartialEq)]
pub struct Coords {
    x: f32, y:f32, z:f32 }

#[derive(Debug, PartialEq)]
pub enum Message {
    EulaUnaccepted,
    Login(String, SocketAddr, u32, Coords), 
    Loading(u8),
    DoneLoading(Duration),
    Overloaded(Duration, usize),
    Other(String),
}

peg::parser! {
    grammar line_parser() for str {

        rule other() -> Message
            = s:$([_]+![_]) {
            Message::Other(s.to_owned())
        }

        rule percentage() -> u8
            = digits:$(['0'..='9']*<1,3>) {
            digits.parse().expect("malformed loading percentage")
        }

        rule loading() -> Message
            = "Preparing spawn area: " p:percentage() "%" {
            Message::Loading(p)
        }

        rule dur_sec() -> Duration 
            = secs:$(['0'..='9' | '.']+ "s") {
            let secs: f32 = secs[..secs.len()-1].parse().expect("malformed seconds duration");
            Duration::from_secs_f32(secs)
        }


        rule dur_min() -> Duration 
            = secs:$(['0'..='9' | '.']+ "m") {
            let secs: f32 = secs[..secs.len()-1].parse().expect("malformed minutes duration");
            Duration::from_secs_f32(secs*60.)
        }

        rule duration() -> Duration 
            = dur_sec () / dur_min()

        rule done_loading() -> Message
            = "Done (" d:duration() ")! For help, type \"help\"" {
            Message::DoneLoading(d)
        }

        rule overloaded() -> Message
            = "Can't keep up! Is the server overloaded? Running " ms:$(['0'..='9']+) "ms or " ticks:$(['0'..='9']+) " ticks behind" {
            let ms = ms.parse().expect("overloaded ms badly formed");
            let ticks = ticks.parse().expect("overloaded ticks badly formed");
            Message::Overloaded(Duration::from_millis(ms), ticks)
        }

        rule eula() -> Message
            = "Failed to load eula.txt" {
            Message::EulaUnaccepted
        }

        rule float() -> f32 
            = numb:$(['0'..='9' | '.']+) {
            numb.parse().unwrap()
        }

        rule coords() -> Coords 
            = x:float() ", " y:float() ", " z:float() {
            Coords {x,y,z}
        }

        rule name() -> String
            = s:$(['a'..='z' | 'A'..='Z' | '0'..='9' | '_' ]+) {
            s.to_owned()
        }

        rule addr() -> SocketAddr 
            = ip:$(['0'..='9' | '.']+) ":" port:$(['0'..='9']+) {
            let ipv4 = Ipv4Addr::from_str(ip).unwrap();
            let port = port.parse().unwrap();
            SocketAddr::V4(SocketAddrV4::new(ipv4, port))
        }

        rule login() -> Message
            = n:name() "[//" a:addr() "] logged in with entity id " id:$(['0'..='9']+) "at (" c:coords() ")" {
            let entity = id.parse().unwrap();
            Message::Login(n, a, entity, c)
        }

        pub rule msg() -> Message
            = loading() / done_loading() / overloaded() / eula() / login() / other() 

        rule info() -> Level
            = "INFO" { Level::Info }

        rule warn() -> Level
            = "WARN" { Level::Warn }

        rule error() -> Level
            = "ERROR" { Level::Error }

        rule level() -> Level
            = info() / warn() / error()

        rule source() -> String
            = source:$(['a'..='z' | 'A'..='Z' | '0'..='9' | '-' | ' ']+) {
            source.to_string()
        }

        rule two_digits() -> u8
            = digits:$(['0'..='9']*<2,2>) {
            digits.parse().expect("could not parse digits")
        }

        rule time() -> Time
            = hour:two_digits() ":" minute:two_digits() ":" second:two_digits() {
            Time::from_hms(hour, minute, second)
                .expect("could not parse time, hour, min or second number invalid")
        }
        pub rule line() -> Line
            = "[" time:time() "] [" source:source() "/" level:level() "]: " msg:msg() {
            Line { time, source, level, msg }
        }
    }
}

pub fn parse(input: impl Into<String> + AsRef<str>) -> Result<Line, Error> {
    line_parser::line(input.as_ref()).map_err(|_| Error::ParsingError)
}

// #[test]
// fn parse_msg() {
//     let input = "Done (9.997s)! For help, type \"help\"";
//     let level = line_parser::done_loading(&input).unwrap();
// }

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
    let input = "[01:08:43] [Server thread/INFO]: Freowin[/212.102.34.132:52788] logged in with entity id 179 at (238.3447269617549, 56.707824678197724, 335.3201202126774)"
    let msg = parse(input).unwrap();
    let correct = Line {
        time: Time::from_hms(1, 8, 43).unwrap(),
        source: "Server thread".to_owned(),
        level: Level::Info,
        msg: Message::Login(),
    };
    assert_eq!(msg, correct);
