use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::io;

/// Stores the original terminal attributes so we can restore them on panic/signals.
///
/// This is intentionally best-effort: saving/restoring termios can fail (e.g. stdin closed),
/// but restoring when possible prevents leaving the user's terminal in an unusable state.
static ORIGINAL_TERMIOS: Lazy<Mutex<Option<libc::termios>>> = Lazy::new(|| Mutex::new(None));

/// Save the current terminal attributes once. Safe to call multiple times.
pub fn save_original_termios() -> io::Result<()> {
    let mut guard = ORIGINAL_TERMIOS.lock().unwrap();
    if guard.is_some() {
        return Ok(());
    }

    let mut t: libc::termios = unsafe { std::mem::zeroed() };
    let r = unsafe { libc::tcgetattr(libc::STDIN_FILENO, &mut t as *mut libc::termios) };
    if r != 0 {
        return Err(io::Error::last_os_error());
    }

    *guard = Some(t);
    Ok(())
}

/// Attempt to restore the saved termios. Best-effort; ignore errors.
pub fn restore_original_termios() {
    let guard = ORIGINAL_TERMIOS.lock().unwrap();
    if let Some(orig) = guard.as_ref() {
        // use TCSANOW to apply immediately; ignore return value (best-effort)
        let _ = unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, orig as *const libc::termios) };
    }
}
