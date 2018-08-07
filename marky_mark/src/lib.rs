#![recursion_limit = "256"]
#[macro_use]
extern crate failure;
#[macro_use]
extern crate quote;
#[macro_use]
extern crate serde_derive;

extern crate digest;
extern crate generic_array;
extern crate proc_macro2;
extern crate serde;
extern crate serde_json;
extern crate sha2;
extern crate syn;
extern crate toml;

#[cfg(test)]
extern crate tempfile;

use std::fs::{create_dir_all, read_to_string, write};
use std::path::Path;

use proc_macro2::Span;
use syn::{Ident as SynIdent, Path as SynPath};

type Result<T> = ::std::result::Result<T, failure::Error>;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, PartialOrd, Ord, Serialize)]
pub struct Benchmark {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runner: Option<String>,
    pub name: String,
    #[serde(rename = "crate")]
    pub crate_name: String,
}

impl Benchmark {
    pub fn new(crate_name: &str, name: &str) -> Self {
        Self {
            name: name.to_string(),
            crate_name: crate_name.to_string(),
            runner: None,
        }
    }

    pub fn set_runner(&mut self, runner: &str) {
        self.runner = Some(runner.to_owned());
    }

    fn source(&self) -> String {
        let name_syn: SynPath = syn::parse_str(&self.name).unwrap();
        let crate_name_syn = SynIdent::new(&self.crate_name, Span::call_site());

        let source = quote! {
            extern crate #crate_name_syn;
            extern crate lolbench_support;

            use lolbench_support::{criterion_from_env, init_logging};

            fn main() {
                init_logging();
                let mut crit = criterion_from_env();
                #crate_name_syn::#name_syn(&mut crit);
            }
        };

        source.to_string()

        // TODO(anp): guarantee that rustfmt is available somehow and run it on the file
    }

    pub fn rendered(&mut self) -> String {
        let source = self.source();
        format!("//{}\n{}", serde_json::to_string(&self).unwrap(), source)
    }

    pub fn parse(s: &str) -> Result<(Self, String)> {
        let mut lines = s.lines();

        let first_line = match lines.next() {
            Some(l) => l.trim_left_matches("//"),
            None => bail!("missing first line"),
        };

        let remaining = lines.fold(String::new(), |remaining, line| remaining + line);

        Ok((serde_json::from_str(first_line)?, remaining))
    }

    pub fn write(&mut self, full_path: &Path) -> Result<bool> {
        // if there's an existing file for this bench's path, we need to know about two questions
        //
        // 1. is there persistent config that was written before which we need to preserve?
        // 2. can we skip writing altogether to limit disk thrash?
        if let Ok(existing_contents) = read_to_string(&full_path) {
            // for now, the only persistent config is what runner, if any, has been configured.
            // however, we don't want to preserve the last runner config if we're *currently in the
            // process of setting it*
            if self.runner.is_none() {
                // we don't care about errors here at all
                if let Ok((existing, _)) = Self::parse(&existing_contents) {
                    if let Some(r) = existing.runner {
                        self.runner = Some(r);
                    }
                }
            }
        }

        write_if_changed(&self.rendered(), &full_path)
    }
}

pub fn test_source(bench_name: &str, crate_name: &str, binary_name: &str) -> String {
    let crate_name_syn = SynIdent::new(crate_name, Span::call_site());

    let bench_source_name = format!("{}.rs", binary_name);

    let source = quote! {
        extern crate #crate_name_syn;
        extern crate lolbench_support;

        use std::path::Path;

        use lolbench_support::{Benchmark, CriterionConfig, RunPlan, r32};

        #[test]
        fn end_to_end() {
            let target_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
            let binary_path = target_dir.join("release").join(#binary_name);

            let plan = RunPlan {
                shield: None,
                toolchain: String::from("stable"),
                source_path: Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("benches")
                    .join(#crate_name)
                    .join("src")
                    .join("bin")
                    .join(#bench_source_name),
                target_dir,
                manifest_path: Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("benches")
                    .join(#crate_name)
                    .join("Cargo.toml"),
                benchmark: Benchmark {
                    runner: None,
                    name: String::from(#bench_name),
                    crate_name: String::from(#crate_name),
                },
                binary_path,
                bench_config: Some(CriterionConfig {
                    confidence_level: r32(0.95),
                    measurement_time_ms: 500,
                    nresamples: 2,
                    noise_threshold: r32(0.0),
                    sample_size: 5,
                    significance_level: r32(0.05),
                    warm_up_time_ms: 1,
                }),
            };

            if let Err(why) = plan.run() {
                panic!("{}", why);
            }
        }
    };

    source.to_string()
}

pub fn write_if_changed(file_contents: &str, test_path: &Path) -> Result<bool> {
    let need_to_write = match read_to_string(&test_path) {
        Ok(existing) => existing != file_contents,
        _ => true,
    };

    if need_to_write {
        create_dir_all(test_path.parent().unwrap())?;
        write(&test_path, file_contents.as_bytes())?;
    }

    Ok(need_to_write)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips() {
        let mut header = Benchmark::new("test_crate", "test_bench");
        let rendered = header.rendered();

        let (mut parsed, remaining) = Benchmark::parse(&rendered).unwrap();
        assert_eq!(header, parsed);
        assert_eq!(remaining, header.source());

        let parsed_rendered = parsed.rendered();
        assert_eq!(rendered, parsed_rendered);
    }

    #[test]
    fn write() {
        let mut header = Benchmark::new("test_crate", "test_bench");
        let rendered = header.rendered();

        let tmpfile = tempfile::NamedTempFile::new().unwrap();
        let bench_path = tmpfile.path();
        header.write(&bench_path).unwrap();

        let written = read_to_string(&bench_path).unwrap();

        assert_eq!(rendered, written);
    }

    #[test]
    fn preserve_runner() {
        let runner = "they-call-me-tim";

        let mut header = Benchmark::new("test_crate", "test_bench");
        let mut without_runner = header.clone();
        header.set_runner(runner);

        let tmpfile = tempfile::NamedTempFile::new().unwrap();
        let bench_path = tmpfile.path();
        header.write(&bench_path).unwrap();

        let written = read_to_string(&bench_path).unwrap();
        let (written_header, _) = Benchmark::parse(&written).unwrap();

        assert_eq!(
            written_header.runner, header.runner,
            "runner should be preserved in writing"
        );

        without_runner.write(&bench_path).unwrap();
        let written = read_to_string(&bench_path).unwrap();
        let (written_header, _) = Benchmark::parse(&written).unwrap();

        let written_runner = written_header
            .runner
            .expect("runner should have been preserved");
        assert_eq!(written_runner, runner);
    }
}