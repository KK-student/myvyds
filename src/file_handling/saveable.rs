use json_escape::escape_str;

pub trait Saveable {
    fn make_json_list_from_strings(list: &Vec<String>) -> String {
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
        result.push_str("]");

        return result;
    }
    fn make_json_list_from_u64_list(list: &Vec<u64>) -> String {
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
        result.push_str("]");

        return result;
    }

    fn to_json(&self, pretty: bool) -> String;
}
