use std::hash::Hash;

use crate::time::get_current_time;

#[derive(Debug, Clone)]
pub struct Session {
    id: u64,
    user_id: String,
    time_created: u64,
    valid_time_length: u64,
}

impl Session {
    pub fn new(id: u64, user_id: &String, valid_time_length: u64) -> Self {
        Self {
            id,
            user_id: user_id.clone(),
            time_created: get_current_time(),
            valid_time_length,
        }
    }

    pub fn get_id(&self) -> u64 {
        self.id
    }

    pub fn get_user_id(&self) -> String {
        self.user_id.clone()
    }

    pub fn get_time_created(&self) -> u64 {
        self.time_created
    }

    pub fn get_valid_time_length(&self) -> u64 {
        self.valid_time_length
    }

    pub fn is_valid(&self, user_id: &String, session_id: u64) -> bool {
        self.get_user_id().eq(user_id) && self.get_id() == session_id && self.is_not_expired()
    }

    pub fn is_not_expired(&self) -> bool {
        get_current_time() <= (self.get_time_created() + self.get_valid_time_length())
    }
}

impl Hash for Session {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
