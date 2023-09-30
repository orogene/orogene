//! Creates shims for package bins on Windows. Basically a Rust port of
//! <https://github.com/npm/cmd-shim>.

// The original project is licensed as follows:
//
// The ISC License
//
// Copyright (c) npm, Inc. and Contributors
//
// Permission to use, copy, modify, and/or distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
// ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR
// IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

use std::path::Path;

use once_cell::sync::Lazy;
use regex::Regex;

static SHEBANG_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^#!\s*(?:/usr/bin/env\s+(?:-S\s+)?(?P<vars>(?:[^ \t=]+=[^ \t=]+\s+)*))?(?P<prog>[^ \t]+)(?P<args>.*)$")
        .unwrap()
});

static DOLLAR_EXPR_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\$\{?(?P<var>[^$@#?\- \t{}:]+)\}?").unwrap());

pub fn shim_bin(source: &Path, to: &Path) -> std::io::Result<()> {
    // First, we blow away anything that already exists there.
    // TODO: get rid of .expect()s?
    let from = pathdiff::diff_paths(source, to.parent().expect("must have parent"))
        .expect("paths should be diffable");
    cleanup_existing(to)?;
    if let Ok(contents) = std::fs::read_to_string(source) {
        let mut lines = contents.lines();
        if let Some(first_line) = lines.next() {
            if let Some(captures) = SHEBANG_REGEX.captures(first_line.trim_end()) {
                let vars = captures.name("vars").map(|m| m.as_str());
                let prog = captures.name("prog").map(|m| m.as_str());
                let args = captures.name("args").map(|m| m.as_str());
                return write_shim(&from, to, vars, prog, args);
            }
        }
    }
    write_shim(&from, to, None, None, None)
}

fn cleanup_existing(to: &Path) -> std::io::Result<()> {
    if let Ok(meta) = to.metadata() {
        if meta.is_dir() {
            std::fs::remove_dir_all(to)?;
        } else {
            std::fs::remove_file(to)?;
        }
    }
    let cmd = to.with_extension("cmd");
    if let Ok(meta) = cmd.metadata() {
        if meta.is_dir() {
            std::fs::remove_dir_all(cmd)?;
        } else {
            std::fs::remove_file(cmd)?;
        }
    }
    let ps1 = to.with_extension("ps1");
    if let Ok(meta) = ps1.metadata() {
        if meta.is_dir() {
            std::fs::remove_dir_all(ps1)?;
        } else {
            std::fs::remove_file(ps1)?;
        }
    }
    Ok(())
}

fn write_shim(
    from: &Path,
    to: &Path,
    vars: Option<&str>,
    prog: Option<&str>,
    args: Option<&str>,
) -> std::io::Result<()> {
    write_cmd_shim(from, to, vars, prog, args)?;
    write_sh_shim(from, to, vars, prog, args)?;
    write_pwsh_shim(from, to, vars, prog, args)?;
    Ok(())
}

fn write_cmd_shim(
    from: &Path,
    to: &Path,
    vars: Option<&str>,
    prog: Option<&str>,
    args: Option<&str>,
) -> std::io::Result<()> {
    let mut cmd = concat!(
        "@ECHO off\r\n",
        "GOTO start\r\n",
        ":find_dp0\r\n",
        "SET dp0=%~dp0\r\n",
        "EXIT /b\r\n",
        ":start\r\n",
        "SETLOCAL\r\n",
        "CALL :find_dp0\r\n"
    )
    .to_string();

    let target = format!(
        "\"%dp0%\\{target}\"",
        target = from.display().to_string().replace('/', "\\")
    );
    if let Some(prog) = prog {
        let args = if let Some(args) = args {
            args.trim()
        } else {
            ""
        };
        cmd.push_str(&convert_to_set_commands(vars.unwrap_or("")));
        cmd.push_str("\r\n");
        cmd.push_str(&format!("IF EXIST \"%dp0%\\{prog}.exe\" (\r\n"));
        cmd.push_str(&format!("  SET \"_prog=%dp0%\\{prog}.exe\"\r\n"));
        cmd.push_str(") ELSE (\r\n");
        cmd.push_str(&format!(
            "  SET \"_prog={}\"\r\n",
            prog.trim_start_matches('"').trim_end_matches('"')
        ));
        cmd.push_str("  SET PATHEXT=%PATHEXT:;.JS;=;%\r\n");
        cmd.push_str(")\r\n");
        cmd.push_str("\r\n");
        cmd.push_str("endLocal & goto #_undefined_# 2>NUL || title %COMSPEC% & ");
        cmd.push_str(&format!("\"%_prog%\" {args} {target} %*\r\n",));
    } else {
        cmd.push_str(&format!("{target} %*\r\n",));
    }

    std::fs::write(to.with_extension("cmd"), cmd)?;

    Ok(())
}

fn write_sh_shim(
    from: &Path,
    to: &Path,
    vars: Option<&str>,
    prog: Option<&str>,
    args: Option<&str>,
) -> std::io::Result<()> {
    let mut sh = concat!(
        "#!/bin/sh\n",
        r#"basedir = $(dirname "$(echo "$0" | sed -e 's,\\,/,g')")"#,
        "\n\n",
        "case `uname` in\n",
        "    *CYGWIN*|*MINGW*|*MSYS*) basedir=`cygpath -w \"$basedir\"`;;\n",
        "esac\n\n"
    )
    .to_string();

    let args = args.unwrap_or("");
    let vars = vars.unwrap_or("");
    let target = from.display().to_string().replace('\\', "/");
    if let Some(prog) = prog {
        let long_prog = format!("\"$basedir/{prog}\"");
        let prog = prog.replace('\\', "/");
        sh.push_str(&format!("if [ -x {long_prog} ]; then\n"));
        sh.push_str(&format!(
            "  exec {vars}{long_prog} {args} \"$basedir/{target}\" \"$@\"\n"
        ));
        sh.push_str("else \n");
        sh.push_str(&format!(
            "  exec {vars}{prog} {args} \"$basedir/{target}\" \"$@\"\n"
        ));
        sh.push_str("fi\n");
    } else {
        sh.push_str(&format!("exec \"$basedir/{target}\" {args} \"$@\"\n"));
    }

    std::fs::write(to, sh)?;

    Ok(())
}

fn write_pwsh_shim(
    from: &Path,
    to: &Path,
    vars: Option<&str>,
    prog: Option<&str>,
    args: Option<&str>,
) -> std::io::Result<()> {
    let mut pwsh = concat!(
        "#!/usr/bin/env pwsh\n",
        "$basedir=Split-Path $MyInvocation.MyCommand.Definition -Parent\n",
        "\n",
        "$exe=\"\"\n",
        "if ($PSVersionTable.PSVersion -lt \"6.0\" -or $IsWindows) {\n",
        "  # Fix case when both the Windows and Linux builds of Node\n",
        "  # are installed in the same directory\n",
        "  $exe=\".exe\"\n",
        "}\n"
    )
    .to_string();

    let args = args.unwrap_or("");
    let target = from.display().to_string().replace('\\', "/");
    if let Some(prog) = prog {
        let long_prog = format!("\"$basedir/{prog}$exe\"");
        let prog = format!("\"{}\"$exe", prog.replace('\\', "/"));
        pwsh.push_str(&convert_to_env_commands(vars.unwrap_or("")));
        pwsh.push_str("$ret=0\n");
        pwsh.push_str(&format!("if (Test-Path {long_prog}) {{\n"));
        pwsh.push_str("  # Support pipeline input\n");
        pwsh.push_str("  if ($MyInvocation.ExpectingInput) {\n");
        pwsh.push_str(&format!(
            "    $input | & {long_prog} {args} \"$basedir/{target}\" $args\n"
        ));
        pwsh.push_str("  } else {\n");
        pwsh.push_str(&format!(
            "    & {long_prog} {args} \"$basedir/{target}\" $args\n"
        ));
        pwsh.push_str("  }\n");
        pwsh.push_str("  $ret=$LASTEXITCODE\n");
        pwsh.push_str("} else {\n");
        pwsh.push_str("  # Support pipeline input\n");
        pwsh.push_str("  if ($MyInvocation.ExpectingInput) {\n");
        pwsh.push_str(&format!(
            "    $input | & {prog} {args} \"$basedir/{target}\" $args\n"
        ));
        pwsh.push_str("  } else {\n");
        pwsh.push_str(&format!(
            "    & {prog} {args} \"$basedir/{target}\" $args\n"
        ));
        pwsh.push_str("  }\n");
        pwsh.push_str("  $ret=$LASTEXITCODE\n");
        pwsh.push_str("}\n");
        pwsh.push_str("exit $ret\n");
    } else {
        pwsh.push_str("# Support pipeline input\n");
        pwsh.push_str("if ($MyInvocation.ExpectingInput) {\n");
        pwsh.push_str(&format!("  $input | & \"$basedir/{target}\" $args\n"));
        pwsh.push_str("} else {\n");
        pwsh.push_str(&format!("  & \"$basedir/{target}\" $args\n"));
        pwsh.push_str("}\n");
        pwsh.push_str("exit $LASTEXITCODE\n");
    }

    std::fs::write(to.with_extension("ps1"), pwsh)?;

    Ok(())
}

fn convert_to_set_commands(variables: &str) -> String {
    let mut var_declarations_as_batch = String::new();
    for var_str in variables.split_whitespace() {
        let mut parts = var_str.splitn(2, '=');
        if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
            var_declarations_as_batch.push_str(&convert_to_set_command(key, value));
        }
    }
    var_declarations_as_batch
}

fn convert_to_set_command(key: &str, value: &str) -> String {
    let key = key.trim();
    let value = value.trim();
    if key.is_empty() || value.is_empty() {
        String::new()
    } else {
        format!("@SET {key}={}\r\n", replace_dollar_with_percent_pair(value))
    }
}

fn replace_dollar_with_percent_pair(value: &str) -> String {
    let mut result = String::new();
    let mut start_idx = 0;
    for capture in DOLLAR_EXPR_REGEX.captures_iter(value) {
        let mat = capture
            .get(0)
            .expect("If we had a capture, there should be a 0-match");
        result.push_str(&value[start_idx..mat.start()]);
        result.push('%');
        result.push_str(&capture["var"]);
        result.push('%');
        start_idx = mat.end();
    }
    result.push_str(&value[start_idx..]);
    result
}

fn convert_to_env_commands(variables: &str) -> String {
    let mut var_declarations_as_batch = String::new();
    for var_str in variables.split_whitespace() {
        let mut parts = var_str.splitn(2, '=');
        if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
            var_declarations_as_batch.push_str(&convert_to_env_command(key, value));
        }
    }
    var_declarations_as_batch
}

fn convert_to_env_command(key: &str, value: &str) -> String {
    let key = key.trim();
    let value = value.trim();
    if key.is_empty() || value.is_empty() {
        String::new()
    } else {
        format!(
            "$env:{key}=\"{}\"\n",
            replace_with_string_interpolation(value)
        )
    }
}

fn replace_with_string_interpolation(value: &str) -> String {
    let mut result = String::new();
    let mut start_idx = 0;
    for capture in DOLLAR_EXPR_REGEX.captures_iter(value) {
        let mat = capture
            .get(0)
            .expect("If we had a capture, there should be a 0-match");
        result.push_str(&value[start_idx..mat.start()]);
        // This doesn't _necessarily_ have to be env:, but it's the most
        // likely/sensible one, so we just go with it.
        result.push_str("${env:");
        result.push_str(&capture["var"]);
        result.push('}');
        start_idx = mat.end();
    }
    result.push_str(&value[start_idx..]);
    result.replace('\"', "`\"")
}
