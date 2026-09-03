use std::{
    collections::HashSet,
    mem,
    ops::{Deref, DerefMut},
    process::{Command, Output},
    sync::{Arc, Mutex, mpsc},
};

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, PartialEq)]
///Represents the different types of the VideoProcessors.
///
///WARNING: ABSOLUTELY NEED TO MAKE SURE THAT ALL OF THE VideoProcessor TYPES IDENTIFY THEMSELVES AS THE CORRECT TYPE VIA THE VideoProcessor::get_type_trait AND VideoProcessor::get_type FUNCTIONS, OTHERWISE THE PROGRAM COULD EITHER PANIC, HAVE MEMORY CORRUPTION ISSUES, OR OTHER UNDEFINED BEHAVIOUR!
pub enum VideoProcessorType {
    ///Represents the LocalVideoProcessor type.
    LOCAL_VIDEO,

    ///Represents the LocalMusicProcessor type.
    LOCAL_MUSIC,
}

///Forcefully casts a Arc Mutex VideoProcessor of type Arc<Mutex<F>> to Arc<Mutex<R>> using the std::mem::transmute function, where F is the inferred type of the VideoProcessor and R is the type of VideoProcessor to cast to.
///
///WARNING: ABSOLUTELY NEED TO MAKE SURE THAT ALL OF THE VideoProcessor TYPES IDENTIFY THEMSELVES AS THE CORRECT TYPE VIA THE VideoProcessor::get_type_trait AND VideoProcessor::get_type FUNCTIONS, OTHERWISE THE PROGRAM COULD EITHER PANIC, HAVE MEMORY CORRUPTION ISSUES, OR OTHER UNDEFINED BEHAVIOUR!
pub fn cast_video_processor_arc<R: VideoProcessor, F: VideoProcessor>(
    processor_arc: Arc<Mutex<F>>,
) -> Arc<Mutex<R>> {
    //Check if processor_arc is being casted to the correct type:
    if R::get_type_trait() != F::get_type_trait() {
        panic!(
            "Cannot cast from {:?} to {:?} VideoProcessor types!",
            F::get_type_trait(),
            R::get_type_trait()
        );
    }

    //Cast:
    let vp_arc_casted: Arc<Mutex<R>>;
    unsafe {
        vp_arc_casted = mem::transmute::<Arc<Mutex<F>>, Arc<Mutex<R>>>(processor_arc);
    }

    return Arc::clone(&vp_arc_casted);
}

///A trait for processing videos.
pub trait VideoProcessor {
    ///Create a new VideoProcessor (return type depends on struct implementing this trait) from the given JSON configuration data (config_json_data).
    fn new(id: u64, config_json_data: &Value) -> Self
    where
        Self: Sized;

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

    ///Get number of currently active jobs:
    fn get_num_active_jobs(&self) -> u64;

    ///Get the type of the VideoProcessor:
    fn get_type_trait() -> VideoProcessorType
    where
        Self: Sized;

    ///Same as get_type_trait, but for trait objects:
    fn get_type(&self) -> VideoProcessorType;

    ///Gets whether or not if this VideoProcessor supports video files.
    fn is_video_processor(&self) -> bool;

    ///Gets whether or not if this VideoProcessor supports audio files.
    fn is_audio_processor(&self) -> bool {
        !self.is_video_processor()
    }

    ///Check whether or not if this VideoProcessor supports a file with the specified file path.
    fn is_supported_file(&self, description: &VeryBasicVideoFileDescription) -> bool {
        // let description: VeryBasicVideoFileDescription = match get_video_description(file_path_str)
        // {
        //     Ok(desc) => desc,
        //     Err(e) => return Err(e),
        // };
        return if self.is_video_processor() {
            if !description.has_video_stream {
                false
            } else if !description.has_audio {
                //TODO: Add support for video without audio.
                false
            } else {
                true
            }
        } else if self.is_audio_processor() {
            if description.has_video_stream {
                false
            } else if !description.has_audio {
                false
            } else {
                true
            }
        } else {
            todo!(
                "It appears that either the VideoProcessor::is_video_processor or the VideoProcessor::is_audio_processor methods have been changed while the VideoProcessor::is_supported_file function has not been changed. Please modify the VideoProcessor::is_supported_file function to cover all cases. "
            );
        };
    }
}

#[derive(Debug)]
///A struct that holds a single boolean value.
pub struct BoolBox {
    value: bool,
}

impl BoolBox {
    ///Creates a new BoolBox with the given value (value).
    pub fn new(value: bool) -> Self {
        Self { value: value }
    }

    ///Gets the boolean value of the BoolBox.
    pub fn get(&self) -> bool {
        self.value
    }

    ///Sets the boolean value of the BoolBox to the value of the "value" parameter.
    pub fn set(&mut self, value: bool) {
        self.value = value;
    }
}

pub struct VeryBasicVideoFileDescription {
    has_video_stream: bool,
    has_audio: bool,
}

pub fn get_video_description(
    file_path_str: &str,
) -> Result<VeryBasicVideoFileDescription, std::io::Error> {
    let file_path_buf = std::fs::canonicalize(&file_path_str).unwrap();
    let file_path = file_path_buf.as_path();
    let fp_canon_str: &str = match file_path.to_str() {
        Some(res) => res,
        None => {
            let err_str = format!(
                "Error: The file path \"{}\" is not valid unicode. ",
                file_path.display()
            );
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, err_str));
        }
    };
    if !file_path.exists() {
        let err_str = format!("Error: The file \"{}\" does not exist. ", fp_canon_str);
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, err_str));
    }

    let ffprobe_output = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_streams",
            fp_canon_str,
        ])
        .output()
        .unwrap();
    if ffprobe_output.status.success() {
        let has_video_stream_box: Arc<Mutex<BoolBox>> = Arc::new(Mutex::new(BoolBox::new(false)));
        let has_audio_box: Arc<Mutex<BoolBox>> = Arc::new(Mutex::new(BoolBox::new(false)));
        match std::panic::catch_unwind(|| -> () {
            // let test = Arc::clone(&has_video_stream_box);
            let json_root: Value =
                serde_json::from_slice(ffprobe_output.stdout.as_slice()).unwrap();
            let streams_json = json_root["streams"].as_array().unwrap();

            let has_video_stream_arc = Arc::clone(&has_video_stream_box);
            let mut has_video_stream_mutg = has_video_stream_arc.lock().unwrap();
            let has_video_stream: &mut BoolBox = has_video_stream_mutg.deref_mut();

            let has_audio_arc = Arc::clone(&has_audio_box);
            let mut has_audio_mutg = has_audio_arc.lock().unwrap();
            let has_audio: &mut BoolBox = has_audio_mutg.deref_mut();

            for stream_json in streams_json {
                let stream_type = stream_json["codec_name"].as_str().unwrap().to_lowercase();
                match stream_type.as_str() {
                    "video" => {
                        if has_video_stream.get() {
                            panic!("Cannot have more than 1 video stream!");
                        } else {
                            has_video_stream.set(true);
                        }
                    }
                    "audio" => {
                        if !has_audio.get() {
                            has_audio.set(true);
                        }
                    }
                    _ => {}
                }
            }
        }) {
            Ok(_) => {}
            Err(e) => {
                let err_str = format!(
                    "Error: Failed to parse output from ffprobe. Error message: {:?}.",
                    e
                );
                return Err(std::io::Error::new(std::io::ErrorKind::NotFound, err_str));
            }
        };

        //Return result:
        let has_video_stream_mutg = has_video_stream_box.lock().unwrap();
        let has_audio_mutg = has_audio_box.lock().unwrap();
        let description: VeryBasicVideoFileDescription = VeryBasicVideoFileDescription {
            has_video_stream: has_video_stream_mutg.deref().get(),
            has_audio: has_audio_mutg.deref().get(),
        };
        return Ok(description);
    } else {
        let err_str = format!(
            "Error: The ffprobe process failed!\nStdout: \n{}\nStderr: \n{}",
            String::from_utf8(ffprobe_output.stdout).unwrap(),
            String::from_utf8(ffprobe_output.stderr).unwrap()
        );
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, err_str));
    }
}

#[cfg(target_family = "unix")]
///Kills a process on unix systems:
pub fn kill_process_by_pid(pid: u32) {
    //Unix systems:
    let _ = nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(pid as i32),
        nix::sys::signal::Signal::SIGKILL,
    );
}

#[cfg(target_os = "windows")]
///Kills a process on windows systems:
pub fn kill_process_by_pid(pid: u32) {
    //Windows:
    Command::new("taskkill")
        .args(["/F", "/PID", pid.to_string().as_str()])
        .output()
        .expect("failed to execute process");
}
