use std::{
    collections::HashSet,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
    thread,
};

use argon2::{Argon2, PasswordHasher};

use crate::{
    file_handling::{
        file_handling::DataManager,
        user::User,
        video_processing::{
            VideoProcessorManager,
            local_music_processor::LocalMusicProcessor,
            local_video_processor::LocalVideoProcessor,
            video_processor::{VideoProcessor, VideoProcessorType},
        },
    },
    management_console::{
        ManagementConsole,
        password_input::{prompt_change_password, prompt_new_password},
        string_input::*,
    },
};

struct VideoProcessorManagerContainer {
    vpm: &'static VideoProcessorManager,
}

impl Container<u64> for VideoProcessorManagerContainer {
    fn contains_generic(&self, value: &u64) -> bool {
        self.vpm.has_processor(value)
    }
}

struct DataManagerUserContainer {
    dm: &'static std::sync::LazyLock<Mutex<DataManager>>,
}

impl Container<String> for DataManagerUserContainer {
    fn contains_generic(&self, value: &String) -> bool {
        let data_manager_mutg = self.dm.lock().unwrap();
        let data_manager = data_manager_mutg.deref();
        return match data_manager.get_user(value) {
            Some(_) => true,
            None => false,
        };
    }
}

struct DataManagerVideoDataContainer {
    dm: &'static std::sync::LazyLock<Mutex<DataManager>>,
}

impl Container<u64> for DataManagerVideoDataContainer {
    fn contains_generic(&self, value: &u64) -> bool {
        let data_manager_mutg = self.dm.lock().unwrap();
        let data_manager = data_manager_mutg.deref();
        return data_manager.has_video_by_id(value);
    }
}

struct DataManagerCommentContainer {
    dm: &'static std::sync::LazyLock<Mutex<DataManager>>,
}

impl Container<u64> for DataManagerCommentContainer {
    fn contains_generic(&self, value: &u64) -> bool {
        let data_manager_mutg = self.dm.lock().unwrap();
        let data_manager = data_manager_mutg.deref();
        return data_manager.has_comment(value);
    }
}

///A ManagementConsole that runs directly on the server and talks to the user via the server's terminal.
pub struct TextBasedManagementConsole {
    data_manager: &'static std::sync::LazyLock<Mutex<DataManager>>,
    data_manager_user_container: DataManagerUserContainer,
    data_manager_video_data_container: DataManagerVideoDataContainer,
    data_manager_comment_container: DataManagerCommentContainer,
    processor_manager: &'static VideoProcessorManager,
    processor_manager_container: VideoProcessorManagerContainer,
}

impl TextBasedManagementConsole {
    pub const fn new(
        data_manager: &'static std::sync::LazyLock<Mutex<DataManager>>,
        processor_manager: &'static VideoProcessorManager,
    ) -> Self {
        return Self {
            data_manager,
            data_manager_user_container: DataManagerUserContainer { dm: data_manager },
            data_manager_video_data_container: DataManagerVideoDataContainer { dm: data_manager },
            data_manager_comment_container: DataManagerCommentContainer { dm: data_manager },
            processor_manager,
            processor_manager_container: VideoProcessorManagerContainer {
                vpm: processor_manager,
            },
        };
    }
}

impl ManagementConsole for TextBasedManagementConsole {
    fn launch(&'static self, pkg_verson: String) {
        thread::spawn(move || {
            let help_text = "General options: 
h -> Show this help message. 

Managing processors: 
vprocs -> Show a list of all processors. 
vproc -> Show the basic information for a processor. 
activate_proc-> Enable a processor. 
deactivate_proc-> Disable a processor. 
activate_all_procs -> Activate all processors. 
deactivate_all_procs -> Deactivate all processors. 
add_processor -> Add a new processor. 
remove_processor -> Deactivate and remove a processor. 
save_processor_data -> Save all of the video processor data. 
save_video_user_data -> Save all of the user and video data. 
save_all_data -> Save all of the data, including all of the user, video, and video processor data. 

Managing user data: 
vusers -> Show a list of all users. 
vuser -> Show the basic information for a user. 
add_user -> Add a new user. 
remove_user -> Remove a user. 
edit_user -> Edit the info for a user. 

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
                    vp_type: VideoProcessorType,
                    speed: u64,
                    num_active_jobs: u64,
                    currently_processing: &HashSet<u64>,
                    is_activated: bool,
                ) {
                    println!(
                        "id {id}: 
\ttype: {:?}
\tactivated: {is_activated}
\tspeed: {}
\tnumber of active jobs: {}
\tvideos currently processing (by id): {:?}",
                        vp_type, speed, num_active_jobs, currently_processing
                    );
                }
                fn view_proc_obj(vproc: &dyn VideoProcessor) {
                    view_proc(
                        vproc.get_id(),
                        vproc.get_type(),
                        vproc.get_speed(),
                        vproc.get_num_active_jobs(),
                        &vproc.get_currently_processing(),
                        vproc.is_activated(),
                    );
                }
                fn view_user_obj(user_obj: &User) {
                    println!(
                        "id (aka username) \"{}\": 
\tpassword_hash: \"{}\"
\tdescription: \"{}\"
\tvideos: {:?}
\tsubscriptions: {:?}
\tliked videos: {:?}
\tdisliked videos: {:?}
\tliked comments: {:?}
\tdisliked comments: {:?}",
                        user_obj.get_user_id(),
                        user_obj.get_password_hash(),
                        user_obj.get_description(),
                        user_obj.get_videos(),
                        user_obj.get_subscriptions(),
                        user_obj.get_liked_vids(),
                        user_obj.get_disliked_vids(),
                        user_obj.get_liked_comments(),
                        user_obj.get_disliked_comments()
                    );
                }
                match input.as_str() {
                    "h" => println!("{help_text}"),
                    "vprocs" => {
                        println!("List of processors: ");
                        let procs_ids = self.processor_manager.get_all_proc_ids();
                        for pr_id in procs_ids {
                            let pr_arc = self.processor_manager.get_processor(&pr_id).unwrap();
                            let pr_mutg = pr_arc.lock().unwrap();
                            let pr = pr_mutg.deref();
                            view_proc(
                                pr_id,
                                pr.get_type(),
                                pr.get_speed(),
                                pr.get_num_active_jobs(),
                                &pr.get_currently_processing(),
                                pr.is_activated(),
                            );
                        }
                    }
                    "vproc" => {
                        // let procs = self.processor_manager.get_processors_clone();
                        // let id: u64 = get_number_in_set(&procs, "Enter processor id");
                        let id: u64 = get_value_in_set(
                            &self.processor_manager_container,
                            "Enter processor id",
                        );
                        let vp_arc: Arc<Mutex<dyn VideoProcessor>> =
                            self.processor_manager.get_processor(&id).unwrap();
                        let vp_mutg = vp_arc.lock().unwrap();
                        view_proc_obj(vp_mutg.deref());
                    }
                    "activate_proc" => {
                        let id: u64 = get_value_in_set(
                            &self.processor_manager_container,
                            "Enter processor id",
                        );
                        if !self.processor_manager.is_activated(&id) {
                            self.processor_manager.activate_processor(id);
                        }
                    }
                    "deactivate_proc" => {
                        let id: u64 = get_value_in_set(
                            &self.processor_manager_container,
                            "Enter processor id",
                        );
                        if self.processor_manager.is_activated(&id) {
                            self.processor_manager.deactivate_processor(&id);
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
                        let menu_options: [&str; 2] = [
                            "LOCAL_VIDEO -> Processes videos locally on this computer's cpu, graphics card, and audio processor (if available)",
                            "LOCAL_MUSIC -> Processes audio files locally on this computer's cpu or audio processor (if audio processor is available)",
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
                            1 => {
                                //Get ffmpeg options:
                                let mut ffmpeg_opts: Vec<String> = Vec::new();
                                let default_ffmpeg_opts: &str = "-strict experimental -c:v copy -map 0:a:0 -c:a:0 flac -b:a:0 192k -map 0:a:0 -c:a:1 flac -b:a:1 128k -map 0:a:0 -c:a:2 flac -b:a:2  64k -map_metadata 0";
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
                                let max_gpu_tflops: u64 =
                                    get_number::<u64>("Enter the speed of your music processor");
                                //Create LocalVideoProcessor:
                                self.processor_manager.add_local_music_processor(Arc::new(
                                    Mutex::new(LocalMusicProcessor::new_with_opts(
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
                    "remove_processor" => {
                        //Deactivate and remove a processor.
                        let id: u64 = get_value_in_set(
                            &self.processor_manager_container,
                            "Enter processor id",
                        );
                        self.processor_manager.remove_processor(&id);
                    }
                    "save_processor_data" => {
                        self.processor_manager.save_data();
                    }
                    "save_video_user_data" => {
                        let mut data_manager_mutg = self.data_manager.lock().unwrap();
                        let data_manager = data_manager_mutg.deref_mut();
                        data_manager.save_data();
                    }
                    "save_all_data" => {
                        let mut data_manager_mutg = self.data_manager.lock().unwrap();
                        let data_manager = data_manager_mutg.deref_mut();
                        data_manager.save_data();
                        self.processor_manager.save_data();
                    }
                    "vusers" => {
                        let data_manager_mutg = self.data_manager.lock().unwrap();
                        let data_manager = data_manager_mutg.deref();
                        data_manager.for_all_users(|user_arc: Arc<Mutex<User>>| -> () {
                            let user_mutg = user_arc.lock().unwrap();
                            let user: &User = user_mutg.deref();
                            view_user_obj(user);
                        });
                    }
                    "vuser" => {
                        let user_id: String = get_value_in_set(
                            &self.data_manager_user_container,
                            "Enter user id (aka username)",
                        );
                        let data_manager_mutg = self.data_manager.lock().unwrap();
                        let data_manager = data_manager_mutg.deref();
                        let user_arc = data_manager.get_user(&user_id).unwrap();
                        let user_mutg = user_arc.lock().unwrap();
                        let user: &User = user_mutg.deref();
                        view_user_obj(user);
                    }
                    "add_user" => {
                        let mut data_manager_mutg = self.data_manager.lock().unwrap();
                        let data_manager = data_manager_mutg.deref_mut();
                        let user_id: &String =
                            &prompt(&"Enter the user id (aka username)".to_string());
                        let description: &String = &prompt(&"Enter the description".to_string());
                        let icon_id: u64 = get_number(&"Enter icon id".to_string());

                        let password: String =
                            prompt_new_password(&"Enter the user's password".to_string());
                        let argon2 = Argon2::default();
                        let password_hash: String = argon2
                            .hash_password(password.as_bytes())
                            .unwrap()
                            .to_string();
                        data_manager.add_user(user_id, &password_hash, description, icon_id);
                    }
                    "remove_user" => {
                        let user_id: String = get_value_in_set(
                            &self.data_manager_user_container,
                            "Enter user id (aka username)",
                        );
                        let mut data_manager_mutg = self.data_manager.lock().unwrap();
                        let data_manager = data_manager_mutg.deref_mut();
                        data_manager.remove_user(&user_id);
                    }
                    "edit_user" => {
                        let user_id: String = get_value_in_set(
                            &self.data_manager_user_container,
                            "Enter user id (aka username)",
                        );
                        loop {
                            match get_menu_item(
                                Some("List of actions"),
                                "Please enter what you want to do",
                                &[
                                    "Change the password",
                                    "Change the description",
                                    "View subscriptions",
                                    "Subscribe to a channel",
                                    "Unsubscribe from a channel",
                                    "View user",
                                    "Make comment",
                                    "Remove comment",
                                    "Quit",
                                ],
                                1,
                            ) {
                                0 => {
                                    let new_password: String = prompt_change_password(
                                        &format!(
                                            "Enter the new password for the user with the user_id \"{user_id}\""
                                        ),
                                        &|password_hash: &String| -> bool {
                                            let data_manager_mutg =
                                                self.data_manager.lock().unwrap();
                                            let data_manager = data_manager_mutg.deref();
                                            let user_arc = data_manager.get_user(&user_id).unwrap();
                                            let user_mutg = user_arc.lock().unwrap();
                                            let user = user_mutg.deref();
                                            user.check_password_hash(password_hash)
                                        },
                                    );
                                    let new_password_hash = Argon2::default()
                                        .hash_password(new_password.as_bytes())
                                        .unwrap()
                                        .to_string();
                                    println!(
                                        "new_password: \"{new_password}\", new_password_hash: \"{new_password_hash}\""
                                    );
                                    let data_manager_mutg = self.data_manager.lock().unwrap();
                                    let data_manager = data_manager_mutg.deref();
                                    let user_arc = data_manager.get_user(&user_id).unwrap();
                                    let mut user_mutg = user_arc.lock().unwrap();
                                    let user = user_mutg.deref_mut();
                                    user.set_password_hash(&new_password_hash);
                                }
                                1 => {
                                    let data_manager_mutg = self.data_manager.lock().unwrap();
                                    let data_manager = data_manager_mutg.deref();
                                    let user_arc = data_manager.get_user(&user_id).unwrap();
                                    let mut user_mutg = user_arc.lock().unwrap();
                                    let user = user_mutg.deref_mut();
                                    user.set_description(&prompt(
                                        &"Enter the new description".to_string(),
                                    ));
                                }
                                2 => {
                                    let data_manager_mutg = self.data_manager.lock().unwrap();
                                    let data_manager = data_manager_mutg.deref();
                                    let user_arc = data_manager.get_user(&user_id).unwrap();
                                    let user_mutg = user_arc.lock().unwrap();
                                    let user = user_mutg.deref();
                                    print!("Subscriptions: ");
                                    {
                                        let mut i = 0;
                                        for subscription in user.get_subscriptions().iter() {
                                            if i > 0 {
                                                print!(", ");
                                            }
                                            print!("{subscription}");
                                            i += 1;
                                        }
                                        println!();
                                    }
                                }
                                3 => {
                                    let channel_user_id: String = get_value_in_set(
                                        &self.data_manager_user_container,
                                        "Enter user id (aka username)",
                                    );

                                    let data_manager_mutg = self.data_manager.lock().unwrap();
                                    let data_manager = data_manager_mutg.deref();
                                    let user_arc = data_manager.get_user(&user_id).unwrap();
                                    let mut user_mutg = user_arc.lock().unwrap();
                                    let user = user_mutg.deref_mut();

                                    user.add_subscription(&channel_user_id);
                                }
                                4 => {
                                    let data_manager_mutg = self.data_manager.lock().unwrap();
                                    let data_manager = data_manager_mutg.deref();
                                    let user_arc = data_manager.get_user(&user_id).unwrap();
                                    let mut user_mutg = user_arc.lock().unwrap();
                                    let user = user_mutg.deref_mut();

                                    let channel_user_id: String =
                                        prompt(&"Enter user id (aka username)".to_string());
                                    user.remove_subscription(&channel_user_id);
                                }
                                5 => {
                                    let data_manager_mutg = self.data_manager.lock().unwrap();
                                    let data_manager = data_manager_mutg.deref();
                                    let user_arc = data_manager.get_user(&user_id).unwrap();
                                    let user_mutg = user_arc.lock().unwrap();
                                    let user = user_mutg.deref();

                                    view_user_obj(user);
                                }
                                6 => {
                                    let video_id: u64 = get_value_in_set(
                                        &self.data_manager_video_data_container,
                                        "Enter the video id of the video you want to comment to",
                                    );
                                    let mut data_manager_mutg = self.data_manager.lock().unwrap();
                                    let data_manager = data_manager_mutg.deref_mut();
                                    let comment_text: String =
                                        prompt(&"Enter the comment text".to_string());
                                    data_manager.make_comment(&user_id, &video_id, &comment_text);
                                }
                                7 => {
                                    let comment_id: u64 = get_value_in_set(
                                        &self.data_manager_comment_container,
                                        "Enter the comment id of the comment you want to remove",
                                    );
                                    let mut data_manager_mutg = self.data_manager.lock().unwrap();
                                    let data_manager = data_manager_mutg.deref_mut();
                                    data_manager.remove_comment(&comment_id);
                                }
                                8 => break,
                                num => todo!("Error: Unimplemented menu option ({num})!"),
                            };
                        }
                    }
                    &_ => println!("Error: \"{input}\" is not a valid command. "),
                };
                println!();
            }
        });
    }
}
