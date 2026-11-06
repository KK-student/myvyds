use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

use libm::log10;
use rand::RngExt;

use crate::{linear_algebra::MathError, search_engine::searchable::Searchable};

pub mod searchable;

pub struct SearchEngine {
    // documents: Vec<Arc<Mutex<dyn Searchable>>>,
    documents: HashMap<u64, Arc<Mutex<dyn Searchable>>>,
}

impl SearchEngine {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    pub fn add_document(
        &mut self,
        document_arc: Arc<Mutex<dyn Searchable>>,
    ) -> Result<u64, MathError> {
        let id: u64 = {
            let mut rnd = rand::rng();
            rnd.random::<u64>()
        };
        {
            let mut document_mutg = document_arc.lock().unwrap();
            let document = document_mutg.deref_mut();
            document.set_searchable_id(id);
            document.preprocess();
        }
        // self.documents.push(document_arc);
        self.documents.insert(id, document_arc);
        Ok(id)
    }

    pub fn remove_document(&mut self, document_arc: Arc<Mutex<dyn Searchable>>) {
        let id: u64 = {
            let document_mutg = document_arc.lock().unwrap();
            let document = document_mutg.deref();
            document.get_searchable_id()
        };
        self.remove_document_id(id);
    }

    pub fn remove_document_id(&mut self, document_id: u64) {
        self.documents.remove(&document_id);
    }

    pub fn search_combined(&self) -> Vec<Arc<Mutex<dyn Searchable>>> {
        let mut results: Vec<Arc<Mutex<dyn Searchable>>> = Vec::new();

        results
    }
}

pub fn compute_word_idf(documents: &[HashMap<String, f64>], word: &String) -> f64 {
    // let mut result: f64 = 0.0;
    let n = documents.len();

    let mut appears_in_num: usize = 0;
    for document in documents {
        if document.contains_key(word) {
            appears_in_num += 1;
        }
    }

    log10((n as f64) / (appears_in_num as f64))
    // result
}

pub fn compute_idfs(documents: &[HashMap<String, f64>]) -> Vec<HashMap<String, f64>> {
    let mut result: Vec<HashMap<String, f64>> = Vec::new();

    for document in documents {
        let document_res: &mut HashMap<String, f64> = result.push_mut(HashMap::new());
        for word in document.keys() {
            document_res.insert(word.clone(), compute_word_idf(documents, word));
        }
    }

    result
}
