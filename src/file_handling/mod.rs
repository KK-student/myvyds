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
        sync::Arc,
    };

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
        videos: Box<Vec<u64>>,
        subscriptions: Box<Vec<String>>,
        // comments: Box<HashMap<u64, Vec<u64>>>,
        comments: Box<Vec<u64>>,
        password_hash: String,
        liked_vids: Box<Vec<u64>>,
        disliked_vids: Box<Vec<u64>>,
        liked_comments: Box<Vec<u64>>,
        disliked_comments: Box<Vec<u64>>,
        dirty: bool,
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
        comments: Box<Vec<Arc<Comment>>>,
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
        replies: Box<Vec<Comment>>, //TODO: Comment values may not be the same across DataManager.comments_by_id and this. Needs to be fixed.
        likes: u64,
        dislikes: u64,
    }

    impl Comment {
        pub fn from_json_value(json: &Value, comment_id: u64) -> Self {
            //{"author": "[authors_user_id]", "text": "[comment text]", "creation_time": [time_uploaded_in_secs_since_epoch_in_u64], "replies": {[replies in the comment data format]}}
            let author: &str = json["author"].as_str().unwrap();
            let text: &str = json["text"].as_str().unwrap();
            let creation_time: u64 = json["creation_time"].as_u64().unwrap();

            let mut replies: Box<Vec<Comment>> = Box::new(Vec::new());
            for (rc_id, rc_json) in json["replies"].as_object().unwrap() {
                replies.push(Comment::from_json_value(
                    rc_json,
                    rc_id.parse::<u64>().unwrap(),
                ));
            }

            Comment {
                id: comment_id,
                author: author.to_string(),
                text: text.to_string(),
                creation_time: creation_time,
                replies: replies,
                likes: 0,
                dislikes: 0,
            }
        }

        pub fn parse_json_comment_indexes(json: &Vec<Value>) -> Box<Vec<u64>> /*Box<HashMap<u64, Vec<u64>>>*/
        {
            // let mut comments: Box<HashMap<u64, Vec<u64>>> = Box::new(HashMap::new());
            // for (comment_vid, comment_ids_json) in json {
            //     let mut comment_ids: Vec<u64> = Vec::new();
            //     for data in comment_ids_json.as_array().unwrap() {
            //         comment_ids.push(data.as_u64().unwrap());
            //     }
            //     comments.insert(comment_vid.parse::<u64>().unwrap(), comment_ids);
            // }
            let mut comments: Box<Vec<u64>> = Box::new(Vec::new());
            for comment_id in json {
                comments.push(comment_id.as_u64().unwrap());
            }

            return comments;
        }
    }

    ///Handles the storage of all of the user accounts, comments, and videos.
    #[derive(Debug)]
    pub struct DataManager {
        users: HashMap<String, Box<User>>,
        videos: HashMap<u64, VideoData>,
        videos_by_name: HashMap<String, u64>,
        comments_by_id: HashMap<u64, Arc<Comment>>,
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
                //time_created, icon_id, and description:
                let time_created: u64 = user_json_value["time_created"].as_u64().unwrap();
                let icon_id: u64 = user_json_value["icon_id"].as_u64().unwrap();
                let description: &String =
                    &String::from_str(user_json_value["description"].as_str().unwrap()).unwrap();

                //videos:
                let videos_vec: &Vec<Value> = user_json_value["videos"].as_array().unwrap();
                let mut videos: Box<Vec<u64>> = Box::new(Vec::new());
                for video_id_json in videos_vec {
                    videos.push(video_id_json.as_u64().unwrap());
                }

                //subscriptions:
                let subscriptions_vec: &Vec<Value> =
                    user_json_value["subscriptions"].as_array().unwrap();
                let mut subscriptions: Box<Vec<String>> = Box::new(Vec::new());
                for subscription_json in subscriptions_vec {
                    subscriptions
                        .push(String::from_str(subscription_json.as_str().unwrap()).unwrap());
                }

                //comments:
                // let comments: Box<HashMap<u64, Vec<u64>>> = Comment::parse_json_comment_indexes(
                //     user_json_value["comments"].as_object().unwrap(),
                // );
                let comments: Box<Vec<u64>> = Comment::parse_json_comment_indexes(
                    user_json_value["comments"].as_array().unwrap(),
                );

                //password_hash:
                let password_hash: String =
                    String::from_str(user_json_value["password_hash"].as_str().unwrap()).unwrap();

                //liked_vids:
                let liked_vids_vec: &Vec<Value> = user_json_value["liked_vids"].as_array().unwrap();
                let mut liked_vids: Box<Vec<u64>> = Box::new(Vec::new());
                for liked_video_id_json in liked_vids_vec {
                    liked_vids.push(liked_video_id_json.as_u64().unwrap());
                }

                //disliked_vids:
                let disliked_vids_vec: &Vec<Value> =
                    user_json_value["disliked_vids"].as_array().unwrap();
                let mut disliked_vids: Box<Vec<u64>> = Box::new(Vec::new());
                for disl_video_id_json in disliked_vids_vec {
                    disliked_vids.push(disl_video_id_json.as_u64().unwrap());
                }

                //liked_comments:
                // let liked_comments: Box<HashMap<u64, Vec<u64>>> =
                //     Comment::parse_json_comment_indexes(
                //         user_json_value["liked_comments"].as_object().unwrap(),
                //     );
                let liked_comments: Box<Vec<u64>> = Comment::parse_json_comment_indexes(
                    user_json_value["liked_comments"].as_array().unwrap(),
                );

                //disliked_comments:
                // let disliked_comments: Box<HashMap<u64, Vec<u64>>> =
                //     Comment::parse_json_comment_indexes(
                //         user_json_value["disliked_comments"].as_object().unwrap(),
                //     );
                let disliked_comments: Box<Vec<u64>> = Comment::parse_json_comment_indexes(
                    user_json_value["disliked_comments"].as_array().unwrap(),
                );

                //Create and add User:
                self.users.insert(
                    user_id.clone(),
                    Box::new(User {
                        user_id: user_id.clone(),
                        time_created: time_created,
                        icon_id: icon_id,
                        description: description.clone(),
                        videos: videos,
                        subscriptions: subscriptions,
                        comments: comments,
                        password_hash: password_hash.clone(),
                        liked_vids: liked_vids,
                        disliked_vids: disliked_vids,
                        liked_comments: liked_comments,
                        disliked_comments: disliked_comments,
                        dirty: false,
                    }),
                );
            }
            println!("\nParsed user data: {:?}", self.users);

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
                println!("{}: {:?}", video_id_string, video_json);
                unbug::prelude::breakpoint!();
                let video_id: u64 = video_id_string.parse::<u64>().unwrap();

                let finished_processing: bool =
                    video_json["finished_processing"].as_bool().unwrap();
                let input_file: &str = if !finished_processing {
                    video_json["input_file"].as_str().unwrap()
                } else {
                    ""
                };

                let name: &str = video_json["name"].as_str().unwrap();
                let description: &str = video_json["description"].as_str().unwrap();

                let uploaded_time_json: &Value = &video_json["uploaded_time"];
                // println!(
                //     "uploaded_time_json: {:?}, is_u64: {}",
                //     uploaded_time_json,
                //     uploaded_time_json.is_u64()
                // );
                let uploaded_time: u64 = uploaded_time_json.as_u64().unwrap();

                let mut comments: Box<Vec<Arc<Comment>>> = Box::new(Vec::new());
                for (comment_id, comment_json) in video_json["comments"].as_object().unwrap() {
                    let comment_id_num: u64 = comment_id.parse::<u64>().unwrap();
                    let comment_rc: Arc<Comment> =
                        Arc::new(Comment::from_json_value(comment_json, comment_id_num));
                    comments.push(Arc::clone(&comment_rc));
                    self.comments_by_id
                        .insert(comment_id_num, Arc::clone(&comment_rc));
                }

                let TODO_MESSAGE: &String = &"NEEDS TO BE SUBSTITUTED!!!!!!!!".to_string();
                self.videos.insert(
                    video_id,
                    VideoData {
                        id: video_id,
                        finished_processing: finished_processing,
                        input_file: input_file.to_string(),
                        name: name.to_string(),
                        description: description.to_string(),
                        author: TODO_MESSAGE.clone(),
                        uploaded_time: uploaded_time,
                        comments: comments,
                        likes: 0,
                        dislikes: 0,
                        dirty: false,
                    },
                );
            }

            //Fill in author, likes, and dislikes
            for (user_id, user) in self.users.iter() {
                //Set author.
                let user_videos: &Vec<u64> = user.videos.deref();
                for video_id in user_videos {
                    let video_data: &mut VideoData = self.videos.get_mut(video_id).unwrap();
                    video_data.author = user_id.clone();
                }

                //Set video likes:
                let user_liked_vids: &Vec<u64> = user.liked_vids.deref();
                for video_id in user_liked_vids {
                    let video_data: &mut VideoData = self.videos.get_mut(video_id).unwrap();
                    video_data.likes += 1;
                }

                //Set video dislikes:
                let user_disliked_vids: &Vec<u64> = user.disliked_vids.deref();
                for video_id in user_disliked_vids {
                    let video_data: &mut VideoData = self.videos.get_mut(video_id).unwrap();
                    video_data.dislikes += 1;
                }

                //TODO: Change Arc::get_mut to Mutex. Not safe to use Arc::get_mut.
                //Set comment likes:
                let user_liked_comments: &Vec<u64> = user.liked_comments.deref();
                for comment_id in user_liked_comments {
                    let comment_arc = self.comments_by_id.get_mut(comment_id).unwrap(); //FIX: Runtime error here: called `Option::unwrap()` on a `None` value. 
                    let comment: &mut Comment = Arc::<Comment>::get_mut(comment_arc).unwrap();
                    comment.likes += 1;
                }

                //Set comment dislikes:
                // let user_disliked_comments: &Vec<u64> = user.disliked_comments.deref();
                // for comment_id in user_disliked_comments {
                //     let comment: &mut Comment =
                //         self.comments_by_id.get_mut(comment_id).unwrap().deref_mut();
                //     comment.dislikes += 1;
                // }
                let user_disliked_comments: &Vec<u64> = user.disliked_comments.deref();
                for comment_id in user_disliked_comments {
                    let comment_arc = self.comments_by_id.get_mut(comment_id).unwrap();
                    let comment: &mut Comment = Arc::<Comment>::get_mut(comment_arc).unwrap();
                    comment.dislikes += 1;
                }
            }

            //Display results:
            println!("\nParsed user data: {:#?}", self.users);
            println!("\nParsed video data: {:#?}", self.videos);
        }
    }
}
