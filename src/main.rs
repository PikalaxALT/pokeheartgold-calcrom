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

pub fn main() {
    let args = Args::parse();

    // Build evaluation plan
    let mut plan = Vec::new();
    match args.arm9subdir {
        Some(subdir) => {
            for buildname in args.buildnames {
                let basedir = std::format!("{}/{}", args.rootdir, subdir);
                plan.push(build_analyzer::BuildAnalyzer {basedir: basedir, name: Some(buildname)});
            }
        },
        None => {},
    }

    match args.arm7subdir {
        Some(subdir) => {
            let basedir = std::format!("{}/{}", args.rootdir, subdir);
            plan.push(build_analyzer::BuildAnalyzer {basedir: basedir, name: None});
        },
        None => {},
    }

    // Execute plan
    for subdir in plan {
        subdir.process();
    }
}
