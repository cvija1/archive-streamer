use archive_streamer::extract_archive;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: archive-streamer-cli <archive> <destination>");
        std::process::exit(1);
    }

    let archive = std::path::PathBuf::from(&args[1]);
    let destination = std::path::PathBuf::from(&args[2]);

    let (tx, rx) = std::sync::mpsc::channel();

    let worker = std::thread::spawn(move || extract_archive(&archive, &destination, &tx));

    for message in rx {
        match message {
            archive_streamer::AppMessage::Progress(message) => {
                println!("{message}");
            }
            archive_streamer::AppMessage::Finished(_) => {}
        }
    }

    match worker.join() {
        Ok(Ok(())) => {
            println!("Extraction completed.");
        }
        Ok(Err(error)) => {
            eprintln!("Error: {error}");
            std::process::exit(1);
        }
        Err(_) => {
            eprintln!("Extraction worker panicked.");
            std::process::exit(1);
        }
    }
}
