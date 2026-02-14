//! CEF subprocess helper binary.
//!
//! Minimal executable used by CEF for its subprocess architecture.
//! CEF spawns multiple processes (render, GPU, utility) and by default
//! uses the main executable with different command line arguments.
//!
//! Providing a separate helper binary avoids Bevy/GTK initialisation
//! conflicts and runaway subprocess spawning.

use cef::args::Args;

fn main() {
    let args = Args::new();

    // `execute_process` returns >= 0 for subprocesses (CEF handles them)
    // and < 0 for the browser process (should not happen for the helper).
    let exit_code = cef::execute_process(Some(args.as_main_args()), None, std::ptr::null_mut());

    std::process::exit(if exit_code >= 0 { exit_code } else { 1 });
}
