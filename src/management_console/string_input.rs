use std::io;
use std::io::Write;

use num_traits::Num;

use std::collections::{HashSet, LinkedList, HashMap};
use std::hash::Hash;

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

pub fn get_input() -> String {
    let _ = io::stdout().flush();
    let mut input_string: String = String::new();
    let _ = io::stdin().read_line(&mut input_string);
    let result: &str = input_string.trim_end();
    return result.to_string();
}

pub fn get_number_in_range<T: Num + std::str::FromStr + std::cmp::PartialOrd>(min: T, max: T, prompt: &str) -> T {
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

pub fn get_number_in_set<T: Num + std::str::FromStr + Eq + Hash>(list: &impl Container<T>, prompt: &str) -> T {
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
