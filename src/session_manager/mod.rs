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
                {
                    let mut sessions_mutg = self.sessions.lock().unwrap();
                    let sessions = sessions_mutg.deref_mut();
                    for (user_id, user_sessions_list) in sessions {
                        self.remove_user_expired_sessions(user_sessions_list);
                    }
                }
                //Sleep for 10min:
                let time = time::Duration::from_mins(10);
                thread::sleep(time);
            }
        });
    }

    ///Logs a user in if the user id (user_id) and password (password) are correct.
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

    ///Removes all expired sessions for a user with the user_sessions parameter being a mutable reference to the user's list of sessions.
    pub fn remove_user_expired_sessions(
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
                if !session.is_not_expired() {
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

    ///Checks whether or not a user with the specified user id (user_id) has a valid session with the specified session id (session_id).
    pub fn verify_session(&'static self, user_id: &String, session_id: u64) -> bool {
        //Remove the user's invalid session ids:
        let mut sessions_mutg = self.sessions.lock().unwrap();
        let sessions = sessions_mutg.deref_mut();
        if !sessions.contains_key(user_id) {
            return false;
        }
        self.remove_user_expired_sessions(sessions.get_mut(user_id).unwrap());

        //Check if there is a valid session for the user:
        for session_arc in sessions.get(user_id).unwrap() {
            let session_mutg = session_arc.lock().unwrap();
            let session = session_mutg.deref();
            if session.is_valid(user_id, session_id) {
                return true;
            }
        }

        return false;
    }

    ///Get a DEEP COPY of all of the sessions.
    pub fn get_all_sessions(&'static self) -> HashMap<String, Vec<Session>> {
        let mut result: HashMap<String, Vec<Session>> = HashMap::new();

        let sessions_mutg = self.sessions.lock().unwrap();
        for (user_id, sessions_vec) in sessions_mutg.deref() {
            let mut new_sessions_vec: Vec<Session> = Vec::new();
            for session_arc in sessions_vec.iter() {
                let session_mutg = session_arc.lock().unwrap();
                new_sessions_vec.push(session_mutg.clone());
            }
            result.insert(user_id.clone(), new_sessions_vec);
        }

        return result;
    }
}
