use argon2::{Argon2, PasswordHasher};

use crate::management_console::string_input::prompt;

pub fn prompt_current_password(prompt_string: &String) -> String {
    prompt(prompt_string)
}

pub fn prompt_change_password<F: Fn(&String) -> bool>(
    prompt_string: &String,
    check_password_function: &F,
) -> String {
    loop {
        let password_input = prompt_current_password(prompt_string);
        // let argon2 = Argon2::default();
        // let password_hash: String = argon2
        //     .hash_password(password_input.as_bytes())
        //     .unwrap()
        //     .to_string();
        if check_password_function(&password_input) {
            //Get new password
            return prompt_new_password(&"Enter the new password".to_string());
        } else {
            println!("Error: Password check failed!");
            continue;
        }
    }
}

///Prompts the user for a good password
pub fn prompt_new_password(prompt_string: &String) -> String {
    loop {
        let password: String = loop {
            let input: String = prompt(prompt_string);
            if input.len() >= 16 {
                break input;
            } else {
                println!("Error: Bad password. Will not accept this password. ");
            }
        };
        let password_str2: String = prompt(&"Enter password again".to_string());
        if password == password_str2 {
            break password;
        } else {
            println!("Error: Passwords didn't match!");
        }
    }
}
