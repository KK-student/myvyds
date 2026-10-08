use std::collections::HashMap;

use image::EncodableLayout;
use rand::{RngExt, random_range};

fn push_bytes_into_u8vec(source: &Vec<u8>, destination: &mut Vec<u8>) {
    for byte in source {
        destination.push(*byte);
    }
}

pub enum MultipartDataPart {
    STRING(String),
    NAMED_FILE(String, Vec<u8>),
    RAW(Vec<u8>),
    SUBFORM(MultipartFormBuilder),
}

pub struct MultipartFormBuilder {
    data: HashMap<String, MultipartDataPart>,
}

impl MultipartFormBuilder {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn add_text(&mut self, entry_name: String, entry_data: String) {
        self.data
            .insert(entry_name, MultipartDataPart::STRING(entry_data));
    }

    pub fn add_named_file(
        &mut self,
        entry_name: String,
        file_name: String,
        raw_entry_data: &Vec<u8>,
    ) {
        self.data.insert(
            entry_name,
            MultipartDataPart::NAMED_FILE(file_name, raw_entry_data.clone()),
        );
    }

    pub fn add_raw_data(&mut self, entry_name: String, raw_entry_data: &Vec<u8>) {
        self.data
            .insert(entry_name, MultipartDataPart::RAW(raw_entry_data.clone()));
    }

    pub fn build(&self, boundary: &str, is_subform: bool) -> Vec<u8> {
        let newl: &str = "\r\n";
        let mut result = Vec::new();
        // let boundary = "frontier";
        push_bytes_into_u8vec(
            &format!(
                "Content-Type: {}; boundary={boundary}{newl}",
                if is_subform {
                    "multipart/mixed"
                } else {
                    "multipart/form-data"
                }
            )
            .as_bytes()
            .to_vec(),
            &mut result,
        );
        for (entry_name, entry_data) in self.data.iter() {
            push_bytes_into_u8vec(
                &format!("--{boundary}{newl}").as_bytes().to_vec(),
                &mut result,
            );
            if !matches!(entry_data, MultipartDataPart::SUBFORM(_)) {
                push_bytes_into_u8vec(
                    &format!("Content-Disposition: form-data; name=\"{entry_name}\"{newl}{newl}")
                        .as_bytes()
                        .to_vec(),
                    &mut result,
                );
            }
            push_bytes_into_u8vec(
                &match entry_data {
                    MultipartDataPart::STRING(data) => data.as_bytes().to_vec(),
                    MultipartDataPart::NAMED_FILE(_, data) => data.as_bytes().to_vec(),
                    MultipartDataPart::RAW(data) => data.clone(),
                    MultipartDataPart::SUBFORM(multipart_form_builder) => {
                        let mut rnd = rand::rng();
                        let new_b = format!("{boundary}-{}", rnd.random::<u8>());
                        multipart_form_builder.build(new_b.as_str(), true)
                    }
                },
                &mut result,
            );
            push_bytes_into_u8vec(&newl.as_bytes().to_vec(), &mut result);
        }

        if !self.data.is_empty() {
            push_bytes_into_u8vec(&format!("--{boundary}--").as_bytes().to_vec(), &mut result);
        }

        result
    }
}
