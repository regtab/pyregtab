//! Phase-timed CLI runner over the pure-Rust core (no Python), for profiling
//! large tables: `cargo run --release --no-default-features --example
//! perf_runner -- <pattern.rtl> <input.csv> [output.csv] [--repeat N]
//! [--threads N] [--jobs-per-thread N] [--profile]`.
//!
//! Mirrors `python -m pyregtab.runner`: RFC 4180 input (universal newlines),
//! `RECORD_FIRST`, every output field double-quoted, LF. Prints the median
//! of each phase over `--repeat` runs (default 1) to stderr. Synthetic modes:
//! `--synthetic N [--shared-values]` and `--synthetic-join N` bypass CSV/RTL and
//! exercise large independent recordsets directly.

use pyregtab::csv::{parse_csv, universal_newlines};
use pyregtab::interp::{interpret, interpret_profile, interpret_timed, InterpreterCfg};
use pyregtab::matcher::match_atp;
use pyregtab::recordset::RecordsetCore;
use pyregtab::rtl::{compile, BindingsCore};
use pyregtab::semantics::{ActionInst, ActionTemplate, CandidateScope, CellItem, CtxItem, Interval, ItemId, OpInst, ProviderInst, SemanticsCore};
use pyregtab::spec::{CellKind, CtxKind, FilterCond, FilterTerm, ItemType, RecordKey, TraversalOrder, UNBOUNDED};
use pyregtab::syntax::SyntaxCore;
use pyregtab::util::Text;
use std::io::Write;
use std::sync::Arc;
use std::time::Instant;

fn write_csv(path: &str, rs: &RecordsetCore) {
    let mut out = std::io::BufWriter::with_capacity(1 << 20, std::fs::File::create(path).unwrap());
    rs.write_csv(&mut out, ",", "", true, "
", true).unwrap();
    out.flush().unwrap();
}

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

#[derive(Clone, Copy)]
struct SyntheticOptions {
    threads: usize,
    jobs_per_thread: usize,
    repeat: usize,
    detailed: bool,
}

fn run_synthetic_core(
    label: &str,
    syntax: SyntaxCore,
    sem: SemanticsCore,
    expected_records: usize,
    options: SyntheticOptions,
) {
    let cfg = InterpreterCfg {
        num_threads: options.threads,
        jobs_per_thread: options.jobs_per_thread,
        ..Default::default()
    };
    for _ in 0..options.repeat {
        let started = Instant::now();
        let (out, profile) = if options.detailed {
            let (out, profile) = interpret_profile(&cfg, &syntax, &sem, None).unwrap();
            (out, Some(profile))
        } else {
            (interpret(&cfg, &syntax, &sem, None).unwrap(), None)
        };
        assert_eq!(out.recordset.len(), expected_records, "diagnostics: {:?}", out.diagnostics);
        assert!(out.diagnostics.is_empty());
        eprintln!("{label}: threads={} records={} total={:.4} s", options.threads, out.recordset.len(), started.elapsed().as_secs_f64());
        if let Some(profile) = profile {
            eprintln!(
                "synthetic-profile: pool={:.4} index={:.4} consistency={:.4} generate={:.4} work={:.4} init={:.4} fill={:.4} jobs={}",
                profile.pool_creation,
                profile.item_index,
                profile.consistency,
                profile.record_generation,
                profile.record_work,
                profile.record_initialization,
                profile.record_fill,
                profile.jobs,
            );
        }
    }
}

fn synthetic_values(n: usize, shared_values: bool, threads: usize, jobs_per_thread: usize, repeat: usize, detailed: bool) {
    let syntax = SyntaxCore::new(n, 1).unwrap();
    let mut sem = SemanticsCore::default();
    sem.ctx_items.push(CtxItem { s: Text::from("v"), ty: ItemType::Attribute, const_value: None });
    let shared = Text::from("shared");
    let avp = Arc::new(ActionTemplate {
        providers: vec![ProviderInst::Ctx { items: vec![0], kind: CtxKind::Attr }],
        op: OpInst::Avp,
        inherited: false,
    });
    let rec = Arc::new(ActionTemplate { providers: Vec::new(), op: OpInst::Rec, inherited: false });
    for i in 0..n {
        let s = if shared_values { shared.clone() } else { Text::from(format!("value-{i}")) };
        let len = s.len() as u32;
        sem.cell_items.push(CellItem {
            s,
            tags: Vec::new(),
            index: 0,
            row: i,
            col: 0,
            ty: ItemType::Value,
            span: (0, len),
        });
        sem.actions.push(ActionInst::new(ItemId::Cell(i), avp.clone()));
        sem.actions.push(ActionInst::new(ItemId::Cell(i), rec.clone()));
    }
    let label = if shared_values { "synthetic-shared-values" } else { "synthetic-unique-values" };
    run_synthetic_core(label, syntax, sem, n, SyntheticOptions { threads, jobs_per_thread, repeat, detailed });
}

fn synthetic_join(n: usize, threads: usize, jobs_per_thread: usize, repeat: usize, detailed: bool) {
    let total_items = 3 + 2 * n;
    let syntax = SyntaxCore::new(total_items, 1).unwrap();
    let mut sem = SemanticsCore::default();
    let rec = Arc::new(ActionTemplate { providers: Vec::new(), op: OpInst::Rec, inherited: false });
    for row in 0..total_items {
        let text = match row {
            0 => "A".to_string(),
            1 => "B".to_string(),
            2 => "C".to_string(),
            _ => format!("v{row}"),
        };
        sem.cell_items.push(CellItem {
            s: Text::from(text),
            tags: Vec::new(),
            index: 0,
            row,
            col: 0,
            ty: ItemType::Value,
            span: (0, 1),
        });
        sem.actions.push(ActionInst::new(ItemId::Cell(row), rec.clone()));
    }
    let mut join = |anchor: usize, lo: usize, hi: usize| {
        let provider = ProviderInst::Cell {
            cond: FilterCond::Bare(FilterTerm::RowAbsoluteRange(lo as i64, hi as i64)),
            order: TraversalOrder::RowMajor,
            cardinality: UNBOUNDED,
            kind: CellKind::Unrestricted,
            exclude_anchor: true,
            lenient: false,
            scope: CandidateScope::rows(Interval::absolute(lo as i64, hi as i64)),
        };
        let template = Arc::new(ActionTemplate {
            providers: vec![provider],
            op: OpInst::Join(RecordKey::empty()),
            inherited: false,
        });
        sem.actions.push(ActionInst::new(ItemId::Cell(anchor), template));
    };
    join(0, 3, n + 2);
    join(1, n + 3, 2 * n + 2);
    join(2, 0, 0);
    join(2, 1, 1);
    let expected = n.checked_mul(n).expect("synthetic JOIN size overflow");
    run_synthetic_core("synthetic-join", syntax, sem, expected, SyntheticOptions { threads, jobs_per_thread, repeat, detailed });
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut repeat = 1usize;
    let mut threads = 1usize;
    let mut jobs_per_thread = 4usize;
    let mut detailed = false;
    let mut synthetic = None;
    let mut synthetic_join_size = None;
    let mut shared_values = false;
    let mut parallel_record_generation = true;
    let mut parallel_item_index = true;
    let mut parallel_consistency = true;
    let mut parallel_action_sorting = true;
    let mut pos: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--repeat" => {
                repeat = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--threads" => {
                threads = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--jobs-per-thread" => {
                jobs_per_thread = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--profile" => {
                detailed = true;
                i += 1;
            }
            "--synthetic" => {
                synthetic = Some(args[i + 1].parse().unwrap());
                i += 2;
            }
            "--synthetic-join" => {
                synthetic_join_size = Some(args[i + 1].parse().unwrap());
                i += 2;
            }
            "--shared-values" => {
                shared_values = true;
                i += 1;
            }
            "--no-records" => {
                parallel_record_generation = false;
                i += 1;
            }
            "--no-index" => {
                parallel_item_index = false;
                i += 1;
            }
            "--no-consistency" => {
                parallel_consistency = false;
                i += 1;
            }
            "--no-action-sorting" => {
                parallel_action_sorting = false;
                i += 1;
            }
            _ => {
                pos.push(&args[i]);
                i += 1;
            }
        }
    }
    if let Some(n) = synthetic {
        synthetic_values(n, shared_values, threads, jobs_per_thread, repeat, detailed);
        return;
    }
    if let Some(n) = synthetic_join_size {
        synthetic_join(n, threads, jobs_per_thread, repeat, detailed);
        return;
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

        let cfg = InterpreterCfg {
            num_threads: threads,
            jobs_per_thread,
            parallel_record_generation,
            parallel_item_index,
            parallel_consistency,
            parallel_action_sorting,
            ..Default::default()
        };
        let t = Instant::now();
        let (out, phases, detailed_profile) = if detailed {
            let (out, profile) = interpret_profile(&cfg, &syntax, &sem, None).unwrap();
            (out, profile.phases, Some(profile))
        } else {
            let (out, phases) = interpret_timed(&cfg, &syntax, &sem, None).unwrap();
            (out, phases, None)
        };
        let t_interp = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let rs = pattern.transform(out.recordset).unwrap();
        let t_pattern = t.elapsed().as_secs_f64();
        times[3].push(phases[0]);
        times[4].push(phases[1]);
        times[5].push(phases[2]);
        times[6].push(phases[3] + t_pattern);
        times[7].push(t_interp);
        if let Some(profile) = detailed_profile {
            eprintln!(
                "profile: pool={:.4} grouping={:.4} sorting={:.4} index={:.4} providers={:.4} apply={:.4} consistency={:.4} schema={:.4} prep={:.4} generate={:.4} work={:.4} init={:.4} fill={:.4} actions={} provider_items={} records={} record_items={} jobs={} parallel_sort={} parallel_index={} parallel_consistency={}",
                profile.pool_creation,
                profile.action_grouping,
                profile.action_sorting,
                profile.item_index,
                profile.provider_evaluation,
                profile.action_application,
                profile.consistency,
                profile.schema,
                profile.record_preparation,
                profile.record_generation,
                profile.record_work,
                profile.record_initialization,
                profile.record_fill,
                profile.actions,
                profile.provider_items,
                profile.records,
                profile.record_items,
                profile.jobs,
                profile.parallel_action_sorting,
                profile.parallel_item_index,
                profile.parallel_consistency,
            );
        }

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
            rs.len(),
            rs.schema.attributes.len()
        );
    }
    eprintln!("{summary}");
    for (n, v) in names.iter().zip(times.iter_mut()) {
        eprintln!("{n:>14}: {:.3} s", median(v));
    }
}
