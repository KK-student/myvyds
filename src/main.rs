///This is a video sharing platform website, similar to YouTube.
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::{Arc, LazyLock, Mutex},
    thread,
    time::Duration,
};

use actix_multipart::Multipart;

use argon2::{Argon2, PasswordHasher};
use futures::TryStreamExt;

use actix_web::{
    App, Error, HttpServer,
    body::MessageBody,
    dev::{ServiceFactory, ServiceRequest, ServiceResponse},
};
use simple_search_engine_lib::DEFAULT_MEMORY_BUDGET;

use crate::{
    comment_main::{
        comment_add_endpoint, comment_add_reply_endpoint, comment_edit_endpoint,
        comment_remove_endpoint, get_comment_endpoint,
    },
    file_handling::{
        DataManager,
        video_processing::{
            VideoProcessorManager, local_thumbnail_video_processor::LocalThumbnailImageProcessor,
            local_video_processor::LocalVideoProcessor,
        },
    },
    management_console::{
        ManagementConsole, text_based_management_console::TextBasedManagementConsole,
    },
    misc::collections::{DynClone, DynCloneBox},
    session_manager::SessionManager,
    tests::DEFAULT_PASSWORD,
    user_main::{
        get_disliked_comments, get_disliked_videos, get_liked_comments, get_liked_videos,
        get_public_info, get_subscriptions, get_user_comments, get_user_icon, login_endpoint,
        signup,
    },
    video_main::{
        get_comments_page, get_video_basic_info, get_video_file, get_video_thumbnail_image,
        is_currently_processing, is_online, search_by_name, video_upload_endpoint,
    },
};

mod comment_main;
mod file_handling;
mod linear_algebra;
mod management_console;
mod misc;
mod session_manager;
mod tests;
mod time;
mod user_main;
mod video_main;

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
    std::sync::LazyLock::new(|| -> Mutex<DataManager> {
        Mutex::new(DataManager::new(DATA_PATH, DEFAULT_MEMORY_BUDGET))
    });
static SESSION_MANAGER: SessionManager<fn(&String, &String) -> bool> =
    SessionManager::new(&_GET_USER_FOR_SESSION_MANAGER_FUNC);
static MANAGEMENT_CONSOLE: TextBasedManagementConsole<fn(&String, &String) -> bool> =
    TextBasedManagementConsole::new(&DATA_MANAGER, &PROCESSOR_MANAGER, &SESSION_MANAGER);

type ThumbnailImageProccessorType = LocalThumbnailImageProcessor;
static PROCESSOR_MANAGER: VideoProcessorManager<fn(u64) -> (), ThumbnailImageProccessorType> =
    VideoProcessorManager::const_default();

pub fn _on_finish_proc_func(video_id: u64) {
    let data_manager_mutg = DATA_MANAGER.lock().unwrap();
    let data_manager = data_manager_mutg.deref();
    let video_arc = data_manager.get_video(&video_id).unwrap();
    let mut video_mutg = video_arc.lock().unwrap();
    let video = video_mutg.deref_mut();
    video.set_finished_processing(true);
}
pub fn _on_finish_success_func(video_id: u64) {
    _on_finish_proc_func(video_id);
}

pub fn _on_finish_fail_func(video_id: u64) {
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

pub fn build_app() -> App<
    impl ServiceFactory<
        ServiceRequest,
        Response = ServiceResponse<impl MessageBody>,
        Config = (),
        InitError = (),
        Error = Error,
    >,
> {
    let mut app = App::new()
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
        .service(search_by_name);
    if cfg!(test) {
        app = app.service(is_online);
    }
    app
}

impl DynClone for u16 {
    fn clone_dyn(self_value: &Self) -> Self
    where
        Self: Sized + DynClone,
    {
        *self_value
    }
}

static SERVER_PORT: LazyLock<Mutex<DynCloneBox<u16>>> =
    LazyLock::new(|| -> Mutex<DynCloneBox<u16>> { Mutex::new(DynCloneBox::new(6000)) });

pub fn get_server_port() -> u16 {
    let sp_mutg = SERVER_PORT.lock().unwrap();
    let sp_box = sp_mutg.deref();
    *sp_box.deref()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    //Get command line args:
    let args: Vec<String> = std::env::args().collect::<Vec<String>>();
    //Get cli optionns:
    let t_mode: bool = args.contains(&"--testing_mode".to_string());
    let is_text_console: bool = args.contains(&"--text_console".to_string());

    // if t_mode {
    //     let mut testing_mode_mutg = TESTING_MODE.lock().unwrap();
    //     let testing_mode_box = testing_mode_mutg.deref_mut();
    //     testing_mode_box.set(true);
    // }

    //Initialize PROCESSOR_MANAGER:
    if !t_mode {
        PROCESSOR_MANAGER.init(
            std::path::Path::new(DATA_PATH),
            ON_FINISH_SUCCESS_FUNC,
            ON_FINISH_FAIL_FUNC,
        );
        PROCESSOR_MANAGER.load_data();
    } else {
        PROCESSOR_MANAGER.init_testing(ON_FINISH_SUCCESS_FUNC, ON_FINISH_FAIL_FUNC, None);
        PROCESSOR_MANAGER
            .add_local_video_processor(Arc::new(Mutex::new(LocalVideoProcessor::default())));
    }

    //Initialize DATA_MANAGER:
    {
        let mut data_manager_guard = DATA_MANAGER.lock().unwrap();
        let data_manager: &mut DataManager = data_manager_guard.deref_mut();
        if !t_mode {
            data_manager.load_data();
            // println!("DATA_MANAGER: \n{:#?}\n\n", data_manager);
        } else {
            data_manager.add_user(
                &"sample_user_1".to_string(),
                &{
                    let argon2 = Argon2::default();
                    argon2
                        .hash_password(DEFAULT_PASSWORD.as_bytes())
                        .unwrap()
                        .to_string()
                },
                &"This is a sample user created only for testing purposes. ".to_string(),
                0,
            );
        }
        // data_manager.save_data();
    }

    // Start autodelete thread for SESSION_MANAGER:
    if !t_mode {
        SESSION_MANAGER.start_autodelete();
    }

    // Start autosave thread for DATA_MANAGER:
    if !t_mode {
        thread::spawn(|| -> () {
            thread::sleep(Duration::from_secs(5));
            loop {
                {
                    println!("Autosaving data: ");
                    let mut data_manager_mutg = DATA_MANAGER.lock().unwrap();
                    let data_manager = data_manager_mutg.deref_mut();
                    data_manager.save_data();
                    println!("Data autosaved. ");
                }
                thread::sleep(Duration::from_mins(5));
            }
        });
    }

    //Start ManagementConsole:
    if is_text_console & !t_mode {
        MANAGEMENT_CONSOLE.launch(std::env::var("CARGO_PKG_VERSION").unwrap());
    } else if is_text_console && t_mode {
        panic!("Cannot use the \"--testing_mode\" option with the \"--text_console\" option!");
    }

    // println!(
    //     "Term frequencies: \n{:#?}",
    //     compute_term_frequencies("Hello ''' \"'''World's'''\" !@#!fjfdjs%#$@#$    !")
    // );

    //Launch Actix-web:
    match HttpServer::new(build_app).bind(("127.0.0.1", get_server_port())) {
        Ok(res) => res,
        Err(e) => {
            println!("Caught error from HttpServer.bind() function: {:#?}", e);
            panic!("Caught error. ");
        }
    }
    .run()
    .await
}
