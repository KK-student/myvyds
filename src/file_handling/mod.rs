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
//!|---info.json
//!
//!./data/media/hidden-vids.json
//!
//!Where video_id is a randomly-generated u64 value, generated when uploading the video - must have 20 digits.
//!
//!###info.json file:
//!
//!{"video": {
//!
//!"id": [video_id],
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
//!"author": "[authors_user_id]",
//!
//!"uploaded_time": [time uploaded in secs since epoch in u64 or 0 if not finished processing],
//!
//!"comments": {[comments in the comment data format]}
//!
//!"likes": [number of likes as u64],
//!
//!"dislikes": [number of dislikes as u64],
//!
//!}}
//!
//!####Comment data format:
//!
//!"comment_id": {"author": "[authors_user_id]", "text": "[comment text]", "creation_time": [time_uploaded_in_secs_since_epoch_in_u64], "replies": {[replies in the comment data format]}, "likes": [number of likes as u64], "dislikes": [number of dislikes as u64]}
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
//!},
//!
//!}}

///Module for handling files.
pub mod file_handling {
    use std::{collections::HashMap, io::ErrorKind, str::FromStr};

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

    ///Represents a user.
    #[derive(Debug, Clone)]
    pub struct User {
        user_id: String,
        time_created: u64,
        icon_id: u64,
        description: String,
        videos: Vec<VideoData>,
        subscriptions: Vec<String>,
        comments: HashMap<u64, Vec<u64>>,
        password_hash: String,
        liked_vids: Vec<u64>,
        disliked_vids: Vec<u64>,
        dirty: bool,
    }

    impl User {
        pub fn get_user_id(&self) -> &String {
            &self.user_id
        }
        pub fn get_time_created(&self) -> &u64 {
            &self.time_created
        }
        pub fn get_icon_id(&self) -> &u64 {
            &self.icon_id
        }
        pub fn set_icon_id(&mut self, icon_id: &u64) {
            self.icon_id = *icon_id;
            self.dirty = true;
        }
        pub fn get_description(&self) -> &String {
            &self.description
        }
        pub fn set_description(&mut self, description: &String) {
            self.description = description.clone();
            self.dirty = true;
        }
        pub fn get_videos(&self) -> Vec<VideoData> {
            self.videos.clone()
        }
        pub fn get_subscriptions(&self) -> Vec<String> {
            self.subscriptions.clone()
        }
        pub fn get_comments(&self) -> HashMap<u64, Vec<u64>> {
            self.comments.clone()
        }
        pub fn get_liked_vids(&self) -> Vec<u64> {
            self.liked_vids.clone()
        }
        pub fn get_disliked_vids(&self) -> Vec<u64> {
            self.disliked_vids.clone()
        }
    }

    ///Represents a video stored on disk.
    #[derive(Debug, Clone)]
    pub struct VideoData {
        id: u64,
        finished_processing: bool,
        input_file: String,
        name: String,
        description: String,
        author: String,
        uploaded_time: u64,
        comments: Vec<Comment>,
        likes: u64,
        dislikes: u64,
        dirty: bool,
    }

    ///Represents a comment stored on disk.
    #[derive(Debug, Clone)]
    pub struct Comment {
        id: u64,
        author: String,
        text: String,
        creation_time: u64,
        replies: Vec<Comment>,
        likes: u64,
        dislikes: u64,
    }

    ///Handles the storage of all of the user accounts, comments, and videos.
    #[derive(Debug)]
    pub struct DataManager {
        users: HashMap<String, User>,
        videos: HashMap<u64, VideoData>,
        videos_by_name: HashMap<String, u64>,
        data_path: String,
    }

    impl DataManager {
        ///Create a new DataManager, with the database in the data_path directory.
        pub fn new(data_path: &str) -> Self {
            let data_manager = DataManager {
                users: HashMap::new(),
                videos: HashMap::new(),
                videos_by_name: HashMap::new(),
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
        pub fn load_data(&self) {
            println!("Loading data from \"{}\". ", self.data_path);

            let data_path = self.get_data_path();
            //Load users:
            //Load JSON:
            let users_json_cont =
                read_small_file(format!("{}/user_data/users.json", data_path).as_str()).unwrap();
            let users_json_cont_slice: &[u8] = users_json_cont.as_slice();
            let users_json_cont_slice_str: &str = str::from_utf8(users_json_cont_slice).unwrap();
            println!(
                "users_json_cont_slice_str: \n{}\n",
                users_json_cont_slice_str
            );
            //Parse JSON:
            let users_json_root: Value = serde_json::from_str(users_json_cont_slice_str).unwrap();
            let users_json: &Value = &users_json_root["users"];
            // println!("users_json_root: {:?}", users_json_root);
            println!("users_json: {:?}", users_json);
            let users_json_vals = users_json.as_object().unwrap();
            println!("\nusers_json_vals: {:?}", users_json_vals);
            //Parse individual users:
            for (user_id, user_json_value) in users_json_vals.iter() {
                let time_created: u64 = user_json_value["time_created"].as_u64().unwrap();
                let icon_id: u64 = user_json_value["icon_id"].as_u64().unwrap();
                let description: &String =
                    &String::from_str(user_json_value["description"].as_str().unwrap()).unwrap();

                let videos_vec: &Vec<Value> = user_json_value["videos"].as_array().unwrap();
                let mut videos: Vec<u64> = Vec::new();
                for video_id_json in videos_vec {
                    videos.push(video_id_json.as_u64().unwrap());
                }

                let subscriptions_vec: &Vec<Value> =
                    user_json_value["subscriptions"].as_array().unwrap();
                let mut subscriptions: Vec<String> = Vec::new();
                for video_id_json in subscriptions_vec {
                    subscriptions.push(String::from_str(video_id_json.as_str().unwrap()).unwrap());
                }

                //Comments
                todo!("Comments");

                let password_hash: &String =
                    &String::from_str(user_json_value["password_hash"].as_str().unwrap()).unwrap();

                let liked_vids_vec: &Vec<Value> = user_json_value["liked_vids"].as_array().unwrap();
                let mut liked_vids: Vec<u64> = Vec::new();
                for video_id_json in liked_vids_vec {
                    liked_vids.push(video_id_json.as_u64().unwrap());
                }

                let disliked_vids_vec: &Vec<Value> =
                    user_json_value["disliked_vids"].as_array().unwrap();
                let mut disliked_vids: Vec<u64> = Vec::new();
                for video_id_json in disliked_vids_vec {
                    disliked_vids.push(video_id_json.as_u64().unwrap());
                }
            }
        }
    }
}
