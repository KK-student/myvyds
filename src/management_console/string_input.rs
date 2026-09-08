use std::io;
use std::io::Write;

use num_traits::Num;

use std::collections::{HashMap, HashSet, LinkedList};
use std::hash::Hash;

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
pub fn get_number_in_set<T: Num + std::str::FromStr + Eq + Hash>(
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
    let full_prompt = format!(
        "{prompt}\nEnter your answer (y/n): "
    );
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
