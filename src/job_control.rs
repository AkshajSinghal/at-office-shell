use once_cell::sync::Lazy;
use std::process::Command;
use std::sync::atomic::{AtomicI32, Ordering};
use std::thread;
use std::io::{self, Read, Write};
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use libc;

use crate::tty;

static FG_PGID: Lazy<AtomicI32> = Lazy::new(|| AtomicI32::new(0));

pub fn init_signal_handlers() {
    static START: std::sync::Once = std::sync::Once::new();
    START.call_once(|| {
        // Spawn a thread to handle signals using signal-hook's iterator
        let _ = thread::spawn(|| {
            let mut signals = match signal_hook::iterator::Signals::new(&[
                libc::SIGCHLD,
                libc::SIGINT,
                libc::SIGTSTP,
                libc::SIGTERM,
                libc::SIGQUIT,
                libc::SIGHUP,
            ]) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to register signal handlers: {}", e);
                    return;
                }
            };

            for signal in signals.forever() {
                match signal {
                    libc::SIGCHLD => {
                        // Reap any finished children (non-blocking)
                        loop {
                            let mut status: libc::c_int = 0;
                            let pid = unsafe { libc::waitpid(-1, &mut status as *mut i32, libc::WNOHANG) };
                            if pid <= 0 {
                                break;
                            }
                        }
                    }
                    libc::SIGINT => {
                        let pgid = FG_PGID.load(Ordering::SeqCst);
                        if pgid > 0 {
                            unsafe { libc::kill(-(pgid as libc::pid_t), libc::SIGINT) };
                        }
                    }
                    libc::SIGTSTP => {
                        let pgid = FG_PGID.load(Ordering::SeqCst);
                        if pgid > 0 {
                            unsafe { libc::kill(-(pgid as libc::pid_t), libc::SIGTSTP) };
                        }
                    }
                    libc::SIGTERM | libc::SIGQUIT | libc::SIGHUP => {
                        // Best-effort: restore terminal state before exiting
                        crate::tty::restore_original_termios();
                        // Exit with a code indicating signal termination.
                        // Use 128 + signo as conventional shell exit code for signals.
                        std::process::exit(128 + signal);
                    }
                    _ => {}
                }
            }
        });
    });
}

pub fn spawn_and_wait(command_name: &str, args: &[String]) {
    // Build the command and ensure the child is placed into its own process group
    let mut cmd = Command::new(command_name);
    cmd.args(args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .before_exec(|| {
            // create new process group for the child
            let r = unsafe { libc::setpgid(0, 0) };
            if r != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });

    match cmd.spawn() {
        Ok(mut child) => {
            let pid = child.id() as i32;
            FG_PGID.store(pid, Ordering::SeqCst);

            // Give terminal control to child process group
            unsafe { libc::tcsetpgrp(libc::STDIN_FILENO, pid) };

            // Wait for the child to exit (blocking)
            let mut status: libc::c_int = 0;
            let _ = unsafe { libc::waitpid(pid, &mut status as *mut i32, 0) };

            // Restore terminal control to the shell
            let shell_pgid = unsafe { libc::getpgrp() };
            unsafe { libc::tcsetpgrp(libc::STDIN_FILENO, shell_pgid) };
            FG_PGID.store(0, Ordering::SeqCst);

            // Attempt to reap the std::process::Child to avoid zombies
            let _ = child.try_wait();
        }
        Err(e) => {
            eprintln!("trushell: command not found '{}': {}", command_name, e);
        }
    }
}
