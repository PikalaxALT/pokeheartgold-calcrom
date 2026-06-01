use clap::Parser;
use std::option::Option;
use std::vec::Vec;
mod build_analyzer;

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

fn report(good_ct: &u32, bad_ct: &u32, total_label: &str, good_label: &str, bad_label: &str) -> () {
    let total = good_ct + bad_ct;
    println!("  {} {}", total, total_label);
    if total != 0 {
        let total_d: f64 = total.into();
        let good_d: f64 = (*good_ct).into();
        let bad_d: f64 = (*bad_ct).into();
            println!("    {} {} ({:.2}%)", good_ct, good_label, good_d / total_d * 100.0);
            println!("    {} {} ({:.2}%)", bad_ct, bad_label, bad_d / total_d * 100.0);
            println!("");
    }
}

pub fn main() {
    let args = Args::parse();

    // Build evaluation plan
    let mut plan = Vec::new();
    match args.arm9subdir {
        Some(subdir) => {
            for buildname in args.buildnames {
                plan.push(build_analyzer::BuildAnalyzer{
                    basedir: std::format!("{}/{}", args.rootdir, subdir),
                    buildname: Some(buildname),
                    name: "main".to_string(),
                });
            }
        }
        None => {}
    }

    match args.arm7subdir {
        Some(subdir) => {
            plan.push(build_analyzer::BuildAnalyzer{
                basedir: std::format!("{}/{}", args.rootdir, subdir),
                buildname: None,
                name: "ichneumon_sub".to_string(),
            });
        }
        None => {}
    }

    // Execute plan
    for subdir in plan {
        let stats = subdir.process();
        println!("Analysis of {} binary:", subdir.buildname.or(Some(subdir.name)).expect("must have a name"));
        report(&stats.c_code_bytes, &stats.asm_code_bytes, "total bytes of code", "bytes of code in src", "bytes of code in asm");
        report(&stats.c_data_bytes, &stats.asm_data_bytes, "total bytes of data", "bytes of data in src", "bytes of data in asm");
        report(&stats.resolved_pointers, &stats.hardcoded_pointers, "total pointers", "properly-linked pointers", "hard-coded pointers");
    };
}
