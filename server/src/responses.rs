use actix::Message;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) trait ResponseResult {
    fn result(&self) -> Option<serde_json::Value>;
}

pub(crate) trait ResponseError {
    fn error(&self) -> Option<serde_json::Value>;
}

pub trait JRPCErrorData {
    fn data(&self) -> Option<serde_json::Value>;
}

#[derive(Serialize, Deserialize)]
pub struct JRPCError {
    code: String,
    message: String,
    data: Option<Value>,
}

impl ResponseError for JRPCError {
    fn error(&self) -> Option<serde_json::Value> {
        Some(serde_json::to_value(self).unwrap())
    }
}

impl ResponseResult for () {
    fn result(&self) -> Option<serde_json::Value> {
        None
    }
}

impl ResponseError for () {
    fn error(&self) -> Option<serde_json::Value> {
        None
    }
}

impl<T: ResponseResult> ResponseResult for Option<T> {
    fn result(&self) -> Option<serde_json::Value> {
        match self {
            Some(result) => result.result(),
            None => None,
        }
    }
}

impl<T: ResponseError> ResponseError for Option<T> {
    fn error(&self) -> Option<serde_json::Value> {
        match self {
            Some(error) => error.error(),
            None => None,
        }
    }
}

#[derive(Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub struct JRPCResponse {
    pub jsonrpc: String,
    pub id: Option<usize>,
    pub result: Option<Value>,
    pub error: Option<Value>,
}

impl JRPCResponse {
    pub fn new(
        id: Option<usize>,
        result: Option<impl ResponseResult>,
        error: Option<impl ResponseError>,
    ) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: id,
            result: result.result(),
            error: error.error(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ChatMessageResult {
    pub message: String,
    pub recipient: usize,
}

impl ResponseResult for ChatMessageResult {
    fn result(&self) -> Option<Value> {
        Some(serde_json::to_value(self).unwrap())
    }
}

#[derive(Serialize, Deserialize)]
pub struct ConnectResult {
    pub id: usize,
}

impl ResponseResult for ConnectResult {
    fn result(&self) -> Option<Value> {
        Some(serde_json::to_value(self).unwrap())
    }
}

#[derive(Serialize, Deserialize)]
pub struct JoinResult {
    pub joined_user: usize,
}

impl ResponseResult for JoinResult {
    fn result(&self) -> Option<Value> {
        Some(serde_json::to_value(self).unwrap())
    }
}

#[derive(Serialize, Deserialize)]
pub struct JoinError<'a> {
    pub error_message: &'a str,
}

impl<'a> ResponseError for JoinError<'a> {
    fn error(&self) -> Option<serde_json::Value> {
        Some(serde_json::to_value(self).unwrap())
    }
}
