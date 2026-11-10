use std::{
    collections::HashMap,
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

use rand::RngExt;
use tantivy::{
    Index, IndexReader, IndexWriter, Searcher, TantivyDocument, Term,
    collector::TopDocs,
    doc,
    query::QueryParser,
    schema::{INDEXED, STORED, Schema, TEXT, Value},
};

use crate::{
    linear_algebra::GenericError, misc::collections::MutableBox,
    search_engine::searchable::Searchable,
};

pub mod searchable;

pub static SEARCH_ENGINE_DEFAULT_MEM_BUDGET: usize = 100_000_000;

pub struct SearchEngine {
    // documents: Vec<Arc<Mutex<dyn Searchable>>>,
    documents: HashMap<u64, Arc<Mutex<dyn Searchable>>>,

    //Fields:
    title_field: tantivy::schema::Field,
    body_field: tantivy::schema::Field,
    id_field: tantivy::schema::Field,

    //Schema:
    schema: Schema,

    //Indexing and writing documents:
    index: Index,
    index_writer: IndexWriter,

    //Searching and reading:
    reader: IndexReader,
    searcher: Searcher,
    query_parser_box: MutableBox<QueryParser>,
}

impl SearchEngine {
    pub fn new(memory_budget: usize) -> Self {
        let mut schema_builder = Schema::builder();
        let title_field = schema_builder.add_text_field("title", TEXT | STORED);
        let body_field = schema_builder.add_text_field("body", TEXT | STORED);
        let id_field = schema_builder.add_u64_field("id", INDEXED | STORED);
        let schema = schema_builder.build();

        let index = Index::create_from_tempdir(schema.clone()).unwrap();
        let index_writer = index.writer(memory_budget).unwrap();

        let reader = index.reader().unwrap();
        let searcher = reader.searcher();

        let mut result = Self {
            documents: HashMap::new(),
            title_field,
            body_field,
            id_field,
            schema,
            index,
            index_writer,
            reader,
            searcher,
            query_parser_box: MutableBox::new(None),
        };
        result.query_parser_box.set(Some(QueryParser::for_index(
            &result.index,
            vec![result.title_field, result.body_field, result.id_field],
        )));
        result.index_writer.commit();
        result
    }

    pub fn add_document(
        &mut self,
        document_arc: Arc<Mutex<dyn Searchable>>,
    ) -> Result<u64, GenericError> {
        let id: u64 = {
            let mut rnd = rand::rng();
            rnd.random::<u64>()
        };

        {
            let document_mutg = document_arc.lock().unwrap();
            let document = document_mutg.deref();
            let prev_id: u64 = document.get_searchable_id();
            if self.has_document(&prev_id) {
                return Err(GenericError::new(format!(
                    "This SearchEngine already has a document with the id of {}",
                    prev_id
                )));
            }
        }

        self.add_document_unchecked(id, document_arc);

        Ok(id)
    }

    fn add_document_unchecked(&mut self, id: u64, document_arc: Arc<Mutex<dyn Searchable>>) {
        {
            let mut document_mutg = document_arc.lock().unwrap();
            let document = document_mutg.deref_mut();
            document.set_searchable_id(id);
            self.index_writer.add_document(doc!(
                self.title_field => document.get_searchable_title(),
                self.body_field => document.get_searchable_contents(),
                self.id_field => document.get_searchable_id()
            ));
        }
        self.documents.insert(id, document_arc);
        self.index_writer.commit();
    }

    pub fn has_document(&self, id: &u64) -> bool {
        self.documents.contains_key(id)
    }

    pub fn remove_document(&mut self, id: &u64) {
        if !self.has_document(id) {
            return;
        }

        self.index_writer
            .delete_term(Term::from_field_u64(self.id_field, *id));
        self.documents.remove(id);
    }

    pub fn update_document(&mut self, document_arc: Arc<Mutex<dyn Searchable>>) {
        let id: u64 = {
            let document_mutg = document_arc.lock().unwrap();
            let document = document_mutg.deref();
            document.get_searchable_id()
        };
        self.remove_document(&id);
        self.add_document_unchecked(id, document_arc);
    }

    pub fn get_document_by_id(&self, id: &u64) -> Option<Arc<Mutex<dyn Searchable>>> {
        match self.documents.get(id) {
            Some(val) => Some(Arc::clone(val)),
            None => return None,
        }
    }

    pub fn search(
        &self,
        query_str: &str,
        max_num_results: usize,
    ) -> Vec<Arc<Mutex<dyn Searchable>>> {
        let mut result: Vec<Arc<Mutex<dyn Searchable>>> = Vec::with_capacity(max_num_results);

        let query_parser = self.query_parser_box.get().unwrap();
        let query = query_parser.parse_query(query_str).unwrap();
        for (_score, doc_address) in self
            .searcher
            .search(
                &query,
                &TopDocs::with_limit(max_num_results).order_by_score(),
            )
            .unwrap()
        {
            let tantivy_document = self.searcher.doc::<TantivyDocument>(doc_address).unwrap();
            let id_value = tantivy_document.get_first(self.id_field).unwrap();
            let id: u64 = id_value.as_u64().unwrap();
            result.push(self.get_document_by_id(&id).unwrap());
        }

        result
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        SearchEngine::new(SEARCH_ENGINE_DEFAULT_MEM_BUDGET)
    }
}

impl Debug for SearchEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchEngine")
            .field("documents", &self.documents)
            .field("title_field", &self.title_field)
            .field("body_field", &self.body_field)
            .field("id_field", &self.id_field)
            .field("schema", &self.schema)
            .field("index", &self.index)
            .field("searcher", &self.searcher)
            .finish()
    }
}
