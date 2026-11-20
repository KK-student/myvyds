use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt::Debug,
    io::Write,
    ops::{Deref, DerefMut},
    panic,
    path::PathBuf,
    sync::{Arc, LazyLock, Mutex},
    thread::{self},
};

use image::ImageError;
use serde_json::Value;

use crate::{
    file_handling::{
        read_small_file,
        saveable::make_json_list_from_saveable_list,
        video_processing::{
            local_music_processor::LocalMusicProcessor,
            local_thumbnail_video_processor::LocalThumbnailImageProcessor,
            local_video_processor::LocalVideoProcessor,
            thumbnail_processor::ThumbnailImageProcessor, video_processor::*,
        },
    },
    misc::collections::MutableBox,
    tests::TestingDefault,
};

pub mod local_music_processor;
pub mod local_thumbnail_video_processor;
pub mod local_video_processor;
pub mod thumbnail_processor;
pub mod video_processor;

pub fn _on_finish_func(_vid: u64) -> () {}
// static DEFAULT_ON_FINISH_SUCCESS_FUNC: fn(u64) -> () = _on_finish_func as fn(u64);
static DEFAULT_ON_FINISH_FAIL_FUNC: fn(u64) -> () = _on_finish_func as fn(u64);

// #[derive(Debug)]
///A struct that manages all of the video processors. All instances of this struct MUST have a 'static lifetime!
pub struct VideoProcessorManager<
    FFin: Fn(u64) + std::marker::Sync + std::marker::Send + Debug + Clone + 'static,
    THIP: ThumbnailImageProcessor
        + std::marker::Send
        + std::marker::Sync
        + std::marker::Sized
        + Clone
        + Any,
> {
    local_video_processors: LazyLock<Mutex<HashMap<u64, Arc<Mutex<LocalVideoProcessor>>>>>,
    local_music_processors: LazyLock<Mutex<HashMap<u64, Arc<Mutex<LocalMusicProcessor>>>>>,
    fastest_local_video_processors: LazyLock<Mutex<Vec<u64>>>,
    fastest_local_music_processors: LazyLock<Mutex<Vec<u64>>>,

    // thumbnail_image_processor:
    //     ArcBox<Box<dyn ThumbnailImageProcessor + std::marker::Send + std::marker::Sync>>,

    // thumbnail_image_processor: LazyLock<Mutex<TestStruct<DynCloneBox<dyn ThumbnailImageProcessor>>>>,
    thumbnail_image_processor: LazyLock<Mutex<MutableBox<THIP>>>,

    currently_processing: LazyLock<Mutex<HashMap<u64, u64>>>,
    finished_processing: LazyLock<Mutex<HashMap<u64, u64>>>,
    failed_processing: LazyLock<Mutex<HashMap<u64, u64>>>, //TODO: Add support for storing error messages.
    // data_path: &'static str,
    data_path: LazyLock<Mutex<MutableBox<String>>>,

    on_finished_success_function: LazyLock<Mutex<MutableBox<FFin>>>,
    on_finished_fail_function: LazyLock<Mutex<MutableBox<FFin>>>,
}

impl<
    THIP: ThumbnailImageProcessor + std::marker::Send + std::marker::Sync + std::marker::Sized + Clone,
> Default for VideoProcessorManager<fn(u64) -> (), THIP>
{
    fn default() -> Self {
        Self::const_default()
    }
}

impl<
    THIP: ThumbnailImageProcessor + std::marker::Send + std::marker::Sync + std::marker::Sized + Clone,
> TestingDefault for VideoProcessorManager<fn(u64) -> (), THIP>
{
    fn testing_default() -> Self {
        // Self::new(
        //     LazyLock::new(|| -> Mutex<MutableBox<String>> { Mutex::new(MutableBox::new(None)) }),
        //     LazyLock::new(|| -> Mutex<MutableBox<fn(u64) -> ()>> {
        //         Mutex::new(MutableBox::new(Some(DEFAULT_ON_FINISH_SUCCESS_FUNC)))
        //     }),
        //     DEFAULT_ON_FINISH_FAIL_FUNC,
        // )
        Self::default()
    }
}

impl<
    FFin: Fn(u64) + std::marker::Sync + std::marker::Send + Debug + Clone + 'static,
    THIP: ThumbnailImageProcessor + std::marker::Send + std::marker::Sync + std::marker::Sized + Clone,
> Debug for VideoProcessorManager<FFin, THIP>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VideoProcessorManager")
            .field("local_video_processors", &self.local_video_processors)
            .field("local_music_processors", &self.local_music_processors)
            .field(
                "fastest_local_video_processors",
                &self.fastest_local_video_processors,
            )
            .field(
                "fastest_local_music_processors",
                &self.fastest_local_music_processors,
            )
            // .field("thumbnail_image_processor", &self.thumbnail_image_processor)
            .field("currently_processing", &self.currently_processing)
            .field("finished_processing", &self.finished_processing)
            .field("failed_processing", &self.failed_processing)
            .field("data_path", &self.get_data_dir())
            .field(
                "on_finished_success_function",
                &self.on_finished_success_function,
            )
            .field("on_finished_fail_function", &self.on_finished_fail_function)
            .finish()
    }
}

impl<
    FFin: Fn(u64) + std::marker::Sync + std::marker::Send + Clone + Debug + 'static,
    THIP: ThumbnailImageProcessor + std::marker::Send + std::marker::Sync + std::marker::Sized + Clone,
> VideoProcessorManager<FFin, THIP>
{
    pub const fn const_default() -> Self {
        Self {
            local_video_processors: LazyLock::new(
                || -> Mutex<HashMap<u64, Arc<Mutex<LocalVideoProcessor>>>> {
                    Mutex::new(HashMap::with_capacity(10))
                },
            ),
            local_music_processors: LazyLock::new(
                || -> Mutex<HashMap<u64, Arc<Mutex<LocalMusicProcessor>>>> {
                    Mutex::new(HashMap::with_capacity(10))
                },
            ),
            fastest_local_video_processors: LazyLock::new(|| -> Mutex<Vec<u64>> {
                Mutex::new(Vec::with_capacity(10))
            }),
            fastest_local_music_processors: LazyLock::new(|| -> Mutex<Vec<u64>> {
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
            // data_path: OnceLock::new(),
            data_path: LazyLock::new(|| -> Mutex<MutableBox<String>> {
                Mutex::new(MutableBox::new(None))
            }),
            on_finished_success_function: LazyLock::new(|| -> Mutex<MutableBox<FFin>> {
                Mutex::new(MutableBox::new(None))
            }),
            on_finished_fail_function: LazyLock::new(|| -> Mutex<MutableBox<FFin>> {
                Mutex::new(MutableBox::new(None))
            }),
            // thumbnail_image_processor: ArcBox::default_const(),
            thumbnail_image_processor: LazyLock::new(|| -> Mutex<MutableBox<THIP>> {
                Mutex::new(MutableBox::new(None))
            }),
        }
    }

    ///Initialize the VideoProcessorManager.
    ///
    /// Parameters:
    ///
    /// - data_dir: The directory from which to read the config file(s) from.
    ///
    /// - on_finish_success_function: A function to execute when a video has SUCCESSFULLY finished processing.
    ///
    /// - on_finish_fail_function: A function to execute when a video has FAILED to process.
    pub fn init(
        &'static self,
        data_dir: &std::path::Path,
        on_finish_success_function: FFin,
        on_finish_fail_function: FFin,
    ) {
        self.init_testing(on_finish_success_function, on_finish_fail_function, None);

        let mut sdata_dir_mutg = self.data_path.lock().unwrap();
        let data_dir_box = sdata_dir_mutg.deref_mut();
        data_dir_box.set(Some(data_dir.to_str().unwrap().to_string()));
    }

    ///Initialize the VideoProcessorManager for use in the testing mode.
    ///
    /// Parameters:
    ///
    /// - on_finish_success_function: A function to execute when a video has SUCCESSFULLY finished processing.
    ///
    /// - on_finish_fail_function: A function to execute when a video has FAILED to process.
    ///
    /// - video_processors: An optional list of video processors to add.
    pub fn init_testing(
        &'static self,
        on_finish_success_function: FFin,
        on_finish_fail_function: FFin,
        video_processors: Option<Vec<Box<dyn Any>>>,
    ) {
        let mut sfs_func_mutg = self.on_finished_success_function.lock().unwrap();
        let fs_func_box = sfs_func_mutg.deref_mut();
        fs_func_box.set(Some(on_finish_success_function));

        let mut sff_func_mutg = self.on_finished_fail_function.lock().unwrap();
        let ff_func_box = sff_func_mutg.deref_mut();
        ff_func_box.set(Some(on_finish_fail_function));

        if video_processors.is_some() {
            for vp_box in video_processors.unwrap().iter() {
                let vp = vp_box.deref();
                let tid = vp.type_id();
                if tid == TypeId::of::<LocalVideoProcessor>() {
                    let vp_casted: LocalVideoProcessor =
                        vp.downcast_ref::<LocalVideoProcessor>().unwrap().clone();
                    let vp_arc: Arc<Mutex<LocalVideoProcessor>> = Arc::new(Mutex::new(vp_casted));
                    self.add_local_video_processor(vp_arc);
                } else if tid == TypeId::of::<LocalMusicProcessor>() {
                    let vp_casted: LocalMusicProcessor =
                        vp.downcast_ref::<LocalMusicProcessor>().unwrap().clone();
                    let vp_arc: Arc<Mutex<LocalMusicProcessor>> = Arc::new(Mutex::new(vp_casted));
                    self.add_local_music_processor(vp_arc);
                } else {
                    panic!(
                        "Found a video processor with an unsupported type. Type id: {:?}",
                        tid
                    );
                }
            }
        }
    }

    ///Load the VideoProcessorManager's configuration data from "{data_path}/processors.json", which must be in the following JSON format:
    ///
    ///{"local_processors": [[local video processor JSON data (see the documentation for the LocalVideoProcessor struct)], ...]}
    pub fn load_data(&'static self) {
        //Load config file:
        let config_file_path = self.get_data_dir().join("./processors.json");
        println!(
            "Loading VideoProcessorManager data from \"{}\". ",
            config_file_path.to_str().unwrap()
        );
        let config_file_contents = read_small_file(config_file_path.to_str().unwrap()).unwrap();
        let json_data: Value = serde_json::from_slice(config_file_contents.as_slice()).unwrap();
        //Process JSON data:
        //Local video processors:
        let local_video_processors_jlist = json_data["local_video_processors"].as_array().unwrap();
        for local_video_processors_jobj in local_video_processors_jlist {
            // for (lvp_id_str, lvp_jobj) in local_video_processors_jobj.as_object().unwrap() {
            self.add_local_video_processor(Arc::new(Mutex::new(LocalVideoProcessor::new(
                local_video_processors_jobj,
            ))));
            // }
        }
        //Local video processors:
        let local_music_processors_jlist = json_data["local_music_processors"].as_array().unwrap();
        for local_music_processors_jobj in local_music_processors_jlist {
            // for (lmp_id_str, lmp_jobj) in local_music_processors_jobj.as_object().unwrap() {
            self.add_local_music_processor(Arc::new(Mutex::new(LocalMusicProcessor::new(
                local_music_processors_jobj,
            ))));
            // }
        }
        //Thumbnail image processor:
        {
            println!("Loading thumbnail processor data. ");
            let mut thip_cont_mutg = self.thumbnail_image_processor.lock().unwrap();
            let thip_cont = thip_cont_mutg.deref_mut();
            thip_cont.set({
                let thumbnail_processor_jobj =
                    json_data["thumbnail_processor"].as_object().unwrap();
                match thumbnail_processor_jobj
                    .get("type")
                    .unwrap()
                    .as_str()
                    .unwrap()
                {
                    "local" => {
                        if TypeId::of::<THIP>() != TypeId::of::<LocalThumbnailImageProcessor>() {
                            panic!("Invalid thumbnail processor type!");
                        }
                        Some(THIP::from_json(
                            thumbnail_processor_jobj.get("options").unwrap(),
                        ))
                    }
                    &_ => unimplemented!("Invalid thumbnail processor type!"),
                }
            });
        }
    }

    ///Change the directory to which the VideoProcessorManager loads from and saves to.
    pub fn set_data_dir(&'static self, data_dir: &std::path::Path) {
        // let path_canon = std::fs::canonicalize(data_dir).unwrap();
        let mut dp_mutg = self.data_path.lock().unwrap();
        let dp = dp_mutg.deref_mut();
        // dp.set(Some(path_canon.to_str().unwrap().to_string()));
        dp.set(Some(data_dir.to_str().unwrap().to_string()));
    }

    ///Get the directory to which the VideoProcessorManager loads and and saves to.
    pub fn get_data_dir(&self) -> std::path::PathBuf {
        let dp_mutg = self.data_path.lock().unwrap();
        let dp = dp_mutg.deref();
        PathBuf::from(dp.get().unwrap())
    }

    ///Save the VideoProcessorManager's data to disk (or wherever the data file is stored).
    pub fn save_data(&'static self) {
        //Create JSON data string:
        //Lock data:
        let local_video_procs_mutg = self.local_video_processors.lock().unwrap();
        let local_music_procs_mutg = self.local_music_processors.lock().unwrap();
        //Local video processors:
        let local_video_procs_map = local_video_procs_mutg.deref();
        let mut local_video_procs_list: Vec<Arc<Mutex<LocalVideoProcessor>>> = Vec::new();
        for item in local_video_procs_map.values() {
            local_video_procs_list.push(Arc::clone(item));
        }
        let local_video_procs_string: String =
            make_json_list_from_saveable_list(&local_video_procs_list);
        //Local music processors:
        let local_music_procs_map = local_music_procs_mutg.deref();
        let mut local_music_procs_list: Vec<Arc<Mutex<LocalMusicProcessor>>> = Vec::new();
        for item in local_music_procs_map.values() {
            local_music_procs_list.push(Arc::clone(item));
        }
        let local_music_procs_string: String =
            make_json_list_from_saveable_list(&local_music_procs_list);
        //Thumbnail processor:
        let thumbnail_processor_string: String = {
            let thumbnail_processor_mutg = self.thumbnail_image_processor.lock().unwrap();
            let thumbnail_processor_box = thumbnail_processor_mutg.deref();
            let thumbnail_processor = thumbnail_processor_box.get().unwrap();
            thumbnail_processor.to_json(false)
        };

        //Final result:
        let result: String = format!(
            "{{\"local_video_processors\": {local_video_procs_string}, \"local_music_processors\": {local_music_procs_string}, \"thumbnail_processor\": {thumbnail_processor_string}}}"
        );

        //Write processors.json file:
        let mut result_path_buf = self.get_data_dir();
        result_path_buf.push("./processors.json");
        let result_path_canon = std::fs::canonicalize(result_path_buf).unwrap();
        let result_path_string: &String = &result_path_canon.to_string_lossy().to_string();
        let mut result_json_file = std::fs::File::create(result_path_canon).expect(
            format!(
                "Error: Could not create or open the file \"{}\" for writing. ",
                result_path_string
            )
            .as_str(),
        );
        result_json_file.write_all(result.as_bytes()).expect(
            format!(
                "Error: Could not write processor data to the file \"{}\". ",
                result_path_string
            )
            .as_str(),
        );
    }

    ///Adds the given LocalVideoProcessor (new_processor_arc) to the appropriate location in self.fastest_local_video_processors.
    fn new_local_video_processor_sort(
        &'static self,
        new_processor_arc: Arc<Mutex<LocalVideoProcessor>>,
    ) {
        let new_processor_mutg = new_processor_arc.lock().unwrap();
        let new_processor: &LocalVideoProcessor = new_processor_mutg.deref();
        let new_id = new_processor.get_id();
        let new_speed = new_processor.get_speed();
        let mut insert_index = 0;
        {
            let mut i = 0;
            let fastp_mutg = self.fastest_local_video_processors.lock().unwrap();
            let fastp = fastp_mutg.deref();
            for current_id in fastp.iter() {
                let current_processor_arc: Arc<Mutex<LocalVideoProcessor>> =
                    self.get_local_video_processor(current_id).unwrap();
                let current_processor_mutg: std::sync::MutexGuard<'_, _> =
                    current_processor_arc.lock().unwrap();
                let current_processor = current_processor_mutg.deref();
                if new_speed <= current_processor.get_speed() {
                    insert_index = i;
                    break;
                }
                i += 1;
            }
        }
        {
            let mut fastp_mutg = self.fastest_local_video_processors.lock().unwrap();
            let fastp = fastp_mutg.deref_mut();
            fastp.insert(insert_index, new_id);
        }
    }

    ///Adds the given LocalMusicProcessor (new_processor_arc) to the appropriate location in self.fastest_local_music_processors.
    fn new_local_music_processor_sort(
        &'static self,
        new_processor_arc: Arc<Mutex<LocalMusicProcessor>>,
    ) {
        let new_processor_mutg = new_processor_arc.lock().unwrap();
        let new_processor: &LocalMusicProcessor = new_processor_mutg.deref();
        let new_id = new_processor.get_id();
        let new_speed = new_processor.get_speed();
        let mut insert_index = 0;
        {
            let mut i = 0;
            let fastp_mutg = self.fastest_local_music_processors.lock().unwrap();
            let fastp = fastp_mutg.deref();
            for current_id in fastp.iter() {
                let current_processor_arc = self.get_local_music_processor(current_id).unwrap();
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
            let mut fastp_mutg = self.fastest_local_music_processors.lock().unwrap();
            let fastp = fastp_mutg.deref_mut();
            fastp.insert(insert_index, new_id);
        }
    }

    fn new_any_processor_sort<T: VideoProcessor>(&'static self, new_processor_arc: Arc<Mutex<T>>) {
        let vp_type: VideoProcessorType;
        {
            let vp_mutg = new_processor_arc.lock().unwrap();
            vp_type = vp_mutg.deref().get_type();
        }
        match vp_type {
            VideoProcessorType::LOCAL_VIDEO => {
                let vp_casted_arc: Arc<Mutex<LocalVideoProcessor>> =
                    cast_video_processor_arc::<LocalVideoProcessor, T>(new_processor_arc);
                self.new_local_video_processor_sort(vp_casted_arc);
            }
            VideoProcessorType::LOCAL_MUSIC => {
                let vp_casted_arc: Arc<Mutex<LocalMusicProcessor>> =
                    cast_video_processor_arc::<LocalMusicProcessor, T>(new_processor_arc);
                self.new_local_music_processor_sort(vp_casted_arc);
            }
        }
    }

    ///Get a video processor by it's id (processor_id). Returns None if the processor with the specified id was not found.
    pub fn get_processor(
        &'static self,
        processor_id: &u64,
    ) -> Option<Arc<Mutex<dyn VideoProcessor>>> {
        let processor_arc_option: Option<Arc<Mutex<dyn VideoProcessor>>> =
            match self.get_local_video_processor(&processor_id) {
                Some(vp_arc) => Some(vp_arc),
                None => match self.get_local_music_processor(&processor_id) {
                    Some(vp_arc) => Some(vp_arc),
                    None => None,
                },
            };
        processor_arc_option
    }

    pub fn get_local_video_processor(
        &'static self,
        processor_id: &u64,
    ) -> Option<Arc<Mutex<LocalVideoProcessor>>> {
        let localvp_mutg = self.local_video_processors.lock().unwrap();
        let localvp = localvp_mutg.deref();
        if localvp.contains_key(processor_id) {
            Some(Arc::clone(localvp.get(processor_id).unwrap()))
        } else {
            None
        }
    }

    pub fn get_local_music_processor(
        &'static self,
        processor_id: &u64,
    ) -> Option<Arc<Mutex<LocalMusicProcessor>>> {
        let localmp_mutg = self.local_music_processors.lock().unwrap();
        let localmp = localmp_mutg.deref();
        if localmp.contains_key(processor_id) {
            Some(Arc::clone(localmp.get(processor_id).unwrap()))
        } else {
            None
        }
    }

    ///Adds a new LocalVideoProcessor (processor_arc).
    pub fn add_local_video_processor(
        &'static self,
        processor_arc: Arc<Mutex<LocalVideoProcessor>>,
    ) {
        {
            let processor_mutg = processor_arc.lock().unwrap();
            let processor: &LocalVideoProcessor = processor_mutg.deref();
            let mut localp_mutg = self.local_video_processors.lock().unwrap();
            let localp = localp_mutg.deref_mut();
            localp.insert(processor.get_id(), Arc::clone(&processor_arc));
        }
        {
            self.new_local_video_processor_sort(processor_arc);
        }
    }

    ///Adds a new LocalMusicProcessor (processor_arc).
    pub fn add_local_music_processor(
        &'static self,
        processor_arc: Arc<Mutex<LocalMusicProcessor>>,
    ) {
        {
            let processor_mutg = processor_arc.lock().unwrap();
            let processor: &LocalMusicProcessor = processor_mutg.deref();
            let mut localp_mutg = self.local_music_processors.lock().unwrap();
            let localp = localp_mutg.deref_mut();
            localp.insert(processor.get_id(), Arc::clone(&processor_arc));
        }
        {
            self.new_local_music_processor_sort(processor_arc);
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
            }
            VideoProcessorType::LOCAL_MUSIC => {
                self.add_local_music_processor(cast_video_processor_arc::<LocalMusicProcessor, F>(
                    processor_arc,
                ));
            }
        };
    }

    ///Activates the VideoProcessor with the specified id (processor_id). Does nothing if the processor with the given id was not found.
    pub fn activate_processor(&'static self, processor_id: u64) {
        let processor_arc_option = self.get_processor(&processor_id);
        match processor_arc_option {
            Some(processor_arc) => {
                let mut processor_mutg = processor_arc.lock().unwrap();
                let processor = processor_mutg.deref_mut();
                processor.activate();
            }
            None => {}
        };
    }

    ///Deactivates the VideoProcessor with the specified id (processor_id).
    pub fn deactivate_processor(
        &'static self,
        processor_id: &u64,
    ) -> Option<Arc<Mutex<dyn VideoProcessor>>> {
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

    #[cfg(target_family = "unix")]
    /// Determines whether or not if there is a VideoProcessor that can handle the given video (file_bytes).
    pub fn is_supported_file_data(&'static self, file_bytes: &Vec<u8>) -> bool {
        // todo!("Finish this!");
        use std::path::Path;

        use nix::{
            sys::{
                stat::Mode,
                sysinfo::{SysInfo, sysinfo},
            },
            unistd::mkfifo,
        };
        //Create parent directory if not exists:
        let exec_name = std::env::current_exe().unwrap();
        let bd_str;
        let base_dir;

        //Create video data pipe or regular file:
        let mut rnd = rand::rng();
        let pipe_path: PathBuf;

        let sysinfo: SysInfo = sysinfo().unwrap();
        let available_mem: u64 = sysinfo.ram_unused() + sysinfo.swap_free();
        let is_low_mem: bool = available_mem < (file_bytes.len() as u64) * 2;
        if is_low_mem {
            //Create regular file.
            bd_str = format!("{}/.tmp", std::env::home_dir().unwrap().to_str().unwrap());
            base_dir = Path::new(&bd_str);
            pipe_path = loop {
                use rand::RngExt;

                let p: PathBuf =
                    PathBuf::from(&base_dir).join(format!("./{}", rnd.random::<u64>()));
                if !std::fs::exists(&p).unwrap_or(true) {
                    break p;
                }
            };
            if !std::fs::exists(&base_dir).unwrap() {
                std::fs::create_dir(&base_dir).unwrap();
            }
            let _ = std::fs::File::create(&pipe_path);
        } else {
            //Create POSIX pipe.
            bd_str = format!("/tmp/{}", exec_name.to_str().unwrap());
            base_dir = Path::new(&bd_str);
            pipe_path = loop {
                use rand::RngExt;

                let p: PathBuf =
                    PathBuf::from(&base_dir).join(format!("./{}", rnd.random::<u64>()));
                if !std::fs::exists(&p).unwrap_or(true) {
                    break p;
                }
            };
            if !std::fs::exists(&base_dir).unwrap() {
                std::fs::create_dir(&base_dir).unwrap();
            }
            match mkfifo(&pipe_path, Mode::S_IRWXU) {
                Ok(_) => {}
                Err(_) => return false,
            };
        }

        //Get video description:
        {
            let mut pipe_file = std::fs::File::create(&pipe_path).unwrap();
            pipe_file.write_all(file_bytes.as_slice()).unwrap();
        }
        let video_description = match get_video_description(pipe_path.to_str().unwrap()) {
            Ok(res) => res,
            Err(_) => return false,
        };

        //Remove the pipe file:
        let _ = std::fs::remove_file(&pipe_path);

        //Determine whether or not if there is a VideoProcessor that supports the given video data:
        let found_arc: Arc<Mutex<BoolBox>> = Arc::new(Mutex::new(BoolBox::new(false)));
        self.for_each_processor(
            |proc_arc: Arc<Mutex<dyn VideoProcessor + 'static>>| -> bool {
                let proc_mutg = proc_arc.lock().unwrap();
                let proc: &dyn VideoProcessor = proc_mutg.deref();

                //Determine if this VideoProcessor supports the given file data.
                if proc.is_supported_file(&video_description) {
                    let mut found_mutg = found_arc.lock().unwrap();
                    let found: &mut BoolBox = found_mutg.deref_mut();
                    found.set(true);
                    return true;
                }

                false
            },
        );

        //Return:
        let found_mutg = found_arc.lock().unwrap();
        let found: &BoolBox = found_mutg.deref();
        found.get()
    }

    #[cfg(target_family = "windows")]
    /// Determines whether or not if there is a VideoProcessor that can handle the given video (file_bytes).
    pub fn is_supported_file_data(&'static self, file_bytes: &Vec<u8>) -> bool {
        //Create parent directory if not exists:
        let exec_name = std::env::current_exe().unwrap();
        let bd_str = format!(
            "{}/{}",
            std::env::temp_dir().to_str().unwrap(),
            exec_name.to_str().unwrap()
        );
        let base_dir = Path::new(bd_str.as_str());
        if !std::fs::exists(&base_dir).unwrap() {
            std::fs::create_dir(&base_dir).unwrap();
        }

        //Get video file path:
        let mut rnd = rand::rng();
        let video_file_path: PathBuf = loop {
            let p = PathBuf::from(&base_dir).join(format!("./{}.dat", rnd.random::<u64>()));
            if !std::fs::exists(&p).unwrap() {
                break p;
            }
        };

        //Write data:
        {
            let mut video_file = std::fs::File::create(&video_file_path).unwrap();
            video_file.write_all(file_bytes.as_slice()).unwrap();
        }

        //Get video description:
        let video_description = match get_video_description(video_file_path.to_str().unwrap()) {
            Ok(res) => res,
            Err(_) => return false,
        };

        //Remove the video file:
        let _ = std::fs::remove_file(&video_file_path);

        //Determine whether or not if there is a VideoProcessor that supports the given video data:
        let found_arc: Arc<Mutex<BoolBox>> = Arc::new(Mutex::new(BoolBox::new(false)));
        self.for_each_processor(
            |proc_arc: Arc<Mutex<dyn VideoProcessor + 'static>>| -> bool {
                let proc_mutg = proc_arc.lock().unwrap();
                let proc: &dyn VideoProcessor = proc_mutg.deref();

                //Determine if this VideoProcessor supports the given file data.
                if proc.is_supported_file(&video_description) {
                    let mut found_mutg = found_arc.lock().unwrap();
                    let found: &mut BoolBox = found_mutg.deref_mut();
                    found.set(true);
                    return true;
                }

                false
            },
        );

        //Return:
        let found_mutg = found_arc.lock().unwrap();
        let found: &BoolBox = found_mutg.deref();
        found.get()
    }

    pub fn for_each_processor<F: Fn(Arc<Mutex<dyn VideoProcessor>>) -> bool>(
        &'static self,
        action: F,
    ) {
        //Video processors:
        let flvps_mutg = self.fastest_local_video_processors.lock().unwrap();
        let flvps = flvps_mutg.deref();
        for lvp_id in flvps.iter() {
            let lvp_arc = self.get_local_video_processor(lvp_id).unwrap();
            if action(lvp_arc) {
                return;
            }
        }

        //Music processors:
        let flmps_mutg = self.fastest_local_music_processors.lock().unwrap();
        let flmps = flmps_mutg.deref();
        for lmp_id in flmps.iter() {
            if action(self.get_local_music_processor(lmp_id).unwrap()) {
                return;
            }
        }
    }

    /// Guesses whether or not if there is enough space to processess the given video data (file_bytes) in the given output directory (output_directory).
    pub fn has_enough_space(
        &'static self,
        file_bytes: &[u8],
        output_directory: &std::path::Path,
    ) -> bool {
        let (num_blocks, block_size) = match get_available_fs_space(output_directory) {
            Some(met) => met,
            None => return false,
        };
        (file_bytes.len() as u64) <= num_blocks * block_size
    }

    ///Get fastest available VideoProcessor. Returns the processor_id of the fastest available VideoProcessor.
    fn get_optimal_available_vp(
        &'static self,
        max_alllowed_jobs: u64,
        video_file_path: &String,
    ) -> Option<u64> {
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
            Err(_) => return None,
        };
        let video_file_path_full_mutg = video_file_path_full_arc.lock().unwrap();
        let video_file_path_full: &String = video_file_path_full_mutg.deref();
        let description: VeryBasicVideoFileDescription =
            match get_video_description(video_file_path_full.as_str()) {
                Ok(desc) => desc,
                Err(e) => return None,
            };

        //Get fastest available VideoProcessor:
        // let video_file_path_full_mutg = video_file_path_full_arc.lock().unwrap();
        // let video_file_path_full: &String = video_file_path_full_mutg.deref();
        let fastlvps_mutg = self.fastest_local_video_processors.lock().unwrap();
        let fastlvps = fastlvps_mutg.deref();
        let fastlmps_mutg = self.fastest_local_music_processors.lock().unwrap();
        let fastlmps = fastlmps_mutg.deref();

        if fastlvps.is_empty() && fastlmps.is_empty() {
            return None;
        }

        let mut ids_by_speed: HashMap<u64, u64> = HashMap::new();
        let mut fastest_speeds: Vec<u64> = Vec::new();
        // fastest_speeds.sort();

        //Find fastest available LocalVideoProcessor:
        // let mut fastest_lvp_id: u64;
        // let mut fastest_lvp_speed: u64;
        // if fastlvps.len() > 0 {
        //     let first_lvp_arc = self.get_processor(fastlvps.get(0).unwrap()).unwrap();
        //     let first_lvp_mutg = first_lvp_arc.lock().unwrap();
        //     let first_lvp = first_lvp_mutg.deref();
        //     fastest_lvp_id = first_lvp.get_id();
        //     fastest_lvp_speed = first_lvp.get_speed();
        // }
        for lvp_id in fastlvps {
            let lvp_arc = self.get_processor(&lvp_id).unwrap();
            let lvp_mutg = lvp_arc.lock().unwrap();
            let lvp = lvp_mutg.deref();
            if lvp.is_activated()
                && lvp.get_num_active_jobs() < max_alllowed_jobs
                && lvp.is_supported_file(&description)
            {
                // fastest_lvp_id = lvp.get_id();
                // fastest_lvp_speed = lvp.get_speed();
                let speed = lvp.get_speed();
                fastest_speeds.push(speed);
                ids_by_speed.insert(speed, lvp.get_id());
                break;
            }
        }

        //Find fastest available LocalMusicProcessor:
        // let mut fastest_lmp_id: u64;
        // let mut fastest_lmp_speed: u64;
        // if fastlmps.len() > 0 {
        //     let first_lmp_arc = self.get_processor(fastlmps.get(0).unwrap()).unwrap();
        //     let first_lmp_mutg = first_lmp_arc.lock().unwrap();
        //     let first_lmp = first_lmp_mutg.deref();
        //     fastest_lmp_id = first_lmp.get_id();
        //     fastest_lmp_speed = first_lmp.get_speed();
        // }
        for lmp_id in fastlmps {
            let lmp_arc = self.get_processor(&lmp_id).unwrap();
            let lmp_mutg = lmp_arc.lock().unwrap();
            let lmp = lmp_mutg.deref();
            if lmp.is_activated()
                && lmp.get_num_active_jobs() < max_alllowed_jobs
                && lmp.is_supported_file(&description)
            {
                // fastest_lmp_id = lmp.get_id();
                // fastest_lmp_speed = lmp.get_speed();
                let speed = lmp.get_speed();
                fastest_speeds.push(speed);
                ids_by_speed.insert(speed, lmp.get_id());
                break;
            }
        }

        //Compare VideoProcessors and return the fastest.
        if fastest_speeds.is_empty() {
            return None;
        }
        fastest_speeds.sort();
        Some(
            *ids_by_speed
                .get(fastest_speeds.get(fastest_speeds.len() - 1).unwrap())
                .unwrap(),
        )
    }

    ///Processes a video, converting it into the MPEG_DASH format, with the specified video id (video_id) and file path of the video (video_file_path) (corresponding to the video id), and ouputs the result into the specified directory (result_directory).
    pub fn process(
        &'static self,
        video_id: u64,
        video_file_path: &String,
        result_directory: &String,
    ) {
        //Create result directory if missing:
        std::fs::create_dir_all(result_directory).unwrap();

        //Process:
        let pr_id_opt = self.get_optimal_available_vp(2, video_file_path);
        let pr_id: &u64 = match pr_id_opt {
            Some(id) => &id.clone(),
            None => return,
        };
        let pr_arc = self.get_processor(&pr_id).unwrap();
        let mut pr_mutg = pr_arc.lock().unwrap();
        let pr = pr_mutg.deref_mut();
        {
            //Process the video.
            {
                //Add to list of currently processing videos:
                let mut cp_mutg = self.currently_processing.lock().unwrap();
                let cp: &mut HashMap<u64, u64> = cp_mutg.deref_mut();
                cp.insert(video_id, pr.get_id());
            }
            let rx = pr
                .process_video(video_id, video_file_path.clone(), result_directory.clone())
                .unwrap();
            let pr_id_copy: u64 = *pr_id;
            thread::spawn(move || {
                let result = rx.recv().unwrap();
                println!("Received data from ffmpeg process!");
                if result {
                    let mut fp_arc_clone_mutg = self.finished_processing.lock().unwrap();
                    let fp_clone = fp_arc_clone_mutg.deref_mut();
                    fp_clone.insert(video_id, pr_id_copy);

                    {
                        let fs_func_mutg = self.on_finished_success_function.lock().unwrap();
                        let fs_func_box = fs_func_mutg.deref();
                        if fs_func_box.get().is_some() {
                            (fs_func_box.get().unwrap())(video_id);
                        }
                    }
                } else {
                    let mut fail_arc_clone_mutg = self.failed_processing.lock().unwrap();
                    let failp_clone = fail_arc_clone_mutg.deref_mut();
                    failp_clone.insert(video_id, pr_id_copy);

                    // (self.on_finished_fail_function)(video_id);
                    {
                        let ff_func_mutg = self.on_finished_fail_function.lock().unwrap();
                        let ff_func_box = ff_func_mutg.deref();
                        if ff_func_box.get().is_some() {
                            (ff_func_box.get().unwrap())(video_id);
                        }
                    }
                }
            });
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
        result
    }

    ///Check whether or not if the video with the specified video_id is currently processing:
    pub fn is_currently_processing(&'static self, video_id: &u64) -> bool {
        let cp_mutg = self.currently_processing.lock().unwrap();
        let cp = cp_mutg.deref();
        cp.contains_key(video_id)
    }

    ///Get a COPY of the list of videos that have SUCCESSFULLY finished processing.
    pub fn get_finished_processing_videos(&'static self) -> Vec<u64> {
        let fp_mutg = self.finished_processing.lock().unwrap();
        let fp = fp_mutg.deref();
        let mut result: Vec<u64> = Vec::new();
        for id in fp.keys() {
            result.push(*id);
        }
        result
    }

    ///Check whether or not if the video with the specified video_id has SUCCESSFULLY finished processing:
    pub fn is_finished_processing(&'static self, video_id: &u64) -> bool {
        let fp_mutg = self.finished_processing.lock().unwrap();
        let fp = fp_mutg.deref();
        fp.contains_key(video_id)
    }

    ///Get a COPY of the list of videos that have failed to process.
    pub fn get_failed_processing_videos(&'static self) -> Vec<u64> {
        let failp_mutg = self.failed_processing.lock().unwrap();
        let failp = failp_mutg.deref();
        let mut result: Vec<u64> = Vec::new();
        for id in failp.keys() {
            result.push(*id);
        }
        result
    }

    ///Check whether or not if the video with the specified video_id has failed to process.
    pub fn is_failed_processing(&'static self, video_id: &u64) -> bool {
        let failp_mutg = self.failed_processing.lock().unwrap();
        let failp = failp_mutg.deref();
        failp.contains_key(video_id)
    }

    ///Deactivates and removes the processor with the specified id (processor_id), if it exists.
    pub fn remove_processor(&'static self, processor_id: &u64) {
        if self.has_processor(processor_id) {
            //Deactivate processor:
            self.deactivate_processor(processor_id);

            //Remove processor:
            let mut localvp_mutg = self.local_video_processors.lock().unwrap();
            let localvp = localvp_mutg.deref_mut();
            let mut localmp_mutg = self.local_music_processors.lock().unwrap();
            let localmp = localmp_mutg.deref_mut();
            if localvp.contains_key(processor_id) {
                localvp.remove(processor_id).unwrap();
            } else if localmp.contains_key(processor_id) {
                localmp.remove(processor_id).unwrap();
            } else {
                todo!(
                    "VideoProcessorManager.has_processor(processor_id) returned true, but was not found in either VideoProcessorManager.local_video_processors or VideoProcessorManager.local_music_processors!"
                );
            }

            //Remove from fastest_local_video_processors:
            let mut fast_lvps_mutg = self.fastest_local_video_processors.lock().unwrap();
            let fast_lvps = fast_lvps_mutg.deref_mut();
            let mut flvps_index: isize = -1;
            {
                let mut i = 0;
                while i < fast_lvps.len() {
                    if *processor_id == *fast_lvps.get(i).unwrap() {
                        flvps_index = i as isize;
                        break;
                    }
                    i += 1;
                }
            }
            if flvps_index >= 0 {
                fast_lvps.remove(flvps_index as usize);
            }

            //Remove from fastest_local_music_processors:
            let mut fast_lmps_mutg = self.fastest_local_music_processors.lock().unwrap();
            let fast_lmps = fast_lmps_mutg.deref_mut();
            let mut flmps_index: isize = -1;
            {
                let mut i = 0;
                while i < fast_lmps.len() {
                    if *processor_id == *fast_lmps.get(i).unwrap() {
                        flmps_index = i as isize;
                        break;
                    }
                    i += 1;
                }
            }
            if flmps_index >= 0 {
                fast_lmps.remove(flmps_index as usize);
            }
        }
    }

    ///Returns whether or not if the VideoProcessor with the specified id (processor_id) exists.
    pub fn has_processor(&'static self, processor_id: &u64) -> bool {
        return self.get_processor(processor_id).is_some();
    }

    // ///Gets a DEEP COPY of the HashMap of all of the video processors.
    // pub fn get_processors_clone(&'static self) -> HashMap<u64, Box<dyn VideoProcessor>> {
    //     let mut result: HashMap<u64, Box<dyn VideoProcessor>> = HashMap::new();
    //     for vp_id in self.get_all_proc_ids() {
    //         let vp_arc = self.get_processor(&vp_id).unwrap();
    //         let vp_mutg = vp_arc.lock().unwrap();
    //         result.insert(vp_id, Box::new(vp_mutg.deref().clone()));
    //     }
    //     return result;
    // }

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

    ///Activates all of the video processors.
    pub fn activate_all_procs(&'static self) {
        let lvps_mutg = self.local_video_processors.lock().unwrap();
        let lvps = lvps_mutg.deref();
        let lmps_mutg = self.local_music_processors.lock().unwrap();
        let lmps = lmps_mutg.deref();
        for lvp_arc in lvps.values() {
            let mut lvp_mutg = lvp_arc.lock().unwrap();
            let lvp = lvp_mutg.deref_mut();
            lvp.activate();
        }
        for lmp_arc in lmps.values() {
            let mut lmp_mutg = lmp_arc.lock().unwrap();
            let lmp = lmp_mutg.deref_mut();
            lmp.activate();
        }
    }

    pub fn get_all_proc_ids(&'static self) -> Vec<u64> {
        let mut result: Vec<u64> = Vec::new();
        let lvps_mutg = self.local_video_processors.lock().unwrap();
        let lvps = lvps_mutg.deref();
        let lmps_mutg = self.local_music_processors.lock().unwrap();
        let lmps = lmps_mutg.deref();

        //Add LocalVideoProcessors:
        for lvp_id in lvps.keys() {
            result.push(*lvp_id);
        }
        //Add LocalMusicProcessors:
        for lmp_id in lmps.keys() {
            result.push(*lmp_id);
        }

        result
    }

    ///Deactivates all of the video processors.
    pub fn deactivate_all_procs(&'static self) {
        for vp_id in self.get_all_proc_ids() {
            self.deactivate_processor(&vp_id);
        }
    }

    pub async fn process_image<
        // THIP: ThumbnailImageProcessor + std::marker::Sized,
        ActSuc: Fn() + std::marker::Send + std::marker::Sync,
        ActFail: Fn(ImageError) + std::marker::Send + std::marker::Sync,
    >(
        &'static self,
        input_image: Vec<u8>,
        output_directory: std::path::PathBuf,
        on_success: &'static ActSuc,
        on_fail: &'static ActFail,
    ) -> () {
        thread::spawn(move || -> () {
            match {
                // let image_proc_arc = self.thumbnail_image_processor.get().unwrap();
                // let image_proc_mutg = image_proc_arc.lock().unwrap();
                // let image_proc: &dyn ThumbnailImageProcessor = image_proc_mutg.deref().deref();
                let thip_cont_mutg = self.thumbnail_image_processor.lock().unwrap();
                let thip_cont = thip_cont_mutg.deref();
                let image_proc = thip_cont.deref().as_ref().unwrap();
                image_proc.process(input_image.as_slice(), &output_directory)
            } {
                Ok(_) => on_success(),
                Err(e) => on_fail(e),
            };
        });
    }
}

fn is_directory(p: &PathBuf) -> bool {
    let dir_canon: std::path::PathBuf = match std::fs::canonicalize(p) {
        Ok(dir) => dir,
        Err(_) => return false,
    };
    //Check if dir_canon is a directory:
    match std::fs::metadata(&dir_canon) {
        Ok(met) => {
            if met.is_symlink() {
                todo!("Symbolic links are not yet supported!");
                false
            } else if !met.is_dir() {
                false
            } else {
                true
            }
        }
        Err(_) => false,
    }
}

#[cfg(target_family = "unix")]
/// Gets the total amount of available blocks and the block size.
///
/// Parameters:
///
///     `directory`: The directory to get the amount of available blocks and block size from.
///
/// Returns:
///     - None: If `directory` is not actually a directory, the directory does not exist, or for whatever reason the metadata cannot be read.
///     - Some((num_blocks, block_size)): When successful. First parameter in the tuple, `num_blocks` represents the number of available blocks as a u64 value. The second parameter, `block_size`, represents the block size in bytes for the given directory.
pub fn get_available_fs_space(directory: &std::path::Path) -> Option<(u64, u64)> {
    use nix::sys::statvfs::{self, Statvfs};

    let dir_canon: std::path::PathBuf = match std::fs::canonicalize(directory) {
        Ok(dir) => dir,
        Err(_) => return None,
    };

    if !is_directory(&dir_canon) {
        return None;
    }

    // let dir_command = Command::new("du");
    let statvfs_obj: Statvfs = match statvfs::statvfs(&dir_canon) {
        Ok(res) => res,
        Err(_) => return None,
    };

    let blocks: u64 = statvfs_obj.blocks_available();
    let block_size: u64 = statvfs_obj.block_size();
    Some((blocks, block_size))
}

#[cfg(target_family = "windows")]
/// Gets the total amount of available blocks and the block size.
///
/// Parameters:
///
///     directory: The directory to get the amount of available blocks and block size from.
///
/// Returns:
///     - None: If `directory` is not actually a directory, the directory does not exist, or for whatever reason the metadata cannot be read.
///     - Some((num_blocks, block_size)): When successful. First parameter in the tuple, `num_blocks` represents the number of available blocks as a u64 value. The second parameter, `block_size`, represents the block size in bytes for the given directory.
pub fn get_available_fs_space(directory: &std::path::Path) -> Option<(u64, u64)> {
    use windows::Win32::Storage::FileSystem;

    let dir_canon: std::path::PathBuf = match std::fs::canonicalize(directory) {
        Ok(dir) => dir,
        Err(_) => return None,
    };

    if !is_directory(&dir_canon) {
        return None;
    }

    let mut free_bytes_available: u64 = 0;
    let mut total_num_bytes: u64 = 0;
    let mut total_num_free_bytes: u64 = 0;
    unsafe {
        match GetDiskFreeSpaceExW(
            &dir_canon,
            &mut free_bytes_available,
            &mut total_num_bytes,
            &mut total_free_bytes,
        ) {
            Ok(_) => {}
            Err(_) => return None,
        }
    }
    let met = match std::fs::metadata(&dir_canon) {
        Ok(_) => {}
        Err(_) => return None,
    };
    let block_size: u64 = met.blksize();

    return Some((free_bytes_available / block_size, block_size));
}
