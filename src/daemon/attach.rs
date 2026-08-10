/// Attach mode: connect to a running torlnk daemon via tmux.
/// If no session exists, starts one. Falls back to printing instructions.

pub async fn run_attach() -> anyhow::Result<()> {
    let session = "torlnk";

    // Check if tmux is available
    let tmux = which::which("tmux").is_ok();

    if !tmux {
        eprintln!("tmux is not installed. Install it to use attach mode.");
        eprintln!("  macOS:  brew install tmux");
        eprintln!("  Ubuntu: sudo apt install tmux");
        return Ok(());
    }

    // Check if session exists
    let has_session = std::process::Command::new("tmux")
        .args(["has-session", "-t", session])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if has_session {
        // Attach to existing session
        let status = std::process::Command::new("tmux")
            .args(["attach-session", "-t", session])
            .status()?;
        if !status.success() {
            anyhow::bail!("failed to attach to tmux session");
        }
    } else {
        // Create new session
        let exe = std::env::current_exe()?;
        let status = std::process::Command::new("tmux")
            .args([
                "new-session",
                "-s", session,
                exe.to_string_lossy().as_ref(),
                "run",
            ])
            .status()?;
        if !status.success() {
            anyhow::bail!("failed to create tmux session");
        }
    }

    Ok(())
}
