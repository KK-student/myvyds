///This is a video sharing platform website, similar to YouTube.
use std::{
    collections::HashMap,
    fs::File,
    io::Write,
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::{Arc, LazyLock, Mutex},
    thread,
    time::Duration,
};

use actix_cors::Cors;
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
    misc::collections::{DynClone, DynCloneBox, MutableBox, linear_search_vec},
    other_main::search_endpoint,
    session_manager::SessionManager,
    tests::DEFAULT_PASSWORD,
    user_main::{
        get_disliked_comments, get_disliked_videos, get_liked_comments, get_liked_videos,
        get_playlist_names, get_public_info, get_public_playlist_names, get_subscriptions,
        get_user_comments, get_user_icon, is_logged_in_endpoint, login_endpoint, signup,
    },
    video_main::{
        get_comments_page, get_video_basic_info, get_video_file, get_video_thumbnail_image,
        is_currently_processing, is_online, video_upload_endpoint,
    },
};

mod comment_main;
mod file_handling;
mod linear_algebra;
mod management_console;
mod misc;
mod other_main;
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

// static DATA_PATH: &str = "./data";
static DATA_PATH: LazyLock<Mutex<MutableBox<String>>> =
    LazyLock::new(|| -> Mutex<MutableBox<String>> {
        Mutex::new(MutableBox::new(Some("./data".to_string())))
    });
static DATA_MANAGER: std::sync::LazyLock<Mutex<DataManager>> =
    std::sync::LazyLock::new(|| -> Mutex<DataManager> {
        Mutex::new(DataManager::new(
            get_data_path().as_str(),
            DEFAULT_MEMORY_BUDGET,
        ))
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

///Get the current value of DATA_PATH:
pub fn get_data_path() -> String {
    let data_path_mutg = DATA_PATH.lock().unwrap();
    let data_path_box = data_path_mutg.deref();
    data_path_box.get().unwrap()
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
    let cors = Cors::default()
        .allow_any_origin()
        .send_wildcard()
        .allow_any_method()
        .allow_any_header();
    let mut app = App::new()
        .wrap(cors)
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
        .service(search_endpoint)
        .service(is_logged_in_endpoint)
        .service(get_public_playlist_names)
        .service(get_playlist_names);
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

// static SERVER_PORT: LazyLock<Mutex<DynCloneBox<u16>>> =
//     LazyLock::new(|| -> Mutex<DynCloneBox<u16>> { Mutex::new(DynCloneBox::new(5999)) });

static SERVER_PORT: LazyLock<Mutex<MutableBox<u16>>> =
    LazyLock::new(|| -> Mutex<MutableBox<u16>> { Mutex::new(MutableBox::new(Some(5999))) });

pub fn get_server_port() -> u16 {
    let sp_mutg = SERVER_PORT.lock().unwrap();
    let sp_box = sp_mutg.deref();
    sp_box.unwrap()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    //Get command line args:
    let args: Vec<String> = std::env::args().collect::<Vec<String>>();
    //Get cli optionns:
    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        println!("--help or -h -> Show this help message.
--testing_mode -> Start in testing mode (does not load any data at all at startup). Cannot be used with \"--text_console\".
--text_console -> Enable the text console.
--data_path [PATH] -> Use the database located in the given directory (PATH).
--create_new -> Create a new database in the directory given by the \"--data_path\" option. All files that were previously located in the given directory ARE GOING TO BE ERASED! Must be used with the \"--data_path\" option.
--port [PORT_NUM] -> Start the server with the given port (PORT_NUM) instead of the default port number of {}. ", get_server_port());
        return Ok(());
    }
    let t_mode: bool = args.contains(&"--testing_mode".to_string());
    let is_text_console: bool = args.contains(&"--text_console".to_string());

    //Set data dir if --data_path flag is used correctly:
    let data_path_flag_set: bool =
        if let Some(flag_index) = linear_search_vec(&args, "--data_path".to_string()) {
            //Check if the data flag has a valid value.
            let value_index = flag_index + 1;
            let invalid_value_message: &str = "Invalid value for the \"--data_path\" option!";
            if value_index < args.len() {
                let value: &String = args.get(value_index).unwrap();
                if value.starts_with("--") {
                    panic!("{invalid_value_message}");
                }
                let mut data_path_mutg = DATA_PATH.lock().unwrap();
                let data_path_box = data_path_mutg.deref_mut();
                data_path_box.set(Some(value.clone()));
                true
            } else {
                panic!("{invalid_value_message}");
            }
        } else {
            false
        };

    //Set port number if "--port" flag is set correctly:
    if let Some(flag_index) = linear_search_vec(&args, "--data_path".to_string()) {
        //Check if the data flag has a valid value.
        let value_index = flag_index + 1;
        let invalid_value_message: &str = "Invalid value for the \"--data_path\" option!";
        if value_index < args.len() {
            let value: &String = args.get(value_index).unwrap();
            if value.starts_with("--") {
                panic!("{invalid_value_message}");
            }
            let mut port_num_mutg = SERVER_PORT.lock().unwrap();
            let port_num_mutbox = port_num_mutg.deref_mut();
            port_num_mutbox.set(Some(match value.parse::<u16>() {
                Ok(num) => num,
                Err(_) => {
                    panic!("{invalid_value_message}");
                }
            }));
        } else {
            panic!("{invalid_value_message}");
        }
    }

    //If "--create_new" and "--data_path" options are used, delete everything in the DATA_PATH directory and create a new database there.
    let create_new_set: bool = args.contains(&"--create_new".to_string());
    if create_new_set && data_path_flag_set {
        let data_path_string = get_data_path();
        let data_path_buf: PathBuf = std::fs::canonicalize(data_path_string.as_str())
            .unwrap_or_else(|_e| -> PathBuf {
                panic!("The directory \"{data_path_string}\" cannot be canonicalized!");
            });
        let data_path: &str = data_path_buf.to_str().unwrap_or_else(|| -> &str {
            panic!("The directory given to the \"--data_path\" option must not contain any non-UTF-8 characters!");
        });
        // Remove all files in the given directory:
        std::fs::remove_dir_all(data_path).unwrap();
        // Create the directory:
        std::fs::create_dir(data_path).unwrap();
        // Create a new database at the given directory:
        {
            let mut temp_data_manager: DataManager =
                DataManager::new(data_path, DEFAULT_MEMORY_BUDGET);
            temp_data_manager.save_data();
        }
        let processors_path_buf: PathBuf = data_path_buf.join("./processors.json");
        let processors_path: &str = processors_path_buf.to_str().unwrap();
        let mut processors_file = File::create_new(processors_path).unwrap_or_else(|_e| -> File {
            panic!("Cannot create a new file at the path \"{processors_path}\"!");
        });
        processors_file
            .write_all("{\"local_video_processors\": [], \"local_music_processors\": [], \"thumbnail_processor\": {\"type\": \"local\", \"options\": {\"id\": 0, \"output_format\": \"png\", \"width\": 720, \"height\": 720, \"filter_type\": \"triangle\"}}}".as_bytes())
            .unwrap_or_else(|_e| {
                panic!("Cannot write to the file at the path \"{processors_path}\"!");
            });
    } else if create_new_set && !data_path_flag_set {
        panic!("\"--create_new\" must be used with the \"--data_path\" option!");
    }

    //Initialize PROCESSOR_MANAGER:
    if !t_mode {
        let data_path_mutg = DATA_PATH.lock().unwrap();
        let data_path_box = data_path_mutg.deref();
        PROCESSOR_MANAGER.init(
            std::path::Path::new(data_path_box.get().unwrap().as_str()),
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
            let data_path_mutg = DATA_PATH.lock().unwrap();
            let data_path_box = data_path_mutg.deref();
            let d_path = data_path_box.get().unwrap();
            data_manager.set_data_path(&d_path);
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
                HashMap::new(),
            );
        }
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
