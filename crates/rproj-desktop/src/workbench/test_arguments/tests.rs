use super::{Mode, Row, Session};

#[test]
fn exact_values_and_mode_changes_preserve_dirty_baseline() {
    let mut state = Session::default();
    assert!(!state.dirty());
    state.add();
    assert!(state.dirty());
    state.rows[0].empty = true;
    assert_eq!(state.values().unwrap(), [""]);
    state.baseline = vec![String::new()];
    state.rows[0].switch_mode(Mode::Json).unwrap();
    assert!(!state.dirty());
    state.rows[0].text = "\"保存\\r\\n\\t\\\"\"".into();
    assert_eq!(state.values().unwrap(), ["保存\r\n\t\""]);
    assert!(state.rows[0].switch_mode(Mode::Literal).is_err());
    state.rows[0].text = "\"invalid".into();
    assert!(state.values().is_err());
    assert!(state.dirty());
}

#[test]
fn multiline_paste_preserves_exact_bytes_and_selected_range() {
    let mut row = Row {
        id: 1,
        text: "a保存z".into(),
        mode: Mode::Literal,
        empty: false,
        last_valid: "a保存z".into(),
    };
    assert!(row.paste("x\r\ny\n\tz", 1..7).unwrap());
    assert_eq!(row.value().unwrap(), "ax\r\ny\n\tzz");
    assert_eq!(row.mode, Mode::Json);
    let before = row.text.clone();
    assert!(row.paste("\r\n", 0..0).is_err());
    assert_eq!(row.text, before);
    assert!(row.paste("\0", 0..0).is_err());
}
