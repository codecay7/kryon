use crate::command::command::Command;

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    EmptyCommand,
    UnknownCommand,
    InvalidArguments,
}

pub fn parse(input: &str) -> Result<Command, ParseError> {
    let parts: Vec<&str> = input.split_whitespace().collect();

    if parts.is_empty() {
        return Err(ParseError::EmptyCommand);
    }

    match parts[0].to_uppercase().as_str() {
        "SET" if parts.len() >= 3 => Ok(Command::Set {
            key: parts[1].to_string(),
            value: parts[2..].join(" "),
        }),

        "GET" if parts.len() == 2 => Ok(Command::Get {
            key: parts[1].to_string(),
        }),

        "DEL" if parts.len() == 2 => Ok(Command::Del {
            key: parts[1].to_string(),
        }),

        "EXISTS" if parts.len() == 2 => Ok(Command::Exists {
            key: parts[1].to_string(),
        }),

        "CLEAR" if parts.len() == 1 => Ok(Command::Clear),

        "LEN" if parts.len() == 1 => Ok(Command::Len),

        "EXPIRE" if parts.len() == 3 => Ok(Command::Expire {
            key: parts[1].to_string(),
            seconds: parts[2].parse().map_err(|_| ParseError::InvalidArguments)?,
        }),

        "TTL" if parts.len() == 2 => Ok(Command::Ttl {
            key: parts[1].to_string(),
        }),

        "SET" | "GET" | "DEL" | "EXISTS" | "CLEAR" | "LEN" | "EXPIRE" | "TTL" => {
            Err(ParseError::InvalidArguments)
        }

        _ => Err(ParseError::UnknownCommand),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_set() {
        assert_eq!(
            parse("SET name Kryon"),
            Ok(Command::Set {
                key: "name".into(),
                value: "Kryon".into()
            })
        );
    }

    #[test]
    fn parse_set_with_spaces() {
        assert_eq!(
            parse("SET message Hello Kryon"),
            Ok(Command::Set {
                key: "message".into(),
                value: "Hello Kryon".into()
            })
        );
    }

    #[test]
    fn parse_get() {
        assert_eq!(parse("GET name"), Ok(Command::Get { key: "name".into() }));
    }

    #[test]
    fn parse_del() {
        assert_eq!(parse("DEL name"), Ok(Command::Del { key: "name".into() }));
    }

    #[test]
    fn parse_exists() {
        assert_eq!(
            parse("EXISTS name"),
            Ok(Command::Exists { key: "name".into() })
        );
    }

    #[test]
    fn invalid_command() {
        assert_eq!(parse("UNKNOWN name"), Err(ParseError::UnknownCommand));
    }

    #[test]
    fn invalid_arguments() {
        assert_eq!(parse("GET"), Err(ParseError::InvalidArguments));
    }
}
