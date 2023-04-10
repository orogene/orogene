use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use backon::{BlockingRetryable, ConstantBuilder};
use miette::{IntoDiagnostic, Result};
use resvg::usvg_text_layout::{fontdb, TreeTextToPath};
use serde::Deserialize;

fn main() -> Result<()> {
    let fontdb = load_fonts();

    let root = PathBuf::from(std::file!())
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned();

    render_to_png(
        &plot_benchmark(
            "Warm Cache Comparison",
            &exec_benchmark("rm -rf node_modules")?,
        )?,
        &root.join("assets").join("benchmarks-warm-cache.png"),
        &fontdb,
    )?;

    render_to_png(
        &plot_benchmark(
            "Cold Cache Comparison",
            &exec_benchmark("rm -rf node_modules pm-cache ~/.bun/install/cache")?,
        )?,
        &root.join("assets").join("benchmarks-cold-cache.png"),
        &fontdb,
    )?;

    render_to_png(
        &plot_benchmark("Resolution + Cold Cache Comparison", &exec_benchmark("rm -rf node_modules pm-cache ~/.bun/install/cache yarn.lock package-lock.kdl package-lock.json bun.lockb pnpm-lock.yaml")?)?,
        &root.join("assets").join("benchmarks-initial-install.png"),
        &fontdb,
    )?;

    Ok(())
}

#[derive(Debug, Deserialize)]
struct BenchmarkResults {
    results: Vec<BenchmarkResult>,
}

#[derive(Debug, Deserialize)]
struct BenchmarkResult {
    command: String,
    mean: f64,
}

fn exec_benchmark(prepare: &str) -> Result<BenchmarkResults> {
    let tempdir = tempfile::tempdir_in(
        PathBuf::from(std::file!())
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("target"),
    )
    .into_diagnostic()?;
    let json_output = tempdir
        .path()
        .join("benchmarks.json")
        .to_string_lossy()
        .into_owned();

    std::fs::write(tempdir.path().join("package.json"), PACKAGE_JSON).into_diagnostic()?;

    let op = || {
        let status = Command::new("hyperfine")
            .current_dir(tempdir.path())
            .args(vec![
                "--export-json",
                &json_output,
                "--warmup",
                "1",
                "../release/oro apply --ignore-scripts --cache pm-cache",
                "bun install --ignore-scripts",
                "npm install --ignore-scripts --cache pm-cache",
                "npx -p pnpm pnpm install --ignore-scripts --store-dir pm-cache",
                "npx -p yarn yarn --ignore-scripts --cache-folder pm-cache",
                "--prepare",
                prepare,
            ])
            .status()
            .into_diagnostic()?;
        if status.success() {
            Ok(())
        } else {
            Err(miette::miette!("hyperfine failed"))
        }
    };
    op.retry(&ConstantBuilder::default().with_delay(Duration::from_millis(100)))
        .call()?;

    let mut results: BenchmarkResults =
        serde_json::from_slice(&std::fs::read(&json_output).into_diagnostic()?)
            .into_diagnostic()?;

    for result in &mut results.results {
        if result.command.starts_with("../release") {
            result.command = "orogene".into();
        } else if result.command.starts_with("bun") {
            result.command = "bun".into();
        } else if result.command.starts_with("npm") {
            result.command = "npm".into();
        } else if result.command.starts_with("npx -p pnpm") {
            result.command = "pnpm".into();
        } else if result.command.starts_with("npx -p yarn") {
            result.command = "yarn".into();
        } else {
            panic!("unknown command: {}", result.command);
        }
    }

    Ok(results)
}

fn plot_benchmark(heading: &str, results: &BenchmarkResults) -> Result<String> {
    let mut data = Vec::new();
    for result in &results.results {
        data.push((result.mean, &result.command));
    }

    poloto::build::bar::gen_simple("", data, [0.0])
        .label((heading, "Time (s)", "Package Manager"))
        .append_to(poloto::header().light_theme())
        .render_string()
        .into_diagnostic()
}

fn render_to_png(data: &str, path: &Path, fontdb: &fontdb::Database) -> Result<()> {
    let mut tree = resvg::usvg::Tree::from_str(data, &Default::default()).into_diagnostic()?;
    tree.convert_text(fontdb);
    let fit_to = resvg::usvg::FitTo::Width(1600);
    let size = fit_to
        .fit_to(tree.size.to_screen_size())
        .ok_or_else(|| miette::miette!("failed to fit to screen size"))?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
    resvg::render(
        &tree,
        fit_to,
        resvg::tiny_skia::Transform::default(),
        pixmap.as_mut(),
    )
    .ok_or_else(|| miette::miette!("failed to render"))?;
    std::fs::create_dir_all(path.parent().unwrap()).into_diagnostic()?;
    pixmap.save_png(path).into_diagnostic()?;
    Ok(())
}

fn load_fonts() -> fontdb::Database {
    let mut fontdb = fontdb::Database::new();
    fontdb.load_system_fonts();
    fontdb.set_serif_family("Times New Roman");
    fontdb.set_sans_serif_family("Arial");
    fontdb.set_cursive_family("Comic Sans MS");
    fontdb.set_fantasy_family("Impact");
    fontdb.set_monospace_family("Courier New");

    fontdb
}

const PACKAGE_JSON: &str = r#"
{
  "name": "floc",
  "version": "0.1.0",
  "private": true,
  "scripts": {
    "css": "unocss 'src/**/*.tsx'",
    "build": "npm run css && next build",
    "dev": "concurrently \"next dev\" \"npm run dev:css\"",
    "dev:css": "npm run css -- --watch",
    "postinstall": "prisma generate",
    "lint": "next lint",
    "start": "next start",
    "typecheck": "tsc -p .",
    "typecheck:watch": "npm run typecheck -- --watch"
  },
  "dependencies": {
    "@iconify-json/ph": "^1.1.4",
    "@iconify-json/uil": "^1.1.4",
    "@iconify-json/uim": "^1.1.5",
    "@iconify-json/uis": "^1.1.4",
    "@iconify-json/uit": "^1.1.4",
    "@prisma/client": "^4.11.0",
    "@react-types/shared": "^3.17.0",
    "@sindresorhus/is": "^5.3.0",
    "@tanstack/react-query": "^4.24.10",
    "@trpc/client": "^10.13.2",
    "@trpc/next": "^10.13.2",
    "@trpc/react-query": "^10.13.2",
    "@trpc/server": "^10.13.2",
    "@types/lodash-es": "^4.17.6",
    "@unocss/cli": "^0.50.3",
    "clsx": "^1.2.1",
    "concurrently": "^7.6.0",
    "eslint-plugin-simple-import-sort": "^10.0.0",
    "eslint-plugin-unused-imports": "^2.0.0",
    "husky": "^8.0.3",
    "i18next": "^22.4.10",
    "i18next-chained-backend": "^4.2.0",
    "i18next-http-backend": "^2.1.1",
    "immer": "^9.0.19",
    "iron-session": "^6.3.1",
    "lodash-es": "^4.17.21",
    "masto": "^5.10.0",
    "next": "13.2.3",
    "next-i18next": "^13.2.0",
    "react": "18.2.0",
    "react-aria": "^3.23.0",
    "react-dom": "18.2.0",
    "react-i18next": "^12.2.0",
    "react-merge-refs": "^2.0.1",
    "react-stately": "^3.21.0",
    "react-use": "^17.4.0",
    "superjson": "^1.12.2",
    "unocss": "^0.50.3",
    "zod": "^3.20.6",
    "zustand": "^4.3.5"
  },
  "devDependencies": {
    "@commitlint/cli": "^17.4.4",
    "@commitlint/config-conventional": "^17.4.4",
    "@types/node": "^18.14.2",
    "@types/prettier": "^2.7.2",
    "@types/react": "^18.0.28",
    "@types/react-dom": "^18.0.11",
    "@typescript-eslint/eslint-plugin": "^5.54.0",
    "@typescript-eslint/parser": "^5.54.0",
    "@unocss/eslint-config": "^0.50.3",
    "eslint": "^8.35.0",
    "eslint-config-next": "13.2.3",
    "lint-staged": "^13.1.2",
    "prettier": "^2.8.4",
    "prisma": "^4.11.0",
    "typescript": "^4.9.5"
  },
  "browserslist": [
    "defaults and supports es6-module"
  ],
  "ct3aMetadata": {
    "initVersion": "7.4.0"
  }
}
"#;
