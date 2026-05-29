use std::{
    env,
    fs::{self, OpenOptions},
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{self, Command, ExitStatus, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::Deserialize;

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

    run_codex(&repo_dir, &instructions, commit_file.path())?;

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

fn run_codex(repo_dir: &Path, instructions: &str, output_path: &Path) -> io::Result<()> {
    let mut child = Command::new("codex")
        .args(["exec", "--cd"])
        .arg(repo_dir)
        .args(["--ephemeral", "--json", "--output-last-message"])
        .arg(output_path)
        .arg(instructions)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("could not read codex stdout"))?;
    let (event_sender, events) = mpsc::channel();

    thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let event = line.map_err(|error| error.to_string());
            if event_sender.send(event).is_err() {
                break;
            }
        }
    });

    let frames = ["-", "\\", "|", "/"];
    let started = Instant::now();
    let mut activity = "starting codex".to_owned();
    let mut read_error = None;
    let mut frame = 0;

    loop {
        while let Ok(event) = events.try_recv() {
            match event {
                Ok(line) => {
                    if let Some(next_activity) = activity_from_event(&line) {
                        activity = next_activity;
                    }
                }
                Err(error) => {
                    read_error = Some(error);
                }
            }
        }

        if let Some(status) = child.try_wait()? {
            clear_line()?;
            if let Some(error) = read_error {
                return Err(io::Error::other(format!(
                    "failed to read codex json output: {error}"
                )));
            }
            return ensure_success(status, "codex exec");
        }

        print!(
            "\rcodex {} {} {}",
            frames[frame % frames.len()],
            activity,
            format_elapsed(started.elapsed())
        );
        io::stdout().flush()?;

        frame += 1;
        thread::sleep(Duration::from_millis(250));
    }
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

fn format_elapsed(elapsed: Duration) -> String {
    let seconds = elapsed.as_secs();
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

fn clear_line() -> io::Result<()> {
    print!("\r\x1b[K");
    io::stdout().flush()
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ThreadEvent {
    #[serde(rename = "thread.started")]
    ThreadStarted {},
    #[serde(rename = "turn.started")]
    TurnStarted {},
    #[serde(rename = "turn.completed")]
    TurnCompleted {},
    #[serde(rename = "turn.failed")]
    TurnFailed { error: ThreadError },
    #[serde(rename = "item.started")]
    ItemStarted { item: ThreadItem },
    #[serde(rename = "item.updated")]
    ItemUpdated { item: ThreadItem },
    #[serde(rename = "item.completed")]
    ItemCompleted { item: ThreadItem },
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize)]
struct ThreadError {
    message: String,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ThreadItem {
    AgentMessage {},
    Reasoning {},
    CommandExecution {
        command: String,
        status: String,
    },
    FileChange {
        changes: Vec<FileUpdateChange>,
        status: String,
    },
    McpToolCall {
        server: String,
        tool: String,
        status: String,
    },
    CollabToolCall {
        tool: String,
        status: String,
    },
    WebSearch {
        query: String,
    },
    TodoList {},
    Error {
        message: String,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize)]
struct FileUpdateChange {
    path: String,
}

fn activity_from_event(line: &str) -> Option<String> {
    match serde_json::from_str(line).ok()? {
        ThreadEvent::ThreadStarted {} => Some("starting thread".to_owned()),
        ThreadEvent::TurnStarted {} => Some("thinking".to_owned()),
        ThreadEvent::TurnCompleted {} => Some("finished".to_owned()),
        ThreadEvent::TurnFailed { error } => {
            Some(format!("failed: {}", truncate(&error.message, 80)))
        }
        ThreadEvent::Error { message } => Some(format!("error: {}", truncate(&message, 80))),
        ThreadEvent::ItemStarted { item } | ThreadEvent::ItemUpdated { item } => {
            activity_from_item(item, false)
        }
        ThreadEvent::ItemCompleted { item } => activity_from_item(item, true),
        ThreadEvent::Unknown => None,
    }
}

fn activity_from_item(item: ThreadItem, completed: bool) -> Option<String> {
    match item {
        ThreadItem::AgentMessage {} => Some("writing commit message".to_owned()),
        ThreadItem::Reasoning {} => Some("thinking".to_owned()),
        ThreadItem::TodoList {} => Some("planning".to_owned()),
        ThreadItem::Error { message } => Some(format!("warning: {}", truncate(&message, 80))),
        ThreadItem::WebSearch { query } => {
            if completed {
                Some("thinking".to_owned())
            } else {
                Some(format!("searching web for {}", truncate(&query, 64)))
            }
        }
        ThreadItem::McpToolCall {
            server,
            tool,
            status,
        } => {
            if is_completed(&status, completed) {
                Some("thinking".to_owned())
            } else if is_failed(&status) {
                Some(format!("failed using {server}/{tool}"))
            } else {
                Some(format!("using {server}/{tool}"))
            }
        }
        ThreadItem::CollabToolCall { tool, status } => {
            if is_completed(&status, completed) {
                Some("thinking".to_owned())
            } else if is_failed(&status) {
                Some(format!("failed {}", describe_collab_tool(&tool)))
            } else {
                Some(describe_collab_tool(&tool))
            }
        }
        ThreadItem::FileChange { changes, status } => {
            Some(file_change_activity(&changes, &status, completed))
        }
        ThreadItem::CommandExecution { command, status } => {
            Some(command_activity(&command, &status, completed))
        }
        ThreadItem::Unknown => None,
    }
}

fn command_activity(command: &str, status: &str, completed: bool) -> String {
    if is_completed(status, completed) {
        return "thinking".to_owned();
    }
    if is_failed(status) {
        return "command failed".to_owned();
    }

    describe_command(command)
}

fn file_change_activity(changes: &[FileUpdateChange], status: &str, completed: bool) -> String {
    let target = describe_file_change_paths(changes);

    if is_completed(status, completed) {
        format!("updated {target}")
    } else {
        format!("editing {target}")
    }
}

fn is_completed(status: &str, completed: bool) -> bool {
    completed || status == "completed"
}

fn is_failed(status: &str) -> bool {
    matches!(status, "failed" | "declined")
}

fn describe_collab_tool(tool: &str) -> String {
    match tool {
        "spawn_agent" => "spawning agent".to_owned(),
        "send_input" => "sending agent input".to_owned(),
        "wait" => "waiting on agent".to_owned(),
        "close_agent" => "closing agent".to_owned(),
        _ => format!("using collab tool {tool}"),
    }
}

fn describe_command(command: &str) -> String {
    let words = shell_words(command);
    let Some(program) = words.first().map(String::as_str) else {
        return "running command".to_owned();
    };

    match program {
        "git" => describe_git_command(&words),
        "rg" => describe_rg_command(&words),
        "sed" | "cat" | "nl" | "head" | "tail" => {
            if let Some(path) = last_path_argument(&words) {
                format!("reading {path}")
            } else {
                format!("running {}", truncate(command, 80))
            }
        }
        "ls" | "find" => "listing files".to_owned(),
        "nix" => describe_nix_command(&words),
        "cargo" => describe_cargo_command(&words),
        _ => format!("running {}", truncate(command, 80)),
    }
}

fn describe_git_command(words: &[String]) -> String {
    match words.get(1).map(String::as_str) {
        Some("status") => "checking git status".to_owned(),
        Some("diff") => {
            let staged = words
                .iter()
                .any(|word| matches!(word.as_str(), "--cached" | "--staged"));
            if staged {
                "inspecting staged diff".to_owned()
            } else if let Some(path) = last_path_argument(words) {
                format!("inspecting diff for {path}")
            } else {
                "inspecting diff".to_owned()
            }
        }
        Some("show") => "inspecting git object".to_owned(),
        Some("log") => "reading git history".to_owned(),
        _ => format!("running {}", truncate(&words.join(" "), 80)),
    }
}

fn describe_rg_command(words: &[String]) -> String {
    if words.iter().any(|word| word == "--files") {
        "listing tracked files".to_owned()
    } else if let Some(pattern) = words.iter().skip(1).find(|word| !word.starts_with('-')) {
        format!("searching files for {}", truncate(pattern, 48))
    } else {
        "searching files".to_owned()
    }
}

fn describe_nix_command(words: &[String]) -> String {
    match (
        words.get(1).map(String::as_str),
        words.get(2).map(String::as_str),
    ) {
        (Some("flake"), Some("check")) => "running nix flake check".to_owned(),
        (Some("fmt"), _) => "formatting with nix".to_owned(),
        _ => format!("running {}", truncate(&words.join(" "), 80)),
    }
}

fn describe_cargo_command(words: &[String]) -> String {
    match words.get(1).map(String::as_str) {
        Some("test") => "running cargo test".to_owned(),
        Some("clippy") => "running clippy".to_owned(),
        Some("fmt") => "formatting Rust".to_owned(),
        _ => format!("running {}", truncate(&words.join(" "), 80)),
    }
}

fn last_path_argument(words: &[String]) -> Option<&str> {
    words
        .iter()
        .rev()
        .find(|word| !word.starts_with('-') && !word.contains('='))
        .map(String::as_str)
}

fn describe_file_change_paths(changes: &[FileUpdateChange]) -> String {
    match changes {
        [] => "files".to_owned(),
        [change] => change.path.clone(),
        [first, second] => format!("{}, {}", first.path, second.path),
        [first, second, ..] => format!("{}, {}, ...", first.path, second.path),
    }
}

fn shell_words(command: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut chars = command.chars();
    let mut quote = None;

    while let Some(char) = chars.next() {
        match (quote, char) {
            (None, '\'' | '"') => quote = Some(char),
            (Some(current), char) if char == current => quote = None,
            (_, '\\') => {
                if let Some(next) = chars.next() {
                    word.push(next);
                }
            }
            (None, char) if char.is_whitespace() => {
                if !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
            }
            (_, char) => word.push(char),
        }
    }

    if !word.is_empty() {
        words.push(word);
    }

    words
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
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
