use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

use actix_multipart::Multipart;
use actix_web::{HttpResponse, Responder, get, http::header, mime, post, web::Query};
use serde::Deserialize;

use crate::{
    DATA_MANAGER, SESSION_MANAGER,
    file_handling::comment::Comment,
    parse_form_data,
    user_main::{authenticate, get_user_id_session_id},
    video_main::VideoIDQuery,
};

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

    return HttpResponse::Ok()
        .append_header(header::ContentType(mime::APPLICATION_JSON))
        .body(result);
}

#[post("/comment/add")]
///Implements "Making a comment".
async fn comment_add_endpoint(
    query: Query<VideoIDQuery>,
    mut payload: Multipart,
) -> impl Responder {
    let response401 = HttpResponse::Unauthorized().finish();
    let response404 = HttpResponse::NotFound().finish();

    //Get DATA_MANAGER:
    let mut data_manager_mutg = DATA_MANAGER.lock().unwrap();
    let data_manager = data_manager_mutg.deref_mut();

    //Get and very user_id and session_id:
    // let mut parsed_form_data: HashMap<String, Vec<u8>> = HashMap::new();
    // parse_form_data(&mut payload, &mut parsed_form_data).await;
    // let (user_id, session_id) = match get_user_id_session_id(&parsed_form_data) {
    //     Some(res) => res,
    //     None => return response401,
    // };
    // if !SESSION_MANAGER.verify_session(&user_id, session_id) {
    //     return response401;
    // }
    let (parsed_form_data, user_id) = match authenticate(&mut payload).await {
        Some(data) => data,
        None => return response401,
    };

    //Get video_id:
    let video_id: u64 = query.v;

    //Check if video_id is valid:
    if !data_manager.has_video_by_id(&video_id) {
        return response404;
    }

    //Make the comment:
    let new_text: String = match parsed_form_data.get("text") {
        Some(tv) => String::from_utf8_lossy(tv.as_slice()).to_string(),
        None => String::new(),
    };
    let comment_id: u64 = match data_manager.make_comment(&user_id, &video_id, &new_text) {
        Some(id) => id,
        None => return HttpResponse::InternalServerError().finish(),
    };

    //Return result:
    return HttpResponse::Ok()
        .append_header(header::ContentType(mime::APPLICATION_OCTET_STREAM))
        .body(comment_id.to_be_bytes().to_vec());
}

#[post("/comment/edit")]
///Implements "Editing a comment or reply".
async fn comment_edit_endpoint(
    query: Query<CommentIDQuery>,
    mut payload: Multipart,
) -> impl Responder {
    let response401 = HttpResponse::Unauthorized().finish();
    let response404 = HttpResponse::NotFound().finish();

    //Get DATA_MANAGER:
    let mut data_manager_mutg = DATA_MANAGER.lock().unwrap();
    let data_manager = data_manager_mutg.deref_mut();

    //Authenticate:
    let (parsed_form_data, user_id) = match authenticate(&mut payload).await {
        Some(data) => data,
        None => return response401,
    };

    //Get the text:
    let new_text: String = match parsed_form_data.get("new_text") {
        Some(tv) => String::from_utf8_lossy(tv.as_slice()).to_string(),
        None => String::new(),
    };

    //Edit comment:
    data_manager.edit_comment(&query.comment_id, &new_text);

    //Return:
    return HttpResponse::Ok().finish();
}

#[post("/comment/add_reply")]
///Implements "Adding a reply to a comment".
async fn comment_add_reply_endpoint(
    query: Query<CommentIDQuery>,
    mut payload: Multipart,
) -> impl Responder {
    let response401 = HttpResponse::Unauthorized().finish();
    let response404 = HttpResponse::NotFound().finish();
    let response409 = HttpResponse::Conflict().finish();

    //Get DATA_MANAGER:
    let mut data_manager_mutg = DATA_MANAGER.lock().unwrap();
    let data_manager = data_manager_mutg.deref_mut();

    //Authenticate:
    let (parsed_form_data, user_id) = match authenticate(&mut payload).await {
        Some(data) => data,
        None => return response401,
    };

    //Check if comment exists:
    if !data_manager.has_comment(&query.comment_id) {
        return response404;
    }

    //Get the text:
    let text: String = match parsed_form_data.get("text") {
        Some(tv) => String::from_utf8_lossy(tv.as_slice()).to_string(),
        None => String::new(),
    };

    //Add the reply:
    let reply_cid: u64 = match data_manager.reply_comment(&query.comment_id, &text, &user_id) {
        Some(id) => id,
        None => return response409,
    };

    //Return:
    return HttpResponse::Ok()
        .append_header(header::ContentType(mime::APPLICATION_OCTET_STREAM))
        .body(reply_cid.to_be_bytes().to_vec());
}

#[post("/comment/remove")]
///Implements "Removing a comment or reply".
async fn comment_remove_endpoint(
    query: Query<CommentIDQuery>,
    mut payload: Multipart,
) -> impl Responder {
    let response401 = HttpResponse::Unauthorized().finish();
    let response404 = HttpResponse::NotFound().finish();

    //Get DATA_MANAGER:
    let mut data_manager_mutg = DATA_MANAGER.lock().unwrap();
    let data_manager = data_manager_mutg.deref_mut();

    //Authenticate:
    let (parsed_form_data, user_id) = match authenticate(&mut payload).await {
        Some(data) => data,
        None => return response401,
    };

    //Check if comment exists:
    if !data_manager.has_comment(&query.comment_id) {
        return response404;
    }

    //Check if user is authorized to delete the comment:
    {
        let comment_arc = data_manager.get_comment(&query.comment_id).unwrap();
        let comment_mutg = comment_arc.lock().unwrap();
        let comment: &Comment = comment_mutg.deref();
        if user_id != comment.get_author() {
            return response401;
        }
    }

    //Remove comment:
    data_manager.remove_comment(&query.comment_id);

    return HttpResponse::Ok().finish();
}
