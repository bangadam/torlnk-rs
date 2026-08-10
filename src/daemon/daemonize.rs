/// Daemonize helpers. On Unix, fork + setsid to detach from terminal.
/// On Windows, this is a no-op (rely on terminal services / sc.exe).

#[cfg(unix)]
pub fn daemonize() -> anyhow::Result<()> {
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    // Re-exec self with the same args, detached
    let exe = std::env::current_exe()?;
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut cmd = Command::new(exe);
    cmd.args(&args);
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }

    cmd.spawn()?;
    std::process::exit(0);
}

#[cfg(not(unix))]
pub fn daemonize() -> anyhow::Result<()> {
    anyhow::bail!("daemonize is not supported on this platform");
}
