use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut},
    process::Command,
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

use serde_json::Value;

pub trait VideoProcessor {
    fn new(config_json_data: &Value) -> Self;

    fn process(
        &mut self,
        video_id: u64,
        video_file_path: String,
        result_directory: String,
    ) -> JoinHandle<()>;

    fn is_processing(&self, video_id: &u64) -> bool;

    fn is_finished_processing(&self, video_id: &u64) -> bool;

    fn is_processed_successfully(&self, video_id: &u64) -> bool;

    fn get_speed(&self) -> u64;
}

pub struct LocalVideoProcessor {
    ffmpeg_options: Vec<String>, //Ffmpeg options, except for "-f dash -o ./output.mpd"

    currently_processing: Arc<Mutex<HashSet<u64>>>,
    finished_processing: Arc<Mutex<HashSet<u64>>>,
    failed_processing: Arc<Mutex<HashMap<u64, String>>>,

    max_gpu_tflops: u64,
}

impl VideoProcessor for LocalVideoProcessor {
    fn new(config_json_data: &Value) -> Self {
        // let ffmpeg_options: &str = config_json_data["ffmpeg_options"].as_str().unwrap();
        let mut ffmpeg_options: Vec<String> = Vec::new();
        for json_item in config_json_data["ffmpeg_options"].as_array().unwrap() {
            ffmpeg_options.push(json_item.as_str().unwrap().to_string());
        }

        let max_gpu_tflops: u64 = config_json_data["max_gpu_tflops"].as_u64().unwrap();

        return Self {
            ffmpeg_options: ffmpeg_options,
            currently_processing: Arc::new(Mutex::new(HashSet::new())),
            finished_processing: Arc::new(Mutex::new(HashSet::new())),
            failed_processing: Arc::new(Mutex::new(HashMap::new())),
            max_gpu_tflops: max_gpu_tflops,
        };
    }

    fn is_processing(&self, video_id: &u64) -> bool {
        // self.currently_processing.contains(video_id)
        let cp_mutg = self.currently_processing.lock().unwrap();
        let cp: &HashSet<u64> = cp_mutg.deref();
        cp.contains(video_id)
    }

    fn is_finished_processing(&self, video_id: &u64) -> bool {
        let fp_mutg = self.finished_processing.lock().unwrap();
        let fp: &HashSet<u64> = fp_mutg.deref();
        fp.contains(video_id)
    }

    fn is_processed_successfully(&self, video_id: &u64) -> bool {
        let mut failp_mutg = self.failed_processing.lock().unwrap();
        let failp: &mut HashMap<u64, String> = failp_mutg.deref_mut();
        !failp.contains_key(video_id)
    }

    fn process(
        &mut self,
        video_id: u64,
        video_file_path: String,
        result_directory: String,
    ) -> JoinHandle<()> {
        //Constructing ffmpeg arguments:
        let mut ffmpeg_full_args: Vec<String> = Vec::new();
        ffmpeg_full_args.push("-i".to_string());
        ffmpeg_full_args.push(video_file_path);
        for arg in self.ffmpeg_options.iter() {
            ffmpeg_full_args.push(arg.clone());
        }
        ffmpeg_full_args.push("-f".to_string());
        ffmpeg_full_args.push("dash".to_string());
        ffmpeg_full_args.push("-o".to_string());
        ffmpeg_full_args.push("./output.mpd".to_string());

        //Starting ffmpeg processing thread:
        let cp_arc = Arc::clone(&self.currently_processing);
        let fp_arc = Arc::clone(&self.finished_processing);
        let failp_arc = Arc::clone(&self.failed_processing);
        let child = thread::spawn(move || -> () {
            {
                let mut cp_mutg = cp_arc.lock().unwrap();
                let cp: &mut HashSet<u64> = cp_mutg.deref_mut();
                cp.insert(video_id);
            }
            let output = Command::new("ffmpeg")
                .args(ffmpeg_full_args)
                .current_dir(std::fs::canonicalize(result_directory).unwrap())
                .output()
                .expect("Ffmpeg failed. ");
            let mut cp_mutg = cp_arc.lock().unwrap();
            let cp: &mut HashSet<u64> = cp_mutg.deref_mut();
            cp.remove(&video_id);

            let mut fp_mutg = fp_arc.lock().unwrap();
            let fp: &mut HashSet<u64> = fp_mutg.deref_mut();
            fp.insert(video_id);
            if !output.status.success() {
                let mut failp_mutg = failp_arc.lock().unwrap();
                let failp: &mut HashMap<u64, String> = failp_mutg.deref_mut();
                failp.insert(
                    video_id,
                    format!(
                        "Stdout: \n{}\nStderr: \n{}",
                        String::from_utf8(output.stdout).unwrap(),
                        String::from_utf8(output.stderr).unwrap()
                    ),
                );
            }
        });
        return child;
    }

    fn get_speed(&self) -> u64 {
        self.max_gpu_tflops
    }
}

#[derive(Debug)]
pub struct VideoProcessorManager {}
