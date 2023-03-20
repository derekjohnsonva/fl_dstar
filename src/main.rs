use clap::Parser;
use fl_dstar::{self, LineInfo};
use std::fs;
use std::io;
/// A simple CLI that will analyze coverage data from passing and failing tests
/// and output lines most likely to contain bugs. This is determined using the dstar
/// suspiciousness metric.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    passing_dir: std::path::PathBuf,
    failing_dir: std::path::PathBuf,
}

fn main() {
    let args = Cli::parse();
    // check that the passed in directories exist
    if !args.passing_dir.exists() {
        eprintln!("The passed in passing directory does not exist");
        std::process::exit(1);
    }
    if !args.failing_dir.exists() {
        eprintln!("The passed in failing directory does not exist");
        std::process::exit(1);
    }
    // get a list of all the files in the passing and failing directories
    let passing_files = fs::read_dir(&args.passing_dir)
        .unwrap()
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, std::io::Error>>()
        .unwrap();

    let failing_files = fs::read_dir(&args.failing_dir)
        .unwrap()
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, std::io::Error>>()
        .unwrap();

    // parse the gcov files
    let passing_files_info: Vec<Vec<LineInfo>> = passing_files
        .iter()
        .map(|file| fl_dstar::parse_gcov_file(file))
        .collect();
    let failing_files_info: Vec<Vec<LineInfo>> = failing_files
        .iter()
        .map(|file| fl_dstar::parse_gcov_file(file))
        .collect();
    // make a list of all the statements in the file. This should be the same for all passing and failing test casees
    let mut statement_info_list: Vec<fl_dstar::StatementInfo> = Vec::new();
    for line in &passing_files_info[0] {
        let statement_info = fl_dstar::StatementInfo::new(
            line.line_number,
            line.statement.clone(),
            failing_files.len() as u32,
        );
        // Skip over lines that have no executable code
        if line.coverage == fl_dstar::Coverage::NoExecutableCode {
            continue;
        }
        statement_info_list.push(statement_info);
    }
    for i in 0..passing_files_info.len() {
        fl_dstar::add_test_to_statements(&mut statement_info_list, &passing_files_info[i], true);
    }
    for i in 0..failing_files_info.len() {
        fl_dstar::add_test_to_statements(&mut statement_info_list, &failing_files_info[i], false);
    }
    statement_info_list.iter_mut().for_each(|statement| {
        statement.calculate_suspiciousness();
    });

    statement_info_list.sort_by(|a, b| {
        let sus_res = b.suspiciousness.partial_cmp(&a.suspiciousness).unwrap();
        if sus_res == std::cmp::Ordering::Equal {
            a.line_number.cmp(&b.line_number)
        } else {
            sus_res
        }
    });
    let mut wtr = csv::Writer::from_writer(io::stdout());
    for statement in statement_info_list {
        wtr.serialize(statement).unwrap();
    }
    wtr.flush().unwrap();
}
