use crate::api::request::QuestionReq;
use crate::api::response::{make_resp_json, Response};
use crate::proposer::ProposerArc;
use actix_web::{post, web};
use serde_json::json;
use tracing::info;

/// WRITE API
#[post("/api/v1/test")]
async fn test(quest: web::Json<QuestionReq>, _prop: web::Data<ProposerArc>) -> web::Json<Response> {
    info!("Receive request, body = {:?}", quest);

    // validate parameter and signature
    
    let json_data = json!({});
    make_resp_json(quest.request_id.clone(), 0, String::new(), json_data)
}
