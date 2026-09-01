use std::{
    collections::HashSet,
    mem,
    sync::{Arc, Mutex, mpsc},
};

use serde_json::Value;

#[derive(Debug, PartialEq)]
///Represents the different types of the VideoProcessors.
///
///WARNING: ABSOLUTELY NEED TO MAKE SURE THAT ALL OF THE VideoProcessor TYPES IDENTIFY THEMSELVES AS THE CORRECT TYPE VIA THE VideoProcessor::get_type_trait AND VideoProcessor::get_type FUNCTIONS, OTHERWISE THE PROGRAM COULD EITHER PANIC, HAVE MEMORY CORRUPTION ISSUES, OR OTHER UNDEFINED BEHAVIOUR!
pub enum VideoProcessorType {
    ///Represents the LocalVideoProcessor type.
    LOCAL_VIDEO,
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
}
