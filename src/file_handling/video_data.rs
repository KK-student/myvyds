use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, MutexGuard},
};

use serde_json::Value;

use crate::file_handling::{comment::Comment, dirty_data::DirtyData, likeable::Likeable};

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
    pub fn get_id(&self) -> u64 {
        self.id
    }

    pub fn is_finished_processing(&self) -> bool {
        self.finished_processing
    }

    pub fn set_finished_processing(&mut self, finished_processing: bool) {
        self.finished_processing = finished_processing;
        self.set_dirty(true);
    }

    pub fn get_input_file(&self) -> String {
        self.input_file.clone()
    }

    pub fn set_input_file(&mut self, input_file: &String) {
        self.input_file = input_file.clone();
        self.set_dirty(true);
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn set_name(&mut self, name: &String) {
        self.name = name.clone();
        self.set_dirty(true);
    }

    pub fn get_description(&self) -> String {
        self.description.clone()
    }

    pub fn set_description(&mut self, description: &String) {
        self.description = description.clone();
        self.set_dirty(true);
    }

    pub fn get_author(&self) -> String {
        self.author.clone()
    }

    pub fn set_author(&mut self, author: &String) {
        self.author = author.clone();
        self.set_dirty(true);
    }

    pub fn get_uploaded_time(&self) -> u64 {
        self.uploaded_time
    }

    pub fn get_comments(&self) -> Box<Vec<Arc<Mutex<Comment>>>> {
        self.comments.clone()
    }

    pub fn add_comment(&mut self, comment_arc: Arc<Mutex<Comment>>) {
        self.comments.push(comment_arc);
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

    //TODO: Implement this!
    pub fn get_comment(&self, comment_id: u64) {
        todo!("Implement this!");
    }

    //TODO: Implement this!
    pub fn remove_comment(&mut self, comment_id: u64) {
        todo!("Implement this!");
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
            )));
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
