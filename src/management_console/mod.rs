use std::{
    collections::HashSet,
    io::{self, Write},
    sync::Mutex,
    thread,
};

use crate::file_handling::{
    file_handling::DataManager,
    video_processing::{VideoProcessor, VideoProcessorManager},
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
                        vproc.is_activated()
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
                                pr.is_activated()
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
                    &_ => println!("Error: \"{input}\" is not a valid command. "),
                };
                println!();
            }
        });
    }
}
