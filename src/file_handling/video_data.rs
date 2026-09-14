use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

use json_escape::escape_str;
use serde_json::Value;

use crate::{
    file_handling::{
        comment::Comment,
        dirty_data::DirtyData,
        likeable::Likeable,
        saveable::{Saveable, make_json_from_saveable_list},
    },
    time::get_current_time,
};

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
    comments: Box<Vec<Arc<Mutex<Comment>>>>, //TODO: Make this into a LinkedList so that adding and deleting would be faster, if memory is not an issue.
    likes: u64,
    dislikes: u64,
    dirty: bool,
}

impl VideoData {
    ///Create a new VideoData that has not yet finished processing, has the uploaded time set to now, and has no comments, likes or dislikes.
    pub fn new(
        id: u64,
        input_file: &String,
        name: &String,
        description: &String,
        author: &String,
    ) -> Self {
        return Self {
            id,
            finished_processing: false,
            input_file: input_file.clone(),
            name: name.clone(),
            description: description.clone(),
            author: author.clone(),
            uploaded_time: get_current_time(),
            comments: Box::new(Vec::new()),
            likes: 0,
            dislikes: 0,
            dirty: true,
        };
    }

    ///Get video id.
    pub fn get_id(&self) -> u64 {
        self.id
    }

    ///Is the video finished processing?
    pub fn is_finished_processing(&self) -> bool {
        self.finished_processing
    }

    ///Set whether or not if the video is finished processing.
    pub fn set_finished_processing(&mut self, finished_processing: bool) {
        self.finished_processing = finished_processing;
        self.set_dirty(true);
    }

    ///Get the video input file used for processing.
    pub fn get_input_file(&self) -> String {
        self.input_file.clone()
    }

    ///Get the video input file used for processing.
    pub fn set_input_file(&mut self, input_file: &String) {
        self.input_file = input_file.clone();
        self.set_dirty(true);
    }

    ///Get the video name.
    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    ///Set the video name.
    pub fn set_name(&mut self, name: &String) {
        self.name = name.clone();
        self.set_dirty(true);
    }

    ///Get the video description.
    pub fn get_description(&self) -> String {
        self.description.clone()
    }

    ///Set the video description.
    pub fn set_description(&mut self, description: &String) {
        self.description = description.clone();
        self.set_dirty(true);
    }

    ///Get the video author.
    pub fn get_author(&self) -> String {
        self.author.clone()
    }

    ///Set the video author.
    pub fn set_author(&mut self, author: &String) {
        self.author = author.clone();
        self.set_dirty(true);
    }

    ///Get video uploaded time as seconds since epoch.
    pub fn get_uploaded_time(&self) -> u64 {
        self.uploaded_time
    }

    ///Get all of the top-level comments.
    pub fn get_comments(&self) -> Box<Vec<Arc<Mutex<Comment>>>> {
        self.comments.clone()
    }

    ///Get number of comments.
    pub fn get_num_comments(&self) -> u64 {
        self.comments.len() as u64
    }

    ///Add a comment to level 0.
    pub fn add_comment(&mut self, comment_arc: Arc<Mutex<Comment>>) {
        self.comments.push(Arc::clone(&comment_arc));
        let mut comment_mutg = comment_arc.lock().unwrap();
        let comment: &mut Comment = comment_mutg.deref_mut();
        comment.set_video_id(self.get_id());
        self.set_dirty(true);
    }

    fn get_top_comment_index(&self, comment_id: u64) -> isize {
        {
            let mut i = 0;
            while i < self.comments.len() {
                let comment_arc = self.comments.get(i).unwrap();
                let comment_mutg = comment_arc.lock().unwrap();
                let comment = comment_mutg.deref();
                if comment.get_id() == comment_id {
                    return i as isize;
                }

                i += 1;
            }
        }
        return -1;
    }

    ///Recursively looks for a comment or reply with the given id (comment_id). Returns true if
    ///found, otherwise false.
    pub fn has_comment(&self, comment_id: u64) -> bool {
        if self.get_top_comment_index(comment_id) >= 0 {
            return true;
        }

        let comments = self.comments.deref();
        for comment_arc in comments {
            let comment_mutg = comment_arc.lock().unwrap();
            let comment: &Comment = comment_mutg.deref();
            if comment.has_reply_recursive(comment_id) {
                return true;
            }
        }

        return false;
    }

    ///Recursively looks for a comment or reply with the given id (comment_id). Returns the comment
    ///object if found, otherwise ().
    pub fn get_comment(&self, comment_id: u64) -> Result<Arc<Mutex<Comment>>, ()> {
        let top_comment_index = self.get_top_comment_index(comment_id);
        if top_comment_index >= 0 {
            Ok(Arc::clone(
                self.comments
                    .deref()
                    .get(top_comment_index as usize)
                    .unwrap(),
            ))
        } else {
            let comments: &Vec<Arc<Mutex<Comment>>> = self.comments.deref();
            for comment_arc in comments {
                let comment_mutg = comment_arc.lock().unwrap();
                let comment = comment_mutg.deref();
                match comment.find_reply_recursive(comment_id) {
                    Ok(res) => return Ok(Arc::clone(&res)),
                    Err(_) => {}
                }
            }
            Err(())
        }
    }

    fn remove_top_comment(&mut self, comment_id: u64) -> bool {
        let top_comment_index = self.get_top_comment_index(comment_id);
        if top_comment_index >= 0 {
            self.set_dirty(true);
            self.comments.deref_mut().remove(top_comment_index as usize);
            return true;
        }

        return false;
    }

    ///Recursively looks for and removes a comment or reply with the given id (comment_id). Returns true if found, otherwise false.
    pub fn remove_comment_recursive(&mut self, comment_id: u64) -> bool {
        let mut result = false;
        {
            if self.remove_top_comment(comment_id) {
                result = true;
            }
        }
        if !result {
            for comment_arc in self.comments.deref() {
                let mut comment_mutg = comment_arc.lock().unwrap();
                let comment: &mut Comment = comment_mutg.deref_mut();
                if comment.remove_reply_recursive(comment_id) {
                    result = true;
                    break;
                }
            }
        }
        if result {
            self.set_dirty(true);
        }

        return result;
    }

    ///Does an action (on_comment) on each comment (and it's replies) recursively.
    pub fn for_each_comment_recursive<F: Fn(Arc<Mutex<Comment>>)>(&self, on_comment: &F) {
        for comment_arc in self.comments.deref() {
            on_comment(Arc::clone(comment_arc));
            let mut comment_mutg = comment_arc.lock().unwrap();
            let comment: &mut Comment = comment_mutg.deref_mut();
            comment.for_each_reply_recursive(on_comment);
        }
    }

    ///Does an action (on_comment) on each comment in the range of indices (inclusive on both ends) non-recursively.
    pub fn for_each_comment_range<F: FnMut(Arc<Mutex<Comment>>)>(
        &self,
        start_inclusive: u64,
        end_inclusive: u64,
        mut on_comment: F,
    ) {
        let mut i: u64 = start_inclusive;
        while i <= end_inclusive && i < self.get_num_comments() {
            let comment_arc: &Arc<Mutex<Comment>> = self.comments.deref().get(i as usize).unwrap();
            on_comment(Arc::clone(comment_arc));
            i += 1;
        }
    }
}

impl DirtyData for VideoData {
    fn set_dirty(&mut self, dirty: bool) {
        self.dirty = dirty;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }
}

impl Likeable for VideoData {
    fn set_likes(&mut self, likes: u64) {
        self.likes = likes;
        self.set_dirty(true);
    }

    fn get_likes(&self) -> u64 {
        self.likes
    }

    fn set_dislikes(&mut self, dislikes: u64) {
        self.dislikes = dislikes;
        self.set_dirty(true);
    }

    fn get_dislikes(&self) -> u64 {
        self.dislikes
    }
}

impl VideoData {
    ///Creates a new VideoData object from the given json data (video_json), video id (video_id), and default message to put in a string variable if value is not known yet (default_message). Also performs a given function for each comment (on_comment(comment_id, comment)).
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
            let comment_rc: Arc<Mutex<Comment>> = Arc::new(Mutex::new(Comment::from_json_value(
                comment_json,
                comment_id_num,
                0,
                default_message,
            )));
            comments.push(Arc::clone(&comment_rc));
            // self.comments_by_id.insert(comment_id_num, Arc::clone(&comment_rc));
            on_comment(comment_id_num, Arc::clone(&comment_rc));

            //Add replies to comments_by_id:
            let replies_map_arc: Arc<Mutex<HashMap<u64, Arc<Mutex<Comment>>>>> =
                Comment::get_children(Arc::clone(&comment_rc));
            let replies_map_mutg = replies_map_arc.lock().unwrap();
            for (reply_id, reply_arc) in replies_map_mutg.deref() {
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

impl Saveable for VideoData {
    fn to_json(&self, pretty: bool) -> String {
        let comments_json = make_json_from_saveable_list(self.comments.deref());
        return if pretty {
            format!(
                "\"{}\": {{
\t\"finished_processing\": {},{}
\t\"name\": \"{}\",
\t\"description\": \"{}\",
\t\"uploaded_time\": {},
\t\"comments\": {}
}}",
                self.get_id(),
                if self.is_finished_processing() {
                    "true"
                } else {
                    "false"
                },
                if self.is_finished_processing() {
                    format!(
                        "\n\t\"input_file\": \"{}\", ",
                        escape_str(&self.get_input_file()).collect::<String>()
                    )
                } else {
                    "".to_string()
                },
                escape_str(&self.get_name()).collect::<String>(),
                escape_str(&self.get_description()).collect::<String>(),
                self.get_uploaded_time(),
                comments_json
            )
        } else {
            format!(
                "\"{}\": {{\"finished_processing\": {},{}\"name\": \"{}\",\"description\": \"{}\",\"uploaded_time\": {},\"comments\": {}}}",
                self.get_id(),
                if self.is_finished_processing() {
                    "true"
                } else {
                    "false"
                },
                if self.is_finished_processing() {
                    format!(
                        "\"input_file\": \"{}\", ",
                        escape_str(&self.get_input_file()).collect::<String>()
                    )
                } else {
                    "".to_string()
                },
                escape_str(&self.get_name()).collect::<String>(),
                escape_str(&self.get_description()).collect::<String>(),
                self.get_uploaded_time(),
                comments_json
            )
        };
    }
}
