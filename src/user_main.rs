use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex},
};

use actix_multipart::Multipart;
use actix_web::{HttpResponse, Responder, get, http::header, mime, post, web::Query};
use argon2::{Argon2, PasswordHasher};
use serde::Deserialize;

use crate::{
    DATA_MANAGER, DATA_PATH, SESSION_MANAGER,
    file_handling::{read_small_file, saveable::make_json_list_from_u64_list, user::User},
    parse_form_data,
};

///Gets the user_id and session_id from the given parsed form data (form_data). Returns None if either user_id or session_id was not found or if session_id is not a valid u64 number.
pub fn get_user_id_session_id(form_data: &HashMap<String, Vec<u8>>) -> Option<(String, u64)> {
    let user_id: String = match form_data.get(&"username".to_string()) {
        Some(id) => String::from_utf8_lossy(id.as_slice()).to_string(),
        None => return None,
    };
    let session_id_string: String = match form_data.get(&"session_id".to_string()) {
        Some(id) => String::from_utf8_lossy(id.as_slice()).to_string(),
        None => return None,
    };
    let session_id: u64 = match session_id_string.parse::<u64>() {
        Ok(id) => id,
        Err(_) => return None,
    };
    return Some((user_id, session_id));
}

pub async fn authenticate(payload: &mut Multipart) -> Option<(HashMap<String, Vec<u8>>, String)> {
    //Get DATA_MANAGER:
    // let data_manager_mutg = DATA_MANAGER.lock().unwrap();
    // let data_manager = data_manager_mutg.deref();

    //Get and very user_id and session_id:
    let mut parsed_form_data: HashMap<String, Vec<u8>> = HashMap::new();
    parse_form_data(payload, &mut parsed_form_data).await;
    let (user_id, session_id) = match get_user_id_session_id(&parsed_form_data) {
        Some(res) => res,
        None => return None,
    };
    if !SESSION_MANAGER.verify_session(&user_id, session_id) {
        return None;
    }

    return Some((parsed_form_data, user_id));
}

#[derive(Deserialize)]
struct UserIdQuery {
    user_id: String,
}
#[get("/user/get_public_info")]
///Getting publicly-available user information:
async fn get_public_info(query: Query<UserIdQuery>) -> impl Responder {
    let response404 = HttpResponse::NotFound().finish();

    let data_manager_mutg = DATA_MANAGER.lock().unwrap();
    let data_manager = data_manager_mutg.deref();

    let user_id: &String = &query.user_id;
    let user_arc = match data_manager.get_user(user_id) {
        Some(user_a) => user_a,
        None => return response404,
    };
    let user_mutg = user_arc.lock().unwrap();
    let user: &User = user_mutg.deref();

    let description_escaped: String =
        json_escape::escape_str(user.get_description().as_str()).collect::<String>();
    let videos_string: String = make_json_list_from_u64_list(user.get_videos().deref());
    let comments_string: String = make_json_list_from_u64_list(user.get_comments().deref());
    let json_result: String = format!(
        "{{\"time_created\": {}, \"description\": \"{}\", \"videos\": {}, \"comments\": {}}}",
        user.get_time_created(),
        description_escaped,
        videos_string,
        comments_string
    );

    return HttpResponse::Ok()
        .append_header(header::ContentType(mime::APPLICATION_JSON))
        .body(json_result);
}

#[get("/user/get_user_icon")]
///Getting user icon:
async fn get_user_icon(query: Query<UserIdQuery>) -> impl Responder {
    let response404: HttpResponse = HttpResponse::NotFound().finish();

    //Get user:
    let data_manager_mutg = DATA_MANAGER.lock().unwrap();
    let data_manager = data_manager_mutg.deref();
    let user_arc: Arc<Mutex<User>> = match data_manager.get_user(&query.user_id) {
        Some(u_arc) => u_arc,
        None => return response404,
    };
    let user_mutg = user_arc.lock().unwrap();
    let user: &User = user_mutg.deref();

    //Get user image:
    let mut icon_image_path: PathBuf = match PathBuf::from_str(DATA_PATH) {
        Ok(buf) => buf,
        Err(_) => return response404,
    };
    icon_image_path.push("./user_data");
    icon_image_path.push(format!("./user-icon-{}.png", user.get_icon_id()));
    let icon_image = match read_small_file(match icon_image_path.to_str() {
        Some(path_str) => path_str,
        None => return response404,
    }) {
        Ok(image) => image,
        Err(_) => return response404,
    };

    return HttpResponse::Ok()
        .append_header(header::ContentType(mime::IMAGE_PNG))
        .body(icon_image);
}

///Gets the user id (username field) and password (password field) from the given parsed form data (form_data). Returns None if either user_id or session_id was not found or if session_id is not a valid u64 number.
fn get_login_data(form_data: &HashMap<String, Vec<u8>>) -> Option<(String, String)> {
    let user_id: String = match form_data.get(&"username".to_string()) {
        Some(id) => String::from_utf8_lossy(id.as_slice()).to_string(),
        None => return None,
    };
    let password: String = match form_data.get(&"password".to_string()) {
        Some(id) => String::from_utf8_lossy(id.as_slice()).to_string(),
        None => return None,
    };
    return Some((user_id, password));
}

///Logging in:
#[post("/user/login")]
async fn login_endpoint(mut payload: Multipart) -> impl Responder {
    let response400 = HttpResponse::BadRequest().finish();

    //Getting username and password:
    let mut parsed_data: HashMap<String, Vec<u8>> = HashMap::new();
    parse_form_data(&mut payload, &mut parsed_data).await;
    let (user_id, password) = match get_login_data(&parsed_data) {
        Some(res) => res,
        None => return response400,
    };

    //Check if username and password is valid. If so, login with SESSION_MANAGER.
    {
        let data_manager_mutg = DATA_MANAGER.lock().unwrap();
        let data_manager = data_manager_mutg.deref();
        if !data_manager.has_user(&user_id) {
            return response400;
        }
    }
    let session_id: u64 = match SESSION_MANAGER.login(&user_id, &password) {
        Ok(id) => id,
        Err(_) => return response400,
    };

    return HttpResponse::Ok()
        .append_header(header::ContentType(mime::APPLICATION_OCTET_STREAM))
        .body(session_id.to_be_bytes().to_vec());
}

#[post("/user/signup")]
///Signing up:
async fn signup(mut payload: Multipart) -> impl Responder {
    let response400 = HttpResponse::BadRequest().finish();

    //Getting username and password:
    let mut parsed_data: HashMap<String, Vec<u8>> = HashMap::new();
    parse_form_data(&mut payload, &mut parsed_data).await;
    let (user_id, password) = match get_login_data(&parsed_data) {
        Some(res) => res,
        None => return response400,
    };

    {
        //Check if there is any user with the given username.
        let mut data_manager_mutg = DATA_MANAGER.lock().unwrap();
        let data_manager = data_manager_mutg.deref_mut();
        if data_manager.has_user(&user_id) {
            return response400;
        }

        //Create the password hash:
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes())
            .unwrap()
            .to_string();

        //Create new user:
        data_manager.add_user(&user_id, &password_hash, &"".to_string(), 0);
    }

    //Get session id:
    let session_id: u64 = match SESSION_MANAGER.login(&user_id, &password) {
        Ok(id) => id,
        Err(_) => return response400,
    };

    return HttpResponse::Ok()
        .append_header(header::ContentType(mime::APPLICATION_OCTET_STREAM))
        .body(session_id.to_be_bytes().to_vec());
}

#[post("/user/get_subscriptions")]
///Implements "Getting user's subscriptions".
async fn get_subscriptions(mut payload: Multipart) -> impl Responder {
    let response401 = HttpResponse::Unauthorized().finish();

    //Get DATA_MANAGER:
    let data_manager_mutg = DATA_MANAGER.lock().unwrap();
    let data_manager = data_manager_mutg.deref();

    // //Get and very user_id and session_id:
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

    //Get user object:
    let user_arc: Arc<Mutex<User>> = data_manager.get_user(&user_id).unwrap();
    let user_mutg = user_arc.lock().unwrap();
    let user = user_mutg.deref();

    //Return result:
    let result: String = format!("{{\"subscriptions\": {:?}}}", user.get_subscriptions());
    return HttpResponse::Ok()
        .append_header(header::ContentType(mime::APPLICATION_JSON))
        .body(result);
}

#[post("/user/get_liked_videos")]
///Implements "Getting user's liked videos".
async fn get_liked_videos(mut payload: Multipart) -> impl Responder {
    let response401 = HttpResponse::Unauthorized().finish();

    //Get DATA_MANAGER:
    let data_manager_mutg = DATA_MANAGER.lock().unwrap();
    let data_manager = data_manager_mutg.deref();

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

    //Get user object:
    let user_arc: Arc<Mutex<User>> = data_manager.get_user(&user_id).unwrap();
    let user_mutg = user_arc.lock().unwrap();
    let user = user_mutg.deref();

    //Return result:
    let result: String = format!("{{\"liked_videos\": {:?}}}", user.get_liked_vids());
    return HttpResponse::Ok()
        .append_header(header::ContentType(mime::APPLICATION_JSON))
        .body(result);
}

#[post("/user/get_disliked_videos")]
///Implements "Getting user's disliked videos".
async fn get_disliked_videos(mut payload: Multipart) -> impl Responder {
    let response401 = HttpResponse::Unauthorized().finish();

    //Get DATA_MANAGER:
    let data_manager_mutg = DATA_MANAGER.lock().unwrap();
    let data_manager = data_manager_mutg.deref();

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

    //Get user object:
    let user_arc: Arc<Mutex<User>> = data_manager.get_user(&user_id).unwrap();
    let user_mutg = user_arc.lock().unwrap();
    let user = user_mutg.deref();

    //Return result:
    let result: String = format!("{{\"disliked_videos\": {:?}}}", user.get_disliked_vids());
    return HttpResponse::Ok()
        .append_header(header::ContentType(mime::APPLICATION_JSON))
        .body(result);
}

#[post("/user/get_user_comments")]
///Implements "Getting user's liked comments and replies".
async fn get_user_comments(mut payload: Multipart) -> impl Responder {
    let response401 = HttpResponse::Unauthorized().finish();

    //Get DATA_MANAGER:
    let data_manager_mutg = DATA_MANAGER.lock().unwrap();
    let data_manager = data_manager_mutg.deref();

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

    //Get user object:
    let user_arc: Arc<Mutex<User>> = data_manager.get_user(&user_id).unwrap();
    let user_mutg = user_arc.lock().unwrap();
    let user = user_mutg.deref();

    //Return result:
    let result: String = format!("{{\"user_comments\": {:?}}}", user.get_comments());
    return HttpResponse::Ok()
        .append_header(header::ContentType(mime::APPLICATION_JSON))
        .body(result);
}

#[post("/user/get_liked_comments")]
///Implements "Getting user's liked comments and replies".
async fn get_liked_comments(mut payload: Multipart) -> impl Responder {
    let response401 = HttpResponse::Unauthorized().finish();

    //Get DATA_MANAGER:
    let data_manager_mutg = DATA_MANAGER.lock().unwrap();
    let data_manager = data_manager_mutg.deref();

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

    //Get user object:
    let user_arc: Arc<Mutex<User>> = data_manager.get_user(&user_id).unwrap();
    let user_mutg = user_arc.lock().unwrap();
    let user = user_mutg.deref();

    //Return result:
    let result: String = format!("{{\"liked_comments\": {:?}}}", user.get_liked_comments());
    return HttpResponse::Ok()
        .append_header(header::ContentType(mime::APPLICATION_JSON))
        .body(result);
}

#[post("/user/get_disliked_comments")]
///Implements "Getting user's disliked comments and replies".
async fn get_disliked_comments(mut payload: Multipart) -> impl Responder {
    let response401 = HttpResponse::Unauthorized().finish();

    //Get DATA_MANAGER:
    let data_manager_mutg = DATA_MANAGER.lock().unwrap();
    let data_manager = data_manager_mutg.deref();

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

    //Get user object:
    let user_arc: Arc<Mutex<User>> = data_manager.get_user(&user_id).unwrap();
    let user_mutg = user_arc.lock().unwrap();
    let user = user_mutg.deref();

    //Return result:
    let result: String = format!(
        "{{\"disliked_comments\": {:?}}}",
        user.get_disliked_comments()
    );
    return HttpResponse::Ok()
        .append_header(header::ContentType(mime::APPLICATION_JSON))
        .body(result);
}
