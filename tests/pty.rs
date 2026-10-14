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
    String::from_utf8(bytes).unwrap()
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
        assert!(
            text.lines()
                .all(|line| width(&sanitize(line)) <= usize::from(columns)),
            "width {columns}: {text}"
        );
    }
}
