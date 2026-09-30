///This is a video sharing platform website, similar to YouTube.
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Mutex,
};

use actix_multipart::Multipart;
use futures::TryStreamExt;

use actix_web::{App, HttpServer};

use crate::{
    comment_main::{
        comment_add_endpoint, comment_add_reply_endpoint, comment_edit_endpoint,
        comment_remove_endpoint, get_comment_endpoint,
    },
    file_handling::{
        file_handling::DataManager,
        video_processing::{
            VideoProcessorManager, local_thumbnail_video_processor::LocalThumbnailImageProcessor,
        },
    },
    management_console::{
        ManagementConsole, text_based_management_console::TextBasedManagementConsole,
    },
    session_manager::SessionManager,
    user_main::{
        get_disliked_comments, get_disliked_videos, get_liked_comments, get_liked_videos,
        get_public_info, get_subscriptions, get_user_comments, get_user_icon, login_endpoint,
        signup,
    },
    video_main::{
        get_comments_page, get_video_basic_info, get_video_file, get_video_thumbnail_image,
        is_currently_processing, video_upload_endpoint,
    },
};

mod comment_main;
mod file_handling;
mod management_console;
mod session_manager;
mod time;
mod user_main;
mod video_main;

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
    user.check_password_hash(password_hash)
}

static _GET_USER_FOR_SESSION_MANAGER_FUNC: fn(&String, &String) -> bool =
    _get_user_for_session_manager;

static ON_FINISH_SUCCESS_FUNC: fn(u64) -> () = _on_finish_success_func;
static ON_FINISH_FAIL_FUNC: fn(u64) -> () = _on_finish_fail_func;

static DATA_PATH: &str = "./data";
static DATA_MANAGER: std::sync::LazyLock<Mutex<DataManager>> =
    std::sync::LazyLock::new(|| -> Mutex<DataManager> { Mutex::new(DataManager::new(DATA_PATH)) });
static SESSION_MANAGER: SessionManager<fn(&String, &String) -> bool> =
    SessionManager::new(&_GET_USER_FOR_SESSION_MANAGER_FUNC);
static MANAGEMENT_CONSOLE: TextBasedManagementConsole<fn(&String, &String) -> bool> =
    TextBasedManagementConsole::new(&DATA_MANAGER, &PROCESSOR_MANAGER, &SESSION_MANAGER);

type ThumbnailImageProccessorType = LocalThumbnailImageProcessor;
static PROCESSOR_MANAGER: VideoProcessorManager<fn(u64) -> (), ThumbnailImageProccessorType> =
    VideoProcessorManager::new(DATA_PATH, ON_FINISH_SUCCESS_FUNC, ON_FINISH_FAIL_FUNC);

pub fn _on_finish_proc_func(video_id: u64) {
    let data_manager_mutg = DATA_MANAGER.lock().unwrap();
    let data_manager = data_manager_mutg.deref();
    let video_arc = data_manager.get_video(&video_id).unwrap();
    let mut video_mutg = video_arc.lock().unwrap();
    let video = video_mutg.deref_mut();
    video.set_finished_processing(true);
}
pub fn _on_finish_success_func(video_id: u64) -> () {
    _on_finish_proc_func(video_id);
}

pub fn _on_finish_fail_func(video_id: u64) -> () {
    _on_finish_proc_func(video_id);
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    //Get command line args:
    let args: Vec<String> = std::env::args().collect::<Vec<String>>();
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
    if args.contains(&"-text_console".to_string()) {
        MANAGEMENT_CONSOLE.launch(std::env::var("CARGO_PKG_VERSION").unwrap());
    }

    //Launch Actix-web:
    match HttpServer::new(move || {
        App::new()
            // .service(hello)
            // .route("/watch", web::get().to(watch))
            .service(get_video_file)
            .service(get_video_basic_info)
            .service(get_comments_page)
            .service(get_video_thumbnail_image)
            .service(video_upload_endpoint)
            .service(is_currently_processing)
            .service(get_public_info)
            .service(get_user_icon)
            .service(login_endpoint)
            .service(signup)
            .service(get_subscriptions)
            .service(get_liked_videos)
            .service(get_disliked_videos)
            .service(get_user_comments)
            .service(get_liked_comments)
            .service(get_disliked_comments)
            .service(get_comment_endpoint)
            .service(comment_add_endpoint)
            .service(comment_edit_endpoint)
            .service(comment_add_reply_endpoint)
            .service(comment_remove_endpoint)
    })
    .bind(("127.0.0.1", 6000))
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
