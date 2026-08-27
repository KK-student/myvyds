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

    fn get_input() -> String {
        let _ = io::stdout().flush();
        let mut input_string: String = String::new();
        let _ = io::stdin().read_line(&mut input_string);
        let result: &str = input_string.trim_end();
        return result.to_string();
    }
}

impl ManagementConsole for TextBasedManagementConsole {
    fn launch(&'static self, pkg_verson: String) {
        thread::spawn(move || {
            let help_text = "h -> Show this help message. 
vprocs -> Show a list of all processors. 
vproc -> Show the basic information for a processor. 
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
                let mut input = Self::get_input();
                println!("You typed: \"{input}\". ");
                input = input.to_lowercase();

                fn view_proc(
                    id: u64,
                    speed: u64,
                    num_active_jobs: u64,
                    currently_processing: &HashSet<u64>,
                ) {
                    println!(
                        "id {id}: 
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
                            );
                        }
                    }
                    "vproc" => {
                        let procs = self.processor_manager.get_processors_clone();
                        let id: u64 = loop {
                            break match {
                                print!("Enter processor id: ");
                                Self::get_input().parse::<u64>()
                            } {
                                Ok(num) => {
                                    if !procs.contains_key(&num) {
                                        println!("Invalid input. ");
                                        continue;
                                    } else {
                                        num
                                    }
                                }
                                Err(_) => {
                                    println!("Invalid input. ");
                                    continue;
                                }
                            };
                        };
                        view_proc_obj(procs.get(&id).unwrap());
                    }
                    &_ => println!("Error: \"{input}\" is not a valid command. "),
                };
                println!();
            }
        });
    }
}
