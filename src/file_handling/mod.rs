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
//!"comment_id": {text": "[comment text]", "creation_time": [time_uploaded_in_secs_since_epoch_in_u64], "replies": {[replies in the comment data format]}}
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
//!"comments": [comment_id, ...],
//!
//!"password_hash": "[hash of user's password]",
//!
//!"liked_vids": [video_id, ...],
//!
//!"disliked_vids": [video_id, ...],
//!
//!"liked_comments": [comment_id, ...],
//!
//!"disliked_comments": [comment_id, ...]
//!
//!}
//!
//!}}

pub mod comment;
pub mod describable;
mod dirty_data;
pub mod from_json;
pub mod likeable;
pub mod saveable;
pub mod user;
pub mod video_data;

pub mod video_processing;

///Module for handling files.
use std::{
    collections::HashMap,
    fs::{self},
    io::Write,
    ops::{Deref, DerefMut},
    path::PathBuf,
    str::FromStr,
    sync::{Arc, LazyLock, Mutex},
};

use crate::{
    file_handling::{
        comment::*, dirty_data::*, likeable::*, saveable::Saveable, user::*, video_data::*,
    },
    search_engine::{self, SearchEngine, searchable::Searchable},
    tests::{self, TestingDefault},
    time::get_current_time,
};

use rand::RngExt;
use serde_json::Value;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

///Read a file from "./" as a stream.
pub async fn file_stream_reader(file_path_str: &str) -> Result<ReaderStream<File>, std::io::Error> {
    let path_str = format!("./{}", file_path_str);
    let video_file = match tokio::fs::File::open(&path_str).await {
        Ok(c) => c,
        Err(_) => {
            return Err(std::io::Error::other(format!(
                "Could not read the file \"{}\"",
                path_str
            )));
        }
    };
    let video_stream = ReaderStream::new(video_file);

    Ok(video_stream)
}

///Read a file and returns it's contents as a Vec of bytes if successful. If not, return an error. Do not use on large files as it will use up a lot of RAM! Note: This function prepends "./" to the beginning of the file path!
pub fn read_small_file(file_path_str: &str) -> Result<Vec<u8>, std::io::Error> {
    let file_path_str_new = format!("./{}", file_path_str);
    // let file_path = std::path::Path::new(&file_path_str);
    let file_path_buf = std::fs::canonicalize(&file_path_str_new).unwrap();
    let file_path = file_path_buf.as_path();
    if !file_path.exists() {
        let err_str = format!("Error: The file \"{}\" does not exist. ", file_path_str_new);
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, err_str));
    }
    match std::fs::read(file_path) {
        Ok(contents) => Ok(contents),
        Err(_) => Err(std::io::Error::other(format!(
            "The file \"{}\" cannot be read. ",
            file_path.display()
        ))),
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

    // search_engine_ref: &'static LazyLock<Mutex<SearchEngine>>,
    search_engine_arc: Arc<Mutex<SearchEngine>>,
}

impl TestingDefault for DataManager {
    fn testing_default() -> Self {
        Self {
            users: HashMap::new(),
            videos: HashMap::new(),
            videos_by_name: HashMap::new(),
            comments_by_id: HashMap::new(),
            data_path: "/dev/null".to_string(),
            // search_engine_ref: &crate::tests::TESTING_DEFAULT_SEARCH_ENGINE,
            search_engine_arc: Arc::new(Mutex::new(SearchEngine::default())),
        }
    }
}

impl DataManager {
    ///Create a new DataManager, with the database in the data_path directory.
    pub fn new(data_path: &str, memory_budget_in_bytes: usize) -> Self {
        Self {
            users: HashMap::new(),
            videos: HashMap::new(),
            videos_by_name: HashMap::new(),
            comments_by_id: HashMap::new(),
            data_path: String::from_str(data_path).unwrap(),
            search_engine_arc: Arc::new(Mutex::new(SearchEngine::new(memory_budget_in_bytes))),
        }
    }

    ///Set the data_path to point to another database.
    pub fn set_data_path(&mut self, data_path: &String) {
        self.data_path = data_path.clone();
    }

    ///Get the current data_path for the database.
    pub fn get_data_path(&self) -> &String {
        &self.data_path
    }

    ///Get an Arc<Mutex<>> pointer that points to the user in the DataManager.
    pub fn get_user(&self, user_id: &String) -> Option<Arc<Mutex<User>>> {
        //Arc::clone(self.users.get(user_id).unwrap())
        match self.users.get(user_id) {
            Some(user_arc) => Some(Arc::clone(user_arc)),
            None => None,
        }
    }

    ///Execute a function (action) on each user in the DataManager.
    pub fn for_all_users<F: Fn(Arc<Mutex<User>>)>(&self, action: F) {
        for user_arc in self.users.values() {
            action(Arc::clone(user_arc));
        }
    }

    ///Whether or not if this DataManager has a user with the specified user id (user_id) or not.
    pub fn has_user(&self, user_id: &String) -> bool {
        self.get_user(user_id).is_some()
    }

    ///Remove the user with the specified user id (user_id).
    pub fn remove_user(&mut self, user_id: &String) {
        if self.has_user(user_id) {
            //Remove user from users map:
            let deleted_user_arc: Arc<Mutex<User>> = self.users.remove(user_id).unwrap();
            let deleted_user_mutg = deleted_user_arc.lock().unwrap();
            let deleted_user: &User = deleted_user_mutg.deref();
            //Delete all videos and their comments:
            for video_id in deleted_user.get_videos().iter() {
                self.remove_video(video_id);
            }
        }
    }

    ///Remove the video with the specified video id (video_id).
    pub fn remove_video(&mut self, video_id: &u64) {
        if self.has_video_by_id(video_id) {
            {
                //Get video:
                // let deleted_video_arc: Arc<Mutex<VideoData>> = self.videos.remove(video_id).unwrap();
                let deleted_video_arc: Arc<Mutex<VideoData>> = self.get_video(video_id).unwrap();
                let mut deleted_video_mutg = deleted_video_arc.lock().unwrap();
                let deleted_video: &mut VideoData = deleted_video_mutg.deref_mut();
                //Remove all comments:
                let comment_arcs_arc: Arc<Mutex<Vec<Arc<Mutex<Comment>>>>> =
                    Arc::new(Mutex::new(Vec::new()));
                deleted_video.for_each_comment_recursive(
                    &|comment_arc: Arc<Mutex<Comment>>| -> () {
                        let comment_arcs_aclone = Arc::clone(&comment_arcs_arc);
                        let mut comment_arcs_mutg = comment_arcs_aclone.lock().unwrap();
                        let comment_arcs: &mut Vec<Arc<Mutex<Comment>>> =
                            comment_arcs_mutg.deref_mut();
                        comment_arcs.push(Arc::clone(&comment_arc));
                    },
                );
                let comment_arcs_mutg = comment_arcs_arc.lock().unwrap();
                for c_arc in comment_arcs_mutg.deref() {
                    let mut c_mutg = c_arc.lock().unwrap();
                    let c = c_mutg.deref_mut();
                    c.remove_all_replies();
                    let c_id: u64 = c.get_id();
                    self.comments_by_id.remove(&c_id);
                    self.neutralize_comment(&c_id);
                }
                self.neutralize_video(video_id);
            }
            //Remove video from SearchEngine:
            self.for_specific_video(video_id, |video_arc: Arc<Mutex<VideoData>>| -> () {
                let mut search_engine_mutg = self.search_engine_arc.lock().unwrap();
                let search_engine = search_engine_mutg.deref_mut();
                let searchable_id: u64 = {
                    let video_mutg = video_arc.lock().unwrap();
                    let video_data = video_mutg.deref();
                    video_data.get_searchable_id()
                };
                search_engine.remove_document(&searchable_id);
            });
            //Remove video from database:
            self.videos.remove(video_id).unwrap();
        }
    }

    fn neutralize_comment(&mut self, comment_id: &u64) {
        self.for_all_users(|user_arc: Arc<Mutex<User>>| -> () {
            let mut user_mutg = user_arc.lock().unwrap();
            let user: &mut User = user_mutg.deref_mut();
            user.remove_like_comment(*comment_id);
            user.remove_dislike_comment(*comment_id);
        });
    }

    fn neutralize_video(&mut self, video_id: &u64) {
        self.for_all_users(|user_arc: Arc<Mutex<User>>| -> () {
            let mut user_mutg = user_arc.lock().unwrap();
            let user: &mut User = user_mutg.deref_mut();
            user.remove_like_vid(*video_id);
            user.remove_dislike_vid(*video_id);
        });
    }

    pub fn for_all_videos<F: Fn(Arc<Mutex<VideoData>>)>(&self, action: F) {
        for video_arc in self.videos.values() {
            action(Arc::clone(video_arc));
        }
    }

    ///Add a new user with the specified user id (user_id), password hash (password_hash), description (description), and icon id (icon_id).
    pub fn add_user(
        &mut self,
        user_id: &String,
        password_hash: &String,
        description: &String,
        icon_id: u64,
    ) {
        if self.has_user(user_id) {
            return;
        }
        //Create the new user:
        let user_arc: Arc<Mutex<User>> = User::new_arc(
            user_id,
            get_current_time(),
            icon_id,
            description,
            &Box::new(Vec::new()),
            &Box::new(Vec::new()),
            &Box::new(Vec::new()),
            password_hash,
            &Box::new(Vec::new()),
            &Box::new(Vec::new()),
            &Box::new(Vec::new()),
            &Box::new(Vec::new()),
        );
        //Add the new user:
        self.users.insert(user_id.clone(), user_arc);
    }

    //Get an Arc<Mutex<>> pointer that points to the video in the DataManager by it's video_id.
    pub fn get_video(&self, video_id: &u64) -> Option<Arc<Mutex<VideoData>>> {
        // Arc::clone(self.videos.get(video_id).unwrap())
        match self.videos.get(video_id) {
            Some(v_arc) => Some(Arc::clone(v_arc)),
            None => None,
        }
    }

    //Gets the directory in which the MPEG_DASH files for the video with the given video id (video_id) are stored.
    pub fn get_video_dir(&self, video_id: &u64) -> PathBuf {
        let mut result: PathBuf =
            std::fs::canonicalize(std::path::PathBuf::from(self.get_data_path())).unwrap();
        result.push("media");
        result.push(video_id.to_string());

        println!("Prints!");
        result
    }

    //Get an Arc<Mutex<>> pointer that points to the video in the DataManager by it's name.
    pub fn get_video_by_name(&self, name: &String) -> Option<Arc<Mutex<VideoData>>> {
        // self.get_video(self.videos_by_name.get(name).unwrap())
        match self.videos_by_name.get(name) {
            Some(id) => self.get_video(id),
            None => None,
        }
    }

    pub fn has_video_by_name(&self, name: &String) -> bool {
        self.get_video_by_name(name).is_some()
    }

    //Get number of videos.
    pub fn get_num_videos(&self) -> u64 {
        self.videos.len() as u64
    }

    //Add a new video.
    pub fn add_video(
        &mut self,
        name: &String,
        author_user_id: &String,
        description: &String,
        input_file_path: &std::path::PathBuf,
    ) -> Option<u64> {
        let mut rand = rand::rng();
        let video_id: u64 = loop {
            let num: u64 = rand.random::<u64>();
            if !self.has_video_by_id(&num) {
                break num;
            }
        };
        match self.add_video_with_id(
            name,
            author_user_id,
            description,
            input_file_path,
            &video_id,
        ) {
            Ok(_) => Some(video_id),
            Err(_) => None,
        }
    }

    pub fn add_video_with_id(
        &mut self,
        name: &String,
        author_user_id: &String,
        description: &String,
        input_file_path: &std::path::PathBuf,
        id: &u64,
    ) -> Result<(), ()> {
        //Check if a video with the given video id already exists:
        if self.has_video_by_id(id) {
            return Err(());
        }

        // Create final name for the video:
        let final_name: String = if !self.has_video_by_name(name) {
            name.clone()
        } else {
            format!("{name} (duplicate)")
        };

        //Create the VideoData object:
        let input_file_canon = std::fs::canonicalize(input_file_path).unwrap();
        let result_arc: Arc<Mutex<VideoData>> = Arc::new(Mutex::new(VideoData::new(
            *id,
            &input_file_canon.to_str().unwrap().to_string(),
            &final_name,
            description,
            author_user_id,
        )));

        //Add to user:
        let user_arc = match self.get_user(author_user_id) {
            Some(u_arc) => u_arc,
            None => return Err(()),
        };
        let mut user_mutg = user_arc.lock().unwrap();
        let user: &mut User = user_mutg.deref_mut();
        user.add_video(*id);

        //Add to DataManager:
        self.videos.insert(*id, result_arc);
        self.videos_by_name.insert(final_name, *id);

        //Add to SearchEngine:
        self.for_specific_video(id, |video_arc: Arc<Mutex<VideoData>>| -> () {
            let mut search_engine_mutg = self.search_engine_arc.lock().unwrap();
            let search_engine = search_engine_mutg.deref_mut();
            search_engine.add_document(video_arc).unwrap();
        });

        Ok(())
    }

    fn for_specific_video<F: Fn(Arc<Mutex<VideoData>>)>(&self, video_id: &u64, action: F) {
        action(self.get_video(video_id).unwrap());
    }

    ///Save the data to the database files and folders.
    pub fn save_data(&mut self) {
        // let data_path = "./data_test_saving";
        let data_path = self.get_data_path();
        println!("Saving data to \"{}\". ", data_path);

        //Convert users to JSON for writing to users.json:
        let mut users_json_string = String::from_str("{\"users\": {").unwrap();
        {
            let mut i = 0;
            for user_arc in self.users.values() {
                if i > 0 {
                    users_json_string.push_str(", ");
                }
                let mut user_mutg = user_arc.lock().unwrap();
                let user: &mut User = user_mutg.deref_mut();
                users_json_string.push_str(user.to_json(false).as_str());
                user.set_dirty(false);
                i += 1;
            }
        }
        users_json_string.push_str("}}");

        //Convert videos and comments to JSON for writing to videos.json:
        let mut videos_json_string = String::from_str("{").unwrap();
        {
            let mut i = 0;
            for video_arc in self.videos.values() {
                if i > 0 {
                    videos_json_string.push_str(", ");
                }
                let mut video_mutg = video_arc.lock().unwrap();
                let video: &mut VideoData = video_mutg.deref_mut();
                videos_json_string.push_str(video.to_json(false).as_str());
                video.set_dirty(false);

                let on_comment_func = |comment_arc: Arc<Mutex<Comment>>| -> () {
                    let mut comment_mutg = comment_arc.lock().unwrap();
                    let comment: &mut Comment = comment_mutg.deref_mut();
                    comment.set_dirty(false);
                };
                video.for_each_comment_recursive(&on_comment_func);

                i += 1;
            }
        }
        videos_json_string.push_str("}");

        //Make directories if they do not exist.
        if !match fs::exists(data_path) {
            Ok(r) => r,
            Err(_) => panic!(
                "The existence of the directory \"{}\" can neither be confirmed nor denied. ",
                data_path
            ),
        } {
            std::fs::create_dir(data_path).expect(
                format!("Error: Could not create the directory \"{}\". ", data_path).as_str(),
            );
        }
        {
            let user_data_folder = format!("{}/user_data", data_path);
            if !match fs::exists(user_data_folder.as_str()) {
                Ok(r) => r,
                Err(_) => panic!(
                    "The existence of the directory \"{}\" can neither be confirmed nor denied. ",
                    user_data_folder
                ),
            } {
                std::fs::create_dir(&user_data_folder).expect(
                    format!("Error: Could not create the directory \"{}\". ", data_path).as_str(),
                );
            }
            //Save users.json
            let users_json_path_string = format!("{}/users.json", user_data_folder);
            let mut users_json_file =
                std::fs::File::create(std::path::Path::new(users_json_path_string.as_str()))
                    .expect(
                        format!(
                            "Error: Could not create or open the file \"{}\" for writing",
                            users_json_path_string
                        )
                        .as_str(),
                    );
            users_json_file
                .write_all(users_json_string.as_bytes())
                .expect(
                    format!(
                        "Error: Could not write user data to the file \"{}\". ",
                        users_json_path_string
                    )
                    .as_str(),
                );
        }
        {
            let media_data_folder = format!("{}/media", data_path);
            if !match fs::exists(media_data_folder.as_str()) {
                Ok(r) => r,
                Err(_) => panic!(
                    "The existence of the directory \"{}\" can neither be confirmed nor denied. ",
                    media_data_folder
                ),
            } {
                std::fs::create_dir(&media_data_folder).expect(
                    format!("Error: Could not create the directory \"{}\". ", data_path).as_str(),
                );
            }
            //Save videos.json file:
            let videos_json_path_string = format!("{}/videos.json", media_data_folder);
            let mut videos_json_file =
                std::fs::File::create(std::path::Path::new(videos_json_path_string.as_str()))
                    .expect(
                        format!(
                            "Error: Could not create or open the file \"{}\" for writing",
                            videos_json_path_string
                        )
                        .as_str(),
                    );
            videos_json_file
                .write_all(videos_json_string.as_bytes())
                .expect(
                    format!(
                        "Error: Could not write user data to the file \"{}\". ",
                        videos_json_path_string
                    )
                    .as_str(),
                );
        }
    }

    ///Load the data from the database.
    pub fn load_data(&mut self) {
        let data_path = &self.data_path;
        // let data_path = self.get_data_path();
        println!("Loading data from \"{}\". ", data_path);

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

            let default_message: &String = &"[DEFAULT VALUE]".to_string();

            let video_data_arc: Arc<Mutex<VideoData>> = VideoData::from_json_value(
                video_id,
                video_json,
                default_message,
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
            //Set author:
            user.for_each_video(|video_id: &u64| -> () {
                let video_data_arc: &mut Arc<Mutex<VideoData>> =
                    self.videos.get_mut(video_id).unwrap();
                let mut video_data_mutg = video_data_arc.lock().unwrap();
                let video_data: &mut VideoData = video_data_mutg.deref_mut();
                video_data.set_author(user_id);
            });

            //Set video likes:
            user.for_each_liked_video(|video_id: &u64| -> () {
                let video_data_arc: &mut Arc<Mutex<VideoData>> =
                    self.videos.get_mut(video_id).unwrap();
                let mut video_data_mutg = video_data_arc.lock().unwrap();
                let video_data: &mut VideoData = video_data_mutg.deref_mut();
                video_data.like();
            });

            //Set video dislikes:
            user.for_each_disliked_video(|video_id: &u64| -> () {
                let video_data_arc: &mut Arc<Mutex<VideoData>> =
                    self.videos.get_mut(video_id).unwrap();
                let mut video_data_mutg = video_data_arc.lock().unwrap();
                let video_data: &mut VideoData = video_data_mutg.deref_mut();
                video_data.dislike();
            });

            //Set comment author:
            user.for_each_comment(|comment_id: &u64| -> () {
                let comment_arc = self.comments_by_id.get_mut(comment_id).unwrap();
                let comment_mutg = &mut comment_arc.lock().unwrap();
                let comment: &mut Comment = comment_mutg.deref_mut();
                comment.set_author(&user.get_user_id());
            });

            //Set comment likes:
            user.for_each_liked_comment(|comment_id: &u64| -> () {
                let comment_arc = self.comments_by_id.get_mut(comment_id).unwrap();
                let comment_mutg = &mut comment_arc.lock().unwrap();
                let comment: &mut Comment = comment_mutg.deref_mut();
                comment.like();
            });

            //Set comment dislikes:
            user.for_each_disliked_comment(|comment_id: &u64| -> () {
                let comment_arc = self.comments_by_id.get_mut(comment_id).unwrap();
                let comment_mutg = &mut comment_arc.lock().unwrap();
                let comment: &mut Comment = comment_mutg.deref_mut();
                comment.dislike();
            });
        }

        //Add videos to search engine:
        self.for_all_videos(|video_data_arc: Arc<Mutex<VideoData>>| -> () {
            let mut search_engine_ref_mutg = self.search_engine_arc.lock().unwrap();
            let search_engine = search_engine_ref_mutg.deref_mut();
            search_engine.add_document(video_data_arc).unwrap();
        });

        println!("Data loaded. ");
    }

    pub fn has_video_by_id(&self, video_id: &u64) -> bool {
        self.videos.contains_key(video_id)
    }

    pub fn has_comment(&self, comment_id: &u64) -> bool {
        self.comments_by_id.contains_key(comment_id)
    }

    ///Makes a comment for a video, returning the comment id of the newly created comment.
    pub fn make_comment(&mut self, user_id: &String, video_id: &u64, text: &String) -> Option<u64> {
        //Get user:
        let user_arc: Arc<Mutex<User>> = match self.get_user(user_id) {
            Some(u_arc) => u_arc,
            None => return None,
        };
        let mut user_mutg = user_arc.lock().unwrap();
        let user: &mut User = user_mutg.deref_mut();
        //Get video:
        let video_arc: Arc<Mutex<VideoData>> = match self.get_video(video_id) {
            Some(v_arc) => v_arc,
            None => return None,
        };
        let mut video_mutg = video_arc.lock().unwrap();
        let video: &mut VideoData = video_mutg.deref_mut();

        //Make the comment:
        let mut rand = rand::rng();
        let comment_id: u64 = loop {
            let val: u64 = rand.random::<u64>();
            if !self.has_comment(&val) {
                break val;
            }
        };
        let comment_arc: Arc<Mutex<Comment>> = Arc::new(Mutex::new(Comment::new_comment(
            comment_id,
            Some(*video_id),
            0,
            user_id,
            text,
        )));
        video.add_comment(Arc::clone(&comment_arc));
        user.add_comment(comment_id);
        self.comments_by_id.insert(comment_id, comment_arc);

        Some(comment_id)
    }

    ///Remove a comment by it's comment id:
    pub fn remove_comment(&mut self, comment_id: &u64) {
        //Get the video_id of the video which the comment is in and the user id of the author:
        let (video_id, author_uid) = {
            let comment_arc = match self.comments_by_id.get(comment_id) {
                Some(c_arc) => c_arc,
                None => return,
            };
            let comment_mutg = comment_arc.lock().unwrap();
            let comment: &Comment = comment_mutg.deref();
            let v_id: u64 = match comment.get_video_id() {
                Some(id) => id,
                None => return,
            };
            (v_id, comment.get_author())
        };
        //Remove all replies:
        {
            for reply_arc in {
                let comment_arc = self.comments_by_id.get(comment_id).unwrap();
                let mut comment_mutg = comment_arc.lock().unwrap();
                let comment: &mut Comment = comment_mutg.deref_mut();
                comment.get_replies()
            }
            .iter()
            {
                let reply_mutg = reply_arc.lock().unwrap();
                let reply: &Comment = reply_mutg.deref();
                //Remove from self.comments_by_id:
                self.comments_by_id.remove(&reply.get_id());
                //Remove from user:
                let user_arc = self.get_user(&reply.get_author()).unwrap();
                let mut user_mutg = user_arc.lock().unwrap();
                let user: &mut User = user_mutg.deref_mut();
                user.remove_comment(reply.get_id());
                //Remove from video:
                match reply.get_video_id() {
                    Some(vid) => {
                        let video_arc = self.get_video(&vid).unwrap();
                        let mut video_mutg = video_arc.lock().unwrap();
                        let video: &mut VideoData = video_mutg.deref_mut();
                        video.remove_comment_recursive(reply.get_id());
                    }
                    None => {}
                }
            }
            let comment_arc = self.comments_by_id.get(comment_id).unwrap();
            let mut comment_mutg = comment_arc.lock().unwrap();
            let comment = comment_mutg.deref_mut();
            comment.remove_all_replies();
        }
        //Remove comment from self.comments_by_id:
        self.comments_by_id.remove(comment_id);
        //Remove comment from user:
        {
            let user_arc = self.get_user(&author_uid).unwrap();
            let mut user_mutg = user_arc.lock().unwrap();
            let user: &mut User = user_mutg.deref_mut();
            user.remove_comment(*comment_id);
        }
        //Remove comment from video:
        let video_arc = match self.get_video(&video_id) {
            Some(v_arc) => v_arc,
            None => return,
        };
        let mut video_mutg = video_arc.lock().unwrap();
        let video: &mut VideoData = video_mutg.deref_mut();
        video.remove_comment_recursive(*comment_id);
    }

    pub fn get_comment(&self, comment_id: &u64) -> Option<&Arc<Mutex<Comment>>> {
        self.comments_by_id.get(comment_id)
    }

    ///Make the user with the specified user id (user_id) like the comment with the specified comment id (comment_id).
    pub fn like_comment(&mut self, user_id: &String, comment_id: &u64) {
        let user_arc = match self.get_user(user_id) {
            Some(a) => a,
            None => return,
        };
        let mut user_mutg = user_arc.lock().unwrap();
        let user: &mut User = user_mutg.deref_mut();

        let comment_arc = match self.get_comment(comment_id) {
            Some(a) => a,
            None => return,
        };
        let mut comment_mutg = comment_arc.lock().unwrap();
        let comment: &mut Comment = comment_mutg.deref_mut();

        user.like_comment(*comment_id);
        comment.like();
    }

    ///Make the user with the specified user id (user_id) remove the like they made for the comment with the specified comment id (comment_id).
    pub fn unlike_comment(&mut self, user_id: &String, comment_id: &u64) {
        let user_arc = match self.get_user(user_id) {
            Some(a) => a,
            None => return,
        };
        let mut user_mutg = user_arc.lock().unwrap();
        let user: &mut User = user_mutg.deref_mut();

        let comment_arc = match self.get_comment(comment_id) {
            Some(a) => a,
            None => return,
        };
        let mut comment_mutg = comment_arc.lock().unwrap();
        let comment: &mut Comment = comment_mutg.deref_mut();

        user.remove_like_comment(*comment_id);
        comment.unlike();
    }

    ///Make the user with the specified user id (user_id) dislike the comment with the specified comment id (comment_id).
    pub fn dislike_comment(&mut self, user_id: &String, comment_id: &u64) {
        let user_arc = match self.get_user(user_id) {
            Some(a) => a,
            None => return,
        };
        let mut user_mutg = user_arc.lock().unwrap();
        let user: &mut User = user_mutg.deref_mut();

        let comment_arc = match self.get_comment(comment_id) {
            Some(a) => a,
            None => return,
        };
        let mut comment_mutg = comment_arc.lock().unwrap();
        let comment: &mut Comment = comment_mutg.deref_mut();

        user.dislike_comment(*comment_id);
        comment.dislike();
    }

    ///Make the user with the specified user id (user_id) remove the dislike they made for the comment with the specified comment id (comment_id).
    pub fn undislike_comment(&mut self, user_id: &String, comment_id: &u64) {
        let user_arc = match self.get_user(user_id) {
            Some(a) => a,
            None => return,
        };
        let mut user_mutg = user_arc.lock().unwrap();
        let user: &mut User = user_mutg.deref_mut();

        let comment_arc = match self.get_comment(comment_id) {
            Some(a) => a,
            None => return,
        };
        let mut comment_mutg = comment_arc.lock().unwrap();
        let comment: &mut Comment = comment_mutg.deref_mut();

        user.remove_dislike_comment(*comment_id);
        comment.undislike();
    }

    ///Edit the comment with the specified comment id (comment_id), setting the new text to the given text (text parameter).
    pub fn edit_comment(&mut self, comment_id: &u64, text: &String) {
        let comment_arc = match self.get_comment(comment_id) {
            Some(c_a) => c_a,
            None => return,
        };
        let mut comment_mutg = comment_arc.lock().unwrap();
        let comment: &mut Comment = comment_mutg.deref_mut();
        comment.set_text(text, get_current_time());
        comment.set_edited(true);
    }

    ///Search for a video by it's name.
    pub fn search_by_name(&self, name: &String) -> HashMap<String, (u64, String, u64)> {
        let mut result: HashMap<String, (u64, String, u64)> = HashMap::new();
        //TODO: Implement the TF-IDF algorithm.

        result
    }

    ///Add a reply to the comment with the specified comment id (comment_id), returning the comment id of the newly created reply if successful.
    pub fn reply_comment(
        &mut self,
        comment_id: &u64,
        text: &String,
        author: &String,
    ) -> Option<u64> {
        let result: Option<Arc<Mutex<Comment>>> = {
            let comment_arc = match self.get_comment(comment_id) {
                Some(a) => a,
                None => return None,
            };
            let mut comment_mutg = comment_arc.lock().unwrap();
            let comment: &mut Comment = comment_mutg.deref_mut();

            let mut rng = rand::rng();
            let reply_cid: u64 = rng.random::<u64>();
            let reply_arc = Arc::new(Mutex::new(Comment::new_comment(
                reply_cid,
                comment.get_video_id(),
                comment.get_level() + 1,
                author,
                text,
            )));

            // return if comment.add_reply(Arc::clone(&reply_arc)) {
            //     self.comments_by_id.insert(reply_cid, Arc::clone(&reply_arc));
            //     Some(reply_cid)
            // } else { None };
            // };
            let r: Option<Arc<Mutex<Comment>>> = if comment.add_reply(Arc::clone(&reply_arc)) {
                Some(reply_arc)
            } else {
                None
            };
            r
        };
        match result {
            Some(reply_arc) => {
                let reply_id: u64 = {
                    let reply_mutg = reply_arc.lock().unwrap();
                    let reply: &Comment = reply_mutg.deref();
                    reply.get_id()
                };
                self.comments_by_id.insert(reply_id, Arc::clone(&reply_arc));
                Some(reply_id)
            }
            None => None,
        }
    }
}
