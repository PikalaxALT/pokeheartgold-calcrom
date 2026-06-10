use clap::Parser;
use std::option::Option;
use std::vec::Vec;

use crate::build_analyzer::Stats;
mod build_analyzer;
mod source_mapper;

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }
    fn log(&self, record: &log::Record) {
        eprintln!("{} - {}", record.level(), record.args());
    }

    fn flush(&self) {}
}

static LOGGER: SimpleLogger = SimpleLogger;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Nintendo DS project root
    #[arg(short = 'd', default_value = ".")]
    rootdir: String,

    /// Subdirectory containing the ARM9 root
    #[arg(short = '9')]
    arm9subdir: Option<String>,

    /// Subdirectory containing the ARM7 root
    #[arg(short = '7')]
    arm7subdir: Option<String>,

    /// Names of the built ROM(s)
    #[arg(num_args(0..))]
    buildnames: Vec<String>,
}

fn report(
    good_ct: usize,
    bad_ct: usize,
    total_label: &str,
    good_label: &str,
    bad_label: &str,
) -> () {
    let total = good_ct + bad_ct;
    println!("  {} {}", total, total_label);
    if total != 0 {
        let total_d = total as f64;
        let good_d = good_ct as f64;
        let bad_d = bad_ct as f64;
        println!(
            "    {} {} ({:.2}%)",
            good_ct,
            good_label,
            good_d / total_d * 100.0
        );
        println!(
            "    {} {} ({:.2}%)",
            bad_ct,
            bad_label,
            bad_d / total_d * 100.0
        );
        println!("");
    }
}

pub fn main() {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(log::LevelFilter::Debug))
        .expect("log init failed");

    let args = Args::parse();

    let mut results = Vec::<(String, Stats)>::new();

    let name_main = "main";
    let name_sub = "ichneumon_sub";

    // Build evaluation plan
    let arm9_basedir = args
        .arm9subdir
        .and_then(|subdir| std::format!("{}/{}", args.rootdir, subdir).into());
    let arm7_basedir = args
        .arm7subdir
        .and_then(|subdir| std::format!("{}/{}", args.rootdir, subdir).into());
    match arm9_basedir {
        Some(ref basedir) => {
            let source_map = source_mapper::get_source_files(basedir, name_main).unwrap();
            args.buildnames.into_iter().for_each(|buildname| {
                let stats = build_analyzer::analyze_build(
                    basedir,
                    Some(&buildname),
                    name_main,
                    &source_map,
                );
                results.push((buildname.clone(), stats));
            });
        }
        None => {}
    }

    match arm7_basedir {
        Some(ref basedir) => {
            let source_map = source_mapper::get_source_files(basedir, name_sub).unwrap();
            let stats = build_analyzer::analyze_build(basedir, None, name_sub, &source_map);
            results.push((name_sub.to_string(), stats));
        }
        None => {}
    }

    // Execute plan
    results.into_iter().for_each(|(buildname, stats)| {
        println!("Analysis of {} binary:", buildname);
        report(
            stats.c_code_bytes,
            stats.asm_code_bytes,
            "total bytes of code",
            "bytes of code in src",
            "bytes of code in asm",
        );
        report(
            stats.c_data_bytes,
            stats.asm_data_bytes,
            "total bytes of data",
            "bytes of data in src",
            "bytes of data in asm",
        );
        report(
            stats.resolved_pointers,
            stats.hardcoded_pointers,
            "total pointers",
            "properly-linked pointers",
            "hard-coded pointers",
        );
    });
}
