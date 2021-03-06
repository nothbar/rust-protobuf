use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::process;

use ::protoc::Protoc;

use crate::gen_and_write::gen_and_write;
use crate::Customize;
mod protoc;
mod pure;

#[derive(Debug)]
enum WhichParser {
    Pure,
    Protoc,
}

impl Default for WhichParser {
    fn default() -> WhichParser {
        WhichParser::Protoc
    }
}

/// `Protoc --rust_out...` args
#[derive(Debug, Default)]
pub struct Codegen {
    /// What parser to use to parse `.proto` files.
    which_parser: WhichParser,
    /// --lang_out= param
    out_dir: PathBuf,
    /// -I args
    includes: Vec<PathBuf>,
    /// List of .proto files to compile
    inputs: Vec<PathBuf>,
    /// Customize code generation
    customize: Customize,
    /// Protoc command path
    protoc: Option<Protoc>,
    /// Extra `protoc` args
    extra_args: Vec<OsString>,
}

impl Codegen {
    /// Create new codegen object.
    ///
    /// Uses `protoc` from `$PATH` by default.
    ///
    /// Can be switched to pure rust parser using [`pure`](Self::pure) function.
    pub fn new() -> Self {
        Self::default()
    }

    /// Switch to pure Rust parser of `.proto` files.
    pub fn pure(&mut self) -> &mut Self {
        self.which_parser = WhichParser::Pure;
        self
    }

    /// Switch to `protoc` parser of `.proto` files.
    pub fn protoc(&mut self) -> &mut Self {
        self.which_parser = WhichParser::Protoc;
        self
    }

    /// Output directory for generated code.
    pub fn out_dir(&mut self, out_dir: impl AsRef<Path>) -> &mut Self {
        self.out_dir = out_dir.as_ref().to_owned();
        self
    }

    /// Add an include directory.
    pub fn include(&mut self, include: impl AsRef<Path>) -> &mut Self {
        self.includes.push(include.as_ref().to_owned());
        self
    }

    /// Add include directories.
    pub fn includes(&mut self, includes: impl IntoIterator<Item = impl AsRef<Path>>) -> &mut Self {
        for include in includes {
            self.include(include);
        }
        self
    }

    /// Append a `.proto` file path to compile
    pub fn input(&mut self, input: impl AsRef<Path>) -> &mut Self {
        self.inputs.push(input.as_ref().to_owned());
        self
    }

    /// Append multiple `.proto` file paths to compile
    pub fn inputs(&mut self, inputs: impl IntoIterator<Item = impl AsRef<Path>>) -> &mut Self {
        for input in inputs {
            self.input(input);
        }
        self
    }

    /// Specify `protoc` command path to be used when invoking code generation.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # mod protoc_bin_vendored {
    /// #   pub fn protoc_bin_path() -> Result<std::path::PathBuf, std::io::Error> {
    /// #       unimplemented!()
    /// #   }
    /// # }
    ///
    /// use protobuf_codegen::Codegen;
    ///
    /// Codegen::new()
    ///     .protoc()
    ///     .protoc_path(protoc_bin_vendored::protoc_bin_path().unwrap())
    ///     // ...
    ///     .run()
    ///     .unwrap();
    /// ```
    ///
    /// This option is ignored when pure Rust parser is used.
    pub fn protoc_path(&mut self, protoc: impl Into<PathBuf>) -> &mut Self {
        self.protoc = Some(Protoc::from_path(&protoc.into()));
        self
    }

    /// Set options to customize code generation
    pub fn customize(&mut self, customize: Customize) -> &mut Self {
        self.customize = customize;
        self
    }

    /// Extra command line flags for `protoc` invocation.
    ///
    /// For example, `--experimental_allow_proto3_optional` option.
    ///
    /// This option is ignored when pure Rust parser is used.
    pub fn extra_arg(&mut self, arg: impl Into<OsString>) -> &mut Self {
        self.extra_args.push(arg.into());
        self
    }

    /// Invoke the code generation.
    ///
    /// This is roughly equivalent to `protoc --rust_out=...` but
    /// without requiring `protoc-gen-rust` command in `$PATH`.
    ///
    /// This function uses pure Rust parser or `protoc` parser depending on
    /// how this object was configured.
    pub fn run(&self) -> anyhow::Result<()> {
        let (parsed_and_typechecked, parser) = match self.which_parser {
            WhichParser::Protoc => protoc::parse_and_typecheck(self)?,
            WhichParser::Pure => pure::parse_and_typecheck(self)?,
        };
        gen_and_write(
            &parsed_and_typechecked.file_descriptors,
            &parser,
            &parsed_and_typechecked.relative_paths,
            &self.out_dir,
            &self.customize,
        )
    }

    /// Similar to `run`, but prints the message to stderr and exits the process on error.
    pub fn run_from_script(&self) {
        if let Err(e) = self.run() {
            eprintln!("protoc-based codegen failed: {}", e);
            process::exit(1);
        }
    }
}

fn remove_path_prefix<'a>(mut path: &'a Path, mut prefix: &Path) -> Option<&'a Path> {
    path = path.strip_prefix(".").unwrap_or(path);
    prefix = prefix.strip_prefix(".").unwrap_or(prefix);
    path.strip_prefix(prefix).ok()
}

#[test]
fn test_remove_path_prefix() {
    assert_eq!(
        Some(Path::new("abc.proto")),
        remove_path_prefix(Path::new("xxx/abc.proto"), Path::new("xxx"))
    );
    assert_eq!(
        Some(Path::new("abc.proto")),
        remove_path_prefix(Path::new("xxx/abc.proto"), Path::new("xxx/"))
    );
    assert_eq!(
        Some(Path::new("abc.proto")),
        remove_path_prefix(Path::new("../xxx/abc.proto"), Path::new("../xxx/"))
    );
    assert_eq!(
        Some(Path::new("abc.proto")),
        remove_path_prefix(Path::new("abc.proto"), Path::new("."))
    );
    assert_eq!(
        Some(Path::new("abc.proto")),
        remove_path_prefix(Path::new("abc.proto"), Path::new("./"))
    );
    assert_eq!(
        None,
        remove_path_prefix(Path::new("xxx/abc.proto"), Path::new("yyy"))
    );
    assert_eq!(
        None,
        remove_path_prefix(Path::new("xxx/abc.proto"), Path::new("yyy/"))
    );
}
