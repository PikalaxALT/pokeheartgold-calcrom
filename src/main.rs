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

struct RunPlan {
    basedir: String,
    buildname: Option<String>,
    name: String,
}

impl Args {
    fn run(&self) -> Result<Vec<(String, Stats)>, Box<dyn Error>> {
        Ok(std::iter::chain(
            self.buildnames.iter().flat_map(|buildname| {
                self.arm9subdir.iter().filter_map(|subdir| {
                    Some(RunPlan {
                        basedir: std::format!("{}/{}", self.rootdir, subdir),
                        buildname: Some(buildname.clone()),
                        name: String::from("main"),
                    })
                })
            }),
            self.arm7subdir.iter().filter_map(|subdir| {
                Some(RunPlan {
                    basedir: std::format!("{}/{}", self.rootdir, subdir),
                    buildname: None,
                    name: String::from("ichneumon_sub"),
                })
            }),
        )
        .map(|plan| -> Result<(String, Stats), Box<dyn Error>> {
            let source_map =
                source_mapper::get_source_files(plan.basedir.clone(), plan.name.clone())?;
            let stats = build_analyzer::analyze_build(
                &plan.basedir,
                &plan.buildname,
                &plan.name,
                &source_map,
            )?;
            Ok((plan.buildname.unwrap_or(plan.name), stats))
        })
        .collect::<Result<Vec<_>, _>>()?)
    }
}

pub fn main() {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(log::LevelFilter::Debug))
        .expect("log init failed");

    let args = Args::parse();
    let results = args.run().expect("processing error");
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
