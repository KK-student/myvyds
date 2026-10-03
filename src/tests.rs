#[cfg(test)]
pub mod test_functions {
    use std::{
        ops::{Deref, DerefMut},
        path::Path,
        sync::{LazyLock, Mutex},
    };

    use argon2::{Algorithm::Argon2id, Argon2, PasswordHasher};
    use rand::{RngExt, distr::uniform::SampleRange};

    use crate::{
        DATA_MANAGER, PROCESSOR_MANAGER, file_handling::video_processing::video_processor::BoolBox,
    };

    use super::*;

    static INIT_STATE: LazyLock<Mutex<BoolBox>> =
        LazyLock::new(|| -> Mutex<BoolBox> { Mutex::new(BoolBox::new(false)) });

    fn initialize_data() {
        if is_data_initialized() {
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

        let mut i_mutg = INIT_STATE.lock().unwrap();
        let i = i_mutg.deref_mut();
        i.set(true);
    }

    fn is_data_initialized() -> bool {
        let i_mutg = INIT_STATE.lock().unwrap();
        i_mutg.deref().get()
    }

    #[test]
    pub fn test_loading() {
        initialize_data();
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
        initialize_data();
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
        data_manager.add_user(&user_id, &password_hash, &description, 0);

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
        println!("All users: ");
        data_manager.for_all_users(
            |user_arc: std::sync::Arc<Mutex<crate::file_handling::user::User>>| -> () {
                let user_mutg = user_arc.lock().unwrap();
                let user = user_mutg.deref();
                println!("{:?}", user);
            },
        );

        assert!(data_manager.has_user(&user_id));
    }
}
