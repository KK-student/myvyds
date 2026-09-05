use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut},
    sync::{Arc, LazyLock, Mutex},
    thread, time,
};

use rand::RngExt;

use crate::{
    file_handling::video_processing::video_processor::BoolBox, session_manager::session::Session,
};

mod session;

pub struct SessionManager<
    F: Fn(&String, &String) -> bool + 'static + std::marker::Send + std::marker::Sync,
> {
    sessions: LazyLock<Mutex<HashMap<String, Vec<Arc<Mutex<Session>>>>>>,
    sessions_by_session_id: LazyLock<Mutex<HashMap<u64, Arc<Mutex<Session>>>>>,

    verify_password_function: &'static F,
}

impl<F> SessionManager<F>
where
    F: Fn(&String, &String) -> bool + std::marker::Send + std::marker::Sync,
{
    pub const fn new(verify_password_function: &'static F) -> Self {
        let result = Self {
            sessions: LazyLock::new(|| -> Mutex<HashMap<String, Vec<Arc<Mutex<Session>>>>> {
                Mutex::new(HashMap::new())
            }),
            sessions_by_session_id: LazyLock::new(
                || -> Mutex<HashMap<u64, Arc<Mutex<Session>>>> {
                    //Return sessions_by_session_id default value:
                    Mutex::new(HashMap::new())
                },
            ),
            verify_password_function: verify_password_function,
        };
        return result;
    }

    pub fn start_autodelete(&'static self) {
        std::thread::spawn(|| -> () {
            loop {
                //Delete all inactive sessions:
                let mut sessions_mutg = self.sessions.lock().unwrap();
                let sessions = sessions_mutg.deref_mut();
                for (user_id, user_sessions_list) in sessions {
                    self.remove_user_invalid_sessions(user_sessions_list);
                }
                //Sleep for 10min:
                let time = time::Duration::from_mins(10);
                thread::sleep(time);
            }
        });
    }

    pub fn login(
        &'static self,
        user_id: &String,
        password: &String,
    ) -> Result<u64, std::io::Error> {
        let verify_password_function = self.verify_password_function;
        if verify_password_function(user_id, password) {
            let mut rng = rand::rng();
            let id: u64 = rng.random::<u64>();
            let session_arc: Arc<Mutex<Session>> =
                Arc::new(Mutex::new(Session::new(id, user_id, 60 * 60 * 12)));

            let mut sessions_mutg = self.sessions.lock().unwrap();
            let sessions: &mut HashMap<String, Vec<Arc<Mutex<Session>>>> =
                sessions_mutg.deref_mut();
            if !sessions.contains_key(user_id) {
                sessions.insert(user_id.clone(), Vec::new());
            }
            let ids_vec: &mut Vec<Arc<Mutex<Session>>> = sessions.get_mut(user_id).unwrap();
            ids_vec.push(Arc::clone(&session_arc));

            let mut sessions_by_session_id_mutg = self.sessions_by_session_id.lock().unwrap();
            let sessions_by_session_id: &mut HashMap<u64, Arc<Mutex<Session>>> =
                sessions_by_session_id_mutg.deref_mut();
            sessions_by_session_id.insert(id, Arc::clone(&session_arc));

            return Ok(id);
        } else {
            let err_str = format!("Either username or password was incorrect",);
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                err_str,
            ));
        }
    }

    pub fn remove_user_invalid_sessions(
        &'static self,
        user_sessions: &mut Vec<Arc<Mutex<Session>>>,
    ) {
        let mut to_remove: Vec<usize> = Vec::new();
        {
            let mut i = 0;
            while i < user_sessions.len() {
                let session_arc = user_sessions.get(i).unwrap();
                let session_mutg = session_arc.lock().unwrap();
                let session = session_mutg.deref();
                if !session.is_valid(&session.get_user_id()) {
                    to_remove.push(i);
                }
                i += 1;
            }
        }
        {
            for i in to_remove {
                user_sessions.remove(i);
            }
        }
    }

    pub fn verify_session(&'static self, user_id: &String, session_id: u64) -> bool {
        //Remove the user's invalid session ids:
        let mut sessions_mutg = self.sessions.lock().unwrap();
        let sessions = sessions_mutg.deref_mut();
        if !sessions.contains_key(user_id) {
            return false;
        }
        self.remove_user_invalid_sessions(sessions.get_mut(user_id).unwrap());

        //Check if there is a valid session for the user:
        for session_arc in sessions.get(user_id).unwrap() {
            let session_mutg = session_arc.lock().unwrap();
            let session = session_mutg.deref();
            if session.is_valid(user_id) {
                return true;
            }
        }

        return false;
    }
}
