// SPDX-License-Identifier: Apache-2.0

//! This module defines the json format for `solang compile --standard-json`.

use crate::abi::ethereum::ABI;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
pub struct EwasmContract {
    pub wasm: String,
}

#[derive(Serialize)]
pub struct JsonContract {
    pub abi: Vec<ABI>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ewasm: Option<EwasmContract>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_space: Option<u32>,
}

#[derive(Serialize)]
pub struct JsonResult {
    pub errors: Vec<OutputJson>,
    pub target: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub program: String,
    pub contracts: HashMap<String, HashMap<String, JsonContract>>,
}

#[derive(Serialize)]
pub struct LocJson {
    pub file: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
pub struct OutputJson {
    pub sourceLocation: Option<LocJson>,
    #[serde(rename = "type")]
    pub ty: String,
    pub component: String,
    pub severity: String,
    pub message: String,
    pub formattedMessage: String,
}
