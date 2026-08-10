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
        sync::{Arc, Mutex, MutexGuard},
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

    impl User {
        pub fn from_json_value(user_json_value: &Value, user_id: &String) -> Arc<Mutex<Self>> {
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
                subscriptions.push(String::from_str(subscription_json.as_str().unwrap()).unwrap());
            }

            //comments:
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
            let liked_comments: Box<Vec<u64>> = Comment::parse_json_comment_indexes(
                user_json_value["liked_comments"].as_array().unwrap(),
            );

            //disliked_comments:
            let disliked_comments: Box<Vec<u64>> = Comment::parse_json_comment_indexes(
                user_json_value["disliked_comments"].as_array().unwrap(),
            );

            return Arc::new(Mutex::new(User {
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
            }));
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
        comments: Box<Vec<Arc<Mutex<Comment>>>>,
        likes: u64,
        dislikes: u64,
        dirty: bool,
    }

    impl VideoData {
        pub fn from_json_value<F>(
            video_id: u64,
            video_json: &Value,
            default_message: &String,
            mut on_comment: F,
        ) -> Arc<Mutex<VideoData>>
        where
            F: FnMut(u64, Arc<Mutex<Comment>>),
        {
            let finished_processing: bool = video_json["finished_processing"].as_bool().unwrap();
            let input_file: &str = if !finished_processing {
                video_json["input_file"].as_str().unwrap()
            } else {
                ""
            };

            let name: &str = video_json["name"].as_str().unwrap();
            let description: &str = video_json["description"].as_str().unwrap();

            let uploaded_time_json: &Value = &video_json["uploaded_time"];
            let uploaded_time: u64 = uploaded_time_json.as_u64().unwrap();

            // unbug::prelude::breakpoint!();
            let mut comments: Box<Vec<Arc<Mutex<Comment>>>> = Box::new(Vec::new());
            for (comment_id, comment_json) in video_json["comments"].as_object().unwrap() {
                let comment_id_num: u64 = comment_id.parse::<u64>().unwrap();
                let comment_rc: Arc<Mutex<Comment>> = Arc::new(Mutex::new(
                    Comment::from_json_value(comment_json, comment_id_num, 0),
                ));
                comments.push(Arc::clone(&comment_rc));
                // self.comments_by_id.insert(comment_id_num, Arc::clone(&comment_rc));
                on_comment(comment_id_num, Arc::clone(&comment_rc));

                //Add replies to comments_by_id:
                let replies_map_arc: Arc<Mutex<HashMap<u64, Arc<Mutex<Comment>>>>> =
                    Comment::get_children(Arc::clone(&comment_rc));
                let replies_map_mutg = replies_map_arc.lock().unwrap();
                for (reply_id, reply_arc) in replies_map_mutg.deref() {
                    // self.comments_by_id.insert(*reply_id, Arc::clone(reply_arc));
                    on_comment(*reply_id, Arc::clone(reply_arc));
                }
            }

            return Arc::new(Mutex::new(VideoData {
                id: video_id,
                finished_processing: finished_processing,
                input_file: input_file.to_string(),
                name: name.to_string(),
                description: description.to_string(),
                author: default_message.clone(),
                uploaded_time: uploaded_time,
                comments: comments,
                likes: 0,
                dislikes: 0,
                dirty: false,
            }));
        }
    }

    ///Represents a comment stored on disk.
    #[derive(Debug, Clone)]
    pub struct Comment {
        id: u64,
        level: u64,
        author: String,
        text: String,
        creation_time: u64,
        replies: Box<Vec<Arc<Mutex<Comment>>>>,
        likes: u64,
        dislikes: u64,
    }

    impl Comment {
        pub fn from_json_value(json: &Value, comment_id: u64, level: u64) -> Self {
            //{"author": "[authors_user_id]", "text": "[comment text]", "creation_time": [time_uploaded_in_secs_since_epoch_in_u64], "replies": {[replies in the comment data format]}}
            if level > 2 {
                panic!("Comment level > 2")
            }
            let author: &str = json["author"].as_str().unwrap();
            let text: &str = json["text"].as_str().unwrap();
            let creation_time: u64 = json["creation_time"].as_u64().unwrap();

            let mut replies: Box<Vec<Arc<Mutex<Comment>>>> = Box::new(Vec::new());
            for (rc_id, rc_json) in json["replies"].as_object().unwrap() {
                let reply_comment_id = rc_id.parse::<u64>().unwrap();
                let reply_arc: Arc<Mutex<Comment>> = Arc::new(Mutex::new(
                    Comment::from_json_value(rc_json, reply_comment_id, level + 1),
                ));
                replies.push(Arc::clone(&reply_arc));
            }

            Comment {
                id: comment_id,
                level: level,
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
            let mut comments: Box<Vec<u64>> = Box::new(Vec::new());
            for comment_id in json {
                comments.push(comment_id.as_u64().unwrap());
            }

            return comments;
        }

        pub fn get_children(
            top_comment_arc: Arc<Mutex<Comment>>,
        ) -> Arc<Mutex<HashMap<u64, Arc<Mutex<Comment>>>>> {
            let top_comment_mutg = top_comment_arc.lock().unwrap();
            let top_comment: &Comment = top_comment_mutg.deref();

            let replies_arc: Arc<Mutex<HashMap<u64, Arc<Mutex<Comment>>>>> =
                Arc::new(Mutex::new(HashMap::new()));
            let mut replies_mutg = replies_arc.lock().unwrap();
            let replies: &mut HashMap<u64, Arc<Mutex<Comment>>> = replies_mutg.deref_mut();

            for comment_reply_arc in top_comment.replies.deref() {
                let comment_reply_mutg = comment_reply_arc.lock().unwrap();
                let comment_reply: &Comment = comment_reply_mutg.deref();
                replies.insert(comment_reply.id, Arc::clone(&comment_reply_arc));

                let comment_reply_replies_arc: Arc<Mutex<HashMap<u64, Arc<Mutex<Comment>>>>> =
                    Self::get_children(Arc::new(Mutex::new(comment_reply.clone())));
                let comment_reply_replies_mutg = comment_reply_replies_arc.lock().unwrap();
                for (k, v) in comment_reply_replies_mutg.deref() {
                    replies.insert(*k, Arc::clone(v));
                }
            }

            return Arc::clone(&replies_arc);
        }

        pub fn can_add_replies(&self) -> bool {
            self.level < 2
        }

        pub fn add_reply(&mut self, reply: Arc<Mutex<Comment>>) -> bool {
            if !Self::can_add_replies(self) {
                return false;
            }

            let replies: &mut Vec<Arc<Mutex<Comment>>> = self.replies.deref_mut();
            replies.push(reply);

            return true;
        }

        // fn get_reply_index(&self, reply_comment_id: u64) -> i64 {
        //     let replies: &mut Vec<Arc<Mutex<Comment>>> = self.replies.deref_mut();
        //     for reply_arc in replies {
        //         let reply_mutg = reply_arc.lock().unwrap();
        //         let reply = reply_mutg.deref();
        //         if reply.id == reply_comment_id {
        //             return reply.id as i64;
        //         }
        //     }
        //
        //     return -1;
        // }
        //
        // pub fn remove_all_replies(&mut self) {
        //     let replies: &mut Vec<Arc<Mutex<Comment>>> = self.replies.deref_mut();
        //
        //     for reply_arc in replies {
        //         let mut reply_mutg = reply_arc.lock().unwrap();
        //         let reply = reply_mutg.deref_mut();
        //         reply.remove_all_replies();
        //     }
        //
        //     replies.clear();
        // }
        //
        // pub fn remove_reply(&mut self, reply_comment_id: u64) {
        //     let replies: &mut Vec<Arc<Mutex<Comment>>> = self.replies.deref_mut();
        // }
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
                self.videos_by_name
                    .insert(video_data.name.to_string(), video_id);
            }

            //Fill in author, likes, and dislikes
            for (user_id, user_arc) in self.users.iter() {
                //Get user.
                let user_mutg = user_arc.lock().unwrap();
                let user = user_mutg.deref();
                //Set author.
                let user_videos: &Vec<u64> = user.videos.deref();
                for video_id in user_videos {
                    let video_data_arc: &mut Arc<Mutex<VideoData>> =
                        self.videos.get_mut(video_id).unwrap();
                    let mut video_data_mutg = video_data_arc.lock().unwrap();
                    let video_data: &mut VideoData = video_data_mutg.deref_mut();
                    video_data.author = user_id.clone();
                }

                //Set video likes:
                let user_liked_vids: &Vec<u64> = user.liked_vids.deref();
                for video_id in user_liked_vids {
                    let video_data_arc: &mut Arc<Mutex<VideoData>> =
                        self.videos.get_mut(video_id).unwrap();
                    let mut video_data_mutg = video_data_arc.lock().unwrap();
                    let video_data: &mut VideoData = video_data_mutg.deref_mut();
                    video_data.likes += 1;
                }

                //Set video dislikes:
                let user_disliked_vids: &Vec<u64> = user.disliked_vids.deref();
                for video_id in user_disliked_vids {
                    let video_data_arc: &mut Arc<Mutex<VideoData>> =
                        self.videos.get_mut(video_id).unwrap();
                    let mut video_data_mutg = video_data_arc.lock().unwrap();
                    let video_data: &mut VideoData = video_data_mutg.deref_mut();
                    video_data.dislikes += 1;
                }

                //Set comment likes:
                let user_liked_comments: &Vec<u64> = user.liked_comments.deref();
                for comment_id in user_liked_comments {
                    let comment_arc = self.comments_by_id.get_mut(comment_id).unwrap();
                    let comment_mutg = &mut comment_arc.lock().unwrap();
                    let comment: &mut Comment = comment_mutg.deref_mut();
                    comment.likes += 1;
                }

                //Set comment dislikes:
                let user_disliked_comments: &Vec<u64> = user.disliked_comments.deref();
                for comment_id in user_disliked_comments {
                    let comment_arc = self.comments_by_id.get_mut(comment_id).unwrap();
                    let comment_mutg = &mut comment_arc.lock().unwrap();
                    let comment: &mut Comment = comment_mutg.deref_mut();
                    comment.dislikes += 1;
                }
            }
        }
    }
}
