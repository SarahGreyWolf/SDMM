#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use interprocess::local_socket::LocalSocketStream;
use std::env;
use std::io;
use std::io::Write;
use std::path::PathBuf;
mod app;
mod download;
const PROJECT_NAME: &str = "SDMM";
fn main() {
    #[allow(unused_assignments)]
    let mut path = PathBuf::new();
    let mut args = env::args().skip(1);
    if let Some(pos_url) = args.next() {
        if !pos_url.is_empty() && pos_url.starts_with("nxm://") {
            path = PathBuf::from(pos_url);
            if let Ok(mut stream) = LocalSocketStream::connect("/tmp/sdmm.sock") {
                println!("{:?}", stream.peer_pid());
                let path_string = path.display().to_string();
                let path_bytes = path_string.as_bytes();
                let mut bytes = vec![0u8; 1024 - path_bytes.len()];
                bytes.append(&mut path_bytes.to_vec());
                let _ = stream.write(&bytes).unwrap();
                stream.flush().unwrap();
                return;
            }
        }
    }

    setup(false).unwrap();
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(eframe::emath::vec2(800., 600.)),
        resizable: true,
        drag_and_drop_support: true,
        ..Default::default()
    };
    eframe::run_native(
        PROJECT_NAME,
        native_options,
        Box::new(|cc| Box::new(app::SDMMApp::new(cc, path))),
    );
}
#[cfg(target_os = "windows")]
fn setup(reset: bool) -> io::Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;
    const FRIENDLY_NAME: &str = "NexusMods";
    const URI_SCHEME: &str = "nxm";
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if hkcu
        .open_subkey(&format!("SOFTWARE\\Classes\\{}", URI_SCHEME))
        .is_ok()
        && !reset
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
    let address = if address.starts_with(r#"\\?\"#) {
        address.split_at(4).1
    } else {
        &address
    };
    command.set_value("", &format!(r#""{}" "%1""#, address))?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn setup() -> io::Result<()> {
}
