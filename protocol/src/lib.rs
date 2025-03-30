use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum Error {
    Message(String),
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum EvalResponseType {
    Void,
    Int,
    String,
    Bool,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum EvalResponse {
    Void,
    Int(i32),
    String(String),
    Bool(bool),
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct EvalRequest {
    pub source: String,
    pub response_type: EvalResponseType,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct RegisterRequest {
    pub name: String,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct CallRegisteredRequest {
    pub name: String,
    pub params: Vec<CallRegisteredParam>,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum CallRegisteredParam {
    Int(i32),
    String(String),
    Bool(bool),
}
