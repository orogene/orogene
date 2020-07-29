use anyhow::Result;
use async_trait::async_trait;
use clap::Clap;
use colored::*;
use humansize::{file_size_opts, FileSize};
use oro_classic_resolver::ClassicResolver;
use oro_command::OroCommand;
use rogga::{Bin, Human, Manifest, Rogga};
use term_grid::{Cell, Direction, Filling, Grid, GridOptions};
use url::Url;

#[derive(Debug, Clap, OroCommand)]
pub struct ViewCmd {
    #[clap(
        about = "Registry to get package data from.",
        default_value = "https://registry.npmjs.org",
        long
    )]
    registry: Url,
    #[clap(from_global)]
    json: bool,
    #[clap(about = "Package spec to look up")]
    pkg: String,
}

#[async_trait]
impl OroCommand for ViewCmd {
    async fn execute(self) -> Result<()> {
        let pkgreq = Rogga::new(&self.registry).arg_request(&self.pkg)?;
        let packument = pkgreq.packument().await?;
        let pkg = pkgreq.resolve_with(ClassicResolver::new()).await?;
        let manifest = pkg.manifest().await?;
        // TODO: oro view pkg [<field>[.<subfield>...]]
        // Probably the best way to do this is to support doing raw
        // packument/manifest requests that just deserialize to
        // serde_json::Value?
        if self.json {
            // TODO: What should this be? NPM is actually a weird mishmash of
            // the packument and the manifest?
            println!("{}", serde_json::to_string_pretty(&manifest)?);
        } else {
            let Manifest {
                ref name,
                ref description,
                ref version,
                ref license,
                ref licence,
                ref dependencies,
                ref homepage,
                ref keywords,
                ref bin,
                ref npm_user,
                ref dist,
                ref deprecated,
                ..
            } = manifest;

            // TODO: unpublished N days ago by Foo

            // name@version | license | deps: 123 | releases: 123
            println!(
                "{}@{} | {} | deps: {} | releases: {}",
                name.bright_green().underline(),
                version.to_string().bright_green().underline(),
                license
                    .clone()
                    .unwrap_or_else(|| licence.clone().unwrap_or_else(|| "Proprietary".to_string()))
                    .green(),
                dependencies.len().to_string().cyan(),
                packument.versions.len().to_string().yellow(),
            );

            // <descrition>
            if let Some(desc) = description.as_ref() {
                println!("{}", desc);
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
                    Bin::Str(_) => vec![name],
                    Bin::Hash(bins) => bins.keys().collect::<Vec<&String>>(),
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
            println!(".tarball: {}", dist.tarball.to_string().cyan());
            println!(".shasum: {}", dist.shasum.yellow());
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
                    print!("dependencies:\n{}", out);
                    let count = dependencies.len();
                    if count > max_deps {
                        println!("(...and {} more)", count - max_deps);
                    }
                }
                println!();
            }

            // maintainers:
            // - Alex <something@email.com>
            if !packument.maintainers.is_empty() {
                println!("maintainers:");
                for Human { name, email } in packument.maintainers.iter() {
                    print!("- {}", name.yellow());
                    if let Some(email) = email {
                        print!(" <{}>", email.cyan());
                    }
                    println!();
                }
                println!();
            }

            // published N days ago by Foo
            if let Some(time) = packument.time.get(&version.to_string()) {
                if let Some(Human { name, email }) = npm_user {
                    let human = chrono_humanize::HumanTime::from(
                        chrono::DateTime::parse_from_rfc3339(&time.to_rfc3339())?,
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
