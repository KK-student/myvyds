use std::ops::Deref;
use std::sync::{Arc, Mutex};

pub fn make_json_from_describable_list(list: &Vec<Arc<Mutex<impl Describable>>>) -> String {
    let mut result = String::from("{");
    {
        let mut i = 0;
        while i < list.len() {
            if i > 0 {
                result.push_str(", ");
            }
            let item_arc = list.get(i).unwrap();
            let item_mutg = item_arc.lock().unwrap();
            let item: &dyn Describable = item_mutg.deref();
            result.push_str(&item.describe());

            i += 1;
        }
    }

    result.push('}');
    return result;
}

pub trait Describable {
    fn describe(&self) -> String;
}
