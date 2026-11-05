// Licensed under the MIT License.

//! Test suite for the top-level `tectonic` executable.

use lazy_static::lazy_static;
use std::io::ErrorKind;
use std::{
    env,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    str,
};
use tempfile::TempDir;

#[path = "util/mod.rs"]
mod util;
use crate::util::{cargo_dir, ensure_plain_format};

lazy_static! {
    static ref TEST_ROOT: PathBuf = {
        util::set_test_root();

        let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        root.push("tests");
        root
    };

    static ref TARGET_RUNNER_WORDS: Vec<String> = {
        // compile-time environment variable from build.rs:
        let target = env!("TARGET").to_owned();
        let mut target = target.replace('-', "_");
        target.make_ascii_uppercase();

        // run-time environment variable check:
        if let Ok(runtext) = env::var(format!("CARGO_TARGET_{target}_RUNNER")) {
            runtext.split_whitespace().map(|x| x.to_owned()).collect()
        } else {
            vec![]
        }
    };
}

fn get_plain_format_arg() -> String {
    util::set_test_root();
    let path = ensure_plain_format().expect("couldn't write format file");
    format!("--format={}", path.display())
}

/// Note the special sauce here — we set the magic environment variable that
/// tells the Tectonic binary to go into "test mode" and use local test
/// assets, rather than an actual network bundle.
fn prep_tectonic(cwd: &Path, args: &[&str]) -> Command {
    let tectonic = cargo_dir()
        .join("texyon")
        .with_extension(env::consts::EXE_EXTENSION);

    if fs::metadata(&tectonic).is_err() {
        panic!("texyon binary not found at {tectonic:?}. Do you need to run `cargo build`?")
    }
    println!("using tectonic binary at {tectonic:?}");
    println!("using cwd {cwd:?}");

    // We may need to wrap the Tectonic invocation. If we're cross-compiling, we
    // might need to use something like QEMU to actually be able to run the
    // executable.
    let mut command = if !TARGET_RUNNER_WORDS.is_empty() {
        let mut cmd = Command::new(&TARGET_RUNNER_WORDS[0]);
        cmd.args(&TARGET_RUNNER_WORDS[1..]).arg(tectonic);
        cmd
    } else {
        Command::new(tectonic)
    };

    command
        .arg("compile")
        .args(args)
        .current_dir(cwd)
        .env(texyon::test_util::TEST_ROOT_ENV_VAR, TEST_ROOT.as_os_str());
    command
}

fn run_tectonic(cwd: &Path, args: &[&str]) -> Output {
    let mut command = prep_tectonic(cwd, args);
    command.env("BROWSER", "echo");
    println!("running {command:?}");
    command.output().expect("tectonic failed to start")
}

fn run_tectonic_with_stdin(cwd: &Path, args: &[&str], stdin: &str) -> Output {
    let mut command = prep_tectonic(cwd, args);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    println!("running {command:?}");
    let mut child = command.spawn().expect("tectonic failed to start");
    match write!(child.stdin.as_mut().unwrap(), "{stdin}") {
        Ok(_) => (),
        // Ignore if the child already died
        Err(e) if e.kind() == ErrorKind::BrokenPipe => (),
        Err(e) => panic!("failed to send data to tectonic subprocess: {e:?}"),
    }
    child
        .wait_with_output()
        .expect("failed to wait on tectonic subprocess")
}

fn setup_and_copy_files(files: &[&str]) -> TempDir {
    let tempdir = tempfile::Builder::new()
        .prefix("texyon_executable_test")
        .tempdir()
        .unwrap();

    let executable_test_dir = if let Some(v) = env::var_os("CARGO_MANIFEST_DIR") {
        PathBuf::from(v)
    } else {
        PathBuf::new()
    }
    .join("tests/executable");

    for file in files {
        // Create parent directories, if the file is not at the root of `tests/executable/`
        let file_path = PathBuf::from(file);
        let parent_dir = file_path.parent().unwrap();
        let mut dirbuilder = fs::DirBuilder::new();
        dirbuilder.recursive(true);
        dirbuilder.create(tempdir.path().join(parent_dir)).unwrap();

        fs::copy(executable_test_dir.join(file), tempdir.path().join(file)).unwrap();
    }

    tempdir
}

fn success_or_panic(output: &Output) {
    if output.status.success() {
        println!("status: {}", output.status);
        println!("stdout:\n{}", String::from_utf8_lossy(&output.stdout));
        println!("stderr:\n{}", String::from_utf8_lossy(&output.stderr));
    } else {
        panic!(
            "Command exited badly:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn error_or_panic(output: &Output) {
    if !output.status.success() {
        println!("status: {}", output.status);
        println!("stdout:\n{}", String::from_utf8_lossy(&output.stdout));
        println!("stderr:\n{}", String::from_utf8_lossy(&output.stderr));
    } else {
        panic!(
            "Command should have failed but didn't:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn check_file(tempdir: &TempDir, rest: &str) {
    let mut p = tempdir.path().to_owned();
    p.push(rest);

    if !p.is_file() {
        panic!(
            "file \"{}\" should have been created but wasn\'t",
            p.to_string_lossy()
        );
    }
}

/* Keep tests alphabetized */

#[test]
fn bad_chatter_1() {
    let output = run_tectonic(&PathBuf::from("."), &["-", "--chatter=reticent"]);
    error_or_panic(&output);
}

#[test]
fn bad_input_path_1() {
    let output = run_tectonic(&PathBuf::from("."), &["/"]);
    error_or_panic(&output);
}

#[test]
fn bad_input_path_2() {
    let output = run_tectonic(&PathBuf::from("."), &["somedir/.."]);
    error_or_panic(&output);
}

#[test]
fn bad_outfmt_1() {
    let output = run_tectonic(&PathBuf::from("."), &["-", "--outfmt=dd"]);
    error_or_panic(&output);
}

fn run_with_biber(args: &str, stdin: &str) -> Output {
    run_with_biber_exe(None, args, stdin, &["subdirectory/empty.bib"])
}

fn run_with_biber_exe(executable: Option<&str>, args: &str, stdin: &str, files: &[&str]) -> Output {
    let fmt_arg = get_plain_format_arg();
    let tempdir = setup_and_copy_files(files);
    let mut command = prep_tectonic(tempdir.path(), &[&fmt_arg, "-"]);

    let test_cmd = if let Some(exe) = executable {
        format!("{exe} {args}")
    } else if cfg!(windows) {
        format!(
            "cmd /c {} {}",
            util::test_path(&["fake-biber.bat"]).display(),
            args
        )
    } else {
        format!("{} {}", util::test_path(&["fake-biber.sh"]).display(), args)
    };

    command.env("TEXYON_TEST_FAKE_BIBER", &test_cmd);

    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    println!("running {command:?}");
    let mut child = command.spawn().expect("tectonic failed to start");

    write!(child.stdin.as_mut().unwrap(), "{stdin}")
        .expect("failed to send data to tectonic subprocess");

    child
        .wait_with_output()
        .expect("failed to wait on tectonic subprocess")
}

const BIBER_TRIGGER_TEX: &str = r#"
% Test if we're on the second pass by seeing if the BCF already exists
\newif\ifsecond
\newread\r
\openin\r=texput.bcf
\ifeof\r
\message{first pass}
\secondfalse
\else
\message{second pass}
\secondtrue
\closein\r
\fi

% Now create BCF
\newwrite\w
\immediate\openout\w=texput.bcf\relax
\immediate\write\w{Hello BCF file}
\immediate\closeout\w

% Now create run.xml
\immediate\openout\w=texput.run.xml\relax
\immediate\write\w{
<requests version="1.0">
    <external package="biblatex" priority="5" active="1">
        <generic>biber</generic>
        <cmdline>
            <binary>biber</binary>
            <infile>texput</infile>
        </cmdline>
        <input>
            <file>texput.bcf</file>
        </input>
        <output>
            <file>texput.bbl</file>
        </output>
        <provides type="dynamic">
            <file>texput.bbl</file>
        </provides>
        <requires type="dynamic">
            <file>texput.bcf</file>
        </requires>
        <requires type="editable">
            <file>subdirectory/empty.bib</file>
        </requires>
    </external>
</requests>
}
\immediate\closeout\w
"#;

#[test]
fn biber_failure() {
    let output = run_with_biber("failure", BIBER_TRIGGER_TEX);
    error_or_panic(&output);
}

#[test]
fn biber_no_such_tool() {
    const REST: &str = r"\bye";
    let tex = format!("{BIBER_TRIGGER_TEX}{REST}");
    let output = run_with_biber_exe(Some("ohnothereisnobiberprogram"), "", &tex, &[]);
    error_or_panic(&output);
}

#[cfg(unix)]
#[test]
fn biber_signal() {
    let output = run_with_biber("signal", BIBER_TRIGGER_TEX);
    error_or_panic(&output);
}

const BIBER_VALIDATE_TEX: &str = r"
\ifsecond
\ifnum\input{biberout.qqq}=456\relax
a
\else
\ohnothebiberdidntwork
\fi
\fi
\bye";

#[test]
fn biber_success() {
    let tex = format!("{BIBER_TRIGGER_TEX}{BIBER_VALIDATE_TEX}");
    let output = run_with_biber("success", &tex);
    success_or_panic(&output);
}

/// Test `tectonic-biber` override: when no args passed, fall back to $PATH
/// lookup for `tectonic-biber` first, and then `biber`. Currently defined in:
/// [`texyon::driver::ProcessingSession::check_biber_requirement`]
#[cfg(unix)]
#[test]
fn biber_tectonic_override() {
    let tex = format!("{BIBER_TRIGGER_TEX}{BIBER_VALIDATE_TEX}");
    let output = run_with_biber_exe(
        Some(""),
        "", // no args passed
        &tex,
        &["subdirectory/empty.bib", "tectonic-biber"],
    );
    success_or_panic(&output);
}

/// #844: biber input with absolute path blows away the file
///
/// We need to create a separate temporary directory to see if the abspath input
/// gets messed up.
///
/// Backslashes in Windows paths mess up our naive test. I can't figure out how
/// to get them to work here (admittedly, not trying too hard) so I'm just
/// skipping this test on that platform.
#[test]
#[cfg(not(windows))]
fn biber_issue_844() {
    let temp_source = setup_and_copy_files(&[]);
    let mut bibpath = std::fs::canonicalize(temp_source.path()).unwrap();
    bibpath.push("single_entry.bib");

    let contents = include_str!("bibtex/cites/single_entry.bib");
    std::fs::write(&bibpath, contents.as_bytes()).unwrap();

    // Futz the basic template to reference our absolute path input file:
    let tex = format!(
        "{}{}",
        BIBER_TRIGGER_TEX.replace(">texput.bcf<", &format!(">{}<", bibpath.to_str().unwrap())),
        "kthx\\bye"
    );

    let output = run_with_biber("success", &tex);
    success_or_panic(&output);

    let stat = std::fs::metadata(&bibpath).unwrap();
    assert_eq!(stat.len(), contents.len() as u64);
}

/// Fakily testing the bibtex implementation even though we don't have LaTeX.
/// This implementation is strongly cribbed from the biber tests, which came
/// first.
#[test]
fn bibtex_multiple_aux_files() {
    let fmt_arg = get_plain_format_arg();
    let tempdir =
        setup_and_copy_files(&["bibtex_multiple_aux_files.tex", "refs.bib", "catchkey.bst"]);
    let output = run_tectonic(tempdir.path(), &[&fmt_arg, "bibtex_multiple_aux_files.tex"]);
    success_or_panic(&output);
}

#[test]
fn help_flag() {
    let output = run_tectonic(&PathBuf::from("."), &["-h"]);
    success_or_panic(&output);
}

#[test]
fn keep_logs_on_error() {
    // No input files here, but output files are created.
    let fmt_arg = get_plain_format_arg();
    let tempdir = setup_and_copy_files(&[]);
    let output = run_tectonic_with_stdin(
        tempdir.path(),
        &[&fmt_arg, "-", "--keep-logs"],
        "no end to this file",
    );
    error_or_panic(&output);

    let mut log = String::new();
    File::open(tempdir.path().join("texput.log"))
        .expect("`texput.log` not found")
        .read_to_string(&mut log)
        .expect("Cannot read `texput.log`");

    assert!(log.contains(r"job aborted, no legal \end found"));
}

#[test]
fn no_color_option() {
    // No input files here, but output files are created.
    let fmt_arg = get_plain_format_arg();

    let tempdir = setup_and_copy_files(&[]);
    let output_nocolor = run_tectonic_with_stdin(
        tempdir.path(),
        &[&fmt_arg, "-", "--color=never"],
        "no end to this file",
    );

    // Output is not a terminal, so these two should be the same
    let tempdir = setup_and_copy_files(&[]);
    let output_autocolor = run_tectonic_with_stdin(
        tempdir.path(),
        &[&fmt_arg, "-", "--color=auto"],
        "no end to this file",
    );

    assert_eq!(output_nocolor, output_autocolor);

    error_or_panic(&output_nocolor);
    error_or_panic(&output_autocolor);
}

#[test]
fn outdir_option() {
    let fmt_arg = get_plain_format_arg();
    let tempdir = setup_and_copy_files(&["subdirectory/content/1.tex"]);

    let output = run_tectonic(
        tempdir.path(),
        &[
            &fmt_arg,
            "subdirectory/content/1.tex",
            "--outdir=subdirectory",
        ],
    );
    success_or_panic(&output);
    check_file(&tempdir, "subdirectory/1.pdf");
}

#[test]
#[should_panic]
// panic unwinding broken: https://github.com/rust-embedded/cross/issues/343
#[cfg(not(all(target_arch = "arm", target_env = "musl")))]
fn outdir_option_bad() {
    let fmt_arg = get_plain_format_arg();
    let tempdir = setup_and_copy_files(&["subdirectory/content/1.tex"]);

    let output = run_tectonic(
        tempdir.path(),
        &[
            &fmt_arg,
            "subdirectory/content/1.tex",
            "--outdir=subdirectory/non_existent",
        ],
    );
    success_or_panic(&output);
}

#[test]
#[should_panic]
// panic unwinding broken: https://github.com/rust-embedded/cross/issues/343
#[cfg(not(all(target_arch = "arm", target_env = "musl")))]
fn outdir_option_is_file() {
    let fmt_arg = get_plain_format_arg();
    let tempdir = setup_and_copy_files(&["test space.tex", "subdirectory/content/1.tex"]);

    let output = run_tectonic(
        tempdir.path(),
        &[
            &fmt_arg,
            "subdirectory/content/1.tex",
            "--outdir=test space.tex",
        ],
    );
    success_or_panic(&output);
}

#[test] // GitHub #31
fn relative_include() {
    let fmt_arg = get_plain_format_arg();
    let tempdir = setup_and_copy_files(&[
        "subdirectory/relative_include.tex",
        "subdirectory/content/1.tex",
    ]);

    let output = run_tectonic(
        tempdir.path(),
        &[&fmt_arg, "subdirectory/relative_include.tex"],
    );
    success_or_panic(&output);
    check_file(&tempdir, "subdirectory/relative_include.pdf");
}

// Regression #36
#[test]
fn space_in_filename() {
    let fmt_arg = get_plain_format_arg();
    let tempdir = setup_and_copy_files(&["test space.tex"]);

    let output = run_tectonic(tempdir.path(), &[&fmt_arg, "test space.tex"]);
    success_or_panic(&output);
}

#[test]
fn stdin_content() {
    // No input files here, but output files are created.
    let fmt_arg = get_plain_format_arg();
    let tempdir = setup_and_copy_files(&[]);
    let output = run_tectonic_with_stdin(
        tempdir.path(),
        &[&fmt_arg, "-"],
        "Standard input content.\\bye",
    );
    success_or_panic(&output);
}

/// Test where `--bundle` may go on the command line
#[test]
fn bundle_overrides() {
    let filename = "subdirectory/content/1.tex";
    let fmt_arg: &str = &get_plain_format_arg();
    let tempdir = setup_and_copy_files(&[filename]);
    let temppath = tempdir.path().to_owned();

    let arg_bad_bundle = ["--bundle", "bad-bundle"];
    let arg_good_bundle = ["--bundle", "test-bundle://"];

    // test with a bad bundle
    let output = run_tectonic(
        &temppath,
        &[&arg_bad_bundle[..], &[fmt_arg, filename]].concat(),
    );
    error_or_panic(&output);

    // test with a good bundle (override)
    let valid_args: Vec<Vec<&str>> = vec![
        // different positions
        [&arg_good_bundle[..], &[fmt_arg, filename]].concat(),
        [&[fmt_arg], &arg_good_bundle[..], &[filename]].concat(),
        [&[fmt_arg], &[filename], &arg_good_bundle[..]].concat(),
    ];

    for args in valid_args {
        let output = run_tectonic(&temppath, &args);
        success_or_panic(&output);
    }
}

const SHELL_ESCAPE_TEST_DOC: &str = r"\immediate\write18{mkdir shellwork}
\immediate\write18{echo 123 >shellwork/persist}
\ifnum123=\input{shellwork/persist}
a
\else
\ohnotheshellescapedidntwork
\fi
\bye
";

/// Test that shell escape actually runs the commands
#[test]
fn shell_escape() {
    let fmt_arg = get_plain_format_arg();
    let tempdir = setup_and_copy_files(&[]);

    let output = run_tectonic_with_stdin(
        tempdir.path(),
        &[&fmt_arg, "-", "-Zshell-escape"],
        SHELL_ESCAPE_TEST_DOC,
    );
    success_or_panic(&output);
}

/// Initial revisions with shell-escape ignored any value specified.
/// Rather than allow this to toggle shell-escape which won't work with old installs.
/// Test that shell-escape=false gives an error.
#[test]
fn shell_escape_arg_err() {
    let fmt_arg = get_plain_format_arg();
    let tempdir = setup_and_copy_files(&[]);

    let output = run_tectonic_with_stdin(
        tempdir.path(),
        &[&fmt_arg, "-", "-Zshell-escape=false"],
        SHELL_ESCAPE_TEST_DOC,
    );
    error_or_panic(&output);
}

/// Test that shell-escape can be killed by command-line-option
#[test]
fn shell_escape_cli_override() {
    let fmt_arg = get_plain_format_arg();
    let tempdir = setup_and_copy_files(&[]);

    let output = run_tectonic_with_stdin(
        tempdir.path(),
        &[&fmt_arg, "--untrusted", "-", "-Zshell-escape"],
        SHELL_ESCAPE_TEST_DOC,
    );
    error_or_panic(&output);
}

/// Test that shell-escape can be killed by environment variable
#[test]
fn shell_escape_env_override() {
    let fmt_arg = get_plain_format_arg();
    let tempdir = setup_and_copy_files(&[]);

    // Note that we intentionally set the variable to 0 below -- it takes it
    // effect if it has ANY value, not just a "truthy" one.

    let mut command = prep_tectonic(tempdir.path(), &[&fmt_arg, "-", "-Zshell-escape"]);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("TEXYON_UNTRUSTED_MODE", "0");

    println!("running {command:?}");
    let mut child = command.spawn().expect("tectonic failed to start");
    write!(child.stdin.as_mut().unwrap(), "{SHELL_ESCAPE_TEST_DOC}")
        .expect("failed to send data to tectonic subprocess");

    let output = child
        .wait_with_output()
        .expect("failed to wait on tectonic subprocess");

    error_or_panic(&output);
}

/// Test that include paths work
#[test]
fn extra_search_paths() {
    let fmt_arg = get_plain_format_arg();
    let tempdir = setup_and_copy_files(&["subdirectory/content/1.tex"]);

    let output = run_tectonic_with_stdin(
        tempdir.path(),
        &[&fmt_arg, "-", "-Zsearch-path=subdirectory/content"],
        "\\input 1.tex\n\\bye",
    );
    success_or_panic(&output);

    let output = run_tectonic_with_stdin(
        tempdir.path(),
        &[
            &fmt_arg,
            "-",
            "--hide=subdirectory/content/1.tex",
            "-Zsearch-path=subdirectory/content",
        ],
        "\\input 1.tex\n\\bye",
    );
    error_or_panic(&output);

    let output = run_tectonic_with_stdin(
        tempdir.path(),
        &[
            &fmt_arg,
            "-",
            "-Zsearch-path=subdirectory/content",
            "--untrusted",
        ],
        "\\input 1.tex\n\\bye",
    );
    error_or_panic(&output);
}
