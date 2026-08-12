use std::{
    str::FromStr,
    sync::{Arc, Mutex},
};

use serde_json::Value;

use crate::file_handling::{comment::Comment, dirty_data::DirtyData};

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
        let subscriptions_vec: &Vec<Value> = user_json_value["subscriptions"].as_array().unwrap();
        let mut subscriptions: Box<Vec<String>> = Box::new(Vec::new());
        for subscription_json in subscriptions_vec {
            subscriptions.push(String::from_str(subscription_json.as_str().unwrap()).unwrap());
        }

        //comments:
        let comments: Box<Vec<u64>> =
            Comment::parse_json_comment_indexes(user_json_value["comments"].as_array().unwrap());

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
        let disliked_vids_vec: &Vec<Value> = user_json_value["disliked_vids"].as_array().unwrap();
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

    pub fn get_user_id(&self) -> String {
        self.user_id.clone()
    }

    pub fn get_time_created(&self) -> u64 {
        self.time_created
    }

    pub fn get_icon_id(&self) -> u64 {
        self.icon_id
    }

    pub fn set_icon_id(&mut self, icon_id: u64) {
        self.icon_id = icon_id;
        self.set_dirty(true);
    }

    pub fn get_description(&self) -> String {
        self.description.clone()
    }

    pub fn set_description(&mut self, description: &String) {
        self.description = description.clone();
        self.set_dirty(true);
    }

    pub fn get_videos(&self) -> Box<Vec<u64>> {
        Box::clone(&self.videos)
    }

    fn get_video_index(&self, video_id: u64) -> isize {
        let videos: &Vec<u64> = &self.videos;
        {
            let mut i = 0;
            while i < videos.len() {
                if *videos.get(i).unwrap() == video_id {
                    return i as isize;
                }
                i += 1;
            }
        }

        return -1;
    }

    pub fn remove_video(&mut self, video_id: u64) -> bool {
        let video_index = self.get_video_index(video_id);
        if video_index < 0 {
            return false;
        }
        self.videos.remove(video_index as usize);

        self.set_dirty(true);
        return true;
    }

    pub fn remove_all_videos(&mut self) {
        self.videos.clear();
        self.set_dirty(true);
    }

    pub fn get_subscriptions(&self) -> Box<Vec<String>> {
        return self.subscriptions.clone();
    }

    pub fn add_subscription(&mut self, subscription: &String) {
        self.subscriptions.push(subscription.clone());
        self.set_dirty(true);
    }

    fn get_subscription_index(&self, subscription: &String) -> isize {
        {
            let mut i = 0;
            while i < self.subscriptions.len() {
                if self.subscriptions.get(i).unwrap() == subscription {
                    return i as isize;
                }

                i += 1;
            }
        }
        return -1;
    }

    pub fn remove_subscription(&mut self, subscription: &String) -> bool {
        let sub_index = self.get_subscription_index(subscription);
        if sub_index < 0 {
            return false;
        }
        self.subscriptions.remove(sub_index as usize);
        self.set_dirty(true);
        return true;
    }

    pub fn remove_all_subscriptions(&mut self) {
        self.subscriptions.clear();
        self.set_dirty(true);
    }

    pub fn get_comments(&self) -> Box<Vec<u64>> {
        return self.comments.clone();
    }

    pub fn add_comment(&mut self, comment_id: u64) {
        self.comments.push(comment_id);
        self.set_dirty(true);
    }

    fn get_comment_index(&self, comment_id: u64) -> isize {
        {
            let mut i = 0;
            while i < self.comments.len() {
                if *self.comments.get(i).unwrap() == comment_id {
                    return i as isize;
                }

                i += 1;
            }
        }
        return -1;
    }

    pub fn remove_comment(&mut self, comment_id: u64) -> bool {
        let comment_index = self.get_comment_index(comment_id);
        if comment_index < 0 {
            return false;
        }
        self.comments.remove(comment_index as usize);
        self.set_dirty(true);
        return true;
    }

    pub fn check_password_hash(&self, password_hash: &String) -> bool {
        return self.password_hash == password_hash.clone();
    }

    pub fn set_password_hash(&mut self, password_hash: &String) {
        self.password_hash = password_hash.clone();
        self.set_dirty(true);
    }

    pub fn get_liked_vids(&self) -> Box<Vec<u64>> {
        self.liked_vids.clone()
    }

    pub fn get_disliked_vids(&self) -> Box<Vec<u64>> {
        self.disliked_vids.clone()
    }

    pub fn is_liked_vid(&self, video_id: u64) -> isize {
        {
            let mut i = 0;
            while i < self.liked_vids.len() {
                if *self.liked_vids.get(i).unwrap() == video_id {
                    return i as isize;
                }

                i += 1;
            }
        }
        return -1;
    }

    pub fn is_disliked_vid(&self, video_id: u64) -> isize {
        {
            let mut i = 0;
            while i < self.disliked_vids.len() {
                if *self.disliked_vids.get(i).unwrap() == video_id {
                    return i as isize;
                }

                i += 1;
            }
        }
        return -1;
    }

    pub fn remove_like_vid(&mut self, video_id: u64) -> bool {
        let vid_index = self.is_liked_vid(video_id);
        if vid_index < 0 {
            return false;
        }

        self.liked_vids.remove(vid_index as usize);
        self.set_dirty(true);
        return true;
    }

    pub fn remove_dislike_vid(&mut self, video_id: u64) -> bool {
        let vid_index = self.is_disliked_vid(video_id);
        if vid_index < 0 {
            return false;
        }

        self.disliked_vids.remove(vid_index as usize);
        self.set_dirty(true);
        return true;
    }

    pub fn like_vid(&mut self, video_id: u64) {
        self.liked_vids.push(video_id);
        self.set_dirty(true);
    }

    pub fn dislike_vid(&mut self, video_id: u64) {
        self.disliked_vids.push(video_id);
        self.set_dirty(true);
    }

    pub fn get_liked_comments(&self) -> Box<Vec<u64>> {
        self.liked_comments.clone()
    }

    pub fn get_disliked_comments(&self) -> Box<Vec<u64>> {
        self.disliked_comments.clone()
    }

    pub fn is_liked_comm(&self, comment_id: u64) -> isize {
        {
            let mut i = 0;
            while i < self.liked_comments.len() {
                if *self.liked_comments.get(i).unwrap() == comment_id {
                    return i as isize;
                }

                i += 1;
            }
        }
        return -1;
    }

    pub fn is_disliked_comm(&self, comment_id: u64) -> isize {
        {
            let mut i = 0;
            while i < self.disliked_comments.len() {
                if *self.disliked_comments.get(i).unwrap() == comment_id {
                    return i as isize;
                }

                i += 1;
            }
        }
        return -1;
    }

    pub fn remove_like_comment(&mut self, comment_id: u64) -> bool {
        let comm_ind = self.is_liked_comm(comment_id);
        if comm_ind < 0 {
            return false;
        }
        self.liked_comments.remove(comm_ind as usize);
        self.set_dirty(true);
        return true;
    }

    pub fn remove_dislike_comment(&mut self, comment_id: u64) -> bool {
        let comm_ind = self.is_disliked_comm(comment_id);
        if comm_ind < 0 {
            return false;
        }
        self.disliked_comments.remove(comm_ind as usize);
        self.set_dirty(true);
        return true;
    }

    pub fn like_comment(&mut self, comment_id: u64) {
        self.liked_comments.push(comment_id);
        self.set_dirty(true);
    }

    pub fn dislike_comment(&mut self, comment_id: u64) {
        self.disliked_comments.push(comment_id);
        self.set_dirty(true);
    }
}

impl DirtyData for User {
    fn set_dirty(&mut self, dirty: bool) {
        self.dirty = dirty;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }
}
