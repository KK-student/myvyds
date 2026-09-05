///This is a video sharing platform website, similar to YouTube.
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

use actix_multipart::{Field, Multipart};
use futures::TryStreamExt;
// use futures::lock::{Mutex, MutexLockFuture};
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
    // for (field_name, field_value) in values {
    //     println!(
    //         "{}: {}",
    //         field_name,
    //         String::from_utf8_lossy(field_value.as_slice())
    //     );
    // }
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

#[get("/test_form.html")]
async fn test_form() -> HttpResponse {
    let html: &str = "<!DOCTYPE html>
<html>
<head><title>Webpage for testing form submission. </title></head>
<body>
<form action=\"/video/is_currently_processing\" method=\"post\" enctype=\"multipart/form-data\">
  Name: <input type=\"text\" name=\"person\"><br>
  File: <input type=\"file\" name=\"secret\"><br>
  <input type=\"submit\" value=\"Submit\">
</form>
</body>
</html>";
    return HttpResponse::Ok()
        .append_header(header::ContentType(mime::TEXT_HTML))
        .body(html);
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // let test_params = "v=1203487watch_me&id=+=asdpfoiu";
    // let test_params = "v=1203487watch_me&id=+=asdpfoiu&ui=&hc&&=;lkj";
    // let result = parse_url_parameters(test_params);

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
    // {
    //     // Test LocalVideoProcessor:
    //     let local_test_processor_config: &str = "{\"max_gpu_tflops\": 34234, \"ffmpeg_options\": [\"-map\", \"0\", \"-c:v:0\", \"libx264\", \"-b:v:0\", \"5000k\", \"-s:v:0\", \"1920x1080\", \"-map\", \"0\", \"-c:v:1\", \"libx264\", \"-b:v:1\", \"3000k\", \"-s:v:1\", \"1280x720\", \"-map\", \"0\", \"-c:v:2\", \"libx264\", \"-b:v:2\", \"1000k\", \"-s:v:2\", \"640x360\", \"-map\", \"0\", \"-c:v:3\", \"libvpx-vp9\", \"-b:v:3\", \"5000k\", \"-s:v:3\", \"1920x1080\", \"-map\", \"0\", \"-c:v:4\", \"libvpx-vp9\", \"-b:v:4\", \"3000k\", \"-s:v:4\", \"1280x720\", \"-map\", \"0\", \"-c:v:5\", \"libvpx-vp9\", \"-b:v:5\", \"1000k\", \"-s:v:5\", \"640x360\", \"-map\", \"0:a:0\", \"-c:a:0\", \"aac\", \"-b:a:0\", \"192k\", \"-map\", \"0:a:0\", \"-c:a:1\", \"aac\", \"-b:a:1\", \"128k\", \"-map\", \"0:a:0\", \"-c:a:2\", \"aac\", \"-b:a:2\", \"64k\", \"-map\", \"0:a:0\", \"-c:a:3\", \"libopus\", \"-b:a:3\", \"192k\", \"-map\", \"0:a:0\", \"-c:a:4\", \"libopus\", \"-b:a:4\", \"128k\", \"-map\", \"0:a:0\", \"-c:a:5\", \"libopus\", \"-b:a:5\", \"64k\"]}";
    //     // println!("{}", local_test_processor_config);
    //     let local_test_processor_config_json: Value =
    //         serde_json::from_str(local_test_processor_config).unwrap();
    //     let mut local_test_processor: LocalVideoProcessor =
    //         LocalVideoProcessor::new(&local_test_processor_config_json);
    //     local_test_processor.process(
    //         2,
    //         "/tmp/rust-process-test/sample-10s.mp4".to_string(),
    //         "/tmp/rust-process-test/output-dir/".to_string(),
    //     );
    //     thread::sleep(time::Duration::from_secs(10));
    //     println!("{:#?}", local_test_processor);
    // }

    // Start autodelete thread for SESSION_MANAGER:
    SESSION_MANAGER.start_autodelete();

    //Start ManagementConsole:
    MANAGEMENT_CONSOLE.launch(std::env::var("CARGO_PKG_VERSION").unwrap());

    //Test user input:
    // let mut test_input: String = String::new();
    // let _ = io::stdin().read_line(&mut test_input);
    // println!("You typed: \"{test_input}\". ");

    match HttpServer::new(move || {
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
            .service(get_video_basic_info)
            .service(get_comments_page)
            .service(get_video_thumbnail_image)
            .service(is_currently_processing)
            .service(test_form)
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
