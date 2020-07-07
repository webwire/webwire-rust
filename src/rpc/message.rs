use nom::{
    branch::alt,
    bytes::complete::{tag, take_while_m_n},
    character::complete::char,
    combinator::{cut, map, map_res, opt},
    sequence::{preceded, tuple},
    IResult,
};

// 2^53-1 is the maximum valid message_id as this is the highest
// integer that can be represented in JavaScript. This should be
// plenty though.
const MAX_SAFE_INTEGER: u64 = 9007199254740991; // = 2^53-1

pub enum InvalidNumber {
    ParseIntError(std::num::ParseIntError),
    MaxValueExceeded,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Message<'a> {
    Heartbeat(Heartbeat),
    Notification(Notification<'a>),
    Request(Request<'a>),
    Response(Response<'a>),
    Error(Error<'a>),
    Disconnect,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Heartbeat {
    last_message_id: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Notification<'a> {
    message_id: u64,
    method_name: &'a str,
    data: Option<&'a [u8]>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Request<'a> {
    message_id: u64,
    method_name: &'a str,
    data: Option<&'a [u8]>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Response<'a> {
    message_id: u64,
    request_message_id: u64,
    data: Option<&'a [u8]>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Error<'a> {
    message_id: u64,
    request_message_id: u64,
    kind: ErrorKind<'a>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ErrorKind<'a> {
    ServiceNotFound,
    MethodNotFound,
    // This is similar to an "HTTP 500 - Internal Server Error" except that
    // both client and server can actually respond with it to a request.
    ProviderError,
    Other(&'a [u8]),
}

impl<'a> Message<'a> {
    pub fn parse(input: &'a [u8]) -> Option<Self> {
        parse_message(input).ok().map(|t| t.1)
    }
}

fn parse_message(input: &[u8]) -> IResult<&[u8], Message> {
    alt((
        map(parse_heartbeat, Message::Heartbeat),
        map(parse_notification, Message::Notification),
        map(parse_request, Message::Request),
        map(parse_response, Message::Response),
        map(parse_error, Message::Error),
        map(parse_disconnect, |_| Message::Disconnect),
    ))(input)
}

fn parse_heartbeat(input: &[u8]) -> IResult<&[u8], Heartbeat> {
    map(preceded(tag("0 "), cut(parse_number)), |last_message_id| {
        Heartbeat { last_message_id }
    })(input)
}

fn parse_notification(input: &[u8]) -> IResult<&[u8], Notification> {
    map(
        preceded(
            tag("1 "),
            cut(tuple((
                parse_number,
                preceded(char(' '), parse_string),
                opt(preceded(char(' '), parse_rest)),
            ))),
        ),
        |(message_id, method_name, data)| Notification {
            message_id,
            method_name,
            data,
        },
    )(input)
}

fn parse_request(input: &[u8]) -> IResult<&[u8], Request> {
    map(
        preceded(
            tag("2 "),
            cut(tuple((
                parse_number,
                preceded(char(' '), parse_string),
                opt(preceded(char(' '), parse_rest)),
            ))),
        ),
        |(message_id, method_name, data)| Request {
            message_id,
            method_name,
            data,
        },
    )(input)
}

fn parse_response(input: &[u8]) -> IResult<&[u8], Response> {
    map(
        preceded(
            tag("3 "),
            cut(tuple((
                parse_number,
                preceded(char(' '), parse_number),
                opt(preceded(char(' '), parse_rest)),
            ))),
        ),
        |(message_id, request_message_id, data)| Response {
            message_id,
            request_message_id,
            data,
        },
    )(input)
}

fn parse_error(input: &[u8]) -> IResult<&[u8], Error> {
    map(
        preceded(
            tag("4 "),
            cut(tuple((
                parse_number,
                preceded(char(' '), parse_number),
                opt(preceded(char(' '), parse_error_kind)),
            ))),
        ),
        |(message_id, request_message_id, kind)| Error {
            message_id,
            request_message_id,
            kind: kind.unwrap_or(ErrorKind::ProviderError),
        },
    )(input)
}

fn parse_error_kind(input: &[u8]) -> IResult<&[u8], ErrorKind> {
    Ok((
        &input[input.len()..],
        match input {
            b"ServiceNotFound" => ErrorKind::ServiceNotFound,
            b"MethodNotFound" => ErrorKind::MethodNotFound,
            b"" => ErrorKind::ProviderError,
            other => ErrorKind::Other(other),
        },
    ))
}

fn parse_disconnect(input: &[u8]) -> IResult<&[u8], ()> {
    map(tag("-1"), |_| ())(input)
}

fn parse_string(input: &[u8]) -> IResult<&[u8], &str> {
    map_res(take_while_m_n(1, 255, |b| b != b' '), std::str::from_utf8)(input)
}

fn parse_number(input: &[u8]) -> IResult<&[u8], u64> {
    map_res(
        take_while_m_n(1, 16, |c| c >= b'0' && c <= b'9'),
        |s| match u64::from_str_radix(std::str::from_utf8(s).unwrap(), 10) {
            Ok(value) if value > MAX_SAFE_INTEGER => Err(InvalidNumber::MaxValueExceeded),
            Err(e) => Err(InvalidNumber::ParseIntError(e)),
            Ok(value) => Ok(value),
        },
    )(input)
}

fn parse_rest(input: &[u8]) -> IResult<&[u8], &[u8]> {
    Ok((&input[input.len()..], input))
}

#[test]
fn test_parse_heartbeat() {
    assert_eq!(
        Message::parse(b"0 42"),
        Some(Message::Heartbeat(Heartbeat {
            last_message_id: 42
        }))
    );
}

#[test]
fn test_parse_notification() {
    assert_eq!(
        Message::parse(b"1 43 hello world"),
        Some(Message::Notification(Notification {
            message_id: 43,
            method_name: "hello",
            data: Some(b"world"),
        }))
    );
}

#[test]
fn test_parse_notification_no_data() {
    assert_eq!(
        Message::parse(b"1 44 ping"),
        Some(Message::Notification(Notification {
            message_id: 44,
            method_name: "ping",
            data: None,
        }))
    );
}

#[test]
fn test_parse_request() {
    assert_eq!(
        Message::parse(b"2 45 add [4, 5]"),
        Some(Message::Request(Request {
            message_id: 45,
            method_name: "add",
            data: Some(b"[4, 5]"),
        }))
    );
}

#[test]
fn test_parse_request_no_data() {
    assert_eq!(
        Message::parse(b"2 46 get_time"),
        Some(Message::Request(Request {
            message_id: 46,
            method_name: "get_time",
            data: None,
        }))
    );
}

#[test]
fn test_parse_response() {
    assert_eq!(
        Message::parse(b"3 47 45 9"),
        Some(Message::Response(Response {
            message_id: 47,
            request_message_id: 45,
            data: Some(b"9"),
        }))
    );
}

#[test]
fn test_parse_response_no_data() {
    assert_eq!(
        Message::parse(b"3 48 46"),
        Some(Message::Response(Response {
            message_id: 48,
            request_message_id: 46,
            data: None,
        }))
    );
}

#[test]
fn test_parse_error_provider_error() {
    assert_eq!(
        Message::parse(b"4 48 46"),
        Some(Message::Error(Error {
            message_id: 48,
            request_message_id: 46,
            kind: ErrorKind::ProviderError,
        }))
    );
}

#[test]
fn test_parse_error_service_not_found() {
    assert_eq!(
        Message::parse(b"4 48 46 ServiceNotFound"),
        Some(Message::Error(Error {
            message_id: 48,
            request_message_id: 46,
            kind: ErrorKind::ServiceNotFound,
        }))
    );
}

#[test]
fn test_parse_error_method_not_found() {
    assert_eq!(
        Message::parse(b"4 48 46 MethodNotFound"),
        Some(Message::Error(Error {
            message_id: 48,
            request_message_id: 46,
            kind: ErrorKind::MethodNotFound,
        }))
    );
}

#[test]
fn test_parse_error_other() {
    assert_eq!(
        Message::parse(b"4 48 46 I'm a teapot!"),
        Some(Message::Error(Error {
            message_id: 48,
            request_message_id: 46,
            kind: ErrorKind::Other(b"I'm a teapot!"),
        }))
    );
}

#[test]
fn test_parse_disconnect() {
    assert_eq!(Message::parse(b"-1"), Some(Message::Disconnect));
}
