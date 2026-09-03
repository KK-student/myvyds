use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut},
    process::Command,
    sync::{Arc, Mutex, mpsc},
    thread,
};

use crate::file_handling::video_processing::video_processor::{
    BoolBox, VideoProcessor, VideoProcessorType, kill_process_by_pid,
};

#[derive(Debug)]
pub struct LocalMusicProcessor {
    id: u64,
    active: Arc<Mutex<BoolBox>>,

    ffmpeg_options: Vec<String>, //Ffmpeg options, except for "-f dash ./output.mpd" and "-i [input_file]"

    currently_processing: Arc<Mutex<HashSet<u64>>>, //List of videos that are currently processing.
    finished_processing: Arc<Mutex<HashSet<u64>>>,  //List of videos that have finished processing.
    failed_processing: Arc<Mutex<HashMap<u64, String>>>, //List of videos that have failed to process with the error messages.

    running_pids: Arc<Mutex<HashMap<u64, u32>>>, //Map of the pids of the ffmpeg commands used to process the videos.

    speed: u64,
}

impl LocalMusicProcessor {
    ///Create a new LocalMusicProcessor with the specified id (id parameter), ffmpeg options needed to convert the video file into MPEG_DASH format for streaming, excluding "-i [input_file]" and "-f dash ./output.mpd", as the ffmpeg_opts parameter, and the maximum amount of tflops the processor's GPU could do in a second.
    pub fn new_with_opts(id: u64, ffmpeg_opts: &Vec<String>, speed: u64) -> Self
    where
        Self: Sized,
    {
        return Self {
            id: id,
            active: Arc::new(Mutex::new(BoolBox::new(true))),
            ffmpeg_options: ffmpeg_opts.clone(),
            currently_processing: Arc::new(Mutex::new(HashSet::new())),
            finished_processing: Arc::new(Mutex::new(HashSet::new())),
            failed_processing: Arc::new(Mutex::new(HashMap::new())),
            running_pids: Arc::new(Mutex::new(HashMap::new())),

            speed,
        };
    }
}

impl VideoProcessor for LocalMusicProcessor {
    ///Creates a new LocalMusicProcessor from the given id (id) and JSON data (config_json_data). The JSON data must be in the following JSON format:
    ///
    ///{"ffmpeg_options": [list of all of the ffmpeg options needed to convert the input audio file into MPEG_DASH format as a list of strings (with no spaces in the items themselves), NOT INCLUDING "-i", the input audio file, "-f", "dash", or "./output.mpd"], "speed": [a number that represents the LocalMusicProcessor's speed, similar to the \"max_gpu_tflops\" value for LocalVideoProcessor]}
    fn new(id: u64, config_json_data: &serde_json::Value) -> Self
    where
        Self: Sized,
    {
        let mut ffmpeg_options: Vec<String> = Vec::new();
        for json_item in config_json_data["ffmpeg_options"].as_array().unwrap() {
            ffmpeg_options.push(json_item.as_str().unwrap().to_string());
        }
        return Self::new_with_opts(
            id,
            &ffmpeg_options,
            config_json_data["speed"].as_u64().unwrap(),
        );
    }

    fn process(
        &mut self,
        video_id: u64,
        video_file_path: String,
        result_directory: String,
    ) -> Result<std::sync::mpsc::Receiver<bool>, std::io::Error> {
        if !self.is_activated() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!(
                    "This LocalVideoProcessor(id = {}) is currently deactivated and cannot process anything until activated again",
                    self.get_id()
                ),
            ));
        }
        //Constructing ffmpeg arguments:
        let mut ffmpeg_full_args: Vec<String> = Vec::new();
        ffmpeg_full_args.push("-i".to_string());
        ffmpeg_full_args.push(video_file_path);
        for arg in self.ffmpeg_options.iter() {
            ffmpeg_full_args.push(arg.clone());
        }
        ffmpeg_full_args.push("-f".to_string());
        ffmpeg_full_args.push("dash".to_string());
        ffmpeg_full_args.push("./output.mpd".to_string());

        //Starting ffmpeg processing thread:
        let cp_arc = Arc::clone(&self.currently_processing);
        let fp_arc = Arc::clone(&self.finished_processing);
        let failp_arc = Arc::clone(&self.failed_processing);
        let running_pids_arc = Arc::clone(&self.running_pids);
        let (tx, rx) = mpsc::channel();
        let _child = thread::spawn(move || -> () {
            {
                let mut cp_mutg = cp_arc.lock().unwrap();
                let cp: &mut HashSet<u64> = cp_mutg.deref_mut();
                cp.insert(video_id);
            }
            //Construct ffmpeg command:
            let result_dir_path = std::fs::canonicalize(result_directory).unwrap();
            let mut new_cmd = Command::new("ffmpeg");
            let ffmpeg_command_obj = new_cmd.args(ffmpeg_full_args).current_dir(result_dir_path);

            //Spawn ffmpeg command and capture it's pid and output:
            let child_process = ffmpeg_command_obj.spawn().unwrap();
            {
                let ffmpeg_pid = child_process.id();
                let mut running_pids_mutg = running_pids_arc.lock().unwrap();
                let running_pids: &mut HashMap<u64, u32> = running_pids_mutg.deref_mut();
                running_pids.insert(video_id, ffmpeg_pid as u32);
            }
            let output = child_process
                .wait_with_output()
                .expect("Failed to wait on ffmpeg. ");

            //Remove ffmpeg's pid:
            {
                let mut running_pids_mutg = running_pids_arc.lock().unwrap();
                let running_pids: &mut HashMap<u64, u32> = running_pids_mutg.deref_mut();
                running_pids.remove(&video_id);
            }

            //Remove from self.currently_processing and add to self.finished_processing:
            let mut cp_mutg = cp_arc.lock().unwrap();
            let cp: &mut HashSet<u64> = cp_mutg.deref_mut();
            cp.remove(&video_id);

            let mut fp_mutg = fp_arc.lock().unwrap();
            let fp: &mut HashSet<u64> = fp_mutg.deref_mut();
            fp.insert(video_id);

            //If failed, add to self.failed_processing:
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
                //Exec fail:
                let _ = tx.send(false);
            } else {
                //Exec success:
                let _ = tx.send(true);
            }
        });
        return Ok(rx);
    }

    fn stop_processing(&mut self, video_id: u64) {
        let mut running_pids_mutg = self.running_pids.lock().unwrap();
        let running_pids: &mut HashMap<u64, u32> = running_pids_mutg.deref_mut();
        match running_pids.get(&video_id) {
            Some(pid) => kill_process_by_pid(*pid),
            None => (),
        };
    }

    fn get_currently_processing(&self) -> HashSet<u64> {
        let cp_mutg = self.currently_processing.lock().unwrap();
        let cp = cp_mutg.deref();
        return cp.clone();
    }

    fn is_processing(&self, video_id: &u64) -> bool {
        let cp_mutg = self.currently_processing.lock().unwrap();
        let cp: &HashSet<u64> = cp_mutg.deref();
        cp.contains(video_id)
    }

    fn is_finished_processing(&self, video_id: &u64) -> bool {
        let fp_mutg = self.finished_processing.lock().unwrap();
        let fp: &HashSet<u64> = fp_mutg.deref();
        fp.contains(video_id)
    }

    fn is_processed_successfully(&self, video_id: &u64) -> Result<(), String> {
        let mut failp_mutg = self.failed_processing.lock().unwrap();
        let failp: &mut HashMap<u64, String> = failp_mutg.deref_mut();
        if failp.contains_key(video_id) {
            Err(failp.get(video_id).unwrap().clone())
        } else {
            Ok(())
        }
    }

    fn get_speed(&self) -> u64 {
        self.speed
    }

    fn get_id(&self) -> u64 {
        self.id
    }

    fn activate(&mut self) {
        let mut active_mutg = self.active.lock().unwrap();
        let active_obj = active_mutg.deref_mut();
        active_obj.set(true);
    }

    fn deactivate(&mut self) {
        //self.active = false;
        let mut active_mutg = self.active.lock().unwrap();
        let active_obj = active_mutg.deref_mut();
        active_obj.set(false);
        //Kill all active ffmpeg processes:
        let mut running_pids_mutg = self.running_pids.lock().unwrap();
        let running_pids: &mut HashMap<u64, u32> = running_pids_mutg.deref_mut();
        for pid in running_pids.values() {
            kill_process_by_pid(*pid);
        }
        running_pids.clear();
    }

    fn is_activated(&self) -> bool {
        let act_mutg = self.active.lock().unwrap();
        return act_mutg.deref().get();
    }

    fn get_num_active_jobs(&self) -> u64 {
        let cp_mutg = self.currently_processing.lock().unwrap();
        let cp: &HashSet<u64> = cp_mutg.deref();
        return cp.len() as u64;
    }

    fn get_type_trait() -> super::video_processor::VideoProcessorType
    where
        Self: Sized,
    {
        VideoProcessorType::LOCAL_MUSIC
    }

    fn get_type(&self) -> super::video_processor::VideoProcessorType {
        VideoProcessorType::LOCAL_MUSIC
    }
}
