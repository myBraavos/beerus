use serde::{Deserialize, Serialize};

use axum::{extract::State, http::StatusCode, Json};

use crate::{
    rpc::context::Context, storage::storage_trait::StorageProviderTrait,
};

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum Request {
    Single(iamgroot::jsonrpc::Request),
    Batch(Vec<iamgroot::jsonrpc::Request>),
}

#[derive(Default, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Response {
    #[default]
    Empty,
    Single(iamgroot::jsonrpc::Response),
    Batch(Vec<iamgroot::jsonrpc::Response>),
}

/// Handle incoming JSON-RPC requests
pub async fn handle_request<S: StorageProviderTrait>(
    State(ctx): State<Context<S>>,
    Json(req): Json<Request>,
) -> Result<Json<Response>, (StatusCode, String)> {
    let res = match req {
        Request::Single(req) => {
            let res = crate::gen::handle(&ctx, &req).await;
            if req.id.is_some() {
                Ok(Json(Response::Single(res)))
            } else {
                Ok(Json::default()) // no response for notifications
            }
        }
        Request::Batch(reqs) => {
            let mut ret = Vec::with_capacity(reqs.len());
            for req in reqs {
                let ctx = ctx.clone();
                let res = crate::gen::handle(&ctx, &req).await;
                if req.id.is_some() {
                    ret.push(res);
                }
            }
            Ok(Json(Response::Batch(ret)))
        }
    };

    res
}
