//! Native-core unit tests (`cargo test`): the RTL conformance corpus and an
//! end-to-end match/interpret smoke test, all without Python.

use crate::interp::{interpret, InterpreterCfg};
use crate::matcher::match_atp;
use crate::rtl::{compile, serialize::serialize, BindingsCore};
use crate::syntax::SyntaxCore;
use std::fs;
use std::path::PathBuf;

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("conformance")
}

fn read(p: &PathBuf) -> String {
    String::from_utf8(fs::read(p).unwrap()).unwrap()
}

#[test]
fn conformance_positive_canonical_and_fixed_point() {
    let bindings = BindingsCore::default();
    let dir = corpus().join("positive");
    let mut n = 0;
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if !name.ends_with(".rtl") || name.ends_with(".expected.rtl") {
            continue;
        }
        let expected_path = dir.join(name.replace(".rtl", ".expected.rtl"));
        let expected = read(&expected_path);
        let expected = expected.trim_end_matches('\n');

        let pattern = compile(&read(&path), &bindings)
            .unwrap_or_else(|e| panic!("{name}: compile failed: {}", e.msg));
        let canonical = serialize(&pattern).unwrap();
        assert_eq!(canonical, expected, "{name}: canonical form mismatch");

        let again = serialize(&compile(expected, &bindings).unwrap()).unwrap();
        assert_eq!(again, expected, "{name}: canonical form is not a fixed point");
        n += 1;
    }
    assert!(n >= 150, "expected at least 150 positive cases, got {n}");
}

#[test]
fn conformance_negative_rejected() {
    let bindings = BindingsCore::default();
    let dir = corpus().join("negative");
    let mut n = 0;
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map(|e| e != "rtl").unwrap_or(true) {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        assert!(
            compile(&read(&path), &bindings).is_err(),
            "{name}: expected a compile error"
        );
        n += 1;
    }
    assert!(n >= 15, "expected at least 15 negative cases, got {n}");
}

#[test]
fn end_to_end_match_and_interpret() {
    let mut syntax = SyntaxCore::new(3, 3).unwrap();
    let texts = [
        (0, 1, "CA"), (0, 2, "HU"),
        (1, 0, "IKT"), (1, 1, "0 Jan"), (1, 2, "8 Feb"),
        (2, 0, "SVO"), (2, 1, "31 Jan"), (2, 2, "40 Feb"),
    ];
    for (r, c, t) in texts {
        syntax.cell_mut(r, c).set_text(t.to_string());
    }

    let pattern = compile(
        "[ [] [VAL : 'AIRLINE'->AVP]+ ]\n\
         [ [VAL : 'AIRPORT'->AVP]\n\
           [VAL : (COL, ROW, CL)->REC, 'ND'->AVP \" \" VAL : 'MON'->AVP]+ ]+",
        &BindingsCore::default(),
    )
    .unwrap();

    let sem = match_atp(&pattern, &mut syntax, Vec::new())
        .unwrap()
        .expect("pattern must match");
    let rs = interpret(&InterpreterCfg::default(), &syntax, &sem, None).unwrap();

    assert_eq!(rs.schema.attributes, vec!["ND", "AIRLINE", "AIRPORT", "MON"]);
    assert_eq!(rs.records.len(), 4);
    assert_eq!(rs.get(0, "ND"), Some("0"));
    assert_eq!(rs.get(3, "MON"), Some("Feb"));
}
