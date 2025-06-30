use inquire::Text;
use std::{
  env, fs,
  path::{Path, PathBuf},
  process::{exit, Command},
};
use windirs::{known_folder_path, FolderId};

#[cfg(feature = "flow_launcher")]
use std::os::windows::process::CommandExt;
#[cfg(feature = "flow_launcher")]
use sysinfo::{Signal, System};

#[derive(Debug, PartialEq, Eq)]
enum Mode {
  System,
  User,
}

fn strip_extended_prefix(path: &Path) -> &Path {
  const EXTENDED_PREFIX: &str = r"\\?\";
  if let Some(str_path) = path.to_str() {
    if str_path.starts_with(EXTENDED_PREFIX) {
      return Path::new(&str_path[EXTENDED_PREFIX.len()..]);
    }
  }
  path
}

fn main() {
  if !cfg!(windows) {
    eprintln!("This program is made for Windows");
    exit(1);
  }

  let args = env::args().collect::<Vec<String>>();

  if args.len() < 2 {
    eprintln!("Usage: {} <path> [<system/s>/<user/u>] [<name>]", args[0]);
    exit(1);
  }

  let path = &args[1];
  let mode = match args.get(2).unwrap_or(&"user".to_string()).as_str() {
    "system" | "s" => Mode::System,
    "user" | "u" => Mode::User,
    _ => {
      eprintln!("Invalid mode: {} (can be system/s or user/u)", args[2]);
      exit(1);
    }
  };
  let suggested_name = match PathBuf::from(&path).file_stem().and_then(|s| s.to_str()) {
    Some(s) => &s.replace("_", " "),
    None => &String::from(""),
  };
  let name = if args.len() > 3 {
    &args[3]
  } else {
    &Text::new(
      format!(
        "Enter the name of the project (default: {})",
        suggested_name
      )
      .as_str(),
    )
    .with_default(suggested_name.as_str())
    .prompt()
    .unwrap_or_else(|err| {
      eprintln!("Error during prompt: {}", err);
      exit(1);
    })
  };

  let programs_path = match known_folder_path(match mode {
    Mode::User => FolderId::Programs,
    Mode::System => FolderId::CommonPrograms,
  }) {
    Ok(p) => p,
    Err(err) => {
      eprintln!("Error while getting programs path: {}", err);
      exit(1);
    }
  };

  let temp_dir = env::temp_dir();
  let script_path = temp_dir.join("payload.ps1");
  fs::write(&script_path, include_bytes!("../payload.ps1")).unwrap_or_else(|err| {
    eprintln!("Error while writing payload.ps1: {}", err);
    exit(1);
  });

  let canonical_path = fs::canonicalize(path).unwrap_or_else(|err| {
    eprintln!("Error while getting canonical path: {}", err);
    exit(1);
  });
  let stripped_path = strip_extended_prefix(&canonical_path);

  let current_dir = env::current_dir().unwrap_or_else(|err| {
    eprintln!("Error while getting current directory: {}", err);
    exit(1);
  });

  // I had no idea how horrible shortcuts are using Rust so I did this
  let output = Command::new("powershell")
    .arg("-ExecutionPolicy")
    .arg("Bypass")
    .arg("-File")
    .arg(&script_path)
    .arg(
      programs_path
        .join(name)
        .with_extension("lnk")
        .to_str()
        .unwrap_or_else(|| {
          eprintln!("Error while converting path to string");
          exit(1);
        }),
    )
    .arg(stripped_path.to_str().unwrap_or_else(|| {
      eprintln!("Error while converting canonical path to string");
      exit(1);
    }))
    .arg("-RequiresElevation")
    .arg(match mode {
      Mode::System => "true",
      _ => "false",
    })
    .arg("-CurrentDirectory")
    .arg(current_dir.to_str().unwrap_or_else(|| {
      eprintln!("Error while converting current directory to string");
      exit(1);
    }))
    .output()
    .unwrap_or_else(|err| {
      eprintln!("Error creating shortcut: {}", err);
      exit(1);
    });

  if !output.status.success() {
    eprintln!(
      "Error creating shortcut: {}",
      String::from_utf8_lossy(&output.stderr)
    );
    exit(1);
  }

  println!("Shortcut created successfully");

  fs::remove_file(script_path).unwrap_or_else(|err| {
    eprintln!("Error while removing payload: {}", err);
    exit(1);
  });

  // Restart Flow Launcher
  #[cfg(feature = "flow_launcher")]
  {
    let mut sys = System::new_all();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    for process in sys.processes().values() {
      if let Some(exe) = process.exe() {
        if exe.file_name().unwrap_or_default().to_ascii_lowercase() == "flow.launcher.exe" {
          process.kill_with(Signal::Kill);
          Command::new(exe.to_path_buf())
            .creation_flags(0x00000008) // DETACHED_PROCESS
            .spawn()
            .unwrap_or_else(|err| {
              eprintln!("Error while starting Flow Launcher again: {}", err);
              exit(1);
            });
          println!("Flow Launcher restarted successfully");
        }
      }
    }
  }
}
