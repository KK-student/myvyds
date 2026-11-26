use std::{
    collections::HashMap,
    ops::Deref,
    sync::{Arc, Mutex},
};

use json_escape::escape_str;
use serde_json::Value;
pub fn make_json_list_from_strings(list: &Vec<String>) -> String {
    let mut result = String::from("[");
    {
        let mut i = 0;
        while i < list.len() {
            if i > 0 {
                result.push_str(", ");
            }
            let item = list.get(i).unwrap();
            let escaped_string: String = escape_str(item).collect();
            result.push('\"');
            result.push_str(&escaped_string);
            result.push('\"');
            i += 1;
        }
    }
    result.push(']');

    result
}

pub fn make_json_list_from_saveable_list(list: &Vec<Arc<Mutex<impl Saveable>>>) -> String {
    let mut result = String::from("[");
    {
        let mut i = 0;
        while i < list.len() {
            if i > 0 {
                result.push_str(", ");
            }
            let item_arc = list.get(i).unwrap();
            let item_mutg = item_arc.lock().unwrap();
            let item: &dyn Saveable = item_mutg.deref();
            result.push_str(&item.to_json(false));

            i += 1;
        }
    }

    result.push(']');
    result
}

pub fn make_json_list_from_u64_list(list: &Vec<u64>) -> String {
    let mut result = String::from("[");
    {
        let mut i = 0;
        while i < list.len() {
            if i > 0 {
                result.push_str(", ");
            }
            let item = list.get(i).unwrap();
            result.push_str(&item.to_string());
            i += 1;
        }
    }
    result.push(']');

    result
}

pub fn make_json_from_saveable_list(list: &Vec<Arc<Mutex<impl Saveable>>>) -> String {
    let mut result = String::from("{");
    {
        let mut i = 0;
        while i < list.len() {
            if i > 0 {
                result.push_str(", ");
            }
            let item_arc = list.get(i).unwrap();
            let item_mutg = item_arc.lock().unwrap();
            let item: &dyn Saveable = item_mutg.deref();
            result.push_str(&item.to_json(false));

            i += 1;
        }
    }

    result.push('}');
    result
}

pub trait Saveable {
    fn to_json(&self, pretty: bool) -> String;
    fn from_json_saveable(json_data: &Value) -> Self
    where
        Self: std::marker::Sized,
    {
        unimplemented!()
    }
}

impl Saveable for HashMap<String, Vec<u64>> {
    fn to_json(&self, _pretty: bool) -> String {
        let mut result: String = '{'.to_string();
        let mut i = 0;
        for (key, value) in self.iter() {
            if i != 0 {
                result.push(',');
            }
            result.push_str(format!("\"{key}\": {}", make_json_list_from_u64_list(value)).as_str());

            i += 1;
        }

        result.push('}');
        result
    }
}

impl<Sav: Saveable> Saveable for HashMap<String, Sav> {
    fn to_json(&self, _pretty: bool) -> String {
        let mut result: String = '{'.to_string();
        let mut i = 0;
        for (key, value) in self.iter() {
            if i != 0 {
                result.push(',');
            }
            result.push_str(format!("\"{key}\": {}", value.to_json(false)).as_str());

            i += 1;
        }

        result.push('}');
        result
    }
}
