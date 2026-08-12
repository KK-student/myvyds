use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, MutexGuard},
};

use serde_json::Value;

use crate::file_handling::{dirty_data::DirtyData, likeable::Likeable};

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

    dirty: bool,
    edited: bool,
}

impl Comment {
    ///Create a new Comment from json data.
    pub fn from_json_value(json: &Value, comment_id: u64, level: u64) -> Self {
        if level > 2 {
            panic!("Comment level > 2")
        }
        let author: &str = json["author"].as_str().unwrap();
        let text: &str = json["text"].as_str().unwrap();
        let creation_time: u64 = json["creation_time"].as_u64().unwrap();

        let mut replies: Box<Vec<Arc<Mutex<Comment>>>> = Box::new(Vec::new());
        for (rc_id, rc_json) in json["replies"].as_object().unwrap() {
            let reply_comment_id = rc_id.parse::<u64>().unwrap();
            let reply_arc: Arc<Mutex<Comment>> = Arc::new(Mutex::new(Comment::from_json_value(
                rc_json,
                reply_comment_id,
                level + 1,
            )));
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
            dirty: false,
            edited: false,
        }
    }

    ///Get comment id.
    pub fn get_id(&self) -> u64 {
        self.id
    }

    ///Get comment level.
    pub fn get_level(&self) -> u64 {
        self.level
    }

    ///Get comment author.
    pub fn get_author(&self) -> String {
        self.author.clone()
    }

    ///Set comment author.
    pub fn set_author(&mut self, author: &String) {
        self.author = author.clone();
        self.set_dirty(true);
    }

    ///Get comment text.
    pub fn get_text(&self) -> String {
        self.text.clone()
    }

    ///Set comment text.
    pub fn set_text(&mut self, text: &String, creation_time: u64) {
        self.text = text.clone();
        self.set_creation_time(creation_time);
        self.set_edited(true);
        self.set_dirty(true);
    }

    ///Get comment creation time (in seconds since epoch).
    pub fn get_creation_time(&self) -> u64 {
        self.creation_time
    }

    ///Set comment creation time (in seconds since epoch).
    pub fn set_creation_time(&mut self, creation_time: u64) {
        self.creation_time = creation_time;
        self.set_dirty(true);
    }

    ///Has this comment been edited?
    pub fn is_edited(&self) -> bool {
        self.edited
    }

    ///Set whether or not if this comment has been edited.
    pub fn set_edited(&mut self, edited: bool) {
        self.edited = edited;
        self.set_dirty(true);
    }

    ///Get a copy of the replies vector.
    pub fn get_replies(&self) -> Box<Vec<Arc<Mutex<Comment>>>> {
        return Box::clone(&self.replies);
    }

    ///Parse json comment indexes.
    pub fn parse_json_comment_indexes(json: &Vec<Value>) -> Box<Vec<u64>> {
        let mut comments: Box<Vec<u64>> = Box::new(Vec::new());
        for comment_id in json {
            comments.push(comment_id.as_u64().unwrap());
        }

        return comments;
    }

    ///Get the replies as a HashMap where keys are comment ids and values are the comments themselves.
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

    ///Determine if replies can be added.
    pub fn can_add_replies(&self) -> bool {
        self.level < 2
    }

    ///Add a reply.
    pub fn add_reply(&mut self, reply: Arc<Mutex<Comment>>) -> bool {
        if !Self::can_add_replies(self) {
            return false;
        }

        let replies: &mut Vec<Arc<Mutex<Comment>>> = self.replies.deref_mut();
        replies.push(reply);

        self.set_dirty(true);
        return true;
    }

    fn get_reply_index(&self, reply_comment_id: u64) -> i64 {
        let replies: &Vec<Arc<Mutex<Comment>>> = self.replies.deref();
        for reply_arc in replies {
            let reply_mutg = reply_arc.lock().unwrap();
            let reply = reply_mutg.deref();
            if reply.id == reply_comment_id {
                return reply.id as i64;
            }
        }

        return -1;
    }

    ///Recursively looks for a reply by it's id (reply_comment_id). Returns true if it exists and
    ///false if not.
    pub fn has_reply_recursive(&self, reply_comment_id: u64) -> bool {
        let top_level_index = self.get_reply_index(reply_comment_id);
        if top_level_index >= 0 {
            return true;
        }

        let replies: &Vec<Arc<Mutex<Comment>>> = self.replies.deref();
        for reply_arc in replies {
            let reply_mutg = reply_arc.lock().unwrap();
            let reply = reply_mutg.deref();
            if reply.has_reply_recursive(reply.get_id()) {
                return true;
            }
        }

        return false;
    }

    ///Recursively looks for a reply by it's id (reply_comment_id). Returns the reply if it exists
    ///and () if not.
    pub fn find_reply_recursive(&self, comment_id: u64) -> Result<Arc<Mutex<Comment>>, ()> {
        let top_level_index = self.get_reply_index(comment_id);
        if top_level_index >= 0 {
            Ok(Arc::clone(
                self.replies.deref().get(top_level_index as usize).unwrap(),
            ))
        } else {
            let replies: &Vec<Arc<Mutex<Comment>>> = self.replies.deref();
            for reply_arc in replies {
                let reply_mutg = reply_arc.lock().unwrap();
                let reply = reply_mutg.deref();
                match reply.find_reply_recursive(comment_id) {
                    Ok(res) => return Ok(Arc::clone(&res)),
                    Err(_) => {}
                }
            }

            Err(())
        }
    }

    fn remove_top_reply(&mut self, reply_comment_id: u64) -> bool {
        let reply_index = self.get_reply_index(reply_comment_id);
        if reply_index < 0 {
            return false;
        } else {
            let replies: &mut Vec<Arc<Mutex<Comment>>> = self.replies.deref_mut();
            replies.remove(reply_index as usize);
            self.set_dirty(true);
            return true;
        }
    }

    ///Recursively looks through all of the replies and removes the reply with the specified id (reply_comment_id).
    pub fn remove_reply_recursive(&mut self, reply_comment_id: u64) -> bool {
        let mut result = false;
        if self.remove_top_reply(reply_comment_id) {
            result = true;
        }
        if !result {
            let replies: &Vec<Arc<Mutex<Comment>>> = self.replies.deref();
            for reply_arc in replies {
                let mut reply_mutg = reply_arc.lock().unwrap();
                let reply: &mut Comment = reply_mutg.deref_mut();
                if reply.remove_reply_recursive(reply_comment_id) {
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

    ///Removes all of the replies.
    pub fn remove_all_replies(&mut self) {
        let replies: &mut Vec<Arc<Mutex<Comment>>> = self.replies.deref_mut();
        replies.clear();
        self.set_dirty(true);
    }
}

impl DirtyData for Comment {
    fn set_dirty(&mut self, dirty: bool) {
        self.dirty = dirty;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }
}

impl Likeable for Comment {
    fn get_likes(&self) -> u64 {
        self.likes
    }

    fn set_likes(&mut self, likes: u64) {
        self.likes = likes;
        self.set_dirty(true);
    }

    fn set_dislikes(&mut self, dislikes: u64) {
        self.dislikes = dislikes;
        self.set_dirty(true);
    }

    fn get_dislikes(&self) -> u64 {
        self.dislikes
    }
}
