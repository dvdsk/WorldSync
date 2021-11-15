use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::str::FromStr;
use std::time::Duration;
use time::Time;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Could not parse minecraft server output")]
    ParsingError,
}

#[derive(Debug, PartialEq)]
pub struct Line {
    pub time: Time,
    pub source: String,
    pub level: Level,
    pub msg: Message,
}

#[derive(Debug, PartialEq)]
pub enum Level {
    Info,
    Warn,
    Error,
}

#[derive(Debug, PartialEq)]
pub struct Coords {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, PartialEq)]
pub enum Exception {
    AddressInUse,
    Unknown(String),
}

/// only java version numbers post 1.0.0 are supported
/// versions earlier then 1.14 will end up as unknown
#[derive(Debug, PartialEq)]
pub enum Version {
    Pre(u8, u8, u8),
    ExpSnapshot(u8, u8, u8),
    Snapshot { year: u8, week: u8, revision: char },
    Rc(u8, u8, u8),
    Full(u8, u8, u8),
    /// could not parse version, probably a pre release before 1.14
    Unknown,
}

#[derive(Debug, PartialEq)]
pub enum Message {
    EulaUnaccepted,
    Joined {
        user: String,
        address: SocketAddr,
        entity_id: u32,
        coords: Coords,
    },
    Left(String),
    Kicked(String),
    Version(Version),
    Loading(u8),
    DoneLoading(Duration),
    Saved,
    Overloaded(Duration, usize),
    Exception(Exception),
    Stopping,
    Other(String),
}

peg::parser! {
    grammar line_parser() for str {

        rule other_msg() -> Message
            = s:$([_]+![_]) { Message::Other(s.to_owned()) }

        rule percentage() -> u8
            = digits:$(['0'..='9']*<1,3>) {
            digits.parse().expect("malformed loading percentage")
        }

        rule loading() -> Message
            = "Preparing spawn area: " p:percentage() "%" { Message::Loading(p) }

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
            = "Failed to load eula.txt" { Message::EulaUnaccepted }

        rule float() -> f32
            = numb:$(['0'..='9' | '.']+) { numb.parse().unwrap() }
        rule coords() -> Coords
            = x:float() ", " y:float() ", " z:float() { Coords {x,y,z} }
        rule name() -> String
            = s:$(['a'..='z' | 'A'..='Z' | '0'..='9' | '_' ]+) { s.to_owned() }

        rule addr() -> SocketAddr
            = addr_str:$(['0'..='9' | '.']+ ":" ['0'..='9']+) {
            SocketAddr::from_str(addr_str).expect("client address could not be parsed")
        }

        rule joined() -> Message
            = user:name() "[/" address:addr() "] logged in with entity id " id:$(['0'..='9']+) " at (" coords:coords() ")" {
            let entity_id = id.parse().unwrap();
            Message::Joined{user, address, entity_id, coords}
        }

        rule left() -> Message
            = user:name() " lost connection: Disconnected" { Message::Left(user) }
        rule kicked() -> Message
            = "Kicked " user:name() ": Kicked by an operator" { Message::Kicked(user) }
        rule saved() -> Message
            = "Saved the game" { Message::Saved }
        rule stopping() -> Message
            = "Stopping server" { Message::Stopping }

        rule addr_in_use() -> Exception
            = [^':']+ ": bind(..) failed: Address already in use" {
            Exception::AddressInUse
        }
        rule unknown_except() -> Exception
            = s:$([_]+![_]) { Exception::Unknown(s.to_owned()) }
        rule exception() -> Message
            = "The exception was: " e:(addr_in_use() / unknown_except()) {
            Message::Exception(e)
        }

        rule full() -> Version
            = major:$(['0'..='9']+) "." minor:$(['0'..='9']+) "." patch:$(['0'..='9']+) {
            Version::Full(
                major.parse().unwrap(),
                minor.parse().unwrap(),
                patch.parse().unwrap(),
            )
        }

        rule pre() -> Version
            = major:$(['0'..='9']+) "." minor:$(['0'..='9']+) " Pre-Release " pre:$(['0'..='9']+) {
            Version::Pre(
                major.parse().unwrap(),
                minor.parse().unwrap(),
                pre.parse().unwrap(),
            )
        }


        rule rc() -> Version
            = major:$(['0'..='9']+) "." minor:$(['0'..='9']+) " Release Candidate " rc:$(['0'..='9']+) {
            Version::Rc(
                major.parse().unwrap(),
                minor.parse().unwrap(),
                rc.parse().unwrap(),
            )
        }

        rule snap() -> Version
            = y:$(['0'..='9']*<2,2>) "w" w:$(['0'..='9']*<2,2>) rev:$(['a'..='z']) {
            Version::Snapshot{
                year: y.parse().unwrap(), 
                week: w.parse().unwrap(), 
                revision: rev.chars().next().unwrap()
            }
        }

        rule exp_snap() -> Version
            = major:$(['0'..='9']+) "." minor:$(['0'..='9']+) " Experimental Snapshot " snap:$(['0'..='9']+) {
            Version::ExpSnapshot(
                major.parse().unwrap(),
                minor.parse().unwrap(),
                snap.parse().unwrap(),
            )
        }

        rule version() -> Message
            = "Starting minecraft server version " v:(full() / rc() /pre() / snap() / exp_snap()) {
            Message::Version(v)
        }

        rule msg() -> Message
            = loading() / done_loading() / overloaded() / eula() / joined() / left()
             / kicked() / saved() / stopping() / exception() / version() / other_msg()

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
