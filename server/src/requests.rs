use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug)]
pub struct JRPCRequest<'a> {
    pub jsonrpc: &'a str,
    pub method: &'a str,
    pub params: Option<Value>,
    pub id: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JRPCJoinRequestParams {
    pub recipient: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JRPCMessageRequestParams {
    pub message: String,
    pub recipient: usize,
}
