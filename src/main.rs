use clap::Parser;
use std::error::Error;
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
        eprintln!(
            "{} - {} - {}",
            record.level(),
            record.target(),
            record.args()
        );
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

/// Print a report for the current segment
///
/// Parameters:
/// good_ct: The first number to print, representing the "good" outcome.
/// bad_ct: The second number to print, representing the "bad" outcome.
/// total_label: The string representing the label for the total count (good + bad).
/// good_label: The string representing the label for the good count (good_ct).
/// bad_label: The string representing the label for the bad count (bad_ct).
///
/// Note:
/// This function will print the breakdown of good vs. bad, including a percent,
/// unless the total is 0.
///
/// Example: if you call:
///   report(40, 10, "eggs", "good eggs", "bad eggs");
///
/// it prints:
///   50 eggs
///   40 good eggs (80.00%)
///   10 bad eggs (20.00%)
///
/// Example 2: if you call:
///   report(0, 0, "eggs", "good eggs", "bad eggs");
///
/// it prints:
///   0 eggs
fn report(
    good_ct: usize,
    bad_ct: usize,
    total_label: &'static str,
    good_label: &'static str,
    bad_label: &'static str,
) -> () {
    let total = good_ct + bad_ct;
    println!("  {} {}", total, total_label);
    if total != 0 {
        // Lossy conversion because we don't actually care about precision
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
        println!(""); // An extra newline for good measure
    }
}

/// Represents a build analysis spec
struct RunPlan {
    /// The base directory containing the source code
    basedir: String,

    /// Optional, the subdir of `{basedir}/build` containing the build artifacts
    buildname: Option<String>,

    /// The stem of the ELF file in the build folder and the LSF file in the source root
    elf_stem: &'static str,
}

impl Args {
    fn run(&self) -> Result<Vec<(String, Stats)>, Box<dyn Error>> {
        Ok(std::iter::chain(
            self.buildnames.iter().flat_map(|buildname| {
                self.arm9subdir.iter().map(|subdir| RunPlan {
                    basedir: std::format!("{}/{}", self.rootdir, subdir),
                    buildname: Some(buildname.to_owned()),
                    elf_stem: "main",
                })
            }),
            self.arm7subdir.iter().map(|subdir| RunPlan {
                basedir: std::format!("{}/{}", self.rootdir, subdir),
                buildname: None,
                elf_stem: "ichneumon_sub",
            }),
        )
        .map(|plan| -> Result<(String, Stats), Box<dyn Error>> {
            // get_source_files is #[cached()] so it needs to own plan.basedir
            let source_map =
                source_mapper::get_source_files(plan.basedir.to_owned(), plan.elf_stem)?;
            let stats = build_analyzer::analyze_build(
                &plan.basedir,
                &plan.buildname,
                plan.elf_stem,
                &source_map,
            )?;
            Ok((plan.buildname.unwrap_or(plan.elf_stem.to_string()), stats))
        })
        .collect::<Result<Vec<_>, _>>()?)
    }
}

pub fn main() {
    // Initialize logging
    // Dev profile: DEBUG level
    // Release profile: INFO level
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(log::LevelFilter::Debug))
        .expect("log init failed");

    // Parse the commandline
    let args = Args::parse();

    // Do the work
    let results = args.run().expect("processing error");

    // Print the results on success
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
