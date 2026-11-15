//! Blockbench, the editor for models, animations and textures. The Pack Creator finds it, installs it when it is missing, sets up the Strut Toolkit plugin in a profile
//! of its own (so the person's usual Blockbench is left as it is), and opens a girl's model with her texture, or an animation on a model, ready to edit.
//!
//! Blockbench is an Electron program. It is started with a debugging port, and the page inside it is driven over that port: the toolkit plugin is loaded, the file is
//! handed to it as if it had been picked in the import dialog, and a "Save to pack" action is added that writes straight back to the file in the pack.

use serde::Serialize;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/* A port of its own, so it does not meet a browser or another tool's debugging port. */
const PORT: u16 = 9339;
const TOOLKIT: &[u8] = include_bytes!("../resources/strut_toolkit.js");
const TOOLKIT_VERSION: &str = "4.6.1";
const MIN_VERSION: (u32, u32) = (5, 1);
const NO_WINDOW: u32 = 0x0800_0000;

/* What the installer is doing: "" when idle, then a sentence for the page to show. Kept here so the page can ask again and again while a download runs. */
static INSTALL: Mutex<String> = Mutex::new(String::new());

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub exe: Option<String>,
    pub version: Option<String>,
    /// Whether the version found is one the toolkit works with (5.1 or newer).
    pub version_ok: bool,
    pub running: bool,
    pub toolkit_version: &'static str,
    pub installing: String,
}

// =================================================================
// Finding Blockbench
// =================================================================

fn data_dir() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());

    base.join("PleasureHorizonsPackCreator").join("blockbench")
}

fn find_exe() -> Option<PathBuf> {
    let mut candidates = Vec::new();

    for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Ok(base) = std::env::var(variable) {
            candidates.push(PathBuf::from(base).join("Blockbench").join("Blockbench.exe"));
        }
    }

    if let Ok(base) = std::env::var("LOCALAPPDATA") {
        candidates.push(PathBuf::from(base).join("Programs").join("Blockbench").join("Blockbench.exe"));
    }

    candidates.into_iter().find(|p| p.is_file())
}

/// The program's own version, read from the file's properties.
fn exe_version(exe: &Path) -> Option<String> {
    let script = format!("(Get-Item -LiteralPath '{}').VersionInfo.ProductVersion", exe.display().to_string().replace('\'', "''"));
    let out = Command::new("powershell").args(["-NoProfile", "-Command", &script]).creation_flags(NO_WINDOW).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();

    if text.is_empty() { None } else { Some(text) }
}

fn version_ok(version: &str) -> bool {
    let mut parts = version.split('.').map(|p| p.trim().parse::<u32>().unwrap_or(0));
    let (major, minor) = (parts.next().unwrap_or(0), parts.next().unwrap_or(0));

    (major, minor) >= MIN_VERSION
}

pub fn status() -> Status {
    let exe = find_exe();
    let version = exe.as_deref().and_then(exe_version);

    Status {
        exe: exe.map(|p| p.display().to_string()),
        version_ok: version.as_deref().map_or(false, version_ok),
        version,
        running: pages().is_some(),
        toolkit_version: TOOLKIT_VERSION,
        installing: INSTALL.lock().unwrap().clone(),
    }
}

// =================================================================
// Installing Blockbench
// =================================================================

fn say(message: &str) {
    *INSTALL.lock().unwrap() = message.to_string();
}

fn curl(args: &[&str]) -> Result<Vec<u8>, String> {
    let out = Command::new("curl.exe").args(args).creation_flags(NO_WINDOW).output().map_err(|e| format!("curl could not run: {e}"))?;

    if !out.status.success() {
        return Err(format!("the download failed: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }

    Ok(out.stdout)
}

/// Starts the install in the background and returns at once; the page watches `status().installing`. It downloads Blockbench's own installer from its GitHub releases
/// (the newest 64-bit one), then runs it, and the person clicks through it.
pub fn install() -> Result<(), String> {
    if !INSTALL.lock().unwrap().is_empty() {
        return Err("Blockbench is already being installed.".into());
    }

    say("Finding the newest Blockbench...");

    std::thread::spawn(|| {
        let result = (|| -> Result<(), String> {
            let body = curl(&["-sSL", "--max-time", "30", "-H", "User-Agent: pleasure-horizons-pack-creator", "https://api.github.com/repos/JannisX11/blockbench/releases/latest"])?;
            let release: Value = serde_json::from_slice(&body).map_err(|_| "GitHub did not answer with the release list; try again in a minute".to_string())?;
            let tag = release["tag_name"].as_str().unwrap_or("").trim_start_matches('v').to_string();
            let assets = release["assets"].as_array().ok_or("the release has no files")?;
            let wanted = [format!("Blockbench_x64_{tag}.exe"), format!("Blockbench_{tag}.exe")];
            let asset = wanted.iter().find_map(|name| assets.iter().find(|a| a["name"].as_str() == Some(name.as_str()))).ok_or("this release has no Windows installer")?;
            let url = asset["browser_download_url"].as_str().ok_or("the installer has no address")?.to_string();
            let name = asset["name"].as_str().unwrap_or("Blockbench_setup.exe").to_string();
            let size = asset["size"].as_u64().unwrap_or(0) / 1024 / 1024;
            let target = std::env::temp_dir().join(&name);

            say(&format!("Downloading Blockbench {tag} ({size} MiB)..."));
            curl(&["-sSL", "--max-time", "1800", "-o", &target.display().to_string(), &url])?;

            if !target.is_file() {
                return Err("the installer was not saved".into());
            }

            say("The Blockbench installer is open. Finish it there, then come back here.");

            let status = Command::new(&target).status().map_err(|e| format!("the installer could not start: {e}"))?;

            let _ = std::fs::remove_file(&target);

            if !status.success() {
                return Err("the installer was closed before it finished".into());
            }

            Ok(())
        })();

        match result {
            Ok(()) => say(""),
            Err(error) => say(&format!("failed: {error}")),
        }
    });

    Ok(())
}

/// Clears a failed install so the page can offer it again.
pub fn clear_install_failure() {
    let mut state = INSTALL.lock().unwrap();

    if state.starts_with("failed") {
        state.clear();
    }
}

// =================================================================
// Talking to the page inside Blockbench
// =================================================================

/// The debugging port's list of pages (a plain local HTTP request), or None when nothing answers.
fn pages() -> Option<Vec<Value>> {
    let mut stream = TcpStream::connect_timeout(&format!("127.0.0.1:{PORT}").parse().ok()?, Duration::from_millis(400)).ok()?;

    stream.set_read_timeout(Some(Duration::from_secs(3))).ok()?;
    stream.write_all(format!("GET /json/list HTTP/1.1\r\nHost: 127.0.0.1:{PORT}\r\nConnection: close\r\n\r\n").as_bytes()).ok()?;

    /* The server may keep the connection open after answering, so the reply is taken as far as it has come and parsed as soon as it is whole. */
    let mut reply: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];

    loop {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => reply.extend_from_slice(&chunk[..n]),
        }

        if let Some(at) = reply.windows(4).position(|w| w == b"\r\n\r\n") {
            let body = String::from_utf8_lossy(&reply[at + 4..]).to_string();

            if let Ok(list) = serde_json::from_str::<Vec<Value>>(&body) {
                return Some(list);
            }
        }
    }

    None
}

struct Page {
    socket: tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>,
    next: u64,
}

impl Page {
    fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.next += 1;

        let id = self.next;

        self.socket.send(tungstenite::Message::text(json!({ "id": id, "method": method, "params": params }).to_string())).map_err(|e| format!("Blockbench did not take the request: {e}"))?;

        loop {
            let message = self.socket.read().map_err(|e| format!("Blockbench stopped answering: {e}"))?;

            if let tungstenite::Message::Text(text) = message {
                let value: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;

                if value["id"].as_u64() == Some(id) {
                    return Ok(value);
                }
            }
        }
    }

    /// Runs a script in Blockbench and returns what it gave back (a string).
    fn run(&mut self, script: &str) -> Result<String, String> {
        let reply = self.call("Runtime.evaluate", json!({ "expression": script, "awaitPromise": true, "returnByValue": true }))?;

        if let Some(problem) = reply["result"]["exceptionDetails"].as_object() {
            let text = problem.get("exception").and_then(|e| e["description"].as_str()).or_else(|| problem.get("text").and_then(Value::as_str)).unwrap_or("a script error");

            return Err(format!("Blockbench reported: {text}"));
        }

        Ok(reply["result"]["result"]["value"].as_str().unwrap_or("").to_string())
    }
}

fn connect() -> Result<Page, String> {
    let list = pages().ok_or("Blockbench is not running")?;
    let page = list.iter().find(|p| p["type"] == "page" && p["url"].as_str().map_or(false, |u| u.contains("index.html"))).ok_or("Blockbench has no window yet")?;
    let address = page["webSocketDebuggerUrl"].as_str().ok_or("Blockbench gave no address to talk to")?;
    let (socket, _) = tungstenite::connect(address).map_err(|e| format!("could not connect to Blockbench: {e}"))?;

    Ok(Page { socket, next: 0 })
}

/// Starts Blockbench with the Pack Creator's own profile and the debugging port, unless it is already running that way, and waits until its window is ready.
fn ensure_running() -> Result<Page, String> {
    if pages().is_none() {
        let exe = find_exe().ok_or("Blockbench is not installed. Press Install Blockbench first.")?;
        let profile = data_dir().join("profile");

        std::fs::create_dir_all(&profile).map_err(|e| format!("could not make the Blockbench profile folder: {e}"))?;

        Command::new(&exe)
            .arg(format!("--remote-debugging-port={PORT}"))
            .arg(format!("--user-data-dir={}", profile.display()))
            .spawn()
            .map_err(|e| format!("Blockbench could not start: {e}"))?;
    }

    let started = Instant::now();

    loop {
        if let Ok(mut page) = connect() {
            /* The page exists before Blockbench has finished loading; wait for its own objects. */
            if page.run("typeof Plugins !== 'undefined' && typeof BarItems !== 'undefined' && !!Plugins.installed ? 'yes' : ''").map_or(false, |v| v == "yes") {
                return Ok(page);
            }
        }

        if started.elapsed() > Duration::from_secs(60) {
            return Err("Blockbench did not finish starting in a minute.".into());
        }

        std::thread::sleep(Duration::from_millis(400));
    }
}

fn toolkit_file() -> Result<PathBuf, String> {
    let folder = data_dir();

    std::fs::create_dir_all(&folder).map_err(|e| format!("could not make {}: {e}", folder.display()))?;

    let path = folder.join("strut_toolkit.js");
    let same = std::fs::read(&path).map(|existing| existing == TOOLKIT).unwrap_or(false);

    if !same {
        std::fs::write(&path, TOOLKIT).map_err(|e| format!("could not write the toolkit: {e}"))?;
    }

    Ok(path)
}

// =================================================================
// Opening files
// =================================================================

/// What to open. Paths are real files on disk, already checked to be inside the pack.
pub struct Job {
    pub model: PathBuf,
    pub texture: Option<PathBuf>,
    pub animation: Option<PathBuf>,
}

const SCRIPT: &str = r#"(async (job) => {
  const wait = ms => new Promise(r => setTimeout(r, ms));
  const bytesOf = b64 => Uint8Array.from(atob(b64), c => c.charCodeAt(0)).buffer;

  if (!Plugins.installed.some(p => p.id === 'strut_toolkit')) {
    await new Plugin().loadFromFile({ path: job.toolkit }, false);
  }

  for (let i = 0; i < 50 && !(BarItems.import_strut_binary_model && BarItems.export_strut_binary_model); i++) await wait(200);

  if (!BarItems.import_strut_binary_model) return 'The Strut Toolkit did not load in Blockbench.';

  /* The page has no direct file access; Blockbench's own writer saves the bytes as they are. */
  const writeBack = (target, bytes) => Filesystem.writeFile(target, { content: bytes, savetype: 'binary' }, () => {});

  /* "Save to pack" writes the project straight over the file it was opened from, instead of asking where to put it. */
  const saver = (id, name, source, property, exporter, message) => {
    if (BarItems[id]) return;

    const action = new Action(id, {
      name, icon: 'save', category: 'file',
      keybind: new Keybind({ key: id === 'pc_save_model' ? 'e' : 'r', ctrl: true, shift: true }),
      click() {
        const target = Project && Project[property];
        if (!target) { Blockbench.showQuickMessage('This project was not opened from the Pack Creator.', 2500); return; }
        const original = Filesystem.exportFile;
        Filesystem.exportFile = options => { writeBack(target, options.content); Blockbench.showQuickMessage(message, 1800); };
        try { BarItems[exporter].click(); } finally { Filesystem.exportFile = original; }
      }
    });
    MenuBar.addAction(action, 'file');
  };

  saver('pc_save_model', 'Save model to the pack', 'model', 'pc_model_path', 'export_strut_binary_model', 'Model saved to the pack');
  saver('pc_save_animations', 'Save animations to the pack', 'animation', 'pc_animation_path', 'export_strut_binary_animations', 'Animations saved to the pack');

  const feed = (actionId, file) => {
    const original = Blockbench.import;
    Blockbench.import = (options, callback) => callback([{ name: file.name, path: file.path, content: bytesOf(file.b64) }]);
    try { BarItems[actionId].click(); } finally { Blockbench.import = original; }
  };

  feed('import_strut_binary_model', job.model);
  await wait(2000);

  if (!Project) return 'The model did not open.';

  Project.pc_model_path = job.model.path;

  if (job.texture && Texture.all.length) {
    const base = job.texture.split('/').pop();
    const target = Texture.all.find(t => t.name === base) || Texture.all[0];
    target.fromPath(job.texture);
  }

  if (job.animation) {
    feed('import_strut_binary_animations', job.animation);
    await wait(1200);
    Project.pc_animation_path = job.animation.path;
    if (typeof Modes !== 'undefined' && Modes.options.animate) Modes.options.animate.select();
  }

  return 'ok';
})"#;

fn file_arg(path: &Path) -> Result<Value, String> {
    use base64::Engine;

    let bytes = std::fs::read(path).map_err(|e| format!("{} could not be read: {e}", path.display()))?;

    Ok(json!({
        "name": path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
        "path": path.display().to_string().replace('\\', "/"),
        "b64": base64::engine::general_purpose::STANDARD.encode(bytes),
    }))
}

/// Opens the model (and the texture, and the animation file on top of it) in Blockbench, starting Blockbench first when it is not running.
pub fn open(job: &Job) -> Result<(), String> {
    let toolkit = toolkit_file()?;
    let mut page = ensure_running()?;
    let mut data = json!({ "toolkit": toolkit.display().to_string().replace('\\', "/"), "model": file_arg(&job.model)? });

    if let Some(texture) = &job.texture {
        data["texture"] = json!(texture.display().to_string().replace('\\', "/"));
    }

    if let Some(animation) = &job.animation {
        data["animation"] = file_arg(animation)?;
    }

    let reply = page.run(&format!("({SCRIPT})({data})"))?;

    let _ = page.call("Page.bringToFront", json!({}));

    if reply == "ok" { Ok(()) } else { Err(reply) }
}

/// Starts Blockbench with the toolkit loaded and nothing open, so the person can see it is ready.
pub fn prepare() -> Result<(), String> {
    let toolkit = toolkit_file()?;
    let mut page = ensure_running()?;
    let script = format!(
        "(async()=>{{ if (!Plugins.installed.some(p => p.id === 'strut_toolkit')) {{ await new Plugin().loadFromFile({{ path: {} }}, false); }} return 'ok'; }})()",
        json!(toolkit.display().to_string().replace('\\', "/"))
    );

    page.run(&script)?;
    let _ = page.call("Page.bringToFront", json!({}));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /* Needs a Blockbench running with the debugging port; run by hand while one is open. */
    #[test]
    #[ignore]
    fn talks_to_a_running_blockbench() {
        let mut page = connect().expect("connect");
        let value = page.run("'hello ' + typeof Plugins").expect("run");

        println!("{value}");
        assert!(value.starts_with("hello"));
    }
}
