use std::env;
use std::path::PathBuf;
use std::process::exit;

use protobuf_parse::pure;

fn main() {
    let args = env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .collect::<Vec<_>>();

    if args.len() != 2 {
        eprintln!(
            "usage: {} <input.proto> <include>",
            env::args().next().unwrap()
        );
        exit(1);
    }

    eprintln!(
        "{} is not a part of public interface",
        env::args().next().unwrap()
    );

    assert!(args.len() >= 2);
    let (input, includes) = args.split_at(1);
    let t = pure::parse_and_typecheck(includes, input).expect("parse_and_typecheck");
    for fd in t.file_descriptors {
        println!("{:#?}", fd);
    }
}
