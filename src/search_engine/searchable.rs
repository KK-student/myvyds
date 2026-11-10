use std::{collections::HashMap, fmt::Debug};

use posix_regex::PosixRegexBuilder;

use crate::linear_algebra::GenericError;

///Represents an object that can be searched by a search engine.
pub trait Searchable: Send + Sync + Debug {
    ///Set the id used to identify the object in a search engine.
    fn set_searchable_id(&mut self, id: u64);
    ///Get the object's id for the search engine.
    fn get_searchable_id(&self) -> u64;
    ///Get the object's document title.
    fn get_searchable_title(&self) -> String;
    ///Get the object's contents.
    fn get_searchable_contents(&self) -> String;
}
