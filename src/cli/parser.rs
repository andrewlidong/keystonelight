/// Parses a command line input into tokens
pub fn parse_input(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut in_quotes = false;
    let mut in_braces = 0;
    let mut escape_next = false;

    for c in input.chars() {
        match c {
            '"' if !escape_next => {
                in_quotes = !in_quotes;
                current_token.push(c);
            }
            '{' if !in_quotes => {
                in_braces += 1;
                current_token.push(c);
            }
            '}' if !in_quotes => {
                in_braces -= 1;
                current_token.push(c);
            }
            '\\' if !escape_next => {
                escape_next = true;
            }
            ' ' if !in_quotes && in_braces == 0 && !escape_next => {
                if !current_token.is_empty() {
                    tokens.push(current_token);
                    current_token = String::new();
                }
            }
            _ => {
                if escape_next {
                    escape_next = false;
                }
                current_token.push(c);
            }
        }
    }

    if !current_token.is_empty() {
        tokens.push(current_token);
    }

    tokens
} 