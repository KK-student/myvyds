use std::collections::HashMap;

use posix_regex::PosixRegexBuilder;

use crate::linear_algebra::MathError;

pub trait Searchable: Send + Sync {
    fn set_searchable_id(&mut self, id: u64);
    fn get_searchable_id(&self) -> u64;

    fn get_searchable_title(&self) -> String;
    fn get_searchable_contents(&self) -> String;
    fn get_searchable_combined_data(&self) -> String;

    fn compute_title_term_frequencies(&self) -> Result<HashMap<String, f64>, MathError> {
        compute_term_frequencies(self.get_searchable_title().as_str())
    }
    fn compute_contents_term_frequencies(&self) -> Result<HashMap<String, f64>, MathError> {
        compute_term_frequencies(self.get_searchable_contents().as_str())
    }
    fn compute_combined_term_frequencies(&self) -> Result<HashMap<String, f64>, MathError> {
        compute_term_frequencies(self.get_searchable_combined_data().as_str())
    }

    fn store_title_term_frequencies(&mut self, term_freqs: HashMap<String, f64>);
    fn store_contents_term_frequencies(&mut self, term_freqs: HashMap<String, f64>);
    fn store_combined_term_frequencies(&mut self, term_freqs: HashMap<String, f64>);

    fn preprocess_title(&mut self) -> Result<(), MathError> {
        self.store_title_term_frequencies(match self.compute_title_term_frequencies() {
            Ok(res) => res,
            Err(e) => return Err(e),
        });
        Ok(())
    }
    fn preprocess_contents(&mut self) -> Result<(), MathError> {
        self.store_contents_term_frequencies(match self.compute_contents_term_frequencies() {
            Ok(res) => res,
            Err(e) => return Err(e),
        });
        Ok(())
    }
    fn preprocess_combined(&mut self) -> Result<(), MathError> {
        self.store_combined_term_frequencies(match self.compute_combined_term_frequencies() {
            Ok(res) => res,
            Err(e) => return Err(e),
        });
        Ok(())
    }
    fn preprocess(&mut self) -> Result<(), MathError> {
        match self.preprocess_title() {
            Ok(_) => {}
            Err(e) => return Err(e),
        };
        match self.preprocess_contents() {
            Ok(_) => {}
            Err(e) => return Err(e),
        };
        match self.preprocess_combined() {
            Ok(_) => {}
            Err(e) => return Err(e),
        };
        Ok(())
    }

    fn get_title_term_frequencies(&self) -> Result<HashMap<String, f64>, MathError>;
    fn get_content_term_frequencies(&self) -> Result<HashMap<String, f64>, MathError>;
    fn get_combined_term_frequencies(&self) -> Result<HashMap<String, f64>, MathError>;

    fn contains_term(&self, term: &String) -> Result<bool, MathError> {
        match self.title_contains_term(term) {
            Ok(res) => {
                if res {
                    return Ok(true);
                }
            }
            Err(e) => return Err(e),
        };
        match self.contents_contains_term(term) {
            Ok(res) => {
                if res {
                    return Ok(true);
                }
            }
            Err(e) => return Err(e),
        };
        Ok(false)
    }
    fn title_contains_term(&self, term: &String) -> Result<bool, MathError> {
        let term_frequencies: HashMap<String, f64> = match self.get_title_term_frequencies() {
            Ok(tf_map) => tf_map,
            Err(e) => return Err(e),
        };
        Ok(term_frequencies.contains_key(term))
    }
    fn contents_contains_term(&self, term: &String) -> Result<bool, MathError> {
        let term_frequencies: HashMap<String, f64> = match self.get_content_term_frequencies() {
            Ok(tf_map) => tf_map,
            Err(e) => return Err(e),
        };
        Ok(term_frequencies.contains_key(term))
    }

    fn store_document_idfs(&mut self, document_idfs: HashMap<String, f64>);
    fn get_document_idfs(&self) -> Result<HashMap<String, f64>, MathError>;
}

fn materialize_matches(input: &str, matches: &Vec<Box<[Option<(usize, usize)>]>>) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    for ends_opt_box in matches {
        let ends_opt = ends_opt_box.first().unwrap();
        if ends_opt.is_some() {
            let (begin, end) = ends_opt.unwrap();
            result.push(input[begin..end].to_string());
        }
    }

    result
}

pub fn compute_word_counts(input: &str) -> (HashMap<String, u64>, u64) {
    let mut result: HashMap<String, u64> = HashMap::new();
    let mut total_count: u64 = 0;

    //Split text into words:
    let input_regexc = PosixRegexBuilder::new("[a-zA-Z0-9']\\{1,\\}".as_bytes());
    let input_regexm = input_regexc.compile().unwrap();
    let input_matches = input_regexm.matches(input.as_bytes(), None);
    let imatches_strings = materialize_matches(input, &input_matches);

    for word in imatches_strings {
        let new_word: String = word
            .trim_end_matches('\'')
            .trim_start_matches('\'')
            .to_lowercase();

        //Add word to results:
        if !new_word.is_empty() {
            if result.contains_key(&new_word) {
                result.insert(new_word.clone(), result.get(&new_word).unwrap() + 1);
            } else {
                result.insert(new_word, 1);
            }
            total_count += 1;
        }
    }

    (result, total_count)
}

pub fn compute_term_frequencies(input: &str) -> Result<HashMap<String, f64>, MathError> {
    let mut result: HashMap<String, f64> = HashMap::new();

    let (word_counts, total_word_count) = compute_word_counts(input);
    if total_word_count == 0 {
        return Err(MathError::new(
            "No words found in the given document".to_string(),
        ));
    }

    for (word, count) in word_counts {
        result.insert(word, (count as f64) / (total_word_count as f64));
    }

    Ok(result)
}
