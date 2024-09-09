use actix_web::web;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize)]
pub struct Response {
    pub request_id: String,
    pub code: u32,
    pub msg: String,
    pub data: Value,
}

#[derive(Serialize, Deserialize)]
pub struct ProposerStatus {
    pub node_id: String,
    pub cpu_percent: String,
    pub cpu_nums: String,
    pub mem_total: String,
    pub mem_used: String,
}

pub fn make_resp_json(
    request_id: String,
    code: u32,
    msg: String,
    data: Value,
) -> web::Json<Response> {
    web::Json(Response {
        request_id,
        code,
        msg,
        data,
    })
}
