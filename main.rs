use std::{
    env,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    process::{self, Command, ExitStatus, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

const INSTRUCTIONS: &str = include_str!("instructions.md");

fn main() {
    if let Err(error) = run() {
        eprintln!("codex-commit: {error}");
        process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let repo_dir = repo_dir()?;
    let instructions = instructions();
    let commit_file = TempFile::create()?;

    env::set_current_dir(&repo_dir)?;
    run_command(Command::new("git").args(["add", "."]), "git add .")?;

    print!("thinking...");
    io::stdout().flush()?;

    let status = Command::new("codex")
        .args(["exec", "--cd"])
        .arg(&repo_dir)
        .args(["--ephemeral", "--output-last-message"])
        .arg(commit_file.path())
        .arg(instructions)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    ensure_success(status, "codex exec")?;

    let mut commit = fs::read_to_string(commit_file.path())?;
    trim_trailing_newlines(&mut commit);

    print!("\r\x1b[K");
    println!("{commit}");
    print!(": ");
    io::stdout().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;
    trim_trailing_newlines(&mut response);

    if !response.is_empty() {
        commit = response;
    }

    run_command(
        Command::new("git").arg("commit").arg("-m").arg(commit),
        "git commit",
    )
}

fn repo_dir() -> io::Result<PathBuf> {
    let fallback = env::current_dir()?;
    let Ok(output) = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .stderr(Stdio::null())
        .output()
    else {
        return Ok(fallback);
    };

    if !output.status.success() {
        return Ok(fallback);
    }

    let mut stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    trim_trailing_newlines(&mut stdout);

    if stdout.is_empty() {
        Ok(fallback)
    } else {
        Ok(PathBuf::from(stdout))
    }
}

fn instructions() -> String {
    let mut instructions = INSTRUCTIONS.to_owned();
    trim_trailing_newlines(&mut instructions);
    instructions
}

fn run_command(command: &mut Command, label: &str) -> io::Result<()> {
    let status = command.status()?;
    ensure_success(status, label)
}

fn ensure_success(status: ExitStatus, label: &str) -> io::Result<()> {
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("{label} failed with {status}")))
    }
}

fn trim_trailing_newlines(value: &mut String) {
    while value.ends_with(['\n', '\r']) {
        value.pop();
    }
}

struct TempFile {
    path: PathBuf,
}

impl TempFile {
    fn create() -> io::Result<Self> {
        let temp_dir = env::temp_dir();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| io::Error::other(format!("system clock before Unix epoch: {error}")))?
            .as_nanos();

        for attempt in 0..100 {
            let path = temp_dir.join(format!(
                "codex-commit-{}-{timestamp}-{attempt}",
                process::id()
            ));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(_) => return Ok(Self { path }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }

        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not create a temporary commit file",
        ))
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
