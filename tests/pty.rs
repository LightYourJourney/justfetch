//! Actual terminal capability tests on Unix; no shell or external test tools.
#![cfg(unix)]

use justfetch::render::layout::{sanitize, width};
use std::{
    ffi::OsStr,
    fs::File,
    io::{self, Read},
    os::fd::FromRawFd,
    process::{Command, Stdio},
};

fn run(columns: u16, args: &[&str], no_color: Option<&OsStr>, term: &str) -> String {
    let (mut master, mut slave) = (-1, -1);
    let mut size = libc::winsize {
        ws_row: 30,
        ws_col: columns,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // SAFETY: descriptor pointers and read-only winsize are valid. NULL name and
    // termios request defaults. Both descriptors are owned only after success.
    let result = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::addr_of_mut!(size),
        )
    };
    assert_eq!(result, 0);
    // SAFETY: openpty returned these distinct owned descriptors successfully.
    let mut master = unsafe { File::from_raw_fd(master) };
    // SAFETY: ownership of the other descriptor is transferred exactly once.
    let slave = unsafe { File::from_raw_fd(slave) };
    let mut command = Command::new(env!("CARGO_BIN_EXE_justfetch"));
    command
        .args(args)
        .stdout(Stdio::from(slave))
        .stderr(Stdio::piped())
        .env("TERM", term)
        // The converter decides the depth, and inheriting the developer's value
        // would make the 256-colour test depend on their shell.
        .env_remove("COLORTERM")
        .env("LC_ALL", "en_US.UTF-8");
    match no_color {
        Some(value) => {
            command.env("NO_COLOR", value);
        }
        None => {
            command.env_remove("NO_COLOR");
        }
    }
    let child = command.spawn().unwrap();
    drop(command); // close the parent's copy of the slave so reads reach EOF.
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        match master.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => bytes.extend_from_slice(&buffer[..n]),
            Err(error) if error.raw_os_error() == Some(libc::EIO) => break, // Linux PTY EOF.
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => panic!("PTY read: {error}"),
        }
    }
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    // A PTY with OPOST delivers each newline as CRLF, and a capture can carry a
    // further trailing CR at a line end depending on the platform. Strip only
    // that trailing one: deleting every CR would also hide a carriage return
    // that leaked into a line from the data, which is a real defect.
    String::from_utf8(bytes)
        .map(|text| {
            text.lines()
                .map(|line| line.strip_suffix('\r').unwrap_or(line))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap()
}

/// Run the binary with a stdout whose reader is already gone, so the first
/// write gets EPIPE instead of fitting into a pipe buffer.
fn run_with_closed_stdout(args: &[&str]) -> std::process::ExitStatus {
    use std::os::fd::FromRawFd;
    let mut fds = [0 as libc::c_int; 2];
    // SAFETY: pipe() fills both descriptors on success and we take ownership of
    // each exactly once below.
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    // SAFETY: the pair is owned here and not used elsewhere.
    let read = unsafe { File::from_raw_fd(fds[0]) };
    // SAFETY: ownership of the other descriptor is transferred exactly once.
    let write = unsafe { File::from_raw_fd(fds[1]) };
    // Closing the only reader first guarantees the child's write gets EPIPE
    // rather than fitting into a pipe buffer.
    drop(read);
    Command::new(env!("CARGO_BIN_EXE_justfetch"))
        .args(args)
        .stdout(Stdio::from(write))
        .status()
        .unwrap()
}

#[test]
fn a_stdout_with_no_reader_exits_successfully() {
    assert!(
        run_with_closed_stdout(&[]).success(),
        "a closed pipe should be a quiet success"
    );
}

/// The subcommands used `println!`, which panics when the write gets EPIPE.
/// They go through the same fallible write path as the main output now.
#[test]
fn subcommands_exit_quietly_when_stdout_has_no_reader() {
    let dir = std::env::temp_dir().join("jf-pty-subcommands");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    let path = path.to_str().unwrap();
    for args in [
        vec!["doctor"],
        vec!["themes"],
        vec!["init", "--config", path],
        vec!["--version"],
    ] {
        let status = run_with_closed_stdout(&args);
        assert!(
            status.success(),
            "{args:?} should be a quiet success, got {status:?}"
        );
    }
}

#[test]
fn tty_colors_and_non_utf8_no_color() {
    use std::os::unix::ffi::OsStrExt;
    assert!(run(80, &[], None, "xterm-256color").contains('\x1b'));
    assert!(!run(80, &[], Some(OsStr::from_bytes(&[0xff])), "xterm-256color").contains('\x1b'));
    assert!(!run(80, &["--no-color"], None, "xterm-256color").contains('\x1b'));
    assert!(!run(80, &[], None, "dumb").contains('\x1b'));
}

#[test]
fn actual_terminal_width_controls_wrapping() {
    for columns in [1, 25, 80] {
        let text = run(columns, &[], None, "xterm-256color");
        // Report the offending line alone: dumping the whole render floods the
        // CI log and gets truncated before the useful part.
        if let Some(line) = text
            .lines()
            .find(|line| width(&sanitize(line)) > usize::from(columns))
        {
            panic!(
                "width {columns}: line {line:?} measures {} columns",
                width(&sanitize(line))
            );
        }
    }
}

#[test]
fn a_theme_is_reduced_to_the_terminals_colour_depth() {
    let deep = run(80, &["--theme", "nord"], None, "xterm-256color");
    assert!(deep.contains("\u{1b}[38;5;"), "256 colours expected");
    assert!(
        !deep.contains("38;2;"),
        "true colour was emitted without asking"
    );

    let basic = run(80, &["--theme", "nord"], None, "xterm");
    assert!(basic.contains('\u{1b}'), "a colour terminal got no colour");
    assert!(
        !basic.contains("38;5;") && !basic.contains("38;2;"),
        "a 16-colour terminal received an extended colour"
    );

    let plain = run(
        80,
        &["--theme", "nord"],
        Some(OsStr::new("1")),
        "xterm-256color",
    );
    assert!(!plain.contains('\u{1b}'));
}

#[test]
fn the_themes_listing_follows_the_same_colour_policy() {
    let coloured = run(80, &["themes"], None, "xterm-256color");
    assert!(
        coloured.contains("\u{1b}[38;5;"),
        "the previews should be colour"
    );

    let plain = run(80, &["themes", "--no-color"], None, "xterm-256color");
    assert!(
        !plain.contains('\u{1b}'),
        "`--no-color` must reach the theme listing too"
    );
}

#[test]
fn the_themes_listing_fits_a_narrow_terminal() {
    let listing = run(40, &["themes", "--no-color"], None, "xterm-256color");
    for line in listing.lines() {
        assert!(
            width(line) <= 40,
            "{} columns in a 40-column terminal: {line:?}",
            width(line)
        );
    }
}

#[test]
fn the_themes_listing_stays_inside_a_very_narrow_terminal() {
    let listing = run(20, &["themes", "--no-color"], None, "xterm-256color");
    for line in listing.lines() {
        assert!(width(line) <= 20, "{} columns in 20: {line:?}", width(line));
    }
}

#[test]
fn a_named_logo_survives_a_narrow_terminal() {
    let narrow = run(
        40,
        &["--logo", "arch", "--no-color", "--modules", "os,cpu"],
        None,
        "xterm",
    );
    assert!(
        narrow.contains("/\\"),
        "the chosen artwork vanished:\n{narrow}"
    );
    assert!(narrow.contains("Example") || narrow.contains("macOS") || narrow.contains("CPU"));
    for line in narrow.lines() {
        assert!(width(line) <= 40, "{} columns: {line:?}", width(line));
    }
    // The automatic choice is still allowed to give way.
    let automatic = run(40, &["--no-color", "--modules", "os,cpu"], None, "xterm");
    assert!(!automatic.contains("/\\"), "{automatic}");
}
