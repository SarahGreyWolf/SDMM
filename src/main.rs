fn main() {
    #[cfg(target_os = "windows")]
    setup().unwrap();

}
#[cfg(target_os = "windows")]
fn setup() -> io::Result<()> {
    const FRIENDLY_NAME: &str = "NexusMods";
    const URI_SCHEME: &str = "nxm";
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(_) = hkcu.open_subkey(&format!("SOFTWARE\\Classes\\{}", URI_SCHEME)) {
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
