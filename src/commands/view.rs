use std::path::PathBuf;

use async_trait::async_trait;
use clap::Args;
use colored::*;
use humansize::{file_size_opts, FileSize};
use miette::{IntoDiagnostic, Result, WrapErr};
use nassun::NassunOpts;
use oro_common::{Bin, Manifest, NpmUser, Person, PersonField, VersionMetadata};
use oro_config::OroConfigLayer;
use term_grid::{Cell, Direction, Filling, Grid, GridOptions};
use url::Url;

use crate::commands::OroCommand;

#[derive(Debug, Args, OroConfigLayer)]
pub struct ViewCmd {
    /// Package spec to look up.
    #[arg()]
    pkg: String,

    #[arg(from_global)]
    registry: Option<Url>,

    #[arg(from_global)]
    root: Option<PathBuf>,

    #[arg(from_global)]
    cache: Option<PathBuf>,

    #[arg(from_global)]
    json: bool,
}

#[async_trait]
impl OroCommand for ViewCmd {
    async fn execute(self) -> Result<()> {
        let mut nassun_opts = NassunOpts::new();
        if let Some(registry) = self.registry {
            nassun_opts = nassun_opts.registry(registry);
        }
        if let Some(root) = self.root {
            nassun_opts = nassun_opts.base_dir(root);
        }
        if let Some(cache) = self.cache {
            nassun_opts = nassun_opts.cache(cache);
        }
        let pkg = nassun_opts.build().resolve(&self.pkg).await?;
        let packument = pkg.packument().await?;
        let metadata = pkg.metadata().await?;
        // TODO: oro view pkg [<field>[.<subfield>...]]
        // Probably the best way to do this is to support doing raw
        // packument/manifest requests that just deserialize to
        // serde_json::Value?
        if self.json {
            // TODO: What should this be? NPM is actually a weird mishmash of
            // the packument and the manifest?
            println!(
                "{}",
                serde_json::to_string_pretty(&metadata)
                    .into_diagnostic()
                    .wrap_err("view::json_serialize")?
            );
        } else {
            let VersionMetadata {
                ref npm_user,
                ref dist,
                ref deprecated,
                ref maintainers,
                manifest:
                    Manifest {
                        ref name,
                        ref description,
                        ref version,
                        ref license,
                        ref dependencies,
                        ref homepage,
                        ref keywords,
                        ref bin,
                        ..
                    },
                ..
            } = metadata;

            // name@version | license | deps: 123 | releases: 123
            println!(
                "{}@{} | {} | deps: {} | releases: {}",
                name.clone()
                    .unwrap_or_else(|| String::from(""))
                    .bright_green()
                    .underline(),
                version
                    .clone()
                    .unwrap_or_else(|| "0.0.0".parse().unwrap())
                    .to_string()
                    .bright_green()
                    .underline(),
                license
                    .clone()
                    .unwrap_or_else(|| "Proprietary".to_string())
                    .green(),
                dependencies.len().to_string().cyan(),
                packument.versions.len().to_string().yellow(),
            );

            // <descrition>
            if let Some(desc) = description.as_ref() {
                println!("{desc}");
            }

            // <homepage>
            if let Some(home) = homepage.as_ref() {
                println!("{}", home.to_string().cyan());
            }
            println!();

            // DEPRECATED - <deprecation message>
            if let Some(msg) = deprecated.as_ref() {
                println!("{} - {}\n", "DEPRECATED".bright_red(), msg);
            }

            // keywords: foo, bar, baz
            if !keywords.is_empty() {
                println!(
                    "keywords: {}\n",
                    keywords
                        .iter()
                        .map(|k| k.yellow().to_string())
                        .collect::<Vec<String>>()
                        .join(", ")
                );
            }

            // bins: foo, bar
            // TODO: directories.bin? (oof)
            if let Some(bin) = bin {
                let bins = match bin {
                    Bin::Str(_) => vec![name.clone().unwrap_or_else(|| String::from(""))],
                    Bin::Hash(bins) => bins.keys().cloned().collect::<Vec<String>>(),
                    Bin::Array(bins) => bins
                        .iter()
                        .filter_map(|bin| {
                            bin.file_name()
                                .map(|name| name.to_string_lossy().to_string())
                        })
                        .collect::<Vec<String>>(),
                };
                println!(
                    "bins: {}\n",
                    bins.iter()
                        .map(|b| b.yellow().to_string())
                        .collect::<Vec<String>>()
                        .join(", ")
                );
            }

            // dist.foo.bar.baz
            println!("dist");
            if let Some(tarball) = &dist.tarball {
                println!(".tarball: {}", tarball.to_string().cyan());
            }
            if let Some(shasum) = &dist.shasum {
                println!(".shasum: {}", shasum.yellow());
            }
            if let Some(sri) = &dist.integrity {
                println!(".integrity: {}", sri.to_string().yellow());
            }
            if let Some(unpacked) = dist.unpacked_size {
                println!(
                    ".unpackedSize: {}",
                    unpacked
                        .file_size(file_size_opts::DECIMAL)
                        .unwrap()
                        .yellow()
                );
            }
            println!();

            // dependencies:
            // foo: ^1.2.3  bar: ^0.1.0
            if !dependencies.is_empty() {
                let max_deps = 25_usize;
                let mut grid = Grid::new(GridOptions {
                    filling: Filling::Spaces(3),
                    direction: Direction::TopToBottom,
                });
                let width = term_size::dimensions().map(|(w, _)| w).unwrap_or(80);
                let mut deps = dependencies.iter().collect::<Vec<(&String, &String)>>();
                deps.sort();
                for (dep, version) in deps.iter().take(max_deps) {
                    let val = format!("{}: {}", dep.yellow(), version);
                    grid.add(Cell::from(val));
                }
                if let Some(out) = grid.fit_into_width(width) {
                    print!("dependencies:\n{out}");
                    let count = dependencies.len();
                    if count > max_deps {
                        println!("(...and {} more)", count - max_deps);
                    }
                }
                println!();
            }

            // maintainers:
            // - Alex <something@email.com>
            if !maintainers.is_empty() {
                println!("maintainers:");
                for person in maintainers.iter() {
                    match person {
                        PersonField::Str(string) => {
                            println!("- {}", string.yellow());
                        }
                        PersonField::Obj(Person {
                            ref name,
                            ref email,
                            ref url,
                        }) => {
                            print!("-");
                            if let Some(name) = name {
                                print!(" {name}");
                            }
                            if let Some(email) = email {
                                print!(" <{}>", email.cyan());
                            }
                            if let Some(url) = url {
                                print!(" ({})", url.cyan());
                            }
                            println!();
                        }
                    }
                }
                println!();
            }

            // published N days ago by Foo
            if let Some(time) = packument.time.get(
                &version
                    .clone()
                    .unwrap_or_else(|| "0.0.0".parse().unwrap())
                    .to_string(),
            ) {
                if let Some(NpmUser { name, email }) = npm_user {
                    let human = chrono_humanize::HumanTime::from(
                        chrono::DateTime::parse_from_rfc3339(time)
                            .into_diagnostic()
                            .wrap_err("view::bad_date")?,
                    );
                    print!(
                        "published {} by {}",
                        human.to_string().yellow(),
                        name.yellow()
                    );
                    if let Some(email) = email {
                        print!(" <{}>", email.cyan());
                    }
                    println!();
                }
            }
        }
        Ok(())
    }
}
