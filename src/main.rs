#![allow(dead_code)]

extern crate clap;

mod semantics;
mod tokenization;
mod parsing;
mod position;
mod error;
mod translation;

use std::{fs::File, io::{Read, Write}, path::{Path, PathBuf}, str::FromStr};
use clap::Clap;
use crate::{
    tokenization::{
        SimpleBuffer,
        TokenStream,
    },
    parsing::code::Code,
};

/// A rudimentary Pascal compiler targeting WebAssembly
#[derive(Clap)]
#[clap(version = "0.8", author = "anonymous")]
struct Args {
    input: String,
    #[clap(short, default_value = "a.wat")]
    output: String
}

fn main() {
    let args: Args = Args::parse();

    match PathBuf::from_str(&args.input) {
        Ok(input_path) => {
            let filepath =
                if let Some(p) = input_path.to_str() {
                    format!("file {}", p)
                } else {
                    "input file".into()
                };
        
            if !input_path.exists() {
                eprintln!("\"{}\" doesn't exists.", filepath);
                return;
            }
        
            if !input_path.is_file() {
                eprintln!("\"{}\" is a directory.", filepath);
                return;
            }

            let output_dir = Path::new(&args.output).parent().unwrap();
            let stem = Path::new(&args.output)
                .file_stem().unwrap()
                .to_str().expect("Bad WASM file name");

            let output = output_dir.join(stem.to_string() + ".wat");
    
            match File::open(&args.input) {
                Ok(mut in_file) => {
                    let mut data = Vec::with_capacity(4096);
                    if let Err(e) = in_file.read_to_end(&mut data) {
                        eprintln!("Error reading input file: {}", e);
                    }
        
                    let buf = SimpleBuffer::new(
                        &data,
                        Some(args.input.clone())
                    );
                    let ts = TokenStream::new(buf);
            
                    match File::create(&output) {
                        Ok(out_file) => {
                            let output = Box::new(out_file);
                            let code = Code::new(ts, output);

                            match code.compile() {
                                Ok(errs) => {
                                    println!("{}", errs);
                                },
                                Err(e) => {
                                    eprintln!("Critical: {}", e)
                                }
                            }
                        },
                        Err(e) => {
                            eprintln!(
                                "Failed to open {}: {}.",
                                args.output, e
                            );
                            return;
                        }
                    }
                },
                Err(e) => {
                    eprintln!("Failed to create {}: {}.", filepath, e);
                    return;
                }
            }

            wat::parse_file(&output).and_then(|binary| {
                let wasm_path = Path::new(&args.output)
                    .parent()
                    .unwrap()
                    .join(format!("{}.wasm", stem));

                let wasm_path = wasm_path
                    .to_str()
                    .expect("Bad WASM file path");

                match File::create(wasm_path) {
                    Ok(mut f) => {
                        if let Err(e) = f.write_all(&binary) {
                            eprintln!(
                                "Failed to write into \"{}\": {}",
                                wasm_path.to_string(), e
                            );
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to create WASM file: {}", e);
                    }
                };

                Ok(())
            }).unwrap_or_else(|e| {
                eprintln!("{}", e)
            });
        }
        Err(e) => {
            eprintln!("Input path is invalid: {}.", e);
            return;
        }
    }
}
