use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut},
    process::Command,
    sync::{Arc, LazyLock, Mutex, mpsc},
    thread::{self},
};

use serde_json::Value;

use crate::file_handling::{
    file_handling::read_small_file,
    video_processing::{local_video_processor::LocalVideoProcessor, video_processor::*},
};

pub mod local_video_processor;
pub mod video_processor;

#[derive(Debug)]
///A struct that manages all of the video processors. All instances of this struct MUST have a 'static lifetime!
pub struct VideoProcessorManager {
    local_processors: LazyLock<Mutex<HashMap<u64, Arc<Mutex<LocalVideoProcessor>>>>>,
    fastest_processors: LazyLock<Mutex<Vec<u64>>>,

    currently_processing: LazyLock<Mutex<HashMap<u64, u64>>>,
    finished_processing: LazyLock<Mutex<HashMap<u64, u64>>>,
    failed_processing: LazyLock<Mutex<HashMap<u64, u64>>>,
    data_path: &'static str,
}

impl VideoProcessorManager {
    ///Creates a new VideoProcessorManager with the specified data directory (data_path).
    pub const fn new(data_path: &'static str) -> Self {
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
            data_path: data_path,
        }
    }

    ///Load the VideoProcessorManager's configuration data from "{data_path}/processors.json", which must be in the following JSON format:
    ///
    ///{"local_processors": [[local video processor JSON data (see the documentation for the LocalVideoProcessor struct)], ...]}
    pub fn load_data(&'static self) {
        //Load config file:
        let config_file_path = std::path::PathBuf::from(self.data_path).join("processors.json");
        println!(
            "Loading VideoProcessorManager data from \"{}\". ",
            config_file_path.to_str().unwrap()
        );
        let config_file_contents = read_small_file(config_file_path.to_str().unwrap()).unwrap();
        let json_data: Value = serde_json::from_slice(config_file_contents.as_slice()).unwrap();
        //Process JSON data:
        let local_processors_jlist = json_data["local_processors"].as_array().unwrap();
        //Local processors:
        for local_processors_jobj in local_processors_jlist {
            for (lp_id_str, lp_jobj) in local_processors_jobj.as_object().unwrap() {
                let lp_id: u64 = lp_id_str.parse::<u64>().unwrap();
                self.add_local_video_processor(Arc::new(Mutex::new(LocalVideoProcessor::new(
                    lp_id, lp_jobj,
                ))));
            }
        }
    }

    ///Adds the given processor (new_processor_arc) to the appropriate location in self.fastest_processors.
    //TODO: Need to either rename or make this function accept not just LocalVideoProcessors, but also other VideoProcessor types.
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

    ///Get a video processor by it's id (processor_id). Returns None if the processor with the specified id was not found.
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

    ///Adds a new LocalVideoProcessor (processor_arc).
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

    ///Adds a new VideoProcessor (processor_arc). Warning: This function may be unsafe to use because it forcefully casts the processor_arc parameter into Arc<Mutex<F>>, where F is the type of the VideoProcessor. MUST make sure that the VideoProcessor has the correct VideoProcessorType enum, otherwise program could panic or have memory corruption issues or other undefined behaviour!
    pub fn add_video_processor<F: VideoProcessor>(&'static self, processor_arc: Arc<Mutex<F>>) {
        //Determine type:
        let vp_type: VideoProcessorType;
        {
            let processor_mutg = processor_arc.lock().unwrap();
            let processor: &dyn VideoProcessor = processor_mutg.deref();
            vp_type = processor.get_type();
        }
        match vp_type {
            VideoProcessorType::LOCAL_VIDEO => {
                self.add_local_video_processor(cast_video_processor_arc::<LocalVideoProcessor, F>(
                    processor_arc,
                ));
            } // _ => {
              //     todo!(
              //         "The VideoProcessorManager::add_video_processor function has not been implemented for the VideoProcessor of the type \"{:?}\"",
              //         vp_type
              //     );
              // }
        };
    }

    ///Activates the VideoProcessor with the specified id (processor_id).
    pub fn activate_processor(&'static self, processor_id: u64) {
        let processor_arc = self.get_processor(&processor_id).unwrap();
        let mut processor_mutg = processor_arc.lock().unwrap();
        let processor = processor_mutg.deref_mut();
        processor.activate();
    }

    ///Deactivates the VideoProcessor with the specified id (processor_id).
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

    ///Processes a video, converting it into the MPEG_DASH format, with the specified video id (video_id) and file path of the video (video_file_path) (corresponding to the video id), and ouputs the result into the specified directory (result_directory).
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
                let rx = pr
                    .process(video_id, video_file_path.clone(), result_directory.clone())
                    .unwrap();
                let pr_id_copy: u64 = *pr_id;
                thread::spawn(move || {
                    let result = rx.recv().unwrap();
                    println!("Received data from ffmpeg process!");
                    if result {
                        let mut fp_arc_clone_mutg = self.finished_processing.lock().unwrap();
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

    ///Stops the processing of a video with the given video_id (video_id).
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

    ///Get a COPY of the list of videos that are currently processing.
    pub fn get_currently_processing_videos(&'static self) -> Vec<u64> {
        let cp_mutg = self.currently_processing.lock().unwrap();
        let cp = cp_mutg.deref();
        let mut result: Vec<u64> = Vec::new();
        for id in cp.keys() {
            result.push(*id);
        }
        return result;
    }

    ///Get a COPY of the list of videos that have SUCCESSFULLY finished processing.
    pub fn get_finished_processing_videos(&'static self) -> Vec<u64> {
        let fp_mutg = self.finished_processing.lock().unwrap();
        let fp = fp_mutg.deref();
        let mut result: Vec<u64> = Vec::new();
        for id in fp.keys() {
            result.push(*id);
        }
        return result;
    }

    ///Get a COPY of the list of videos that have failed to process.
    pub fn get_failed_processing_videos(&'static self) -> Vec<u64> {
        let failp_mutg = self.failed_processing.lock().unwrap();
        let failp = failp_mutg.deref();
        let mut result: Vec<u64> = Vec::new();
        for id in failp.keys() {
            result.push(*id);
        }
        return result;
    }

    ///Deactivates and removes the processor with the specified id (processor_id), if it exists.
    pub fn remove_processor(&'static self, processor_id: &u64) {
        if self.has_processor(processor_id) {
            self.deactivate_processor(processor_id);
            let mut localp_mutg = self.local_processors.lock().unwrap();
            let localp = localp_mutg.deref_mut();
            localp.remove(processor_id).unwrap();
        }
    }

    ///Returns whether or not if the VideoProcessor with the specified id (processor_id) exists.
    pub fn has_processor(&'static self, processor_id: &u64) -> bool {
        return match self.get_processor(processor_id) {
            Some(_) => true,
            None => false,
        };
    }

    ///Gets a COPY of the HashMap of all of the video processors.
    pub fn get_processors_clone(&'static self) -> HashMap<u64, LocalVideoProcessor> {
        let lps_mutg = self.local_processors.lock().unwrap();
        let lps = lps_mutg.deref();
        let mut result: HashMap<u64, LocalVideoProcessor> = HashMap::new();
        for (pr_id, vp_arc) in lps {
            let vp_mutg = vp_arc.lock().unwrap();
            result.insert(*pr_id, vp_mutg.deref().clone());
        }
        return result;
    }

    ///Returns whether or not if the VideoProcessor with the given processor id (processor_id) exists and is activated.
    pub fn is_activated(&'static self, processor_id: &u64) -> bool {
        return match self.get_processor(processor_id) {
            Some(proc_arc) => {
                let proc_mutg = proc_arc.lock().unwrap();
                let proc = proc_mutg.deref();
                proc.is_activated()
            }
            None => false,
        };
    }

    ///Performs a given function (action parameter) on a DEEP COPY of all of the video processors. It should be noted that changes made to the VideoProcessor given to the action parameter function will NOT affect the actual VideoProcessor in the VideoProcessorManager.
    pub fn for_all_procs<F: Fn(Box<dyn VideoProcessor>)>(&'static self, action: &F) {
        let lps_mutg = self.local_processors.lock().unwrap();
        let lps = lps_mutg.deref();
        for vp_arc in lps.values() {
            action({
                || {
                    let vp_mutg = vp_arc.lock().unwrap();
                    Box::new(vp_mutg.deref().clone())
                }
            }());
        }
    }

    ///Activates all of the video processors.
    //TODO: Need to make sure this function works with not only just LocalVideoProcessors, but also other types of VideoProcessors.
    pub fn activate_all_procs(&'static self) {
        let lps_mutg = self.local_processors.lock().unwrap();
        let lps = lps_mutg.deref();
        for vp_arc in lps.values() {
            let mut vp_mutg = vp_arc.lock().unwrap();
            let vp = vp_mutg.deref_mut();
            vp.activate();
        }
    }

    ///Deactivates all of the video processors.
    //TODO: Need to make sure this function works with not only just LocalVideoProcessors, but also other types of VideoProcessors.
    pub fn deactivate_all_procs(&'static self) {
        let lps_mutg = self.local_processors.lock().unwrap();
        let lps = lps_mutg.deref();
        for vp_id in lps.keys() {
            self.deactivate_processor(vp_id);
        }
    }
}
