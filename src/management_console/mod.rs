use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    thread,
};

use crate::file_handling::{
    file_handling::DataManager,
    video_processing::{LocalVideoProcessor, VideoProcessor, VideoProcessorManager},
};

mod string_input;

use string_input::*;

pub trait ManagementConsole {
    fn launch(&'static self, pkg_version: String);
}

pub struct TextBasedManagementConsole {
    data_manager: &'static std::sync::LazyLock<Mutex<DataManager>>,
    processor_manager: &'static VideoProcessorManager,
}

impl TextBasedManagementConsole {
    pub const fn new(
        data_manager: &'static std::sync::LazyLock<Mutex<DataManager>>,
        processor_manager: &'static VideoProcessorManager,
    ) -> Self {
        return Self {
            data_manager,
            processor_manager,
        };
    }
}

impl ManagementConsole for TextBasedManagementConsole {
    fn launch(&'static self, pkg_verson: String) {
        thread::spawn(move || {
            let help_text = "h -> Show this help message. 
vprocs -> Show a list of all processors. 
vproc -> Show the basic information for a processor. 
activate_proc-> Enable a processor. 
deactivate_proc-> Disable a processor. 
activate_all_procs -> Activate all processors. 
deactivate_all_procs -> Deactivate all processors. 
add_processor -> Add a new processor. 
To quit, press Ctrl+C. ";
            println!(
                "Myvids v{} text-based management console: ",
                pkg_verson // std::env::var("CARGO_PKG_VERSION").unwrap()
            );
            let mut first_exec = true;
            loop {
                if first_exec {
                    println!("{help_text}");
                    first_exec = false;
                }
                print!("Enter a command: ");
                let input = get_input().to_lowercase();

                fn view_proc(
                    id: u64,
                    speed: u64,
                    num_active_jobs: u64,
                    currently_processing: &HashSet<u64>,
                    is_activated: bool,
                ) {
                    println!(
                        "id {id}: 
\tactivated: {is_activated}
\tspeed: {}
\tnumber of active jobs: {}
\tvideos currently processing (by id): {:?}",
                        speed, num_active_jobs, currently_processing
                    );
                }
                fn view_proc_obj<T: VideoProcessor>(vproc: &T) {
                    view_proc(
                        vproc.get_id(),
                        vproc.get_speed(),
                        vproc.get_num_active_jobs(),
                        &vproc.get_currently_processing(),
                        vproc.is_activated(),
                    );
                }
                match input.as_str() {
                    "h" => println!("{help_text}"),
                    "vprocs" => {
                        println!("List of processors: ");
                        let procs = self.processor_manager.get_processors_clone();
                        for (pr_id, pr) in procs {
                            view_proc(
                                pr_id,
                                pr.get_speed(),
                                pr.get_num_active_jobs(),
                                &pr.get_currently_processing(),
                                pr.is_activated(),
                            );
                        }
                    }
                    "vproc" => {
                        let procs = self.processor_manager.get_processors_clone();
                        let id: u64 = get_number_in_set(&procs, "Enter processor id");
                        view_proc_obj(procs.get(&id).unwrap());
                    }
                    "activate_proc" => {
                        let procs = self.processor_manager.get_processors_clone();
                        let id: u64 = get_number_in_set(&procs, "Enter processor id");
                        if !self.processor_manager.is_activated(&id) {
                            self.processor_manager.activate_processor(id);
                        }
                    }
                    "deactivate_proc" => {
                        let procs = self.processor_manager.get_processors_clone();
                        let id: u64 = get_number_in_set(&procs, "Enter processor id");
                        if self.processor_manager.is_activated(&id) {
                            self.processor_manager.activate_processor(id);
                        }
                    }
                    "activate_all_procs" => {
                        self.processor_manager.activate_all_procs();
                    }
                    "deactivate_all_procs" => {
                        self.processor_manager.deactivate_all_procs();
                    }
                    "add_processor" => {
                        //Get processor type:
                        let menu_options: [&str; 1] = [
                            "LOCAL_VIDEO -> Processes videos locally on this computer's cpu, graphics card, and audio processor (if available)",
                        ];
                        let item_index = get_menu_item(
                            Some("Processor types"),
                            "Enter processor type",
                            &menu_options,
                            1,
                        );
                        match item_index {
                            0 => {
                                //Get ffmpeg options:
                                let mut ffmpeg_opts: Vec<String> = Vec::new();
                                let default_ffmpeg_opts: &str = "-map 0 -c:v:0 libx264 -b:v:0 5000k -s:v:0 1920x1080 -map 0 -c:v:1 libx264 -b:v:1 3000k -s:v:1 1280x720 -map 0 -c:v:2 libx264 -b:v:2 1000k -s:v:2 640x360 -map 0 -c:v:3 libvpx-vp9 -b:v:3 5000k -s:v:3 1920x1080 -map 0 -c:v:4 libvpx-vp9 -b:v:4 3000k -s:v:4 1280x720 -map 0 -c:v:5 libvpx-vp9 -b:v:5 1000k -s:v:5 640x360 -map 0:a:0 -c:a:0 aac -b:a:0 192k -map 0:a:0 -c:a:1 aac -b:a:1 128k -map 0:a:0 -c:a:2 aac -b:a:2  64k -map 0:a:0 -c:a:3 libopus -b:a:3 192k -map 0:a:0 -c:a:4 libopus -b:a:4 128k -map 0:a:0 -c:a:5 libopus -b:a:5  64k";
                                let use_default_ffmpeg_opts =
                                    get_yes_or_no(format!("The default command line options for ffmpeg, which is used to convert the input video file into MPEG_DASH format, are the following \"{default_ffmpeg_opts}\". Do you want to use them?").as_str());
                                if use_default_ffmpeg_opts {
                                    for part in default_ffmpeg_opts.split_whitespace() {
                                        ffmpeg_opts.push(part.to_string());
                                    }
                                } else {
                                    // todo!("Implement this!");
                                    loop {
                                        let _ois = get_input();
                                        let opts_input = _ois.as_str();
                                        for part in opts_input.split_whitespace() {
                                            ffmpeg_opts.push(part.to_string());
                                        }
                                        if get_yes_or_no(format!("You entered the following ffmpeg options: {:?}. Do you want to enter different ones?", ffmpeg_opts).as_str()) {
                                            ffmpeg_opts.clear();
                                            continue;
                                        } else {
                                            break;
                                        }
                                    }
                                }
                                //Get ID:
                                let id: u64 = get_number::<u64>("Enter processor id");
                                //Get max_gpu_tflops:
                                let max_gpu_tflops: u64 = get_number::<u64>(
                                    "Enter the maximum amount of tflops your GPU can do in a second",
                                );
                                //Create LocalVideoProcessor:
                                self.processor_manager.add_local_video_processor(Arc::new(
                                    Mutex::new(LocalVideoProcessor::new_with_opts(
                                        id,
                                        &ffmpeg_opts,
                                        max_gpu_tflops,
                                    )),
                                ));
                            }
                            _ => {
                                panic!(
                                    "{item_index} is not a valid option! get_menu_item function failed!"
                                )
                            }
                        }
                    }
                    &_ => println!("Error: \"{input}\" is not a valid command. "),
                };
                println!();
            }
        });
    }
}
