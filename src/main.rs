use std::fs::{File, remove_file, rename, remove_dir};
use std::path::Path;
use std::time::Duration;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, Read, Write};
use reqwest::blocking::get;
use argh::FromArgs;
use ansi_term::Colour::*;

/// CLi tool to install/delete a file
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
     match (args.install, args.delete, args.move_file) {
        (true, false, false) => {}
        (false, true, false) => {}
        (false, false, true) => {}

        (false, false, false) => {
            return Err("No action specified".to_string());
        }
        _ => {
            return Err("Can only use one of --install, --delete, or --move-file at a time".to_string());
        }
    }

    if (args.delete || args.move_file) && args.url.is_some() {
        return Err("delete/move mode does not take a URL".into());
    }

    if args.move_file && args.move_to.is_none() {
        return Err("move mode requires --move-to".into());
    }


    if args.install && args.url.is_none() {
        return Err("install mode requires a URL".into());
    }

    Ok(())
}

fn move_file(from: &str, to: &str) -> Result<(), Box<dyn std::error::Error>> {
    rename(from, to)
        .map_err(|e: io::Error| format!("failed to move file: {}", e))?;

    println!(
        "{}: Moved {} → {}",
        Green.paint("Success"),
        Blue.paint(from),
        Yellow.paint(to)
    );

    Ok(())
}


fn install(url: &str, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut response = get(url)?;

    // Decide which indicator to use
    let pb: ProgressBar = match response.content_length() {
        Some(size) => {
            let pb: ProgressBar = ProgressBar::new(size);
            pb.set_style(
                ProgressStyle::with_template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})",
                )?
                .progress_chars("#>-"),
            );
            pb
        }
        None => {
            let pb: ProgressBar = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::with_template("{spinner:.green} {elapsed_precise} {msg}")?,
            );
            pb.set_message("Downloading...");
            pb.enable_steady_tick(Duration::from_millis(100));
            pb
        }
    };

    let mut file: File = File::create(path)?;
    let mut downloaded: u64 = 0;
    let mut buffer: [u8; 8192] = [0u8; 8192];

    loop {
        let n: usize = response.read(&mut buffer)?;
        if n == 0 {
            break;
        }

        file.write_all(&buffer[..n])?;
        downloaded += n as u64;

        // Only updates position if this is a real progress bar
        pb.set_position(downloaded);
    }

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
    let path: &Path = Path::new(&path);

    if path.exists() {
        if path.is_file() {
            remove_file(path)?;
        } else {
            remove_dir(path)?;
        }
    } else {
        println!(
            "{} path: {} does {} exist",
            Red.paint("Error"), Yellow.paint(format!("{:#?}", path)), Red.paint("not")
        );
    }

    println!(
        "{}: Uninstalled path: {:#?}",
        Green.paint("Success"), path
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Args = argh::from_env();
    validate(&args)?;

    if args.install {
        let url: &str = args.url.as_ref().expect("install requires a URL");
        let err: Result<(), Box<dyn std::error::Error>> = install(&url, &args.path);

        if let Err(e) = err {
            eprintln!("{}", Red.paint(format!("Error: {:#?}", e)));
        }

    } else if args.delete {
        println!(
            "{}: This command will remove the following file: {}\nAre you sure you want to continue (y/n)?",
            Red.paint("WARNING"), args.path
        );
        let mut confirmation: String = String::new();
        io::stdin()
            .read_line(&mut confirmation)
            .expect("Please enter a valid response (y/n)");

        confirmation = confirmation.trim().to_lowercase();

        if confirmation == "n" || confirmation == "no" {
            println!("Safely exiting");
            return Ok(());
        }

        let err: Result<(), Box<dyn std::error::Error>> = uninstall(&args.path);

        if let Err(e) = err {
            eprintln!("{}", Red.paint(format!("Error: {:#?}", e)));
        }

    } else {
        let err: Result<(), Box<dyn std::error::Error>> = move_file(&args.path, &args.move_to.unwrap());

        if let Err(e) = err {
            eprintln!("{}", Red.paint(format!("Error: {:#?}", e)));
        }
    }

    Ok(())
}