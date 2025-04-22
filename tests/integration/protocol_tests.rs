use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use keystonelight::protocol::{parse_command, Command};

#[test]
fn test_parse_get_command() {
    let cmd = parse_command("get mykey").unwrap();
    assert!(matches!(cmd, Command::Get(key) if key == "mykey"));
}

#[test]
fn test_parse_set_command_text() {
    let cmd = parse_command("set mykey hello").unwrap();
    assert!(matches!(cmd, Command::Set(key, value) if key == "mykey" && value == b"hello"));
}

#[test]
fn test_parse_set_command_binary() {
    let binary_data = vec![0, 159, 146, 150];
    let encoded = format!("base64:{}", BASE64.encode(&binary_data));
    let cmd = parse_command(&format!("set mykey {}", encoded)).unwrap();
    assert!(matches!(cmd, Command::Set(key, value) if key == "mykey" && value == binary_data));
}

#[test]
fn test_parse_delete_command() {
    let cmd = parse_command("delete mykey").unwrap();
    assert!(matches!(cmd, Command::Delete(key) if key == "mykey"));
}

#[test]
fn test_parse_compact_command() {
    let cmd = parse_command("compact").unwrap();
    assert!(matches!(cmd, Command::Compact));
}

#[test]
fn test_case_insensitive() {
    let cmd = parse_command("GET mykey").unwrap();
    assert!(matches!(cmd, Command::Get(key) if key == "mykey"));
    let cmd = parse_command("SET mykey value").unwrap();
    assert!(matches!(cmd, Command::Set(key, value) if key == "mykey" && value == b"value"));
    let cmd = parse_command("DELETE mykey").unwrap();
    assert!(matches!(cmd, Command::Delete(key) if key == "mykey"));
    let cmd = parse_command("COMPACT").unwrap();
    assert!(matches!(cmd, Command::Compact));
}

#[test]
fn test_command_display() {
    assert_eq!(
        format!("{}", Command::Get("mykey".to_string())),
        "get mykey"
    );
    assert_eq!(
        format!("{}", Command::Set("mykey".to_string(), b"hello".to_vec())),
        "set mykey hello"
    );
    assert_eq!(
        format!(
            "{}",
            Command::Set("mykey".to_string(), vec![0, 159, 146, 150])
        ),
        "set mykey [binary data]"
    );
    assert_eq!(
        format!("{}", Command::Delete("mykey".to_string())),
        "delete mykey"
    );
    assert_eq!(format!("{}", Command::Compact), "compact");
}

#[test]
fn test_invalid_commands() {
    assert!(parse_command("").is_none());
    assert!(parse_command("unknown command").is_none());
    assert!(parse_command("get").is_none());
    assert!(parse_command("get key extra").is_none());
    assert!(parse_command("delete").is_none());
    assert!(parse_command("delete key extra").is_none());
    assert!(parse_command("compact extra").is_none());
}
