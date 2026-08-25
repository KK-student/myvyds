use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet, LinkedList},
    ops::{Deref, DerefMut},
    process::{Child, Command},
    sync::{Arc, LazyLock, Mutex, mpsc},
    thread::{self, JoinHandle},
};

use serde_json::Value;

///A trait for processing videos.
pub trait VideoProcessor {
    ///Create a new VideoProcessor (return type depends on struct implementing this trait) from the given JSON configuration data (config_json_data).
    fn new(id: u64, config_json_data: &Value) -> Self;

    ///Process a video with the specified video id (video_id), input video file path (video_file_path), and output directory (result_directory) into MPEG_DASH format using ffmpeg.
    fn process(
        &mut self,
        video_id: u64,
        video_file_path: String,
        result_directory: String,
    ) -> Result<mpsc::Receiver<bool>, std::io::Error>;

    ///Stop processing a video with the specified video id (video_id).
    fn stop_processing(&mut self, video_id: u64);

    ///Get the list of videos that are currently processing.
    fn get_currently_processing(&self) -> HashSet<u64>;

    ///Is this VideoProcessor currently processing a video with the specified video id (video_id)?
    fn is_processing(&self, video_id: &u64) -> bool;

    ///Has this VideoProcessor finished processing a video with the specified video id (video_id)?
    fn is_finished_processing(&self, video_id: &u64) -> bool;

    ///Has this VideoProcessor finished processing a video with the specified video id (video_id) successfully? If so, returns the ok value (which is null), otherwise it returns the error message(s) of whatever command(s) was/were used.
    fn is_processed_successfully(&self, video_id: &u64) -> Result<(), String>;

    ///Get this VideoProcessor's speed factor. Could be something like the maximum possible FLOPS of the VideoProcessor's GPU.
    fn get_speed(&self) -> u64;

    ///Get this VideoProcessor's id.
    fn get_id(&self) -> u64;

    ///Activate this VideoProcessor.
    fn activate(&mut self);

    ///Deactivate this VideoProcessor. Will not be able to process anything until activated again.
    fn deactivate(&mut self);

    ///Is this VideoProcessor currently activated?
    fn is_activated(&self) -> bool;

    fn get_num_active_jobs(&self) -> u64;
}

#[derive(Debug, Clone)]
///A VideoProcessor that processes the videos on the current local server (ie. the same server that contains the MPEG_DASH files and the information about the videos, comments and users).
pub struct LocalVideoProcessor {
    id: u64,
    active: bool,

    ffmpeg_options: Vec<String>, //Ffmpeg options, except for "-f dash -o ./output.mpd" and "-i [input_file]"

    currently_processing: Arc<Mutex<HashSet<u64>>>, //List of videos that are currently processing.
    finished_processing: Arc<Mutex<HashSet<u64>>>,  //List of videos that have finished processing.
    failed_processing: Arc<Mutex<HashMap<u64, String>>>, //List of videos that have failed to process with the error messages.

    running_pids: Arc<Mutex<HashMap<u64, u32>>>, //Map of the pids of the ffmpeg commands used to process the videos.

    max_gpu_tflops: u64,
}

impl VideoProcessor for LocalVideoProcessor {
    ///Create a new LocalVideoProcessor with the given id (id) and from the given JSON configuration data (config_json_data). The given JSON data must be in the following format:
    ///
    ///{"ffmpeg_options": [list of all of the ffmpeg options needed to convert the input video file into MPEG_DASH format as a list of strings (with no spaces in the items themselves), NOT INCLUDING "-i", the input video file, "-f", "dash", or "./output.mpd"], "max_gpu_tflops": [the maximum amount of tflops the server's GPU could do as a u64]}
    fn new(id: u64, config_json_data: &Value) -> Self {
        let mut ffmpeg_options: Vec<String> = Vec::new();
        for json_item in config_json_data["ffmpeg_options"].as_array().unwrap() {
            ffmpeg_options.push(json_item.as_str().unwrap().to_string());
        }

        return Self {
            id: id,
            active: true,
            ffmpeg_options: ffmpeg_options,
            currently_processing: Arc::new(Mutex::new(HashSet::new())),
            finished_processing: Arc::new(Mutex::new(HashSet::new())),
            failed_processing: Arc::new(Mutex::new(HashMap::new())),
            max_gpu_tflops: config_json_data["max_gpu_tflops"].as_u64().unwrap(),
            running_pids: Arc::new(Mutex::new(HashMap::new())),
        };
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

    fn process(
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
        let child = thread::spawn(move || -> () {
            {
                let mut cp_mutg = cp_arc.lock().unwrap();
                let cp: &mut HashSet<u64> = cp_mutg.deref_mut();
                cp.insert(video_id);
            }
            // let output = Command::new("ffmpeg")
            //     .args(ffmpeg_full_args)
            //     .current_dir(std::fs::canonicalize(result_directory).unwrap())
            //     .output()
            //     .expect("Ffmpeg failed. ");
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
                //Exec fail action:
                // let mut on_fail_mutg = on_fail_clone.lock().unwrap();
                // let on_fail_arc_clone = rx.recv().unwrap();
                // let mut on_fail_mutg = on_fail_arc_clone.lock().unwrap();
                // let on_fail_func = on_fail_mutg.deref_mut();
                // on_fail_func();
                tx.send(false);
            } else {
                //Exec success function:
                // let mut on_success_mutg = on_success_arc.lock().unwrap();
                // let on_success_func = on_success_mutg.deref_mut();
                // on_success_func();
                tx.send(true);
            }
        });
        // return Ok(child);
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
        self.active = true;
    }

    fn deactivate(&mut self) {
        self.active = false;
        //Kill all active ffmpeg processes:
        let mut running_pids_mutg = self.running_pids.lock().unwrap();
        let running_pids: &mut HashMap<u64, u32> = running_pids_mutg.deref_mut();
        for pid in running_pids.values() {
            kill_process_by_pid(*pid);
        }
        running_pids.clear();
    }

    fn is_activated(&self) -> bool {
        self.active
    }

    fn get_num_active_jobs(&self) -> u64 {
        let cp_mutg = self.currently_processing.lock().unwrap();
        let cp: &HashSet<u64> = cp_mutg.deref();
        return cp.len() as u64;
    }
}

#[cfg(target_family = "unix")]
///Kills a process on unix systems:
fn kill_process_by_pid(pid: u32) {
    //Unix systems:
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(pid as i32),
        nix::sys::signal::Signal::SIGKILL,
    );
}

// #[cfg(target_os = "linux")]
// ///Kills a process on linux systems:
// fn kill_process_by_pid(pid: u32) {
//     //Unix systems:
//     nix::sys::signal::kill(
//         nix::unistd::Pid::from_raw(pid as i32),
//         nix::sys::signal::Signal::SIGKILL,
//     );
// }

#[cfg(target_os = "windows")]
///Kills a process on windows systems:
fn kill_process_by_pid(pid: u32) {
    //Windows:
    Command::new("taskkill")
        .args(["/F", "/PID", pid.to_string().as_str()])
        .output()
        .expect("failed to execute process");
}

#[derive(Debug)]
pub struct VideoProcessorManager {
    // local_processors: Vec<LocalVideoProcessor>,
    local_processors: LazyLock<Mutex<HashMap<u64, Arc<Mutex<LocalVideoProcessor>>>>>,
    // fastest_processors: LinkedList<u64>,
    fastest_processors: LazyLock<Mutex<Vec<u64>>>,

    currently_processing: LazyLock<Mutex<HashMap<u64, u64>>>,
    finished_processing: LazyLock<Mutex<HashMap<u64, u64>>>,
    failed_processing: LazyLock<Mutex<HashMap<u64, u64>>>,
    // join_handles: Arc<Mutex<HashMap<u64, JoinHandle<()>>>>,
}

impl VideoProcessorManager {
    pub const fn new() -> Self {
        Self {
            local_processors: LazyLock::new(
                || -> Mutex<HashMap<u64, Arc<Mutex<LocalVideoProcessor>>>> {
                    Mutex::new(HashMap::with_capacity(10))
                },
            ),
            fastest_processors: LazyLock::new(|| -> Mutex<Vec<u64>> {
                Mutex::new(Vec::with_capacity(10))
            }),
            currently_processing: LazyLock::new(|| -> Mutex<HashMap<u64, u64>> {
                Mutex::new(HashMap::new())
            }),
            finished_processing: LazyLock::new(|| -> Mutex<HashMap<u64, u64>> {
                Mutex::new(HashMap::new())
            }),
            failed_processing: LazyLock::new(|| -> Mutex<HashMap<u64, u64>> {
                Mutex::new(HashMap::new())
            }),
        }
    }
    // pub fn new_arc(json_data: &serde_json::Value) -> Arc<Mutex<Self>> {
    //     //Get basic data:
    //     let local_processors_jobj = json_data["local_processors"].as_object().unwrap();
    //     let num_local_processors = local_processors_jobj.len();
    //     let capacity = if num_local_processors < 10 {
    //         10
    //     } else {
    //         num_local_processors
    //     };
    //     // Create VideoProcessorManager:
    //     let manager_arc: Arc<Mutex<Self>> = Arc::new(Mutex::new(Self {
    //         local_processors: Arc::new(Mutex::new(HashMap::with_capacity(capacity))),
    //         // local_processors: LazyLock::new(
    //         //     || -> Mutex<HashMap<u64, Arc<Mutex<LocalVideoProcessor>>>> {
    //         //         Mutex::new(HashMap::with_capacity(10))
    //         //     },
    //         // ),
    //         // fastest_processors: LinkedList::new(),
    //         fastest_processors: Arc::new(Mutex::new(Vec::with_capacity(capacity))),
    //         currently_processing: Arc::new(Mutex::new(HashMap::new())),
    //         finished_processing: Arc::new(Mutex::new(HashMap::new())),
    //         failed_processing: Arc::new(Mutex::new(HashMap::new())),
    //         // join_handles: Arc::new(Mutex::new(HashMap::new())),
    //     }));
    //     //Populate the video manager:
    //     {
    //         let mut manager_mutg = manager_arc.lock().unwrap();
    //         let video_pmanager: &mut Self = manager_mutg.deref_mut();
    //         //Local processors:
    //         for (lp_id_str, lp_jobj) in local_processors_jobj {
    //             let lp_id: u64 = lp_id_str.parse::<u64>().unwrap();
    //             video_pmanager.add_local_video_processor(Arc::new(Mutex::new(
    //                 LocalVideoProcessor::new(lp_id, lp_jobj),
    //             )));
    //         }
    //     }
    //     return Arc::clone(&manager_arc);
    // }

    pub fn load_data(&'static self, json_data: &serde_json::Value) {
        //Get basic data:
        let local_processors_jobj = json_data["local_processors"].as_object().unwrap();
        //Local processors:
        for (lp_id_str, lp_jobj) in local_processors_jobj {
            let lp_id: u64 = lp_id_str.parse::<u64>().unwrap();
            self.add_local_video_processor(Arc::new(Mutex::new(LocalVideoProcessor::new(
                lp_id, lp_jobj,
            ))));
        }
    }

    fn new_processor_sort(&'static self, new_processor_arc: Arc<Mutex<LocalVideoProcessor>>) {
        let new_processor_mutg = new_processor_arc.lock().unwrap();
        let new_processor: &LocalVideoProcessor = new_processor_mutg.deref();
        let new_id = new_processor.get_id();
        let new_speed = new_processor.get_speed();
        let mut insert_index = 0;
        {
            let mut i = 0;
            let fastp_mutg = self.fastest_processors.lock().unwrap();
            let fastp = fastp_mutg.deref();
            for current_id in fastp.iter() {
                let current_processor_arc = self.get_processor(current_id).unwrap();
                let current_processor_mutg = current_processor_arc.lock().unwrap();
                let current_processor = current_processor_mutg.deref();
                if new_speed <= current_processor.get_speed() {
                    insert_index = i;
                    break;
                }
                i += 1;
            }
        }
        {
            let mut fastp_mutg = self.fastest_processors.lock().unwrap();
            let fastp = fastp_mutg.deref_mut();
            fastp.insert(insert_index, new_id);
        }
    }

    pub fn get_processor(
        &'static self,
        processor_id: &u64,
    ) -> Option<Arc<Mutex<impl VideoProcessor>>> {
        let localp_mutg = self.local_processors.lock().unwrap();
        let localp = localp_mutg.deref();
        return if localp.contains_key(processor_id) {
            Some(Arc::clone(localp.get(processor_id).unwrap()))
        } else {
            None
        };
    }
    pub fn add_local_video_processor(
        &'static self,
        processor_arc: Arc<Mutex<LocalVideoProcessor>>,
    ) {
        {
            let processor_mutg = processor_arc.lock().unwrap();
            let processor: &LocalVideoProcessor = processor_mutg.deref();
            let mut localp_mutg = self.local_processors.lock().unwrap();
            let localp = localp_mutg.deref_mut();
            localp.insert(processor.get_id(), Arc::clone(&processor_arc));
        }
        {
            self.new_processor_sort(processor_arc);
        }
    }

    pub fn activate_processor(&'static self, processor_id: u64) {
        let processor_arc = self.get_processor(&processor_id).unwrap();
        let mut processor_mutg = processor_arc.lock().unwrap();
        let processor = processor_mutg.deref_mut();
        processor.activate();
    }

    pub fn deactivate_processor(
        &'static self,
        processor_id: &u64,
    ) -> Option<Arc<Mutex<impl VideoProcessor>>> {
        // let processor_arc = self.get_processor(&processor_id).unwrap();
        // let mut processor_mutg = processor_arc.lock().unwrap();
        // let processor = processor_mutg.deref_mut();
        // processor.activate();
        return match self.get_processor(processor_id) {
            Some(pr_arc) => {
                let mut pr_mutg = pr_arc.lock().unwrap();
                let pr = pr_mutg.deref_mut();
                let pr_cp = pr.get_currently_processing();
                pr.deactivate();

                let mut cp_mutg = self.currently_processing.lock().unwrap();
                let cp = cp_mutg.deref_mut();
                let mut failp_mutg = self.failed_processing.lock().unwrap();
                let failp = failp_mutg.deref_mut();
                for video_id in pr_cp {
                    cp.remove(&video_id);
                    failp.insert(video_id, *processor_id);
                }
                Some(Arc::clone(&pr_arc))
            }
            None => None,
        };
    }

    pub fn process(
        &'static self,
        video_id: u64,
        video_file_path: &String,
        result_directory: &String,
    ) {
        let fastp_mutg = self.fastest_processors.lock().unwrap();
        let fastp = fastp_mutg.deref();
        for pr_id in fastp.iter() {
            let max_allowed_jobs = 2;
            let pr_arc = self.get_processor(&pr_id).unwrap();
            let mut pr_mutg = pr_arc.lock().unwrap();
            let pr = pr_mutg.deref_mut();
            if pr.is_activated() && pr.get_num_active_jobs() < max_allowed_jobs {
                //Process the video.
                {
                    //Add to list of currently processing videos:
                    let mut cp_mutg = self.currently_processing.lock().unwrap();
                    let cp: &mut HashMap<u64, u64> = cp_mutg.deref_mut();
                    cp.insert(video_id, pr.get_id());
                }
                // {
                //     //Process video:
                //     let mut jh_mutg = self.join_handles.lock().unwrap();
                //     let jh: &mut HashMap<u64, JoinHandle<()>> = jh_mutg.deref_mut();
                //     jh.insert(
                //         video_id,
                //         pr.process(video_id, video_file_path.clone(), result_directory.clone())
                //             .unwrap(),
                //     );
                // }
                let rx = pr
                    .process(video_id, video_file_path.clone(), result_directory.clone())
                    .unwrap();
                // let pr_arc_clone = Arc::clone(&pr_arc);
                // let fp_arc_clone = LazyLock::clone(&self.finished_processing);
                // let failp_arc_clone = LazyLock::clone(&self.failed_processing);
                let pr_id_copy: u64 = *pr_id;
                thread::spawn(move || {
                    let result = rx.recv().unwrap();
                    println!("Received data from ffmpeg process!");
                    if result {
                        let mut fp_arc_clone_mutg = self.finished_processing.lock().unwrap();
                        // fp_arc_clone_mutg.insert(video_id, *pr_id);
                        let fp_clone = fp_arc_clone_mutg.deref_mut();
                        fp_clone.insert(video_id, pr_id_copy);
                    } else {
                        let mut fail_arc_clone_mutg = self.failed_processing.lock().unwrap();
                        let failp_clone = fail_arc_clone_mutg.deref_mut();
                        failp_clone.insert(video_id, pr_id_copy);
                    }
                });
            }
        }
    }

    pub fn stop_processing(&'static self, video_id: u64) {
        let mut cp_mutg = self.currently_processing.lock().unwrap();
        let cp = cp_mutg.deref_mut();
        let pr_id = match cp.remove(&video_id) {
            Some(id) => id,
            None => return,
        };
        let pr_arc = self.get_processor(&pr_id).unwrap();
        let mut pr_mutg = pr_arc.lock().unwrap();
        let pr = pr_mutg.deref_mut();
        pr.stop_processing(video_id);
    }

    pub fn get_currently_processing_videos(&'static self) -> Vec<u64> {
        let cp_mutg = self.currently_processing.lock().unwrap();
        let cp = cp_mutg.deref();
        let mut result: Vec<u64> = Vec::new();
        for id in cp.keys() {
            result.push(*id);
        }
        return result;
    }

    pub fn get_finished_processing_videos(&'static self) -> Vec<u64> {
        let fp_mutg = self.finished_processing.lock().unwrap();
        let fp = fp_mutg.deref();
        let mut result: Vec<u64> = Vec::new();
        for id in fp.keys() {
            result.push(*id);
        }
        return result;
    }

    pub fn get_failed_processing_videos(&'static self) -> Vec<u64> {
        let failp_mutg = self.failed_processing.lock().unwrap();
        let failp = failp_mutg.deref();
        let mut result: Vec<u64> = Vec::new();
        for id in failp.keys() {
            result.push(*id);
        }
        return result;
    }

    pub fn remove_processor(&'static self, processor_id: &u64) {
        self.deactivate_processor(processor_id);
        let mut localp_mutg = self.local_processors.lock().unwrap();
        let localp = localp_mutg.deref_mut();
        localp.remove(processor_id).unwrap();
    }
}

static TEST_MANAGER: VideoProcessorManager = VideoProcessorManager::new();
