use smol::core::TaskId;
use std::str::FromStr;

/// Test that TaskId::new() always produces an 8-char alphanumeric string.
#[test]
fn test_task_id_new_length() {
    for _ in 0..100 {
        let id = TaskId::new();
        assert_eq!(id.as_str().len(), 8);
        assert!(id.as_str().chars().all(|c| c.is_ascii_alphanumeric()));
    }
}

/// Test that TaskId::from_str rejects empty strings.
#[test]
fn test_task_id_from_str_empty() {
    let result = TaskId::from_str("");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "TaskId must be exactly 8 characters");
}

/// Test that TaskId::from_str rejects strings with special characters.
#[test]
fn test_task_id_from_str_special_chars() {
    let result = TaskId::from_str("abc_def!");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "TaskId must be alphanumeric (base62)");
}

/// Test from_raw produces a valid TaskId (no validation).
#[test]
fn test_task_id_from_raw() {
    let id = TaskId::from_raw("Test1234".to_string());
    assert_eq!(id.as_str(), "Test1234");
    assert_eq!(id.to_string(), "Test1234");
}

/// Test TaskId default produces a random ID.
#[test]
fn test_task_id_default() {
    let id1 = TaskId::default();
    let id2 = TaskId::default();
    assert_eq!(id1.as_str().len(), 8);
    // It's extremely unlikely (but possible) two random IDs match
    assert_ne!(id1.as_str(), id2.as_str());
}

/// Test TaskId ordering by string value.
#[test]
fn test_task_id_ordering() {
    let a = TaskId::from_raw("AAAA0000".to_string());
    let b = TaskId::from_raw("BBBB0000".to_string());
    assert!(a < b);
    assert!(b > a);
}

/// Test TaskId equality.
#[test]
fn test_task_id_equality() {
    let a = TaskId::from_raw("EqId1234".to_string());
    let b = TaskId::from_raw("EqId1234".to_string());
    let c = TaskId::from_raw("Other567".to_string());
    assert_eq!(a, b);
    assert_ne!(a, c);
}

/// Test round-trip from_str/Display.
#[test]
fn test_task_id_roundtrip() {
    let original = "AbCd1234";
    let id = TaskId::from_str(original).unwrap();
    assert_eq!(id.to_string(), original);
    assert_eq!(id.as_str(), original);
}
