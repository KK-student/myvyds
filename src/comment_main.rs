use std::{
    ops::Deref,
    sync::{Arc, Mutex},
};

use actix_web::{HttpResponse, Responder, get, web::Query};
use serde::Deserialize;

use crate::{DATA_MANAGER, file_handling::comment::Comment};

#[derive(Deserialize)]
struct CommentIDQuery {
    comment_id: u64,
}

#[get("/comment/get_comment")]
///Implements "Getting a specific comment or reply".
async fn get_comment_endpoint(query: Query<CommentIDQuery>) -> impl Responder {
    let response404 = HttpResponse::NotFound().finish();

    let data_manager_mutg = DATA_MANAGER.lock().unwrap();
    let data_manager = data_manager_mutg.deref();

    let comment_arc: &Arc<Mutex<Comment>> = match data_manager.get_comment(&query.comment_id) {
        Some(a) => a,
        None => return response404,
    };
    let comment_mutg = comment_arc.lock().unwrap();
    let comment: &Comment = comment_mutg.deref();

    let result: String = comment.display_json_string(true);

    return HttpResponse::Ok().body(result);
}
