use ansi_term::Colour::*;
use argh::FromArgs;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::get;
use std::fs::{remove_dir_all, remove_file, rename, File};
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// CLI tool to install/delete/move a file
#[derive(FromArgs)]
struct Args {
    /// delete mode
    #[argh(switch, short = 'd')]
    delete: bool,

    /// install mode
    #[argh(switch, short = 'i')]
    install: bool,

    /// move mode
    #[argh(switch, short = 'm')]
    move_file: bool,

    /// path to install/delete/move
    #[argh(positional)]
    path: String,

    /// path to move to
    #[argh(option)]
    move_to: Option<String>,

    /// url to install
    #[argh(option)]
    url: Option<String>,
}

fn validate(args: &Args) -> Result<(), String> {
    // Enforce exactly one execution mode so command intent is unambiguous.
    match (args.install, args.delete, args.move_file) {
        (true, false, false) => {}
        (false, true, false) => {}
        (false, false, true) => {}
        (false, false, false) => return Err("No action specified".to_string()),
        _ => {
            return Err(
                "Can only use one of --install, --delete, or --move-file at a time".to_string(),
            )
        }
    }

    // URL is only valid for install mode.
    if (args.delete || args.move_file) && args.url.is_some() {
        return Err("delete/move mode does not take a URL".into());
    }

    // Move mode requires a destination path.
    if args.move_file && args.move_to.is_none() {
        return Err("move mode requires --move-to".into());
    }

    // Install mode requires the source URL.
    if args.install && args.url.is_none() {
        return Err("install mode requires a URL".into());
    }

    Ok(())
}

fn move_file(from: &str, to: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Rename performs the move when source and destination are on the same filesystem.
    if let Err(e) = rename(from, to) {
        eprintln!(
            "{} {}",
            Red.paint("Error:"),
            Red.paint(format!("failed to move file: {}", e))
        );
        return Err(Box::new(e));
    }

    // Keep user-facing success output colorized and explicit.
    println!(
        "{}: Moved {} -> {}",
        Green.paint("Success"),
        Blue.paint(from),
        Yellow.paint(to)
    );

    Ok(())
}

fn install(url: &str, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut response: reqwest::blocking::Response = get(url)?;
    let content_length: Option<u64> = response.content_length();

    // =========================
    // Progress Bar
    // =========================
    let pb: ProgressBar = match content_length {
        Some(size) => {
            let pb: ProgressBar = ProgressBar::new(size);
            pb.set_style(
                ProgressStyle::with_template(
                    "{spinner:.green} [{elapsed_precise}] \
                     [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})",
                )?
                .progress_chars("#>-"),
            );
            pb
        }
        None => {
            let pb: ProgressBar = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::with_template(
                    "{spinner:.green} {elapsed_precise} {msg}"
                )?,
            );
            pb.set_message("Downloading...");
            pb.enable_steady_tick(Duration::from_millis(100));
            pb
        }
    };

    // =========================
    // Shared State
    // =========================
    let buffer: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let finished: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    let downloaded: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));

    // =========================
    // READER THREAD
    // =========================
    let buffer_reader = Arc::clone(&buffer);
    let finished_reader = Arc::clone(&finished);
    let downloaded_reader = Arc::clone(&downloaded);

    let reader: thread::JoinHandle<Result<(), Box<dyn Error + 'static>>> = thread::spawn(move || -> Result<(), Box<dyn std::error::Error>> {
        let mut local: [u8; 8192] = [0u8; 8192];

        loop {
            let n: usize = response.read(&mut local)?;
            if n == 0 {
                break;
            }

            {
                let mut shared = buffer_reader.lock().unwrap();
                shared.extend_from_slice(&local[..n]);
            }

            {
                let mut d = downloaded_reader.lock().unwrap();
                *d += n as u64;
            }
        }

        let mut done = finished_reader.lock().unwrap();
        *done = true;

        Ok(())
    });

    // =========================
    // WRITER THREAD
    // =========================
    let buffer_writer = Arc::clone(&buffer);
    let finished_writer = Arc::clone(&finished);
    let downloaded_writer = Arc::clone(&downloaded);
    let pb_writer: ProgressBar = pb.clone();
    let path_string: String = path.to_string();

    let writer: thread::JoinHandle<Result<(), Box<dyn Error + 'static>>> = thread::spawn(move || -> Result<(), Box<dyn std::error::Error>> {
        let mut file: File = File::create(path_string)?;

        loop {
            {
                let mut shared: [u8] = buffer_writer.lock().unwrap();
                if !shared.is_empty() {
                    file.write_all(&shared)?;
                    shared.clear();
                }
            }

            {
                let d = downloaded_writer.lock().unwrap();
                pb_writer.set_position(*d);
            }

            if *finished_writer.lock().unwrap() {
                break;
            }

            thread::yield_now();
        }

        Ok(())
    });

    reader.join().unwrap()?;
    writer.join().unwrap()?;

    pb.finish_with_message("Download complete");

    println!(
        "{}: Downloaded {} → {}",
        Green.paint("Success"),
        Blue.paint(url),
        Yellow.paint(path)
    );

    Ok(())
}


fn uninstall(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Convert user input into a Path for filesystem checks and operations.
    let path: &Path = Path::new(path);

    // Fail fast with a colorized message when the target does not exist.
    if !path.exists() {
        eprintln!(
            "{} path: {} does {} exist",
            Red.paint("Error:"),
            Yellow.paint(format!("{:#?}", path)),
            Red.paint("not")
        );
        return Err(io::Error::new(io::ErrorKind::NotFound, "path does not exist").into());
    }

    // Remove files directly; remove directories recursively.
    if path.is_file() {
        remove_file(path)?;
    } else {
        remove_dir_all(path)?;
    }

    // Report successful deletion with the resolved path.
    println!("{}: Uninstalled path: {:#?}", Green.paint("Success"), path);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments once at startup.
    let args: Args = argh::from_env();

    // Validate mode/argument combinations before any filesystem or network action.
    if let Err(e) = validate(&args) {
        eprintln!("{} {}", Red.paint("Error:"), Red.paint(format!("{:#?}", e)));
        return Err(io::Error::new(io::ErrorKind::InvalidInput, e).into());
    }

    // Install branch: download from URL to target path.
    if args.install {
        let url: &str = args.url.as_deref().expect("install requires a URL");
        if let Err(e) = install(url, &args.path) {
            eprintln!("{}", Red.paint(format!("Error: {:#?}", e)));
            return Err(e);
        }
    // Delete branch: explicit confirmation guard before destructive action.
    } else if args.delete {
        println!(
            "{}: This command will remove the following file: {}\nAre you sure you want to continue (y/n)?",
            Red.paint("WARNING"),
            args.path
        );

        // Normalize user confirmation to make matching case-insensitive.
        let mut confirmation: String = String::new();
        io::stdin().read_line(&mut confirmation)?;
        let confirmation: String = confirmation.trim().to_lowercase();

        if confirmation == "n" || confirmation == "no" {
            println!("Safely exiting");
            return Ok(());
        }

        if let Err(e) = uninstall(&args.path) {
            eprintln!("{}", Red.paint(format!("Error: {:#?}", e)));
            return Err(e);
        }
    // Move branch: relocate file to provided destination.
    } else {
        let move_to: &str = args.move_to.as_deref().expect("move mode requires --move-to");
        if let Err(e) = move_file(&args.path, move_to) {
            eprintln!("{}", Red.paint(format!("Error: {:#?}", e)));
            return Err(e);
        }
    }

    Ok(())
}
