use std::{ops::Index, u64};

use json_escape::escape_str;

use crate::file_handling::saveable::{Saveable, make_json_list_from_u64_list};

#[derive(Debug, Clone)]
pub struct Playlist {
    name: String,
    items: Vec<u64>,
    public: bool,
}

impl Playlist {
    pub fn new(name: String, items: Vec<u64>, public: bool) -> Self {
        Self {
            name,
            items,
            public,
        }
    }

    pub fn new_empty(name: String, public: bool) -> Self {
        Self::new(name, Vec::new(), public)
    }

    pub fn is_public(&self) -> bool {
        self.public
    }

    pub fn set_public(&mut self, public: bool) {
        self.public = public;
    }

    pub fn get_video_ids(&self) -> Vec<u64> {
        self.items.clone()
    }

    pub fn add_video_ids(&mut self, items: Vec<u64>) {
        for item in items {
            self.items.push(item);
        }
    }

    pub fn add_video_id(&mut self, item: u64) {
        self.items.push(item);
    }

    pub fn find_item(&self, item: u64) -> isize {
        for (i, list_item) in self.items.iter().enumerate() {
            if *list_item == item {
                return i as isize;
            }
        }
        -1
    }

    pub fn contains(&self, video_id: u64) -> bool {
        self.find_item(video_id) >= 0
    }

    pub fn remove_video_id(&mut self, video_id: u64) {
        let i = self.find_item(video_id);
        if i >= 0 {
            self.items.remove(i as usize);
        }
    }

    pub fn remove_all_videos(&mut self) {
        self.items = Vec::<u64>::new();
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}

impl Saveable for Playlist {
    fn to_json(&self, _pretty: bool) -> String {
        format!(
            "{{\"name\": \"{}\", \"video_ids\": {}, \"public\": {}}}",
            escape_str(self.get_name().as_str()),
            make_json_list_from_u64_list(&self.get_video_ids()),
            self.is_public()
        )
    }

    fn from_json_saveable(json_data: &serde_json::Value) -> Self
    where
        Self: std::marker::Sized,
    {
        let name: String = json_data["name"].as_str().unwrap().to_string();
        let is_public: bool = json_data["public"].as_bool().unwrap();
        let mut video_ids: Vec<u64> = Vec::<u64>::new();
        for video_id_json in json_data["video_ids"].as_array().unwrap() {
            video_ids.push(video_id_json.as_u64().unwrap());
        }

        Self {
            name,
            items: video_ids,
            public: is_public,
        }
    }
}

impl Index<usize> for Playlist {
    type Output = u64;

    fn index(&self, index: usize) -> &Self::Output {
        &self.items[index]
    }
}
