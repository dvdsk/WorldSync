use std::time::Duration;

use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammer.pest"]
struct OutputParser;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Could not parse minecraft server output")]
    ParsingError,
}

#[derive(Debug, PartialEq)]
pub enum Message {
    Loading(u8),
    DoneLoading,
    Overloaded(Duration, usize),
    Other(String),
}

pub fn parse(input: impl Into<String> + AsRef<str>) -> Result<Message, Error> {
    let mut pairs = OutputParser::parse(Rule::l, input.as_ref()).unwrap();

    let pair = pairs.next().unwrap();
    match pair.into_inner().next().unwrap().as_rule() {
    }

    Ok(Message::Other(input.into()))
}

#[test]
fn test_overloaded() {
    let input = "[10:13:02] [Worker-Main-9/INFO]: Preparing spawn area: 68%";
    let msg = parse(input).unwrap();
    assert_eq!(msg, Message::Loading(68));
}
