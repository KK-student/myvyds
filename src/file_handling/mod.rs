//! Module for handling files (including the webpage and media files).
//!
//! ##Structure for media files:
//!
//!./data/media/{video_id}/
//!
//!|---[MPEG-DASH files generated my ffmpeg with ./output.mpd file]
//!
//!|---thumbnail.png
//!
//!|---thumbnail-video.webm
//!
//!|---[input video for while not finished processing - must be deleted after finished processing]
//!
//!./data/media/hidden-vids.json
//!
//!./data/media/videos.json
//!
//!Where video_id is a randomly-generated u64 value, generated when uploading the video - must have 20 digits.
//!
//!###videos.json file:
//!
//!{"[video_id]": {
//!
//!"finished_processing": true/false,
//!
//!"input_file": [file name of input video while not finished processing - must be removed after
//!
//!finished processing],
//!
//!"name": "[video_name]",
//!
//!"description": "[video_description]",
//!
//!"uploaded_time": [time uploaded in secs since epoch in u64 or 0 if not finished processing],
//!
//!"comments": {[comments in the comment data format]}
//!
//!}, ...}
//!
//!####Comment data format:
//!
//!"comment_id": {"author": "[authors_user_id]", "text": "[comment text]", "creation_time": [time_uploaded_in_secs_since_epoch_in_u64], "replies": {[replies in the comment data format]}}
//!
//!###hidden-vids.json file:
//!
//!{"hidden-vids": [video_ids of hidden videos (including the video that are not finished processing) as an array]}
//!
//!##Structure for user data:
//!
//!./data/user_data/
//!
//!|---user-icon-[icon_id].png
//!
//!|---users.json
//!
//!###users.json file:
//!
//!{"users": {
//!
//!"[user_id]": {
//!
//!"time_created": [time user was created in secs since epoch in u64]
//!
//!"icon_id": [icon_id],
//!
//!"description": "[description]",
//!
//!"videos": [video_id, ...],
//!
//!"subscriptions": ["[user_id of another user]", ...],
//!
//!"comments": {"[video_id]": [comment_id, ...], ...},
//!
//!"password_hash": "[hash of user's password]",
//!
//!"liked_vids": [video_id, ...],
//!
//!"disliked_vids": [video_id, ...],
//!
//!"liked_comments": {"[video_id]": [comment_id, ...], ...}
//!
//!"disliked_comments": {"[video_id]": [comment_id, ...], ...}
//!
//!},
//!
//!}}

mod comment;
mod dirty_data;
mod likeable;
mod user;
mod video_data;

///Module for handling files.
pub mod file_handling {
    use std::{
        collections::HashMap,
        ffi::OsString,
        fs::{self, DirEntry, FileType, ReadDir},
        io::ErrorKind,
        ops::{Deref, DerefMut},
        rc::Rc,
        str::FromStr,
        sync::{Arc, Mutex, MutexGuard},
    };

    use crate::file_handling::{comment::*, dirty_data::*, likeable::*, user::*, video_data::*};

    use serde_json::Value;
    use tokio::fs::File;
    use tokio_util::io::{ReaderStream, simplex::new};

    ///Get the mime type for a file. Supports only a limited set of mime types.
    pub fn get_mime_type(file_path: &std::path::Path) -> &str {
        match file_path.extension().and_then(std::ffi::OsStr::to_str) {
            Some("png") => "image/png",
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("gif") => "image/gif",
            Some("html") => "text/html",
            Some("txt") => "text/plain",
            Some("json") => "application/json",
            Some("js") => "text/javascript",
            Some("jsx") => "text/jsx",
            Some("css") => "text/css",

            //Video and audio:
            Some("webm") => "video/webm",

            //Generic:
            _ => "application/octet-stream", // Default MIME type
        }
    }

    ///Read a file from "./" as a stream.
    pub async fn file_stream_reader(
        file_path_str: &str,
    ) -> Result<ReaderStream<File>, std::io::Error> {
        let path_str = format!("./{}", file_path_str);
        let video_file = match tokio::fs::File::open(&path_str).await {
            Ok(c) => c,
            Err(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Could not read the file \"{}\"", path_str),
                ));
            }
        };
        let video_stream = ReaderStream::new(video_file);

        return Ok(video_stream);
    }
    pub fn read_small_file(file_path_str: &str) -> Result<Vec<u8>, std::io::Error> {
        let file_path_str = format!("./{}", file_path_str);
        let file_path = std::path::Path::new(&file_path_str);
        if !file_path.exists() {
            let err_str = format!(
                "Error: The file \"{}\" does not exist. ",
                file_path.display()
            );
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, err_str));
        }
        match std::fs::read(file_path) {
            Ok(contents) => Ok(contents),
            Err(_) => Err(std::io::Error::new(
                ErrorKind::Other,
                format!("The file \"{}\" cannot be read. ", file_path.display()),
            )),
        }
    }

    ///Handles the storage of all of the user accounts, comments, and videos.
    #[derive(Debug)]
    pub struct DataManager {
        users: HashMap<String, Arc<Mutex<User>>>,
        videos: HashMap<u64, Arc<Mutex<VideoData>>>,
        videos_by_name: HashMap<String, u64>,
        comments_by_id: HashMap<u64, Arc<Mutex<Comment>>>,
        data_path: String,
    }

    impl DataManager {
        ///Create a new DataManager, with the database in the data_path directory.
        pub fn new(data_path: &str) -> Self {
            let data_manager = DataManager {
                users: HashMap::new(),
                videos: HashMap::new(),
                videos_by_name: HashMap::new(),
                comments_by_id: HashMap::new(),
                data_path: String::from_str(data_path).unwrap(),
            };
            return data_manager;
        }

        ///Set the data_path to point to another database.
        pub fn set_data_path(&mut self, data_path: &String) {
            self.data_path = data_path.clone();
        }

        ///Get the current data_path for the database.
        pub fn get_data_path(&self) -> &String {
            &self.data_path
        }

        ///Load the data from the database.
        pub fn load_data(&mut self) {
            println!("Loading data from \"{}\". ", self.data_path);

            let data_path = &self.data_path;
            //Load users:
            //Load JSON:
            let users_json_cont =
                read_small_file(format!("{}/user_data/users.json", data_path).as_str()).unwrap();
            let users_json_cont_slice: &[u8] = users_json_cont.as_slice();
            let users_json_cont_slice_str: &str = str::from_utf8(users_json_cont_slice).unwrap();
            //Parse JSON:
            let users_json_root: Value = serde_json::from_str(users_json_cont_slice_str).unwrap();
            let users_json: &Value = &users_json_root["users"];
            let users_json_vals = users_json.as_object().unwrap();
            //Parse individual users:
            for (user_id, user_json_value) in users_json_vals.iter() {
                self.users.insert(
                    user_id.clone(),
                    User::from_json_value(user_json_value, user_id),
                );
            }

            //Parse videos:
            let media_path_string: &String = &format!("{}/media/", data_path);
            //Load JSON:
            let videos_json_cont =
                read_small_file(format!("{}/videos.json", media_path_string).as_str()).unwrap();
            let videos_json_cont_slice: &[u8] = videos_json_cont.as_slice();
            let videos_json_cont_slice_str: &str = str::from_utf8(videos_json_cont_slice).unwrap();
            //Parse JSON:
            let videos_json_root: Value = serde_json::from_str(videos_json_cont_slice_str).unwrap();
            for (video_id_string, video_json) in videos_json_root.as_object().unwrap() {
                let video_id: u64 = video_id_string.parse::<u64>().unwrap();

                let TODO_MESSAGE: &String = &"[DEFAULT VALUE]".to_string();

                let video_data_arc: Arc<Mutex<VideoData>> = VideoData::from_json_value(
                    video_id,
                    video_json,
                    TODO_MESSAGE,
                    |comment_id_num: u64, comment_arc: Arc<Mutex<Comment>>| -> () {
                        self.comments_by_id.insert(comment_id_num, comment_arc);
                    },
                );
                self.videos.insert(video_id, Arc::clone(&video_data_arc));

                let video_data_mutg = video_data_arc.lock().unwrap();
                let video_data = video_data_mutg.deref();
                self.videos_by_name.insert(video_data.get_name(), video_id);
            }

            //Fill in author, likes, and dislikes
            for (user_id, user_arc) in self.users.iter() {
                //Get user.
                let user_mutg = user_arc.lock().unwrap();
                let user = user_mutg.deref();
                //Set author.
                // for video_id in user.get_videos().deref() {
                //     let video_data_arc: &mut Arc<Mutex<VideoData>> =
                //         self.videos.get_mut(video_id).unwrap();
                //     let mut video_data_mutg = video_data_arc.lock().unwrap();
                //     let video_data: &mut VideoData = video_data_mutg.deref_mut();
                //     video_data.set_author(user_id);
                // }
                user.for_each_video(|video_id: &u64| -> () {
                    let video_data_arc: &mut Arc<Mutex<VideoData>> =
                        self.videos.get_mut(video_id).unwrap();
                    let mut video_data_mutg = video_data_arc.lock().unwrap();
                    let video_data: &mut VideoData = video_data_mutg.deref_mut();
                    video_data.set_author(user_id);
                });

                //Set video likes:
                // for video_id in user.get_liked_vids().deref() {
                //     let video_data_arc: &mut Arc<Mutex<VideoData>> =
                //         self.videos.get_mut(video_id).unwrap();
                //     let mut video_data_mutg = video_data_arc.lock().unwrap();
                //     let video_data: &mut VideoData = video_data_mutg.deref_mut();
                //     video_data.like();
                // }
                user.for_each_liked_video(|video_id: &u64| -> () {
                    let video_data_arc: &mut Arc<Mutex<VideoData>> =
                        self.videos.get_mut(video_id).unwrap();
                    let mut video_data_mutg = video_data_arc.lock().unwrap();
                    let video_data: &mut VideoData = video_data_mutg.deref_mut();
                    video_data.like();
                });

                //Set video dislikes:
                // for video_id in user.get_disliked_vids().deref() {
                //     let video_data_arc: &mut Arc<Mutex<VideoData>> =
                //         self.videos.get_mut(video_id).unwrap();
                //     let mut video_data_mutg = video_data_arc.lock().unwrap();
                //     let video_data: &mut VideoData = video_data_mutg.deref_mut();
                //     // video_data.dislikes += 1;
                //     video_data.dislike();
                // }
                user.for_each_disliked_video(|video_id: &u64| -> () {
                    let video_data_arc: &mut Arc<Mutex<VideoData>> =
                        self.videos.get_mut(video_id).unwrap();
                    let mut video_data_mutg = video_data_arc.lock().unwrap();
                    let video_data: &mut VideoData = video_data_mutg.deref_mut();
                    video_data.dislike();
                });

                //Set comment likes:
                for comment_id in user.get_liked_comments().deref() {
                    let comment_arc = self.comments_by_id.get_mut(comment_id).unwrap();
                    let comment_mutg = &mut comment_arc.lock().unwrap();
                    let comment: &mut Comment = comment_mutg.deref_mut();
                    comment.like();
                }

                //Set comment dislikes:
                for comment_id in user.get_disliked_comments().deref() {
                    let comment_arc = self.comments_by_id.get_mut(comment_id).unwrap();
                    let comment_mutg = &mut comment_arc.lock().unwrap();
                    let comment: &mut Comment = comment_mutg.deref_mut();
                    comment.dislike();
                }
            }
        }
    }
}
