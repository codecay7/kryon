mod command;
mod error;
mod storage;

use std::io::{self, Write};

use command::command::Command;
use command::parser::parse;
use storage::{Store, Value};

fn execute(store: &mut Store, command: Command) -> String {
    match command {
        Command::Set { key, value } => match store.set(key, Value::String(value)) {
            Ok(()) => "OK".to_string(),
            Err(err) => format!("ERR {}", err),
        },

        Command::Get { key } => match store.get(&key) {
            Some(Value::String(value)) => value.clone(),
            None => "(nil)".to_string(),
        },

        Command::Del { key } => {
            if store.delete(&key) {
                "1".to_string()
            } else {
                "0".to_string()
            }
        }

        Command::Exists { key } => {
            if store.exists(&key) {
                "1".to_string()
            } else {
                "0".to_string()
            }
        }

        Command::Clear => {
            store.clear();
            "OK".to_string()
        }

        Command::Len => store.len().to_string(),
    }
}

fn main() {
    let mut store = Store::new();

    println!("Kryon RKV");
    println!("Type commands or 'QUIT' to exit.");

    loop {
        print!("kryon> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();

        if io::stdin().read_line(&mut input).is_err() {
            break;
        }

        let input = input.trim();

        if input.eq_ignore_ascii_case("QUIT") {
            println!("Bye!");
            break;
        }

        if input.is_empty() {
            continue;
        }

        for command_input in input.split(';') {
            let command_input = command_input.trim();

            if command_input.is_empty() {
                continue;
            }

            if command_input.eq_ignore_ascii_case("QUIT") {
                println!("Bye!");
                return;
            }

            match parse(command_input) {
                Ok(command) => println!("{}", execute(&mut store, command)),
                Err(error) => println!("ERR {:?}", error),
            }
        }
    }
}
