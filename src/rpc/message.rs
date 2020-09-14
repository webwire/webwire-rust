//! This module contains the definitions and parsing functions of
//! the webwire protocol messages.

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while_m_n},
    character::complete::char,
    combinator::{cut, map, map_res, opt},
    sequence::{preceded, tuple},
    IResult,
};

use bytes::{BufMut, Bytes, BytesMut};

// 2^53-1 is the maximum valid message_id as this is the highest
// integer that can be represented in JavaScript. This should be
// plenty though.
const MAX_SAFE_INTEGER: u64 = 9007199254740991; // = 2^53-1

/// This error is returned when parsing fails
pub enum ParseError {
    /// A std::num::ParseIntError occured
    ParseIntError(std::num::ParseIntError),
    /// The maximum value of 9007199254740991 (2^53-1) was exceeded
    /// while trying to parse a number.
    MaxValueExceeded,
    /// The method did not contain at least two identifiers
    /// so a service and method name could be extracted.
    InvalidServiceMethod,
}

/// A parsed message
#[derive(Debug, Eq, PartialEq)]
pub enum Message {
    /// Heartbeat
    Heartbeat(Heartbeat),
    /// Request or Notification
    Request(Request),
    /// Response or error response
    Response(Response),
    /// Disconnect
    Disconnect,
}

/// A parsed heartbeat message
#[derive(Debug, Eq, PartialEq)]
pub struct Heartbeat {
    /// The ID of the last received message
    pub last_message_id: u64,
}

/// A parsed request message or notification
#[derive(Debug, Eq, PartialEq)]
pub struct Request {
    /// This boolean is set if that request is actually a
    /// notification.
    pub is_notification: bool,
    /// The ID of this message
    pub message_id: u64,
    /// The service name this notification is trying to call
    pub service: String,
    /// The method name this notification is trying to call
    pub method: String,
    /// The message payload
    pub data: Bytes,
}

/// A parsed response message
#[derive(Debug, Eq, PartialEq)]
pub struct Response {
    /// The ID of this message
    pub message_id: u64,
    /// The message ID of the request this response belongs to.
    pub request_message_id: u64,
    /// The message payload
    pub data: Result<Bytes, ErrorKind>,
}

/// A parsed error response
#[derive(Debug, Eq, PartialEq)]
pub enum ErrorKind {
    /// This error is returned if the target service was not found.
    ServiceNotFound,
    /// This error is returned if the target method was not found.
    MethodNotFound,
    /// This is similar to an "HTTP 500 - Internal Server Error" except that
    /// both client and server can actually respond with it to a request.
    ProviderError,
    /// The remote side responded with an unknown error.
    Other(Bytes),
}

impl Message {
    /// Parse the given input buffer into a Message object.
    pub fn parse(input: &Bytes) -> Option<Self> {
        parse_message(input).ok().map(|t| t.1)
    }
}

impl Heartbeat {
    /// Serialize this heartbeat into `Bytes`
    pub fn to_bytes(&self) -> Bytes {
        Bytes::from(format!("0 {}", self.last_message_id))
    }
}

impl Request {
    /// Serialize this request into `Bytes`
    pub fn to_bytes(&self) -> Bytes {
        let code = if self.is_notification { 1 } else { 2 };
        let header = format!("{} {} {}.{}", code, self.message_id, self.service, self.method);
        let capacity = header.len() + 1 + self.data.len();
        let mut buf = BytesMut::with_capacity(capacity);
        buf.put(header.as_bytes());
        if !self.data.is_empty() {
            buf.put_slice(b" ");
            buf.put_slice(&self.data);
        }
        buf.into()
    }
}

impl Response {
    /// Serialzie this response into `Bytes`
    pub fn to_bytes(&self) -> Bytes {
        let (code, data) = match &self.data {
            Ok(data) => ('3', data.clone()),
            Err(e) => (
                '4',
                match e {
                    ErrorKind::ServiceNotFound => Bytes::from("ServiceNotFound"),
                    ErrorKind::MethodNotFound => Bytes::from("MethodNotFound"),
                    ErrorKind::ProviderError => Bytes::from(""),
                    ErrorKind::Other(data) => data.clone(),
                },
            ),
        };
        let header = format!("{} {} {}", code, self.message_id, self.request_message_id);
        let capacity = header.len() + 1 + data.len();
        let mut buf = BytesMut::with_capacity(capacity);
        buf.put(header.as_bytes());
        if !data.is_empty() {
            buf.put_slice(b" ");
            buf.put(data);
        }
        buf.into()
    }
}

fn _get_data(bytes: &Bytes, slice: &[u8]) -> Bytes {
    bytes.slice((slice.as_ptr() as usize - bytes.as_ptr() as usize)..)
}

fn parse_message(input: &Bytes) -> IResult<&[u8], Message> {
    alt((
        map(parse_heartbeat, Message::Heartbeat),
        map(
            parse_notification,
            |(message_id, (service, method), data)| {
                Message::Request(Request {
                    is_notification: true,
                    message_id,
                    service: service.to_owned(),
                    method: method.to_owned(),
                    data: data.map(|data| _get_data(input, data)).unwrap_or_default(),
                })
            },
        ),
        map(parse_request, |(message_id, (service, method), data)| {
            Message::Request(Request {
                is_notification: false,
                message_id,
                service: service.to_owned(),
                method: method.to_owned(),
                data: data.map(|data| _get_data(input, data)).unwrap_or_default(),
            })
        }),
        map(parse_response, |(message_id, request_message_id, data)| {
            Message::Response(Response {
                message_id,
                request_message_id,
                data: Ok(data.map(|data| _get_data(input, data)).unwrap_or_default()),
            })
        }),
        map(parse_error, |(message_id, request_message_id, kind)| {
            Message::Response(Response {
                message_id,
                request_message_id,
                data: Err(match kind {
                    Ok(e) => e,
                    Err(e) => ErrorKind::Other(_get_data(input, e)),
                }),
            })
        }),
        map(parse_disconnect, |_| Message::Disconnect),
    ))(input)
}

fn parse_heartbeat(input: &[u8]) -> IResult<&[u8], Heartbeat> {
    map(preceded(tag("0 "), cut(parse_number)), |last_message_id| {
        Heartbeat { last_message_id }
    })(input)
}

fn parse_notification(input: &[u8]) -> IResult<&[u8], (u64, (&str, &str), Option<&[u8]>)> {
    preceded(
        tag("1 "),
        cut(tuple((
            parse_number,
            preceded(char(' '), parse_service_method),
            opt(preceded(char(' '), parse_rest)),
        ))),
    )(input)
}

fn parse_request(input: &[u8]) -> IResult<&[u8], (u64, (&str, &str), Option<&[u8]>)> {
    preceded(
        tag("2 "),
        cut(tuple((
            parse_number,
            preceded(char(' '), parse_service_method),
            opt(preceded(char(' '), parse_rest)),
        ))),
    )(input)
}

fn parse_response(input: &[u8]) -> IResult<&[u8], (u64, u64, Option<&[u8]>)> {
    preceded(
        tag("3 "),
        cut(tuple((
            parse_number,
            preceded(char(' '), parse_number),
            opt(preceded(char(' '), parse_rest)),
        ))),
    )(input)
}

fn parse_error(input: &[u8]) -> IResult<&[u8], (u64, u64, Result<ErrorKind, &[u8]>)> {
    preceded(
        tag("4 "),
        cut(tuple((
            parse_number,
            preceded(char(' '), parse_number),
            map(
                opt(preceded(char(' '), parse_error_kind)),
                |opt| match opt {
                    Some(o) => o,
                    None => Ok(ErrorKind::ProviderError),
                },
            ),
        ))),
    )(input)
}

fn parse_error_kind(input: &[u8]) -> IResult<&[u8], Result<ErrorKind, &[u8]>> {
    Ok((
        &input[input.len()..],
        match input {
            b"ServiceNotFound" => Ok(ErrorKind::ServiceNotFound),
            b"MethodNotFound" => Ok(ErrorKind::MethodNotFound),
            b"" => Ok(ErrorKind::ProviderError),
            other => Err(other),
        },
    ))
}

fn parse_disconnect(input: &[u8]) -> IResult<&[u8], ()> {
    map(tag("-1"), |_| ())(input)
}

fn parse_string(input: &[u8]) -> IResult<&[u8], &str> {
    map_res(take_while_m_n(1, 255, |b| b != b' '), std::str::from_utf8)(input)
}

fn parse_service_method(input: &[u8]) -> IResult<&[u8], (&str, &str)> {
    map_res(parse_string, |name| {
        println!("splitting name: {}", name);
        let parts = name.rsplitn(2, '.').collect::<Vec<_>>();
        if parts.len() == 2 {
            Ok((parts[1], parts[0]))
        } else {
            Err(ParseError::InvalidServiceMethod)
        }
    })(input)
}

fn parse_number(input: &[u8]) -> IResult<&[u8], u64> {
    map_res(
        take_while_m_n(1, 16, |c| c >= b'0' && c <= b'9'),
        |s| match u64::from_str_radix(std::str::from_utf8(s).unwrap(), 10) {
            Ok(value) if value > MAX_SAFE_INTEGER => Err(ParseError::MaxValueExceeded),
            Err(e) => Err(ParseError::ParseIntError(e)),
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
        Message::parse(&Bytes::from("0 42")),
        Some(Message::Heartbeat(Heartbeat {
            last_message_id: 42
        }))
    );
}

#[test]
fn test_parse_notification() {
    assert_eq!(
        Message::parse(&Bytes::from("1 43 example.hello world")),
        Some(Message::Request(Request {
            is_notification: true,
            message_id: 43,
            service: "example".to_string(),
            method: "hello".to_string(),
            data: Bytes::from("world"),
        }))
    );
}

#[test]
fn test_parse_notification_no_data() {
    assert_eq!(
        Message::parse(&Bytes::from("1 44 example.ping")),
        Some(Message::Request(Request {
            is_notification: true,
            message_id: 44,
            service: "example".to_string(),
            method: "ping".to_string(),
            data: Bytes::default(),
        }))
    );
}

#[test]
fn test_parse_request() {
    assert_eq!(
        Message::parse(&Bytes::from("2 45 example.add [4, 5]")),
        Some(Message::Request(Request {
            is_notification: false,
            message_id: 45,
            service: "example".to_string(),
            method: "add".to_string(),
            data: Bytes::from("[4, 5]"),
        }))
    );
}

#[test]
fn test_parse_request_no_data() {
    assert_eq!(
        Message::parse(&Bytes::from("2 46 example.get_time")),
        Some(Message::Request(Request {
            is_notification: false,
            message_id: 46,
            service: "example".to_string(),
            method: "get_time".to_string(),
            data: Bytes::default(),
        }))
    );
}

#[test]
fn test_parse_response() {
    assert_eq!(
        Message::parse(&Bytes::from("3 47 45 9")),
        Some(Message::Response(Response {
            message_id: 47,
            request_message_id: 45,
            data: Ok(Bytes::from("9")),
        }))
    );
}

#[test]
fn test_parse_response_no_data() {
    assert_eq!(
        Message::parse(&Bytes::from("3 48 46")),
        Some(Message::Response(Response {
            message_id: 48,
            request_message_id: 46,
            data: Ok(Bytes::default()),
        }))
    );
}

#[test]
fn test_parse_error_provider_error() {
    assert_eq!(
        Message::parse(&Bytes::from("4 48 46")),
        Some(Message::Response(Response {
            message_id: 48,
            request_message_id: 46,
            data: Err(ErrorKind::ProviderError),
        }))
    );
}

#[test]
fn test_parse_error_service_not_found() {
    assert_eq!(
        Message::parse(&Bytes::from("4 48 46 ServiceNotFound")),
        Some(Message::Response(Response {
            message_id: 48,
            request_message_id: 46,
            data: Err(ErrorKind::ServiceNotFound),
        }))
    );
}

#[test]
fn test_parse_error_method_not_found() {
    assert_eq!(
        Message::parse(&Bytes::from("4 48 46 MethodNotFound")),
        Some(Message::Response(Response {
            message_id: 48,
            request_message_id: 46,
            data: Err(ErrorKind::MethodNotFound),
        }))
    );
}

#[test]
fn test_parse_error_other() {
    assert_eq!(
        Message::parse(&Bytes::from("4 48 46 I'm a teapot!")),
        Some(Message::Response(Response {
            message_id: 48,
            request_message_id: 46,
            data: Err(ErrorKind::Other(Bytes::from("I'm a teapot!"))),
        }))
    );
}

#[test]
fn test_parse_disconnect() {
    assert_eq!(
        Message::parse(&Bytes::from("-1")),
        Some(Message::Disconnect)
    );
}
