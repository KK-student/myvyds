///This is a video sharing platform website, similar to YouTube.
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex},
};

use actix_multipart::{Field, Multipart};
use futures::TryStreamExt;
use serde::Deserialize;

use actix_web::{
    App, HttpResponse, HttpServer, Responder, get,
    http::header,
    mime, post,
    web::{self, Bytes, Query},
};

use crate::{
    file_handling::{
        comment::Comment,
        describable::Describable,
        file_handling::{DataManager, read_small_file},
        likeable::Likeable,
        saveable::make_json_list_from_u64_list,
        user::User,
        video_processing::VideoProcessorManager,
    },
    management_console::{
        ManagementConsole, text_based_management_console::TextBasedManagementConsole,
    },
    session_manager::SessionManager,
};

use crate::file_handling::video_data::VideoData;

mod file_handling;
mod management_console;
mod session_manager;
mod time;

// #[get("/")]
// async fn hello() -> impl Responder {
//     HttpResponse::Ok().body("Hello world!")
// }
//
// async fn manual_hello() -> impl Responder {
//     HttpResponse::Ok().body("Hey there!")
// }

// async fn get_lofi_music() -> impl Responder {
//     let file_path_str = "./media/coding music synthwave beats to program to-fhL67fnDXcU-av.webm";
//     let reader_stream: ReaderStream<tokio::fs::File> = match file_stream_reader(file_path_str).await
//     {
//         Ok(r) => r,
//         Err(e) => {
//             return HttpResponse::Ok()
//                 .body(format!("<!DOCTYPE html><html><head><title>Error</title></head><body><p>Error: {}</p></body></html>", e.to_string()));
//         }
//     };
//     return HttpResponse::Ok()
//         .content_type(get_mime_type(std::path::Path::new(file_path_str)))
//         //.insert_header(ContentDisposition::attachment(String::from(filename)))
//         .streaming(reader_stream);
// }

pub fn _get_user_for_session_manager(user_id: &String, password_hash: &String) -> bool {
    let data_manager_mutg = DATA_MANAGER.lock().unwrap();
    let data_manager = data_manager_mutg.deref();
    let user_arc = match data_manager.get_user(user_id) {
        Some(u_arc) => u_arc,
        None => return false,
    };
    let user_mutg = user_arc.lock().unwrap();
    let user = user_mutg.deref();
    return user.check_password_hash(password_hash);
}

static _GET_USER_FOR_SESSION_MANAGER_FUNC: fn(&String, &String) -> bool =
    _get_user_for_session_manager;

static DATA_PATH: &str = "./data";
static DATA_MANAGER: std::sync::LazyLock<Mutex<DataManager>> =
    std::sync::LazyLock::new(|| -> Mutex<DataManager> { Mutex::new(DataManager::new(DATA_PATH)) });
static PROCESSOR_MANAGER: VideoProcessorManager = VideoProcessorManager::new(DATA_PATH);
static MANAGEMENT_CONSOLE: TextBasedManagementConsole =
    TextBasedManagementConsole::new(&DATA_MANAGER, &PROCESSOR_MANAGER);
static SESSION_MANAGER: SessionManager<fn(&String, &String) -> bool> =
    SessionManager::new(&_GET_USER_FOR_SESSION_MANAGER_FUNC);

///Gets a specific video file, serving the "/video/get_video_file/[video_id]/[video_seg_file]" endpoint. See the "Getting a video" section in the "./docs/SERVER_API.md" for more details.
#[get("/video/get_video_file/{video_id}/{video_seg_file}")]
async fn get_video_file(url_params: web::Path<(String, String)>) -> impl Responder {
    //404 response:
    let response_e404 = HttpResponse::NotFound().finish();
    //Extract parameters:
    let (video_id_string, vid_seg_file_tpath_string) = url_params.into_inner();

    //Check if video_id is valid:
    let video_id: u64 = match video_id_string.parse::<u64>() {
        Ok(r) => r,
        Err(_) => return response_e404,
    };
    {
        let data_manager_guard = DATA_MANAGER.lock().unwrap();
        let data_manager: &DataManager = data_manager_guard.deref();
        if !data_manager.has_video_by_id(&video_id) {
            return response_e404;
        }
    }

    //Get video file:
    let file_path_string = format!(
        "{}/media/{}/{}",
        DATA_PATH, video_id, vid_seg_file_tpath_string
    );
    match read_small_file(file_path_string.as_str()) {
        Ok(file_contents) => HttpResponse::Ok().body(file_contents),
        Err(_) => response_e404,
    }
}

#[derive(Deserialize, Debug)]
struct VideoIDQuery {
    v: u64,
}
#[get("/video/basic_info")]
async fn get_video_basic_info(url_query: Query<VideoIDQuery>) -> impl Responder {
    //Prepare 404 response:
    let response_404: HttpResponse = HttpResponse::NotFound().finish();
    //Check if video_id is valid:
    let video_id: &u64 = &url_query.v;
    {
        let data_manager_mutg = DATA_MANAGER.lock().unwrap();
        let data_manager: &DataManager = data_manager_mutg.deref();
        if data_manager.has_video_by_id(video_id) {
            //Get video:
            let video_arc: Arc<Mutex<VideoData>> = data_manager.get_video(video_id);
            let video_mutg = video_arc.lock().unwrap();
            let video: &VideoData = video_mutg.deref();

            //Return video data:
            let result_string = format!(
                "{{\"name\": \"{}\", \"description\": \"{}\", \"author\": \"{}\", \"uploaded_time\": {}, \"likes\": {}, \"dislikes\": {}}}",
                video.get_name(),
                video.get_description(),
                video.get_author(),
                video.get_uploaded_time(),
                video.get_likes(),
                video.get_dislikes(),
            );
            return HttpResponse::Ok()
                .append_header(header::ContentType(mime::APPLICATION_JSON))
                .body(result_string);
        }
    }

    return response_404;
}

#[get("/video/thumbnail_image")]
async fn get_video_thumbnail_image(url_query: Query<VideoIDQuery>) -> impl Responder {
    //404 response:
    let response_404: HttpResponse = HttpResponse::NotFound().finish();
    //Get and check video_id:
    let video_id: &u64 = &url_query.v;
    {
        let data_manager_mutg = DATA_MANAGER.lock().unwrap();
        let data_manager: &DataManager = data_manager_mutg.deref();
        if !data_manager.has_video_by_id(video_id) {
            return response_404;
        }
    }
    //Get thumbnail image:
    let thumbnail_image_path: String = format!("{}/media/{}/thumbnail.png", DATA_PATH, video_id);
    return match read_small_file(thumbnail_image_path.as_str()) {
        Ok(contents) => HttpResponse::Ok()
            .append_header(header::ContentType(mime::IMAGE_PNG))
            .body(contents),
        Err(_) => response_404,
    };
}

#[derive(Deserialize)]
struct GetCommentsPageQuery {
    v: u64,
    page_index: i64,
}
///Get video comments page by page ("Getting video comments (page by page)" section).
#[get("/video/get_comments_page")]
async fn get_comments_page(url_query: Query<GetCommentsPageQuery>) -> impl Responder {
    let page_size: u64 = 50;
    //Get 404 response:
    let response_404 = HttpResponse::NotFound().finish();
    {
        //Get DataManager:
        let data_manager_mutg = DATA_MANAGER.lock().unwrap();
        let data_manager: &DataManager = data_manager_mutg.deref();

        //Check video_id:
        let video_id: &u64 = &url_query.v;
        let page_index: &i64 = &url_query.page_index;
        if data_manager.has_video_by_id(video_id) {
            let video_arc = data_manager.get_video(video_id);
            let video_mutg = video_arc.lock().unwrap();
            let video: &VideoData = video_mutg.deref();

            // {
            // "[comment_id]": {
            // "author": "[user_id of comment's author]",
            // "text": "[comment text]",
            // "creation_time": [time comment was created in seconds since epoch as a u64],
            // "replies": {[replies in the same JSON format as the comment]}
            // }
            // }
            let mut response_string = String::from("{");
            let mut i = 0;
            let on_comment = |comment_arc: Arc<Mutex<Comment>>| -> () {
                let mut comment_mutg = comment_arc.lock().unwrap();
                let comment: &mut Comment = comment_mutg.deref_mut();
                // let replies_string =
                //     make_json_from_describable_list(comment.get_replies().deref());
                if i > 0 {
                    response_string.push(',');
                }
                response_string.push_str(comment.describe().as_str());
                i += 1;
            };
            if *page_index >= 0 && ((*page_index as u64) * page_size) < video.get_num_comments() {
                let start = (*page_index as u64) * page_size;
                let end = ((*page_index as u64) + 1) * page_size - 1;
                // println!("start: {}, end: {}", start, end);
                video.for_each_comment_range(start, end, on_comment);
                response_string.push_str("}");

                return HttpResponse::Ok()
                    .append_header(header::ContentType(mime::APPLICATION_JSON))
                    .body(response_string);
            } else if *page_index == -2 {
                video.for_each_comment_range(0, video.get_num_comments() - 1, on_comment);
                response_string.push_str("}");

                return HttpResponse::Ok()
                    .append_header(header::ContentType(mime::APPLICATION_JSON))
                    .body(response_string);
            }
        }
    }

    return response_404;
}

///Implements the "/video/is_currently_processing" endpoint.
#[post("/video/is_currently_processing")]
async fn is_currently_processing(
    mut payload: Multipart,
    query: Query<VideoIDQuery>,
) -> HttpResponse {
    let response_404 = HttpResponse::NotFound().finish();
    let response_401 = HttpResponse::Unauthorized().finish();
    // Parse form data:
    let mut values: HashMap<String, Vec<u8>> = HashMap::new();
    parse_form_data(&mut payload, &mut values).await;
    let (user_id, session_id) = match get_user_id_session_id(&values) {
        Some(res) => res,
        None => return response_401,
    };

    //Verify user_id and session_id:
    if SESSION_MANAGER.verify_session(&user_id, session_id) {
        let video_id: u64 = query.v;
        let data_manager_mutg = DATA_MANAGER.lock().unwrap();
        let data_manager = data_manager_mutg.deref();
        if data_manager.has_video_by_id(&video_id) {
            let cp: bool = PROCESSOR_MANAGER.is_currently_processing(&video_id);
            let fp: bool = PROCESSOR_MANAGER.is_finished_processing(&video_id);
            let failp: bool = PROCESSOR_MANAGER.is_failed_processing(&video_id);

            let mut result_string: String = "".to_string();
            if !cp && !fp && !failp {
                result_string =
                    format!("\"{{\"video_id\":{video_id}, \"processing_status\": 4}}\"");
            } else if cp && !fp && !failp {
                result_string =
                    format!("\"{{\"video_id\":{video_id}, \"processing_status\": 1}}\"");
            } else if !cp && fp && !failp {
                result_string =
                    format!("\"{{\"video_id\":{video_id}, \"processing_status\": 0}}\"");
            } else if !cp && !fp && failp {
                result_string = format!(
                    "\"{{\"video_id\":{video_id}, \"processing_status\": 2, \"error_message\": \"{}\"}}\"",
                    "Unknown error message (need to add support for error messages)"
                );
            } else {
                todo!("Invalid combination of cp, fp, and failp. ");
            }
            return HttpResponse::Ok()
                .append_header(header::ContentType(mime::APPLICATION_JSON))
                .body(result_string);
        } else {
            return response_404;
        }
    }

    return HttpResponse::Ok().finish();
}

///Gets the user_id and session_id from the given parsed form data (form_data). Returns None if either user_id or session_id was not found or if session_id is not a valid u64 number.
fn get_user_id_session_id(form_data: &HashMap<String, Vec<u8>>) -> Option<(String, u64)> {
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

///Given the multipart form data (payload), this parses all of the form data and stores it in the given HashMap (result).
async fn parse_form_data(payload: &mut Multipart, result: &mut HashMap<String, Vec<u8>>) {
    while let Ok(Some(mut field)) = payload.try_next().await {
        let data: &mut Vec<u8> = {
            let field_name = field.name().unwrap().to_string();
            result.insert(field_name.clone(), Vec::new());
            result.get_mut(&field_name).unwrap()
        };

        while let Ok(Some(chunk)) = field.try_next().await {
            data.extend_from_slice(&chunk);
        }
    }
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
        "{{\"time_created\": {}, \"description\": {}, \"videos\": {}, \"comments\": {}}}",
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

///Gets the user id (username field) and password hash (password field) from the given parsed form data (form_data). Returns None if either user_id or session_id was not found or if session_id is not a valid u64 number.
fn get_login_data(form_data: &HashMap<String, Vec<u8>>) -> Option<(String, String)> {
    let user_id: String = match form_data.get(&"username".to_string()) {
        Some(id) => String::from_utf8_lossy(id.as_slice()).to_string(),
        None => return None,
    };
    let password_hash: String = match form_data.get(&"password".to_string()) {
        Some(id) => String::from_utf8_lossy(id.as_slice()).to_string(),
        None => return None,
    };
    return Some((user_id, password_hash));
}

#[get("/user/login")]
///Logging in:
async fn login(mut payload: Multipart) -> impl Responder {
    let response400 = HttpResponse::BadRequest().finish();

    //Getting username and password:
    let mut parsed_data: HashMap<String, Vec<u8>> = HashMap::new();
    parse_form_data(&mut payload, &mut parsed_data).await;
    let (user_id, password_hash) = match get_login_data(&parsed_data) {
        Some(res) => res,
        None => return response400,
    };

    //Check if username and password is valid. If so, login with SESSION_MANAGER.
    let data_manager_mutg = DATA_MANAGER.lock().unwrap();
    let data_manager = data_manager_mutg.deref();
    if !data_manager.has_user(&user_id) {
        return response400;
    }
    let session_id: u64 = match SESSION_MANAGER.login(&user_id, &password_hash) {
        Ok(id) => id,
        Err(_) => return response400,
    };

    return HttpResponse::Ok()
        .append_header(header::ContentType(mime::APPLICATION_OCTET_STREAM))
        .body(session_id.to_be_bytes().to_vec());
}

#[get("/user/signup")]
///Signing up:
async fn signup(mut payload: Multipart) -> impl Responder {
    let response400 = HttpResponse::BadRequest().finish();

    //Getting username and password:
    let mut parsed_data: HashMap<String, Vec<u8>> = HashMap::new();
    parse_form_data(&mut payload, &mut parsed_data).await;
    let (user_id, password_hash) = match get_login_data(&parsed_data) {
        Some(res) => res,
        None => return response400,
    };

    //Check if there is any user with the given username.
    let mut data_manager_mutg = DATA_MANAGER.lock().unwrap();
    let data_manager = data_manager_mutg.deref_mut();
    if data_manager.has_user(&user_id) {
        return response400;
    }

    //Create new user:
    data_manager.add_user(&user_id, &password_hash, &"".to_string(), 0);
    let session_id: u64 = match SESSION_MANAGER.login(&user_id, &password_hash) {
        Ok(id) => id,
        Err(_) => return response400,
    };

    return HttpResponse::Ok()
        .append_header(header::ContentType(mime::APPLICATION_OCTET_STREAM))
        .body(session_id.to_be_bytes().to_vec());
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    //Initialize PROCESSOR_MANAGER:
    {
        PROCESSOR_MANAGER.load_data();
    }

    //Initialize DATA_MANAGER:
    {
        let mut data_manager_guard = DATA_MANAGER.lock().unwrap();
        let data_manager: &mut DataManager = data_manager_guard.deref_mut();
        data_manager.load_data();
        // println!("DATA_MANAGER: \n{:#?}\n\n", data_manager);
        data_manager.save_data();
    }

    // Start autodelete thread for SESSION_MANAGER:
    SESSION_MANAGER.start_autodelete();

    //Start ManagementConsole:
    MANAGEMENT_CONSOLE.launch(std::env::var("CARGO_PKG_VERSION").unwrap());

    match HttpServer::new(move || {
        App::new()
            // .service(hello)
            // .route("/watch", web::get().to(watch))
            .service(get_video_file)
            .service(get_video_basic_info)
            .service(get_comments_page)
            .service(get_video_thumbnail_image)
            .service(is_currently_processing)
            .service(get_public_info)
    })
    .bind(("127.0.0.1", 7000))
    {
        Ok(res) => res,
        Err(e) => {
            println!("Caught error from HttpServer.bind() function: {:#?}", e);
            panic!("Caught error. ");
        }
    }
    .run()
    .await
}
