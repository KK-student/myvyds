use std::{
    collections::HashSet,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
    thread,
};

use argon2::{Argon2, PasswordHasher};

use crate::{
    ThumbnailImageProccessorType,
    file_handling::{
        DataManager,
        comment::Comment,
        likeable::Likeable,
        user::User,
        video_data::VideoData,
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
    session_manager::SessionManager,
    time::get_current_time,
};

struct VideoProcessorManagerContainer {
    vpm: &'static VideoProcessorManager<fn(u64) -> (), ThumbnailImageProccessorType>,
}

impl Container<u64> for VideoProcessorManagerContainer {
    fn contains_generic(&self, value: &u64) -> bool {
        self.vpm.has_processor(value)
    }
}

pub struct DataManagerUserContainer {
    pub dm: &'static std::sync::LazyLock<Mutex<DataManager>>,
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
pub struct TextBasedManagementConsole<
    F: Fn(&String, &String) -> bool + std::marker::Send + std::marker::Sync + 'static,
> {
    data_manager: &'static std::sync::LazyLock<Mutex<DataManager>>,
    data_manager_user_container: DataManagerUserContainer,
    data_manager_video_data_container: DataManagerVideoDataContainer,
    data_manager_comment_container: DataManagerCommentContainer,
    processor_manager: &'static VideoProcessorManager<fn(u64) -> (), ThumbnailImageProccessorType>,
    processor_manager_container: VideoProcessorManagerContainer,

    session_manager: &'static SessionManager<F>,
}

impl<F: Fn(&String, &String) -> bool + std::marker::Send + std::marker::Sync + 'static>
    TextBasedManagementConsole<F>
{
    pub const fn new(
        data_manager: &'static std::sync::LazyLock<Mutex<DataManager>>,
        processor_manager: &'static VideoProcessorManager<
            fn(u64) -> (),
            ThumbnailImageProccessorType,
        >,
        session_manager: &'static SessionManager<F>,
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
            session_manager,
        };
    }
}

impl<F: Fn(&String, &String) -> bool + std::marker::Send + std::marker::Sync + 'static>
    ManagementConsole for TextBasedManagementConsole<F>
{
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

Managing comment: 
vcomment -> View a comment. 

Managing video data: 
vvideos -> Show a list of all videos. 
vvideo -> Show the basic information for a video. 
add_video -> Add a new video. 
remove_video -> Remove a video. 
edit_video -> Edit a video. 

Session management: 
vsessions -> Show a list of all active sessions (and when they expire). 

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
\tcomments: {:?}
\tliked comments: {:?}
\tdisliked comments: {:?}",
                        user_obj.get_user_id(),
                        user_obj.get_password_hash(),
                        user_obj.get_description(),
                        user_obj.get_videos(),
                        user_obj.get_subscriptions(),
                        user_obj.get_liked_vids(),
                        user_obj.get_disliked_vids(),
                        user_obj.get_comments(),
                        user_obj.get_liked_comments(),
                        user_obj.get_disliked_comments()
                    );
                }
                fn view_video_data_obj(video_data_obj: &VideoData) {
                    let comment_ids_arc: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(Vec::new()));
                    video_data_obj.for_each_comment_recursive(&|comment_arc: Arc<
                        Mutex<Comment>,
                    >|
                     -> () {
                        let comment_mutg = comment_arc.lock().unwrap();
                        let comment: &Comment = comment_mutg.deref();

                        let mut comment_ids_mutg = comment_ids_arc.lock().unwrap();
                        let comment_ids: &mut Vec<u64> = comment_ids_mutg.deref_mut();
                        comment_ids.push(comment.get_id());
                    });
                    let comment_ids_mutg = comment_ids_arc.lock().unwrap();
                    let comment_ids: &Vec<u64> = comment_ids_mutg.deref();
                    println!(
                        "id \"{}\":
\tauthor: \"{}\"
\tdescription: \"{}\"
\tnumber of likes: \"{}\"
\tnumber of dislikes: \"{}\"
\tnumber of comments: \"{}\"
\tcomment ids: \"{:?}\"
\tname: \"{}\"",
                        video_data_obj.get_id(),
                        video_data_obj.get_author(),
                        video_data_obj.get_description(),
                        video_data_obj.get_likes(),
                        video_data_obj.get_dislikes(),
                        video_data_obj.get_num_comments(),
                        comment_ids,
                        video_data_obj.get_name(),
                    );
                }
                fn view_comment_obj(comment_obj: &Comment) {
                    println!(
                        "id {}: 
\tauthor: \"{}\"
\ttext: \"{}\"
\ttime created (in seconds since epoch): {}
\tedited: {}
\tlikes: {}
\tdislikes: {}",
                        comment_obj.get_id(),
                        comment_obj.get_author(),
                        comment_obj.get_text(),
                        comment_obj.get_creation_time(),
                        comment_obj.is_edited(),
                        comment_obj.get_likes(),
                        comment_obj.get_dislikes(),
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
                                    "Edit comment",
                                    "Like comment",
                                    "Remove like for comment",
                                    "Dislike comment",
                                    "Remove dislike for comment",
                                    "Quit",
                                ],
                                1,
                            ) {
                                0 => {
                                    //Set password:
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
                                    //Set description:
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
                                    //View subscriptions:
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
                                    //Subscribe:
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
                                    //Unsubscribe:
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
                                    //View user:
                                    let data_manager_mutg = self.data_manager.lock().unwrap();
                                    let data_manager = data_manager_mutg.deref();
                                    let user_arc = data_manager.get_user(&user_id).unwrap();
                                    let user_mutg = user_arc.lock().unwrap();
                                    let user = user_mutg.deref();

                                    view_user_obj(user);
                                }
                                6 => {
                                    //Add comment:
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
                                    //Remove comment:
                                    let comment_id: u64 = get_value_in_set(
                                        &self.data_manager_comment_container,
                                        "Enter the comment id of the comment you want to remove",
                                    );
                                    let mut data_manager_mutg = self.data_manager.lock().unwrap();
                                    let data_manager = data_manager_mutg.deref_mut();
                                    data_manager.remove_comment(&comment_id);
                                }
                                8 => {
                                    //Edit comment:
                                    let comment_id: u64 = get_value_in_set(
                                        &self.data_manager_comment_container,
                                        "Enter the comment id of the comment you want to edit",
                                    );
                                    let text: String =
                                        prompt(&"Enter the new text of the comment".to_string());
                                    let mut data_manager_mutg = self.data_manager.lock().unwrap();
                                    let data_manager = data_manager_mutg.deref_mut();
                                    let comment_arc = match data_manager.get_comment(&comment_id) {
                                        Some(c_arc) => c_arc,
                                        None => {
                                            println!("Error: Invalid comment id: {comment_id}");
                                            continue;
                                        }
                                    };
                                    let mut comment_mutg = comment_arc.lock().unwrap();
                                    let comment: &mut Comment = comment_mutg.deref_mut();
                                    comment.set_text(&text, get_current_time());
                                }
                                9 => {
                                    //Like comment:
                                    let comment_id: u64 = get_value_in_set(
                                        &self.data_manager_comment_container,
                                        "Enter the comment id of the comment you want to like",
                                    );
                                    let mut data_manager_mutg = self.data_manager.lock().unwrap();
                                    let data_manager = data_manager_mutg.deref_mut();
                                    data_manager.like_comment(&user_id, &comment_id);
                                }
                                10 => {
                                    //Unlike comment:
                                    let comment_id: u64 = get_value_in_set(
                                        &self.data_manager_comment_container,
                                        "Enter the comment id of the comment you want to remove a like for",
                                    );
                                    let mut data_manager_mutg = self.data_manager.lock().unwrap();
                                    let data_manager = data_manager_mutg.deref_mut();
                                    data_manager.unlike_comment(&user_id, &comment_id);
                                }
                                11 => {
                                    //Dislike comment:
                                    let comment_id: u64 = get_value_in_set(
                                        &self.data_manager_comment_container,
                                        "Enter the comment id of the comment you want to dislike",
                                    );
                                    let mut data_manager_mutg = self.data_manager.lock().unwrap();
                                    let data_manager = data_manager_mutg.deref_mut();
                                    data_manager.dislike_comment(&user_id, &comment_id);
                                }
                                12 => {
                                    //Undislike comment:
                                    let comment_id: u64 = get_value_in_set(
                                        &self.data_manager_comment_container,
                                        "Enter the comment id of the comment you want to remove a dislike for",
                                    );
                                    let mut data_manager_mutg = self.data_manager.lock().unwrap();
                                    let data_manager = data_manager_mutg.deref_mut();
                                    data_manager.undislike_comment(&user_id, &comment_id);
                                }
                                13 => break,
                                num => todo!("Error: Unimplemented menu option ({num})!"),
                            };
                        }
                    }
                    "vcomment" => {
                        //View a comment:
                        let comment_id: u64 = get_value_in_set(
                            &self.data_manager_comment_container,
                            "Enter the comment id of the comment you want to view",
                        );
                        let data_manager_mutg = self.data_manager.lock().unwrap();
                        let data_manager = data_manager_mutg.deref();
                        let comment_arc = data_manager.get_comment(&comment_id).unwrap();
                        let comment_mutg = comment_arc.lock().unwrap();
                        view_comment_obj(comment_mutg.deref());
                    }
                    "vvideos" => {
                        //View all video data:
                        let data_manager_mutg = self.data_manager.lock().unwrap();
                        let data_manager = data_manager_mutg.deref();
                        data_manager.for_all_videos(
                            |video_data_arc: Arc<Mutex<VideoData>>| -> () {
                                let video_data_mutg = video_data_arc.lock().unwrap();
                                let video_data: &VideoData = video_data_mutg.deref();
                                view_video_data_obj(video_data);
                            },
                        );
                    }
                    "vvideo" => {
                        let video_id: u64 = get_value_in_set::<u64>(
                            &self.data_manager_video_data_container,
                            "Enter the video id",
                        );
                        let data_manager_mutg = self.data_manager.lock().unwrap();
                        let data_manager = data_manager_mutg.deref();
                        let video_arc = data_manager.get_video(&video_id).unwrap();
                        let video_mutg = video_arc.lock().unwrap();
                        let video: &VideoData = video_mutg.deref();
                        view_video_data_obj(video);
                    }
                    "add_video" => {
                        //Add video:
                        let name: String = prompt(&"Enter the name of the new video".to_string());
                        let author_user_id: String = get_value_in_set(
                            &self.data_manager_user_container,
                            "Enter the user_id (aka username) of the author",
                        );
                        let description: String =
                            prompt(&"Enter the description of the new video".to_string());
                        let input_file: std::path::PathBuf = prompt_file(
                            "Enter the file path of the video input file (for processing)",
                            false,
                        );
                        let video_id: u64 = match {
                            let mut data_manager_mutg = self.data_manager.lock().unwrap();
                            let data_manager = data_manager_mutg.deref_mut();
                            data_manager.add_video(
                                &name,
                                &author_user_id,
                                &description,
                                &input_file,
                            )
                        } {
                            Some(id) => id,
                            None => {
                                println!("Error: Failed to add video. ");
                                continue;
                            }
                        };

                        //Process video:
                        thread::spawn(move || -> () {
                            let result_directory: String = {
                                let data_manager_mutg = self.data_manager.lock().unwrap();
                                let data_manager = data_manager_mutg.deref();
                                data_manager
                                    .get_video_dir(&video_id)
                                    .to_str()
                                    .unwrap()
                                    .to_string()
                            };
                            self.processor_manager.process(
                                video_id,
                                &input_file.to_str().unwrap().to_string(),
                                &result_directory,
                            );
                        });
                    }
                    "vsessions" => {
                        let sessions_map = self.session_manager.get_all_sessions();
                        for (user_id, user_sessions_list) in sessions_map {
                            println!("user_id \"{}\": ", user_id);
                            for session in user_sessions_list {
                                println!("\tid: {}", session.get_id());
                                println!(
                                    "\t\ttime created (in seconds since epoch): {}",
                                    session.get_time_created()
                                );
                                println!(
                                    "\t\texpiry time (in seconds since epoch): {}",
                                    session.get_time_created() + session.get_valid_time_length()
                                );
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
