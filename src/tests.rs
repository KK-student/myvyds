use std::{
    ops::DerefMut,
    path::Path,
    sync::{LazyLock, Mutex},
};

use simple_search_engine_lib::SearchEngine;

use crate::{
    DATA_MANAGER, PROCESSOR_MANAGER, file_handling::video_processing::video_processor::BoolBox,
};

static DATA_INIT_STATE: LazyLock<Mutex<BoolBox>> =
    LazyLock::new(|| -> Mutex<BoolBox> { Mutex::new(BoolBox::new(false)) });

pub static TESTING_DEFAULT_SEARCH_ENGINE: LazyLock<Mutex<SearchEngine>> =
    LazyLock::new(|| -> Mutex<SearchEngine> { Mutex::new(SearchEngine::default()) });

pub static DEFAULT_PASSWORD: &str = "password123456789";

pub trait TestingDefault {
    fn testing_default() -> Self;
}

pub fn initialize_test_data() {
    let mut i_mutg = DATA_INIT_STATE.lock().unwrap();
    let i = i_mutg.deref_mut();
    if i.get() {
        return;
    }

    println!(
        "Current directory: \"{:?}\"",
        std::fs::canonicalize(".").unwrap().to_str().unwrap()
    );

    let data_dir: String = "./data.empty-sample".to_string();

    let mut data_manager_mutg = DATA_MANAGER.lock().unwrap();
    let data_manager = data_manager_mutg.deref_mut();
    data_manager.set_data_path(&data_dir);
    data_manager.load_data();

    PROCESSOR_MANAGER.set_data_dir(Path::new(data_dir.as_str()));
    PROCESSOR_MANAGER.load_data();

    i.set(true);
}

#[cfg(test)]
pub mod test_functions {
    use std::{
        collections::HashMap,
        ops::{Deref, DerefMut},
        sync::{Arc, Mutex},
    };

    use argon2::{Argon2, PasswordHasher};
    use rand::RngExt;
    use simple_search_engine_lib::searchable::Searchable;

    use crate::{
        DATA_MANAGER,
        file_handling::{
            DEFAULT_MAX_SEARCH_RESULTS, document_type_ids::SAMPLE_TEST_DOCUMENT_TYPE_ID, user::User,
        },
    };

    use super::*;

    #[test]
    pub fn test_loading_data() {
        initialize_test_data();
    }

    fn random_text(length: u64) -> String {
        let mut rng = rand::rng();
        let mut result = String::new();
        {
            let mut i = 0;
            while i < length {
                result.push(rng.random_range((' ' as u8)..('~' as u8)) as char);
                i += 1;
            }
        }

        result
    }

    fn add_random_user() -> Result<String, ()> {
        initialize_test_data();
        let mut data_manager_mutg = DATA_MANAGER.lock().unwrap();
        let data_manager = data_manager_mutg.deref_mut();
        let user_id: String = {
            let mut rng = rand::rng();
            loop {
                let val: String = format!("random-user-{}", rng.random::<u64>());
                if !data_manager.has_user(&val) {
                    break val;
                }
            }
        };
        let password_hash: String = {
            let argon2 = Argon2::default();
            let password = random_text(32);
            argon2
                .hash_password(password.as_bytes())
                .unwrap()
                .to_string()
        };
        let description: String = "This is a randomly generated test user. ".to_string();
        data_manager.add_user(&user_id, &password_hash, &description, 0, HashMap::new());

        if data_manager.has_user(&user_id) {
            Ok(user_id)
        } else {
            Err(())
        }
    }

    #[test]
    pub fn add_random_user_test() {
        let user_id = add_random_user().unwrap();

        let data_manager_mutg = DATA_MANAGER.lock().unwrap();
        let data_manager = data_manager_mutg.deref();

        assert!(data_manager.has_user(&user_id));
    }

    #[test]
    pub fn add_then_remove_user_test() {
        let user_id = add_random_user().unwrap();
        let mut data_manager_mutg = DATA_MANAGER.lock().unwrap();
        let data_manager = data_manager_mutg.deref_mut();
        assert!(data_manager.has_user(&user_id));

        data_manager.remove_user(&user_id);
        assert!(!data_manager.has_user(&user_id));
    }

    fn get_all_users() -> Arc<Mutex<Vec<String>>> {
        let result_arc = Arc::new(Mutex::new(Vec::new()));

        let data_manager_mutg = DATA_MANAGER.lock().unwrap();
        let data_manager = data_manager_mutg.deref();
        data_manager.for_all_users(|user_arc: Arc<Mutex<User>>| -> () {
            let user_mutg = user_arc.lock().unwrap();
            let user = user_mutg.deref();

            let mut result_mutg = result_arc.lock().unwrap();
            let result = result_mutg.deref_mut();
            result.push(user.get_user_id());
        });

        result_arc
    }

    #[test]
    pub fn remove_all_users_test() {
        {
            let users_arc = get_all_users();
            let users_mutg = users_arc.lock().unwrap();

            let mut dm_mutg = DATA_MANAGER.lock().unwrap();
            let dm = dm_mutg.deref_mut();
            for user_id in users_mutg.deref() {
                dm.remove_user(user_id);
            }
        }

        assert_eq!(
            {
                let u_arc = get_all_users();
                let u_mutg = u_arc.lock().unwrap();
                u_mutg.deref().len()
            },
            0
        );
    }

    #[derive(Debug)]
    struct SampleTestDocument {
        id: u64,
        title: String,
        contents: String,
    }

    impl SampleTestDocument {
        pub fn new(id: u64, title: String, contents: String) -> Self {
            Self {
                id,
                title,
                contents,
            }
        }
    }

    impl Searchable for SampleTestDocument {
        fn set_searchable_id(&mut self, id: u64) {
            self.id = id;
        }

        fn get_searchable_id(&self) -> u64 {
            self.id
        }

        fn get_searchable_title(&self) -> String {
            self.title.clone()
        }

        fn get_searchable_contents(&self) -> String {
            self.contents.clone()
        }

        fn get_document_type(&self) -> u64 {
            Self::get_class_document_type()
        }

        fn get_class_document_type() -> u64
        where
            Self: Sized,
        {
            SAMPLE_TEST_DOCUMENT_TYPE_ID
        }
    }

    #[test]
    pub fn test_search() {
        let mut search_engine: SearchEngine = SearchEngine::default();

        let mut sample_documents: Vec<Arc<Mutex<SampleTestDocument>>> = Vec::new();
        sample_documents.push(Arc::new(Mutex::new(SampleTestDocument::new(
            0,
            "Lorem ipsum blah blah blah".to_string(),
            "Lorem ipsum blah blah blah".to_string(),
        ))));

        for document_arc in sample_documents {
            search_engine.add_document(document_arc);
        }
        println!("Search engine: \n{:#?}", search_engine);

        let result = search_engine.search("blah", DEFAULT_MAX_SEARCH_RESULTS);
        println!("Results: {:?}", result);
        assert_ne!(result.len(), 0);
    }
}

#[cfg(test)]
pub mod actix_web_test_functions {
    use std::{
        fs::File,
        io::BufReader,
        ops::{Deref, DerefMut},
        process::Command,
        sync::{LazyLock, Mutex},
        thread,
        time::Duration,
    };

    use curl::easy::Easy;
    use nix::libc::MAP_HUGE_64KB;
    use tokio_util::io::StreamReader;

    use crate::{
        file_handling::video_processing::video_processor::BoolBox, get_server_port,
        misc::collections::MutableBox, tests::DEFAULT_PASSWORD,
    };

    static SERVER_INIT_STATE: LazyLock<Mutex<BoolBox>> =
        LazyLock::new(|| -> Mutex<BoolBox> { Mutex::new(BoolBox::new(false)) });

    pub async fn initialize_test_web() {
        //Initialize server:
        let mut init_state_mutg = SERVER_INIT_STATE.lock().unwrap();
        let init_state_box = init_state_mutg.deref_mut();
        if !init_state_box.get() {
            println!("Building server (if needed): ");
            {
                let mut c = Command::new("cargo");
                let build_cmd = c.arg("build");
                let child_processes = build_cmd.spawn().unwrap();
                let output = child_processes.wait_with_output().unwrap();
                println!(
                    "Build output stdout: \n{}",
                    String::from_utf8_lossy(output.stdout.as_slice())
                );
                println!(
                    "Build output stderr: \n{}",
                    String::from_utf8_lossy(output.stderr.as_slice())
                );
            }
            thread::spawn(|| {
                println!("Initializing server: ");
                let mut c = Command::new("cargo");
                let run_cmd = c.args(["run", "--", "--testing_mode"]);
                let mut child_process = run_cmd.spawn().unwrap();
                child_process.wait().unwrap();
            });
            println!("Sleeping for 30s for server to start: ");
            thread::sleep(Duration::from_secs(30));
            println!("Finished sleeping. ");
            init_state_box.set(true);
        }
    }

    // #[ignore]
    // #[actix_web::test]
    // pub async fn test_curl_post() {
    //     initialize_test_web().await;

    //     //Create the easy handle:
    //     let mut easy = Easy::new();

    //     //Create the form:
    //     let mut form = curl::easy::Form::new();
    //     form.part("username")
    //         .contents("sample_test_user_1".as_bytes())
    //         .add()
    //         .unwrap();
    //     form.part("password")
    //         .contents(DEFAULT_PASSWORD.as_bytes())
    //         .add()
    //         .unwrap();

    //     //Set the url:
    //     easy.url("https://posttestserver.dev/p/7fqsqctdr5apu7rn/post")
    //         .unwrap();

    //     //Set POST data:
    //     easy.post(true).unwrap();
    //     // easy.post_fields_copy(form.into()).unwrap();
    //     easy.httppost(form).unwrap();

    //     //Callback function:
    //     easy.write_function(|data| {
    //         println!("Response data: \n{}", String::from_utf8_lossy(data));
    //         Ok(data.len())
    //     })
    //     .unwrap();

    //     //Perform the request:
    //     easy.perform().unwrap();
    // }
    //

    static SU1_SESSION_ID: LazyLock<Mutex<MutableBox<u64>>> =
        LazyLock::new(|| Mutex::new(MutableBox::new(None)));
    static SU2_SESSION_ID_SIGNUP: LazyLock<Mutex<MutableBox<u64>>> =
        LazyLock::new(|| Mutex::new(MutableBox::new(None)));
    static DUMMY_SESSION_ID_SIGNUP: LazyLock<Mutex<MutableBox<u64>>> =
        LazyLock::new(|| Mutex::new(MutableBox::new(None)));

    async fn login<F: Fn()>(
        username: &str,
        password: &str,
        session_id_output: &'static LazyLock<Mutex<MutableBox<u64>>>,
        wait_time_secs: u64,
        after_wait_function: F,
    ) {
        initialize_test_web().await;

        //Create the easy handle:
        let mut easy = Easy::new();

        //Create the form:
        let mut form = curl::easy::Form::new();
        form.part("username")
            .contents(username.as_bytes())
            .add()
            .unwrap();
        form.part("password")
            .contents(password.as_bytes())
            .add()
            .unwrap();

        //Set the url:
        easy.url(format!("http://127.0.0.1:{}/user/login", get_server_port()).as_str())
            .unwrap();

        //Set POST data:
        easy.post(true).unwrap();
        // easy.post_fields_copy(form.into()).unwrap();
        easy.httppost(form).unwrap();

        //Callback function:
        easy.write_function(move |data| {
            println!("Response data: {:?}", data);
            let mut sid_mut = session_id_output.lock().unwrap();
            let mut_box = sid_mut.deref_mut();
            if data.len() != 8 {
                panic!("Invalid response size!");
            }
            let id: u64 = u64::from_be_bytes({
                let mut bytes: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];
                {
                    let mut i = 0;
                    while i < 8 {
                        bytes[i] = data[i];
                        i += 1;
                    }
                }
                bytes
            });
            mut_box.set(Some(id));
            println!("Session id: {id}");
            Ok(data.len())
        })
        .unwrap();

        //Perform the request:
        easy.perform().unwrap();

        //Wait for 15s:
        println!("Sleeping for {}s ...", wait_time_secs);
        thread::sleep(Duration::from_secs(wait_time_secs));
        println!("Finished sleeping. ");
        after_wait_function();
    }

    async fn signup<F: Fn()>(
        username: &str,
        password: &str,
        session_id_output: &'static LazyLock<Mutex<MutableBox<u64>>>,
        wait_time_secs: u64,
        after_wait_function: F,
    ) {
        initialize_test_web().await;

        //Create the easy handle:
        let mut easy = Easy::new();

        //Create the form:
        let mut form = curl::easy::Form::new();
        form.part("username")
            .contents(username.as_bytes())
            .add()
            .unwrap();
        form.part("password")
            .contents(password.as_bytes())
            .add()
            .unwrap();

        //Set the url:
        easy.url(format!("http://127.0.0.1:{}/user/signup", get_server_port()).as_str())
            .unwrap();

        //Set POST data:
        easy.post(true).unwrap();
        easy.httppost(form).unwrap();

        //Callback function:
        easy.write_function(move |data| {
            println!("Response data: {:?}", data);
            let mut sid_mut = session_id_output.lock().unwrap();
            let mut_box = sid_mut.deref_mut();
            if data.len() != 8 {
                panic!("Invalid response size!");
            }
            let id: u64 = u64::from_be_bytes({
                let mut bytes: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];
                {
                    let mut i = 0;
                    while i < 8 {
                        bytes[i] = data[i];
                        i += 1;
                    }
                }
                bytes
            });
            mut_box.set(Some(id));
            println!("Session id: {id}");
            Ok(data.len())
        })
        .unwrap();

        //Perform the request:
        easy.perform().unwrap();

        //Wait for 15s:
        println!("Sleeping for {}s ...", wait_time_secs);
        thread::sleep(Duration::from_secs(wait_time_secs));
        println!("Finished sleeping. ");
        after_wait_function();
    }

    fn wait_for_session_id(
        sid_lock: &'static LazyLock<Mutex<MutableBox<u64>>>,
        on_some_func: impl Fn(u64),
    ) -> impl Fn() {
        move || -> () {
            let sid_mutg = sid_lock.lock().unwrap();
            let sid_box = sid_mutg.deref();
            match sid_box.get() {
                Some(val) => {
                    on_some_func(val);
                }
                None => {
                    panic!(
                        "Either write function did not execute or took too long for response to arrive!"
                    )
                }
            }
        }
    }

    #[actix_web::test]
    pub async fn test_signup() {
        signup(
            "sample_user_2",
            DEFAULT_PASSWORD,
            &SU2_SESSION_ID_SIGNUP,
            15,
            wait_for_session_id(&SU2_SESSION_ID_SIGNUP, |_val: u64| -> () {}),
        )
        .await;
        login(
            "sample_user_2",
            DEFAULT_PASSWORD,
            &SU2_SESSION_ID_SIGNUP,
            15,
            wait_for_session_id(&SU2_SESSION_ID_SIGNUP, |_val: u64| -> () {}),
        )
        .await;
    }

    #[actix_web::test]
    pub async fn test_login() {
        login(
            "sample_user_1",
            DEFAULT_PASSWORD,
            &SU1_SESSION_ID,
            15,
            wait_for_session_id(&SU1_SESSION_ID, |_val: u64| -> () {}),
        )
        .await;
    }

    fn is_main_logged_in() -> bool {
        let su1_smutg = SU1_SESSION_ID.lock().unwrap();
        let su1 = su1_smutg.deref();
        su1.get().is_some()
    }

    #[actix_web::test]
    pub async fn test_upload_mp4() {
        //Log in if needed:
        if !is_main_logged_in() {
            login(
                "sample_user_1",
                DEFAULT_PASSWORD,
                &SU1_SESSION_ID,
                15,
                || {},
            )
            .await;
        }
        let user_1s: u64 = {
            let su1_mutg = SU1_SESSION_ID.lock().unwrap();
            su1_mutg.deref().unwrap()
        };

        //Load data:
        let mut data_reader =
            BufReader::new(File::open("./testing_videos/sample-10s-720p.mp4").unwrap());
    }
}
