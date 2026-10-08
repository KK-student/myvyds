use std::{
    collections::HashMap,
    io::Write,
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};

use actix_multipart::Multipart;
use actix_web::{
    HttpResponse, Responder, get,
    http::header,
    mime, post,
    web::{self, Query},
};
use futures::FutureExt;
use rand::RngExt;
use serde::Deserialize;

use crate::{
    DATA_MANAGER, DATA_PATH, PROCESSOR_MANAGER, SESSION_MANAGER,
    file_handling::{
        comment::Comment,
        describable::Describable,
        file_handling::{DataManager, read_small_file},
        likeable::Likeable,
        video_data::VideoData,
    },
    parse_form_data,
    user_main::{authenticate, get_user_id_session_id},
};

#[get("/is_online")]
pub async fn is_online() -> impl Responder {
    HttpResponse::Ok().body("Sever is online!")
}

///Gets a specific video file, serving the "/video/get_video_file/[video_id]/[video_seg_file]" endpoint. See the "Getting a video" section in the "./docs/SERVER_API.md" for more details.
#[get("/video/get_video_file/{video_id}/{video_seg_file}")]
pub async fn get_video_file(url_params: web::Path<(String, String)>) -> impl Responder {
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
pub struct VideoIDQuery {
    pub v: u64,
}
#[get("/video/basic_info")]
pub async fn get_video_basic_info(url_query: Query<VideoIDQuery>) -> impl Responder {
    //Prepare 404 response:
    let response_404: HttpResponse = HttpResponse::NotFound().finish();
    //Check if video_id is valid:
    let video_id: &u64 = &url_query.v;
    {
        let data_manager_mutg = DATA_MANAGER.lock().unwrap();
        let data_manager: &DataManager = data_manager_mutg.deref();
        if data_manager.has_video_by_id(video_id) {
            //Get video:
            let video_arc: Arc<Mutex<VideoData>> = data_manager.get_video(video_id).unwrap();
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

    response_404
}

#[get("/video/thumbnail_image")]
pub async fn get_video_thumbnail_image(url_query: Query<VideoIDQuery>) -> impl Responder {
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
    match read_small_file(thumbnail_image_path.as_str()) {
        Ok(contents) => HttpResponse::Ok()
            .append_header(header::ContentType(mime::IMAGE_PNG))
            .body(contents),
        Err(_) => response_404,
    }
}

#[derive(Deserialize)]
struct GetCommentsPageQuery {
    v: u64,
    page_index: i64,
}
///Get video comments page by page ("Getting video comments (page by page)" section).
#[get("/video/get_comments_page")]
pub async fn get_comments_page(url_query: Query<GetCommentsPageQuery>) -> impl Responder {
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
            let video_arc = data_manager.get_video(video_id).unwrap();
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
                response_string.push('}');

                return HttpResponse::Ok()
                    .append_header(header::ContentType(mime::APPLICATION_JSON))
                    .body(response_string);
            } else if *page_index == -2 {
                video.for_each_comment_range(0, video.get_num_comments() - 1, on_comment);
                response_string.push('}');

                return HttpResponse::Ok()
                    .append_header(header::ContentType(mime::APPLICATION_JSON))
                    .body(response_string);
            }
        }
    }

    response_404
}

///Implements the "/video/is_currently_processing" endpoint.
#[post("/video/is_currently_processing")]
pub async fn is_currently_processing(
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

            // let result_string: String;
            // if !cp && !fp && !failp {
            //     result_string =
            //         format!("\"{{\"video_id\":{video_id}, \"processing_status\": 4}}\"");
            // } else if cp && !fp && !failp {
            //     result_string =
            //         format!("\"{{\"video_id\":{video_id}, \"processing_status\": 1}}\"");
            // } else if !cp && fp && !failp {
            //     result_string =
            //         format!("\"{{\"video_id\":{video_id}, \"processing_status\": 0}}\"");
            // } else if !cp && !fp && failp {
            //     result_string = format!(
            //         "\"{{\"video_id\":{video_id}, \"processing_status\": 2, \"error_message\": \"{}\"}}\"",
            //         "Unknown error message (need to add support for error messages)"
            //     );
            // } else {
            //     todo!("Invalid combination of cp, fp, and failp. ");
            // }
            return HttpResponse::Ok()
                .append_header(header::ContentType(mime::APPLICATION_JSON))
                .body({if !cp && !fp && !failp {
                    format!("\"{{\"video_id\":{video_id}, \"processing_status\": 4}}\"")
                } else if cp && !fp && !failp {
                    format!("\"{{\"video_id\":{video_id}, \"processing_status\": 1}}\"")
                } else if !cp && fp && !failp {
                    format!("\"{{\"video_id\":{video_id}, \"processing_status\": 0}}\"")
                } else if !cp && !fp && failp {
                    format!(
                        "\"{{\"video_id\":{video_id}, \"processing_status\": 2, \"error_message\": \"{}\"}}\"",
                        "Unknown error message (need to add support for error messages)"
                    )
                } else {
                    todo!("Invalid combination of cp, fp, and failp. ");
                }});
        } else {
            return response_404;
        }
    }

    HttpResponse::Ok().finish()
}

#[post{"/video/upload"}]
pub async fn video_upload_endpoint(mut payload: Multipart) -> impl Responder {
    // ### Uploading a video:

    // 1. Client sends a form to the server with the fields "username"=[user's user_id], "session_id"=[user's current session_id], "title"=[title of the video], "description"=[description of the video], "video_file"=[the video file as multipart data in supported file format with supported codecs], "thumbnail_video"=[the thumbnail video file as multipart data in supported file format with supported codecs], and "thumbnail_image"=[the thumbnail image file as multipart data in png format] to the endpoint "/video/upload".

    // 2.
    // - If username and session_id are valid:
    //   - If video title is not already being used for another video:
    //     - If video file format and audio/video codecs are supported:
    //       - If server determines that it has enough storage: Server sends OK response with status code 201 (Created) with the body being the video_id of the newly-uploaded video, which, in a separate thread, the server will be processing to create the needed MPEG_DASH files.
    //       - Otherwise, server sends error message with the error code 507 (Insufficient Storage).
    //     - Otherwise, server sends error message with the error code 415 (Unsupported Media Type).
    //   - Otherwise, server sends error message with the error code 409 (Conflict).
    // - Otherwise, server sends error message with the error code 401 (Unauthorized).

    let res401 = HttpResponse::Unauthorized().finish();
    let res409 = HttpResponse::Conflict().finish();
    let res415 = HttpResponse::UnsupportedMediaType().finish();
    let res507 = HttpResponse::InsufficientStorage().finish();
    let resBadReq = HttpResponse::BadRequest().finish();

    let video_id: u64;
    let video_dir: PathBuf;
    let video_file_path: PathBuf;
    {
        //Get DATA_MANAGER:
        let mut data_manager_mutg = DATA_MANAGER.lock().unwrap();
        let data_manager = data_manager_mutg.deref_mut();

        //Authenticate:
        let (parsed_form_data, user_id) = match authenticate(&mut payload).await {
            Some(data) => data,
            None => return res401,
        };

        //Parse parameters:
        let video_title: String = match parsed_form_data.get("title") {
            Some(tv) => String::from_utf8_lossy(tv.as_slice()).to_string(),
            None => String::new(),
        };
        let video_description: String = match parsed_form_data.get("description") {
            Some(tv) => String::from_utf8_lossy(tv.as_slice()).to_string(),
            None => String::new(),
        };
        let video_file_bytes: &Vec<u8> = match parsed_form_data.get("video_file") {
            Some(data) => data,
            None => return resBadReq,
        };
        let thumbnail_video_file_bytes: &Vec<u8> = match parsed_form_data.get("thumbnail_video") {
            Some(data) => data,
            None => return resBadReq,
        };
        let thumbnail_image_file_bytes: &Vec<u8> = match parsed_form_data.get("thumbnail_image") {
            Some(data) => data,
            None => return resBadReq,
        };

        //TODO: Finish the bellow methods and section:
        //Check if video title is being used for another video:
        if !data_manager.has_video_by_name(&video_title) {
            return res409;
        }

        //Check if video file is supported:
        if !PROCESSOR_MANAGER.is_supported_file_data(video_file_bytes) {
            return res415;
        }

        //Get video_id:
        video_id = {
            let mut r = rand::rng();
            let mut v: u64;
            loop {
                v = r.random::<u64>();
                if !data_manager.has_video_by_id(&v) {
                    break v;
                }
            }
        };

        //Check if there is enough space available:
        video_dir = std::fs::canonicalize(data_manager.get_video_dir(&video_id)).unwrap();
        if !PROCESSOR_MANAGER.has_enough_space(video_file_bytes, video_dir.as_path()) {
            return res507;
        }

        //Create the video directory:
        std::fs::create_dir(&video_dir).unwrap();
        //Write the video file:
        video_file_path = video_dir.join("./input.bin");
        {
            let mut video_file = std::fs::File::create(&video_file_path).unwrap();
            video_file.write_all(video_file_bytes.as_slice()).unwrap();
        }
        //Add the video to data_manager:
        data_manager
            .add_video_with_id(
                &video_title,
                &user_id,
                &video_description,
                &video_file_path,
                &video_id,
            )
            .unwrap();

        //Add the thumbnail video:
        // TODO: Thumbnail video needs to be processed with ffmpeg. However, for now, I'm just storing the input file exactly as it was uploaded:
        {
            let thumbnail_video_path = video_dir.join("./thumbnail-video.webm");
            let mut thumbnail_video_file = std::fs::File::create(thumbnail_video_path).unwrap();
            thumbnail_video_file
                .write_all(thumbnail_video_file_bytes.as_slice())
                .unwrap();
        }

        //Add the thumbnail image:
        // TODO: Needs to be processed as well, just like the thumbnail video, but for now I'm just storing it exactly as it was uploaded:
        {
            // let thumbnail_image_path = video_dir.join("./thumbnail.png");
            // let mut thumbnail_image_file = std::fs::File::create(thumbnail_image_path).unwrap();
            // thumbnail_image_file
            //     .write_all(thumbnail_image_file_bytes.as_slice())
            //     .unwrap();
            PROCESSOR_MANAGER
                .process_image(
                    thumbnail_image_file_bytes.clone(),
                    video_dir.clone(),
                    &|| -> () {},
                    &|_| -> () {},
                )
                .now_or_never();
        }
    }
    //Process the video:
    thread::spawn(move || -> () {
        PROCESSOR_MANAGER.process(
            video_id,
            &video_file_path.to_str().unwrap().to_string(),
            &video_dir.to_str().unwrap().to_string(),
        );
    });

    //Return:
    HttpResponse::Ok()
        .append_header(header::ContentType(mime::APPLICATION_OCTET_STREAM))
        .body(video_id.to_be_bytes().to_vec())
}
