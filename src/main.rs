///This is a video sharing platform website, similar to YouTube.
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use futures::lock::{Mutex, MutexLockFuture};
use serde::Deserialize;

use actix_web::{
    App, HttpRequest, HttpResponse, HttpServer, Responder, get,
    http::StatusCode,
    web::{self, Query},
};
use posix_regex::{PosixRegex, PosixRegexBuilder};
use tokio_util::io::ReaderStream;

use crate::file_handling::file_handling::{
    DataManager, file_stream_reader, get_mime_type, read_small_file,
};

mod file_handling;

// #[get("/")]
// async fn hello() -> impl Responder {
//     HttpResponse::Ok().body("Hello world!")
// }
//
// async fn manual_hello() -> impl Responder {
//     HttpResponse::Ok().body("Hey there!")
// }
//
// fn parse_url_parameters(query: &str) -> HashMap<&str, &str> {
//     let mut result: HashMap<&str, &str> = HashMap::new();
//     let group_regex_builder: PosixRegexBuilder = PosixRegexBuilder::new(
//         /*"[^&]*=*[^&]"*/ "[^&]\\{1,\\}=*[^&]\\{1,\\}".as_bytes(),
//     );
//     let group_regex: PosixRegex = match group_regex_builder.compile() {
//         Ok(r) => r,
//         Err(_) => {
//             println!("Error: Failed to compile regex. ");
//             return result;
//         }
//     };
//
//     let kv_regex: PosixRegex = match PosixRegexBuilder::new("^[^=]\\{1,\\}".as_bytes()).compile() {
//         Ok(r) => r,
//         Err(_) => {
//             println!("Error: Failed to compile regex. ");
//             return result;
//         }
//     };
//
//     println!("Matches: ");
//     let matches = group_regex.matches(query.as_bytes(), Option::from(10));
//     {
//         let mut i = 0;
//         while i < matches.len() {
//             let r_match = &matches[i][0].unwrap();
//             let (start, end) = r_match;
//             let kv_group = &query[*start..*end];
//
//             let kv_match = kv_regex.matches(kv_group.as_bytes(), Option::None);
//             let (key_start, key_end) = if kv_match.len() != 0 {
//                 kv_match[0][0].unwrap()
//             } else {
//                 (0, 0)
//             };
//             let key = &kv_group[key_start..key_end];
//             let value = if key_end + 1 >= kv_group.len() {
//                 ""
//             } else {
//                 &kv_group[(key_end + 1)..]
//             };
//             println!(
//                 "{}: {:?} -> start: {}, end: {}, kv_group: \"{}\", key_start: {}, key_end: {}, key: \"{}\", value: \"{}\"",
//                 i, r_match, start, end, kv_group, key_start, key_end, key, value
//             );
//             result.insert(key, value);
//
//             i += 1;
//         }
//     }
//
//     return result;
// }
//
// async fn get_url_parameters(req: HttpRequest) -> impl Responder {
//     let query_string = req.query_string();
//     HttpResponse::Ok().body(format!(
//         "<!DOCTYPE html>
// <html>
//     <head><title>URL parameters</title></head>
//     <body>
//         <p>Query String: {}</p>
//         <p>URL parameters: </p>
//         <p>{:?}</p>
//     </body>
// </html>
// ",
//         query_string,
//         parse_url_parameters(query_string)
//     ))
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

// async fn get_lofi_music_html() -> impl Responder {
//     HttpResponse::Ok().body(
//         "<!DOCTYPE html>
// <html>
//     <head>
//         <title>coding music synthwave beats to program to-fhL67fnDXcU</title>
//     </head>
//     <body>
//         <video src=\"/media/coding music synthwave beats to program to-fhL67fnDXcU-av.webm\" />
//     </body>
// </html>",
//     )
// }

// #[derive(Debug, Deserialize)]
// struct Info {
//     name: String,
//     age: u32,
// }
// async fn url_parameters_form(info: actix_web::web::Query<Info>) -> impl Responder {
//     HttpResponse::Ok().body(format!(
//         "<!DOCTYPE html>
// <html>
//     <head><title>URL parameters</title></head>
//     <body>
//         <p>URL parameters: </p>
//         <p>{:?}</p>
//     </body>
// </html>
// ",
//         info
//     ))
// }

// #[derive(Deserialize, Debug)]
// struct VideoID {
//     id: u64,
// }
// async fn get_video(v_query: Query<VideoID>) -> impl Responder {
//     let video_id = v_query.id;
//     if video_id == 0 {
//         let file_path_str =
//             "./media/coding music synthwave beats to program to-fhL67fnDXcU-av.webm";
//         let reader_stream: ReaderStream<tokio::fs::File> = match file_stream_reader(file_path_str)
//             .await
//         {
//             Ok(r) => r,
//             Err(e) => {
//                 return HttpResponse::Ok()
//                 .body(format!("<!DOCTYPE html><html><head><title>Error</title></head><body><p>Error: {}</p></body></html>", e.to_string()));
//             }
//         };
//         HttpResponse::Ok()
//             .content_type(get_mime_type(std::path::Path::new(file_path_str)))
//             //.insert_header(ContentDisposition::attachment(String::from(filename)))
//             .streaming(reader_stream)
//     } else {
//         HttpResponse::Ok().body(
//             "<!DOCTYPE html>
// <html>
//     <head>
//         <title>Error</title>
//     </head>
//     <body>
// <p>Could not file video. </p>
//     </body>
// </html>",
//         )
//     }
// }
//
// fn get_error_page(message: &str) -> String {
//     format!(
//         "<!DOCTYPE html>
// <html>
// <head><title>Error</title></head>
// <body>
// <h1>Error: </h1>
// <p>{}</p>
// </body>
// </html>
// ",
//         message
//     )
// }
//
// async fn watch(v_qid: Query<VideoID>) -> impl Responder {
//     let watch_js_bytes: Vec<u8> = match read_small_file("./webpage/watch.html") {
//         Ok(r) => r,
//         Err(e) => get_error_page(&e.to_string()).as_bytes().to_vec(),
//     };
//     HttpResponse::Ok().body(watch_js_bytes)
// }
//

static DATA_PATH: &str = "./data";
static DATA_MANAGER: std::sync::LazyLock<Mutex<DataManager>> =
    std::sync::LazyLock::new(|| -> Mutex<DataManager> { Mutex::new(DataManager::new(DATA_PATH)) });

//Getting a video:
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
        let data_manager_guard = DATA_MANAGER.lock().await;
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // let test_params = "v=1203487watch_me&id=+=asdpfoiu";
    // let test_params = "v=1203487watch_me&id=+=asdpfoiu&ui=&hc&&=;lkj";
    // let result = parse_url_parameters(test_params);

    {
        let mut data_manager_guard = DATA_MANAGER.lock().await;
        let data_manager: &mut DataManager = data_manager_guard.deref_mut();
        data_manager.load_data();
        // println!("DATA_MANAGER: \n{:#?}\n\n", data_manager);
        data_manager.save_data();
    }

    HttpServer::new(move || {
        App::new()
            // .service(hello)
            // .route("/hey", web::get().to(manual_hello))
            // .route(
            //     "/media/coding music synthwave beats to program to-fhL67fnDXcU-av.webm",
            //     web::get().to(get_lofi_music),
            // )
            // .route("/lofi_music", web::get().to(get_lofi_music_html))
            // .route("/url_params", web::get().to(get_url_parameters))
            // .route("/url_params_form", web::get().to(url_parameters_form))
            // .route("/get_video", web::get().to(get_video))
            // .route("/watch", web::get().to(watch))
            .service(get_video_file)
    })
    .bind(("127.0.0.1", 6000))?
    .run()
    .await
}
