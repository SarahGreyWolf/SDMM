use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};
use serde::{Deserialize, Serialize};
use std::env;
use std::io;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::thread;
mod gui;
fn main() {
    let mut path = Path::new("");
    let mut args = env::args().skip(1);
    if let Some(pos_url) = args.next() {
        if !pos_url.is_empty() {
            path = Path::new(&pos_url);
            if path.starts_with("nxm://") {
                if let Ok(mut stream) = LocalSocketStream::connect("/tmp/sdmm.sock") {
                    println!("{:?}", stream.peer_pid());
                    let _ = stream
                        .write(format!("{}\n", path.display()).as_bytes())
                        .unwrap();
                    return;
                }
            }
        }
    }
    setup().unwrap();

    let listener = LocalSocketListener::bind("/tmp/sdmm.sock").unwrap();
    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let mut reader = BufReader::new(stream);
                    let mut buffer = String::new();
                    reader.read_line(&mut buffer).unwrap();
                    // TODO: Handle Downloading here
                    println!("{}", buffer);
                }
                Err(_) => {
                    break;
                }
            }
        }
    });
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Stardew Mod Manager",
        native_options,
        Box::new(|cc| Box::new(gui::SDMMApp::new(cc))),
    );
}
#[cfg(target_os = "windows")]
fn setup() -> io::Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;
    const FRIENDLY_NAME: &str = "NexusMods";
    const URI_SCHEME: &str = "nxm";
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if hkcu
        .open_subkey(&format!("SOFTWARE\\Classes\\{}", URI_SCHEME))
        .is_ok()
    {
        return Ok(());
    }
    let (protocol, _) = hkcu.create_subkey(&format!("SOFTWARE\\Classes\\{}", URI_SCHEME))?;
    protocol.set_value("", &format!("URL:{}", FRIENDLY_NAME))?;
    protocol.set_value("URL Protocol", &"")?;
    let (icon, _) = protocol.create_subkey("DefaultIcon")?;
    icon.set_value("", &"")?;
    let (command, _) = protocol.create_subkey("shell\\open\\command")?;
    let address = env::current_exe().unwrap().display().to_string();
    let address = address.split_at(4).1;
    command.set_value("", &format!(r#""{}" "%1""#, address))?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn setup() -> io::Result<()> {
}
#[derive(Serialize, Deserialize, Default)]
struct GameMod {
    name: String,
    version: String,
    author: String,
    link: String,
    active: bool,
}
