use std::io::{self, Write};

use kryon::command::commands::Command;
use kryon::command::parser::parse;
use kryon::storage::{Store, Value};

fn execute(store: &mut Store, command: Command) -> String {
    match command {
        Command::Set { key, value } => match store.set(key, Value::String(value)) {
            Ok(()) => "OK".to_string(),
            Err(err) => format!("ERR {}", err),
        },

        Command::Get { key } => match store.get(&key) {
            Some(Value::String(value)) => value.clone(),
            Some(Value::Tombstone) => "(nil)".to_string(),
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

        Command::Expire { key, seconds } => {
            if store.expire(&key, seconds) {
                "1".to_string()
            } else {
                "0".to_string()
            }
        }

        Command::Ttl { key } => store.ttl(&key).to_string(),
    }
}

fn main() {
    let wal_path = std::env::var("KRYON_WAL_PATH").unwrap_or_else(|_| "kryon.wal".to_string());

    let mut store = Store::open(&wal_path).expect("failed to open Kryon WAL");

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
