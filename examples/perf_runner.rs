//! Phase-timed CLI runner over the pure-Rust core (no Python), for profiling
//! large tables: `cargo run --release --no-default-features --example
//! perf_runner -- <pattern.rtl> <input.csv> [output.csv] [--repeat N]`.
//!
//! Mirrors `python -m pyregtab.runner`: RFC 4180 input (universal newlines),
//! `RECORD_FIRST`, every output field double-quoted, LF. Prints the median
//! of each phase over `--repeat` runs (default 1) to stderr.

use pyregtab::csv::{parse_csv, universal_newlines};
use pyregtab::interp::{interpret_timed, InterpreterCfg};
use pyregtab::matcher::match_atp;
use pyregtab::recordset::RecordsetCore;
use pyregtab::rtl::{compile, BindingsCore};
use pyregtab::syntax::SyntaxCore;
use pyregtab::util::Text;
use std::io::Write;
use std::time::Instant;

fn write_csv(path: &str, rs: &RecordsetCore) {
    let mut out = std::io::BufWriter::with_capacity(1 << 20, std::fs::File::create(path).unwrap());
    rs.write_csv(&mut out, ",", "", true, "
").unwrap();
    out.flush().unwrap();
}

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut repeat = 1usize;
    let mut pos: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--repeat" {
            repeat = args[i + 1].parse().unwrap();
            i += 2;
        } else {
            pos.push(&args[i]);
            i += 1;
        }
    }
    let (rtl_path, csv_path) = (pos[0], pos[1]);
    let out_path = pos.get(2).copied();
    let rtl = std::fs::read_to_string(rtl_path).unwrap();
    let bytes = std::fs::read(csv_path).unwrap();

    let names = ["read", "compile", "match", "ws-init", "ws-complete", "extract", "transforms", "interp-total", "write"];
    let mut times: Vec<Vec<f64>> = vec![Vec::new(); names.len()];
    let mut summary = String::new();
    for _ in 0..repeat {
        let t = Instant::now();
        let text = String::from_utf8(bytes.clone()).unwrap();
        let text = universal_newlines(&text);
        let mut syntax = SyntaxCore::from_rows(parse_csv::<Text>(&text), None).unwrap();
        times[0].push(t.elapsed().as_secs_f64());

        let t = Instant::now();
        let pattern = compile(&rtl, &BindingsCore::default()).unwrap_or_else(|e| panic!("{}", e.msg));
        times[1].push(t.elapsed().as_secs_f64());

        let t = Instant::now();
        let sem = match_atp(&pattern, &mut syntax, Vec::new()).unwrap().expect("no match");
        times[2].push(t.elapsed().as_secs_f64());

        let cfg = InterpreterCfg::default();
        let t = Instant::now();
        let (out, phases) = interpret_timed(&cfg, &syntax, &sem, None).unwrap();
        let t_interp = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let rs = pattern.transform(out.recordset).unwrap();
        let t_pattern = t.elapsed().as_secs_f64();
        times[3].push(phases[0]);
        times[4].push(phases[1]);
        times[5].push(phases[2]);
        times[6].push(phases[3] + t_pattern);
        times[7].push(t_interp);

        let t = Instant::now();
        if let Some(p) = out_path {
            write_csv(p, &rs);
        }
        times[8].push(t.elapsed().as_secs_f64());
        summary = format!(
            "rows={} cols={} items={} actions={} out_rows={} out_cols={}",
            syntax.num_rows,
            syntax.num_cols,
            sem.cell_items.len(),
            sem.actions.len(),
            rs.records.len(),
            rs.schema.attributes.len()
        );
    }
    eprintln!("{summary}");
    for (n, v) in names.iter().zip(times.iter_mut()) {
        eprintln!("{n:>14}: {:.3} s", median(v));
    }
}
