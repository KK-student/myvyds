use std::io;
use std::io::Write;

use argon2::{Argon2, PasswordHasher};
use num_traits::Num;

use std::collections::{HashMap, HashSet, LinkedList};
use std::hash::Hash;

use crate::file_handling::file_handling::file_stream_reader;

///Represents a set that can be used for the get_number_in_set function.
pub trait Container<T: Eq + Hash> {
    fn contains_generic(&self, value: &T) -> bool;
}

impl<T: Eq + Hash> Container<T> for HashSet<T> {
    fn contains_generic(&self, value: &T) -> bool {
        return self.contains(value);
    }
}

impl<T: Eq + Hash> Container<T> for Vec<T> {
    fn contains_generic(&self, value: &T) -> bool {
        return self.contains(value);
    }
}

impl<T: Eq + Hash> Container<T> for LinkedList<T> {
    fn contains_generic(&self, value: &T) -> bool {
        return self.contains(value);
    }
}

impl<T: Eq + Hash, V> Container<T> for HashMap<T, V> {
    fn contains_generic(&self, value: &T) -> bool {
        return self.contains_key(value);
    }
}

///Get a single line of user input from stdin.
pub fn get_input() -> String {
    let _ = io::stdout().flush();
    let mut input_string: String = String::new();
    let _ = io::stdin().read_line(&mut input_string);
    let result: &str = input_string.trim_end();
    return result.to_string();
}

///Prompts a user for a line of text.
pub fn prompt(prompt: &String) -> String {
    print!("{prompt}: ");
    return get_input();
}

///Prompts a single user for a number within a specified range. Min, max, and the resulting number can be of any type covered by the Num trait and must also implement the std::str::FromStr trait.
pub fn get_number_in_range<T: Num + std::str::FromStr + std::cmp::PartialOrd>(
    min: T,
    max: T,
    prompt: &str,
) -> T {
    return loop {
        break match {
            print!("{prompt}: ");
            get_input().parse::<T>()
        } {
            Ok(num) => {
                if !(num <= max && num >= min) {
                    println!("Invalid input. ");
                    continue;
                } else {
                    num
                }
            }
            Err(_) => {
                println!("Invalid input. ");
                continue;
            }
        };
    };
}

///Prompts a single user for a number that is in the specified set (list). Min, max, and the resulting number can be of any type covered by the Num trait and must also implement the std::str::FromStr trait.
pub fn get_value_in_set<T: std::str::FromStr + Eq + Hash>(
    list: &impl Container<T>,
    prompt: &str,
) -> T {
    return loop {
        break match {
            print!("{prompt}: ");
            get_input().parse::<T>()
        } {
            Ok(num) => {
                if !list.contains_generic(&num) {
                    println!("Invalid input. ");
                    continue;
                } else {
                    num
                }
            }
            Err(_) => {
                println!("Invalid input. ");
                continue;
            }
        };
    };
}

///Prompts the user for a number.
pub fn get_number<T: Num + std::str::FromStr>(prompt: &str) -> T {
    return loop {
        break match {
            print!("{prompt}: ");
            get_input().parse::<T>()
        } {
            Ok(num) => num,
            Err(_) => {
                println!("Invalid input. ");
                continue;
            }
        };
    };
}

///Displays a menu (menu_items) with a specified message to be printed before the menu (before_message), the prompt text (prompt), and number of '\t' characters to be printed at the beginning of each line of the menu (num_indents).
pub fn get_menu_item(
    before_message: Option<&str>,
    prompt: &str,
    menu_items: &[&str],
    num_indents: u8,
) -> usize {
    match before_message {
        Some(val) => println!("{val}: "),
        None => (),
    };
    {
        let mut i = 0;
        while i < menu_items.len() {
            {
                let mut i2 = 0;
                while i2 < num_indents {
                    print!("\t");
                    i2 += 1;
                }
            }
            println!("[{i}]: {}", menu_items[i]);
            i += 1;
        }
    }

    return get_number_in_range(0, menu_items.len(), prompt);
}

///Prompts the user for either a yes or no answer.
pub fn get_yes_or_no(prompt: &str) -> bool {
    let full_prompt = format!("{prompt}\nEnter your answer (y/n): ");
    loop {
        print!("{full_prompt}");
        let input = get_input().to_lowercase();
        if input == "y" || input == "yes" {
            break true;
        } else if input == "n" || input == "no" {
            break false;
        } else {
            println!("Error: Invalid input: \"{input}\". ");
            continue;
        }
    }
}

fn is_file_readable(file_path: &std::path::PathBuf) -> bool {
    match std::fs::File::open(file_path) {
        Ok(_) => true,
        Err(_) => false,
    }
}

fn is_file_existing(file_path: &std::path::PathBuf) -> bool {
    match std::fs::exists(file_path) {
        Ok(val) => val,
        Err(_) => false,
    }
}

fn is_file_writeable(file_path: &std::path::PathBuf) -> bool {
    if !is_file_existing(file_path) {
        return false;
    }
    let file = match std::fs::File::open(file_path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let metadata = match file.metadata() {
        Ok(m) => m,
        Err(_) => return false,
    };
    return !metadata.permissions().readonly();
}

pub fn prompt_file(prompt_str: &str, write_permission: bool) -> std::path::PathBuf {
    let prompt_string: &String = &prompt_str.to_string();
    loop {
        let input: String = prompt(prompt_string);
        let mut result_path = std::path::PathBuf::from(input);
        result_path = match std::fs::canonicalize(result_path) {
            Ok(p) => p,
            Err(_) => {
                println!("Error: Invalid input - cannot be canonicalized. ");
                continue;
            }
        };
        match result_path.to_str() {
            Some(_) => {}
            None => {
                println!("Error: Invalid input - file path is not valid unicode. ");
                continue;
            }
        };
        if !is_file_existing(&result_path)
            || !is_file_readable(&result_path)
            || (write_permission && !is_file_writeable(&result_path))
        {
            println!(
                "Error: Invalid input - the given file either cannot be opened/read or doesn't have the correct permissions. "
            );
            continue;
        }
        return result_path;
    }
}
