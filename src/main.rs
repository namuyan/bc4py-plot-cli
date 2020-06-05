extern crate bc4py_hash;
use bc4py_hash::plotfile::*;
use clap::{App, Arg};
use std::env;
use std::fs::{create_dir_all, remove_file};
use std::path::Path;
use std::process::exit;
use std::time::Instant;

fn main() {
    let plot_app = App::new("plot")
        .about("generate unoptimized file method")
        .after_help("EXAMPLE:\n  `bc4py-plot-cli plot 003d35d49f2d6ff6a8fe0ba147d7b409585a43ca18 0 100 /path/to/output`")
        .args(&[
            Arg::with_name("addr").about("address of 21 bytes hex of string"),
            Arg::with_name("start").about("start nonce index of uint"),
            Arg::with_name("end").about("end nonce index of uint"),
            Arg::with_name("output-dir").about("output dir path (recommend SSD)"),
        ]);
    let convert_app = App::new("convert")
        .about("convert unoptimized to optimized method")
        .after_help("EXAMPLE:\n  `bc4py-plot-cli convert -i /path/to/input1 /path/to/input2 -- /path/to/output`")
        .args(&[
            Arg::with_name("input-dirs")
                .short('i')
                .multiple(true)
                .required(true)
                .about("input unoptimized file's directories (multiple)"),
            Arg::with_name("output-dir")
                .about("output directory (recommend HDD)"),
            Arg::with_name("remove")
                .about("remove unoptimized file after")
                .default_value("true"),
        ]);
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("bc4py proof of capacity plot file generation tool")
        .after_help("plot file size calculation is simple, nonce / 2000 Gb.")
        .subcommand(plot_app)
        .subcommand(convert_app)
        .get_matches();

    // execute plot
    if let Some(args) = matches.subcommand_matches("plot") {
        // 0. addr
        let mut addr = [0u8; 21];
        {
            let addr_hex: String = args.value_of_t_or_exit("addr");
            let decoded = hex::decode(&addr_hex);
            if decoded.is_err() {
                eprintln!(
                    "address decode failed by: {}",
                    decoded.unwrap_err().to_string()
                );
                exit(1);
            } else if decoded.as_ref().unwrap().len() != 21 {
                eprintln!(
                    "address decode failed by: unexpected len {}bytes",
                    decoded.as_ref().unwrap().len()
                );
                exit(1);
            } else {
                addr.clone_from_slice(decoded.as_ref().unwrap());
            }
        }

        // 1. start
        let start: usize = args.value_of_t_or_exit("start");

        // 2. end
        let end: usize = args.value_of_t_or_exit("end");

        // 3. output-dir
        let output_dir = {
            let dir_str: String = args.value_of_t_or_exit("output-dir");
            let dir = Path::new(&dir_str);
            if !dir.exists() {
                if let Err(err) = create_dir_all(&dir) {
                    eprintln!("output-dir creation failed by: {}", err.to_string());
                    exit(1);
                }
            }
            dir.to_path_buf()
        };

        // check
        if !(start < end) {
            eprintln!(
                "not correct nonce index start<end (start={} end={})",
                start, end
            );
            exit(1);
        }

        // plot
        let now = Instant::now();
        println!("note: accept the task, please wait..");
        plot_unoptimized_file(&addr, start, end, &output_dir);

        // success
        println!("note: success plot file {}secs", now.elapsed().as_secs());
        exit(0);
    }

    // execute convert
    if let Some(args) = matches.subcommand_matches("convert") {
        // 0. input-dirs
        let mut files: Vec<PlotFile> = vec![];
        {
            let dirs_str: Vec<String> = args.values_of_t_or_exit("input-dirs");
            for path in dirs_str.iter() {
                let path = Path::new(path);
                if path.exists() && path.is_dir() {
                    files.extend(PlotFile::restore_from_dir(path).into_iter());
                } else {
                    eprintln!("input-dir is not exist or not directory path={:?}", path);
                    exit(1);
                }
            }
        };

        // 1. output-dir
        let output_dir = {
            let dir: String = args.value_of_t_or_exit("output-dir");
            let path = Path::new(&dir);
            if !path.exists() {
                create_dir_all(path).unwrap();
            }
            path.to_path_buf()
        };

        // 2. remove
        let remove = &args.value_of_t_or_exit::<String>("remove") == "true";

        // need: at least 1 plot file
        if files.len() < 1 {
            eprintln!("plot file you specified dirs is empty");
            exit(1);
        }

        // show plot files to user
        println!("note: plot files list");
        for (index, plot) in files.iter().enumerate() {
            println!(
                "  No.{} {}-{} {:?} ({})",
                index,
                plot.start,
                plot.end,
                plot.flag,
                plot.path.to_str().unwrap()
            );
        }
        println!();

        // filter and sort by start key
        let addr = files.first().unwrap().addr.clone();
        let mut files = files
            .into_iter()
            .filter(|plot| plot.flag == PlotFlag::Unoptimized)
            .filter(|plot| plot.addr == addr)
            .collect::<Vec<PlotFile>>();
        files.sort_by_key(|plot| plot.start);

        // need: at least 1 plot file (filtered)
        if files.len() < 1 {
            eprintln!("find plot files but filtered result show empty");
            exit(1);
        }

        // show plot files to user
        println!("note: filtered plot files list");
        for (index, plot) in files.iter().enumerate() {
            println!(
                "  No.{} {}-{} ({})",
                index,
                plot.start,
                plot.end,
                plot.path.to_str().unwrap()
            );
        }
        println!();

        // files order check
        for (index, plot) in files.iter().enumerate().skip(1) {
            let previous = files.get(index - 1).unwrap();
            if previous.end != plot.start {
                eprintln!("cannot concat {:?} with {:?}", previous, plot);
                exit(1);
            }
        }

        // convert
        let now = Instant::now();
        let plot = convert_to_optimized_file(files.clone(), &output_dir);

        // success
        println!("note: success converting {}sec", now.elapsed().as_secs());
        println!("note: generate to '{}'", plot.path.to_str().unwrap());
        println!();

        // remove (option)
        if remove {
            for plot in files.iter() {
                remove_file(&plot.path).unwrap();
                println!(
                    "note: success remove old unoptimized file '{}'",
                    plot.path.to_str().unwrap()
                );
            }
        } else {
            println!("note: don't remove old plot files")
        }
        exit(0);
    }

    // no option input
    eprintln!("please see help by `--help`");
    exit(1);
}
