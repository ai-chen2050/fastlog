use crate::api::response::{make_resp_json, ProposerStatus, Response};
use crate::proposer::ProposerArc;
use actix_web::{get, web, HttpRequest};
use node_api::error::ErrorCodes;
use node_api::error::ProposerAPIError::APIFailToJson;
use tools::helper::machine_used;

pub async fn not_found(_: web::Data<ProposerArc>, request: HttpRequest) -> String {
    format!("Not support api {}", request.uri())
}

/// READ API
#[get("/")]
async fn index() -> String {
    format!("Welcome to visit aos proposer node! \n")
}

#[get("/api/v1/status")]
async fn status(_req: HttpRequest, op: web::Data<ProposerArc>) -> web::Json<Response> {
    let (cpu_percent, cpu_nums, memory_total, memory_used) = machine_used();

    let resp_data = ProposerStatus {
        node_id: op.config.node.node_id.clone(),
        cpu_percent: format!("{:.2}%", cpu_percent),
        cpu_nums: format!("{} cores", cpu_nums),
        mem_total: format!("{} M", memory_total / 1024 / 1024),
        mem_used: format!("{} M", memory_used / 1024 / 1024),
    };

    let json_data = serde_json::to_value(&resp_data);

    match json_data {
        Err(_err) => make_resp_json(
            String::new(),
            ErrorCodes::API_FAIL_TO_JSON,
            APIFailToJson.to_string(),
            serde_json::Value::default(),
        ),
        Ok(json_value) => make_resp_json(String::new(), 0, String::new(), json_value),
    }
}
