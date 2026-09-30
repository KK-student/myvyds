use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut},
    panic,
    process::Command,
    sync::{Arc, Mutex, mpsc},
    thread,
};

use serde_json::Value;

use crate::file_handling::{
    saveable::{Saveable, make_json_list_from_strings},
    video_processing::video_processor::{
        BoolBox, VeryBasicVideoFileDescription, VideoProcessor, VideoProcessorType,
        get_video_description, kill_process_by_pid,
    },
};

#[derive(Debug)]
///A VideoProcessor that processes the videos on the current local server (ie. the same server that contains the MPEG_DASH files and the information about the videos, comments and users).
pub struct LocalVideoProcessor {
    id: u64,
    active: Arc<Mutex<BoolBox>>,

    ffmpeg_options: Vec<String>, //Ffmpeg options, except for "-f dash -o ./output.mpd" and "-i [input_file]"

    currently_processing: Arc<Mutex<HashSet<u64>>>, //List of videos that are currently processing.
    finished_processing: Arc<Mutex<HashSet<u64>>>,  //List of videos that have finished processing.
    failed_processing: Arc<Mutex<HashMap<u64, String>>>, //List of videos that have failed to process with the error messages.

    running_pids: Arc<Mutex<HashMap<u64, u32>>>, //Map of the pids of the ffmpeg commands used to process the videos.

    max_gpu_tflops: u64,
}

impl LocalVideoProcessor {
    ///Create a new LocalVideoProcessor with the specified id (id parameter), ffmpeg options needed to convert the video file into MPEG_DASH format for streaming, excluding "-i [input_file]" and "-f dash ./output.mpd", as the ffmpeg_opts parameter, and the maximum amount of tflops the processor's GPU could do in a second.
    pub fn new_with_opts(id: u64, ffmpeg_opts: &Vec<String>, max_gpu_tflops: u64) -> Self
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

            max_gpu_tflops: max_gpu_tflops,
        };
    }
}

impl Saveable for LocalVideoProcessor {
    fn to_json(&self, pretty: bool) -> String {
        let ffmpeg_options_string: String = make_json_list_from_strings(&self.ffmpeg_options);
        let max_gpu_flops_string: String = self.max_gpu_tflops.to_string();
        return format!(
            "{{\"id\": {}, \"ffmpeg_options\": {ffmpeg_options_string}, \"max_gpu_tflops\": {max_gpu_flops_string}}}",
            self.get_id()
        );
    }
}

impl VideoProcessor for LocalVideoProcessor {
    ///Create a new LocalVideoProcessor with the given id (id) and from the given JSON configuration data (config_json_data). The given JSON data must be in the following format:
    ///
    ///{"id": [processor_id], "ffmpeg_options": [list of all of the ffmpeg options needed to convert the input video file into MPEG_DASH format as a list of strings (with no spaces in the items themselves), NOT INCLUDING "-i", the input video file, "-f", "dash", or "./output.mpd"], "max_gpu_tflops": [the maximum amount of tflops the server's GPU could do as a u64]}
    fn new(config_json_data: &Value) -> Self
    where
        Self: Sized,
    {
        let mut ffmpeg_options: Vec<String> = Vec::new();
        for json_item in config_json_data["ffmpeg_options"].as_array().unwrap() {
            ffmpeg_options.push(json_item.as_str().unwrap().to_string());
        }
        let id: u64 = config_json_data["id"].as_u64().unwrap();

        return Self::new_with_opts(
            id,
            &ffmpeg_options,
            config_json_data["max_gpu_tflops"].as_u64().unwrap(),
        );
    }

    fn get_id(&self) -> u64 {
        self.id
    }

    fn is_processing(&self, video_id: &u64) -> bool {
        // self.currently_processing.contains(video_id)
        let cp_mutg = self.currently_processing.lock().unwrap();
        let cp: &HashSet<u64> = cp_mutg.deref();
        cp.contains(video_id)
    }

    fn get_currently_processing(&self) -> HashSet<u64> {
        let cp_mutg = self.currently_processing.lock().unwrap();
        let cp = cp_mutg.deref();
        return cp.clone();
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

    fn process_video(
        &mut self,
        video_id: u64,
        video_file_path: String,
        result_directory: String,
    ) -> Result<mpsc::Receiver<bool>, std::io::Error> {
        if !self.is_activated() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!(
                    "This LocalVideoProcessor(id = {}) is currently deactivated and cannot process anything until activated again",
                    self.get_id()
                ),
            ));
        }

        //Check if this VideoProcessor supports the given file:
        //Canonicalize video_file_path:
        let video_file_path_full_arc: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
        match panic::catch_unwind(|| -> () {
            let path_arc = Arc::clone(&video_file_path_full_arc);
            let mut path_mutg = path_arc.lock().unwrap();
            let full_path: &mut String = path_mutg.deref_mut();
            full_path.push_str(
                std::fs::canonicalize(video_file_path.clone())
                    .unwrap()
                    .to_str()
                    .unwrap(),
            );
        }) {
            Ok(_) => {}
            Err(e) => {
                let err_str = format!(
                    "Error: Failed to canonicalize the following file path \"{video_file_path}\". Error message: {:?}.",
                    e
                );
                return Err(std::io::Error::new(std::io::ErrorKind::NotFound, err_str));
            }
        };
        let video_file_path_full_mutg = video_file_path_full_arc.lock().unwrap();
        let video_file_path_full: &String = video_file_path_full_mutg.deref();
        let description: VeryBasicVideoFileDescription =
            match get_video_description(video_file_path_full.as_str()) {
                Ok(desc) => desc,
                Err(e) => return Err(e),
            };
        if !self.is_supported_file(&description) {
            let err_str = format!(
                "Error: The file with the following file path \"{video_file_path}\" is not supported by the VideoProcessor with the id of {}.",
                self.get_id()
            );
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                err_str,
            ));
        }

        //Constructing ffmpeg arguments:
        let mut ffmpeg_full_args: Vec<String> = Vec::new();
        ffmpeg_full_args.push("-i".to_string());
        ffmpeg_full_args.push(String::from(video_file_path_full));
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

    fn get_speed(&self) -> u64 {
        self.max_gpu_tflops
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
        let active_mutg = self.active.lock().unwrap();
        let active_obj = active_mutg.deref();
        active_obj.get()
    }

    fn get_num_active_jobs(&self) -> u64 {
        let cp_mutg = self.currently_processing.lock().unwrap();
        let cp: &HashSet<u64> = cp_mutg.deref();
        return cp.len() as u64;
    }

    fn get_type_trait() -> VideoProcessorType {
        VideoProcessorType::LOCAL_VIDEO
    }

    fn get_type(&self) -> VideoProcessorType {
        VideoProcessorType::LOCAL_VIDEO
    }

    fn is_video_processor(&self) -> bool {
        true
    }
}

impl Clone for LocalVideoProcessor {
    fn clone(&self) -> Self {
        let cp_mutg = self.currently_processing.lock().unwrap();
        let cp_clone = cp_mutg.deref().clone();

        let fp_mutg = self.finished_processing.lock().unwrap();
        let fp_clone = fp_mutg.deref().clone();

        let failp_mutg = self.failed_processing.lock().unwrap();
        let failp_clone = failp_mutg.deref().clone();

        let r_mutg = self.running_pids.lock().unwrap();
        let r_clone = r_mutg.deref().clone();
        Self {
            id: self.get_id(),
            active: Arc::new(Mutex::new(BoolBox::new(self.is_activated()))),
            ffmpeg_options: self.ffmpeg_options.clone(),
            currently_processing: Arc::new(Mutex::new(cp_clone)),
            finished_processing: Arc::new(Mutex::new(fp_clone)),
            failed_processing: Arc::new(Mutex::new(failp_clone)),
            running_pids: Arc::new(Mutex::new(r_clone)),
            max_gpu_tflops: self.get_speed(),
        }
    }
}
