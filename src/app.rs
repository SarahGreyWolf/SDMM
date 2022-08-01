use crate::download::{handle_download_requests, ModDetails, ModFileDetails};
use core::panic;
use directories_next::ProjectDirs;
use eframe::{egui, CreationContext, Storage};
use egui_extras::{Size, TableBuilder};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{create_dir, create_dir_all, read_dir, remove_dir_all, File};
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::{sync_channel, Receiver};

#[derive(Serialize, Deserialize, Default, Clone)]
struct DepGameMod {
    name: String,
    zip_name: String,
    folder_name: String,
    version: String,
    author: String,
    link: String,
    id: u64
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct GameMod {
    name: String,
    zip_name: String,
    folder_name: String,
    version: String,
    author: String,
    link: String,
    mod_id: u64,
    file_id: u64
}

#[derive(Default, PartialEq)]
enum Menus {
    Browse,
    Downloading,
    #[default]
    Mods,
    Settings,
}

pub struct SDMMApp {
    downloads_receiver: Receiver<(String, usize, usize, usize, usize)>,
    web_client: reqwest::blocking::Client,
    state: Menus,
    download_path: PathBuf,
    game_path: PathBuf,
    last_download: PathBuf,
    api_key: String,
    needs_key: bool,
    downloads: HashMap<String, (usize, usize, usize, usize, bool)>,
    inactive: Vec<GameMod>,
    active: Vec<GameMod>,
    // TODO: Add some kind of popups list that show
}

impl SDMMApp {
    pub fn new(context: &eframe::CreationContext<'_>, download: PathBuf) -> SDMMApp {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "CodeNewRoman".to_owned(),
            egui::FontData::from_static(include_bytes!(
                "../assets/Code New Roman Bold Nerd Font Complete Windows Compatible.otf"
            ))
            .tweak(egui::FontTweak {
                scale: 0.9,
                ..Default::default()
            }),
        );
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "CodeNewRoman".to_owned());
        context.egui_ctx.set_fonts(fonts);
        context.egui_ctx.set_visuals(egui::Visuals::dark());

        let download_path = setup_download_path(context);

        let web_client = reqwest::blocking::Client::new();

        let mut active: Vec<GameMod> = vec![];
        let mut inactive: Vec<GameMod> = vec![];
        let mut game_path = PathBuf::new();
        let mut last_download = download.display().to_string();
        let mut api_key = String::new();
        if let Some(storage) = context.storage {
            let loaded = load_from_storage(storage, &web_client);
            active = loaded.0;
            inactive = loaded.1;
            game_path = loaded.2;
            if last_download.is_empty() {
                last_download = loaded.3;
            }
            api_key = loaded.4;
        }

        let (sync_sender, receiver) = sync_channel::<(String, usize, usize, usize, usize)>(1);
        let download_path_clone = download_path.clone();
        // TODO: Continue downloads that weren't finished previously?
        handle_download_requests(
            sync_sender,
            download_path,
            api_key.clone(),
            last_download.clone(),
        );

        let mut needs_key = true;
        if !api_key.is_empty() {
            needs_key = false;
            last_download = String::new();
        }

        SDMMApp {
            downloads_receiver: receiver,
            web_client,
            state: Menus::default(),
            download_path: download_path_clone,
            game_path,
            last_download: PathBuf::from(last_download),
            api_key,
            needs_key,
            downloads: HashMap::new(),
            inactive,
            active,
        }
    }

    fn mods_display(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.columns(2, |columns| {
                if columns.len() == 0 {
                    panic!("Expected 2 columns, 0 were given");
                }
                if let [left_panel, right_panel] = &mut columns[0..2] {
                    // Left (inactive) Table
                    left_panel.heading("Inactive");
                    left_panel.separator();
                    left_panel.push_id(0, |ui| {
                        egui::ScrollArea::vertical()
                            .id_source("inactive")
                            .show(ui, |ui| {
                                TableBuilder::new(ui)
                                    .cell_layout(
                                        egui::Layout::left_to_right()
                                            .with_cross_align(egui::Align::Center),
                                    )
                                    .resizable(true)
                                    .columns(Size::remainder().at_least(5.), 3)
                                    .header(20.0, |mut header| {
                                        header.col(|ui| {
                                            ui.heading("Name");
                                        });
                                        header.col(|ui| {
                                            ui.heading("Version");
                                        });
                                        header.col(|ui| {
                                            ui.heading("Author");
                                        });
                                    })
                                    .body(|mut body| {
                                        for (index, r#mod) in
                                            &mut self.inactive.clone().iter_mut().enumerate()
                                        {
                                            body.row(20., |mut row| {
                                                row.col(|ui| {
                                                    ui.hyperlink_to(&r#mod.name, &r#mod.link);
                                                    let sense = ui.interact(
                                                        ui.max_rect(),
                                                        egui::Id::new(&format!(
                                                            "name-inactive{:#}",
                                                            index
                                                        )),
                                                        egui::Sense::click(),
                                                    );
                                                    if sense.double_clicked()
                                                        || sense.triple_clicked()
                                                    {
                                                        self.switch_active_inactive(
                                                            r#mod, index, false,
                                                        );
                                                    }
                                                });
                                                row.col(|ui| {
                                                    ui.label(&r#mod.version);
                                                    let sense = ui.interact(
                                                        ui.max_rect(),
                                                        egui::Id::new(&format!(
                                                            "version-inactive{:#}",
                                                            index
                                                        )),
                                                        egui::Sense::click(),
                                                    );
                                                    if sense.double_clicked()
                                                        || sense.triple_clicked()
                                                    {
                                                        self.switch_active_inactive(
                                                            r#mod, index, false,
                                                        );
                                                    }
                                                });
                                                row.col(|ui| {
                                                    ui.label(&r#mod.author);
                                                    let sense = ui.interact(
                                                        ui.max_rect(),
                                                        egui::Id::new(&format!(
                                                            "author-inactive{:#}",
                                                            index
                                                        )),
                                                        egui::Sense::click(),
                                                    );
                                                    if sense.double_clicked()
                                                        || sense.triple_clicked()
                                                    {
                                                        self.switch_active_inactive(
                                                            r#mod, index, false,
                                                        );
                                                    }
                                                });
                                            });
                                        }
                                    });
                            });
                    });
                    // Right (active) Table
                    right_panel.heading("Active");
                    right_panel.separator();
                    right_panel.push_id(1, |ui| {
                        egui::ScrollArea::vertical()
                            .id_source("active")
                            .show(ui, |ui| {
                                TableBuilder::new(ui)
                                    .cell_layout(
                                        egui::Layout::left_to_right()
                                            .with_cross_align(egui::Align::Center),
                                    )
                                    .resizable(true)
                                    .columns(Size::remainder().at_least(5.), 3)
                                    .header(20.0, |mut header| {
                                        header.col(|ui| {
                                            ui.heading("Name");
                                        });
                                        header.col(|ui| {
                                            ui.heading("Version");
                                        });
                                        header.col(|ui| {
                                            ui.heading("Author");
                                        });
                                    })
                                    .body(|mut body| {
                                        for (index, r#mod) in
                                            &mut self.active.clone().iter_mut().enumerate()
                                        {
                                            body.row(20., |mut row| {
                                                row.col(|ui| {
                                                    ui.hyperlink_to(&r#mod.name, &r#mod.link);
                                                    let sense = ui.interact(
                                                        ui.max_rect(),
                                                        egui::Id::new(&format!(
                                                            "name-active{:#}",
                                                            index
                                                        )),
                                                        egui::Sense::click(),
                                                    );
                                                    if sense.double_clicked()
                                                        || sense.triple_clicked()
                                                    {
                                                        self.switch_active_inactive(
                                                            r#mod, index, true,
                                                        );
                                                    }
                                                });
                                                row.col(|ui| {
                                                    ui.label(&r#mod.version);
                                                    let sense = ui.interact(
                                                        ui.max_rect(),
                                                        egui::Id::new(&format!(
                                                            "version-active{:#}",
                                                            index
                                                        )),
                                                        egui::Sense::click(),
                                                    );
                                                    if sense.double_clicked()
                                                        || sense.triple_clicked()
                                                    {
                                                        self.switch_active_inactive(
                                                            r#mod, index, true,
                                                        );
                                                    }
                                                });
                                                row.col(|ui| {
                                                    ui.label(&r#mod.author);
                                                    let sense = ui.interact(
                                                        ui.max_rect(),
                                                        egui::Id::new(&format!(
                                                            "author-active{:#}",
                                                            index
                                                        )),
                                                        egui::Sense::click(),
                                                    );
                                                    if sense.double_clicked()
                                                        || sense.triple_clicked()
                                                    {
                                                        self.switch_active_inactive(
                                                            r#mod, index, true,
                                                        );
                                                    }
                                                });
                                            });
                                        }
                                    });
                            });
                    });
                }
            });
        });
    }

    fn downloads_display(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Downloads");
                ui.separator();
                let mut removal: Vec<String> = vec![];
                for (mod_name, (downloaded, total, _, _, _)) in self.downloads.iter_mut() {
                    ui.heading(mod_name);
                    let mut animate = true;
                    if downloaded == total {
                        if ui.button("X").clicked() {
                            removal.push(mod_name.clone());
                        }
                        animate = false;
                    }
                    ui.add(
                        egui::ProgressBar::new(*downloaded as f32 / *total as f32)
                            .animate(animate)
                            .show_percentage(),
                    );
                }
                for name in removal {
                    self.downloads.remove(&name);
                }
            });
        });
    }

    fn browse(&mut self, ctx: &egui::Context) {
        // TODO: Panel for browsing Mods:
        //      Some mods will be taken from sources like github if they have releases available.
        //      Possible that I may build and host versions myself from github to improve
        //      simplicity.
        //      All other mods will just require opening the NexusMods page and downloading
        //          MAYBE: unless a premium user that can oauth using the app and then select
        //              and download from inside the app.
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Browse");
            ui.separator();
            ui.heading("COMING SOON");
        });
    }

    fn switch_active_inactive(&mut self, r#mod: &mut GameMod, index: usize, is_active: bool) {
        if is_active {
            let mods_path = if r#mod.mod_id != 2400 {
                self.game_path.join("mods")
            } else {
                self.game_path.clone()
            };
            let mod_path = mods_path.join(&r#mod.folder_name);
            if let Err(e) = remove_dir_all(mod_path) {
                eprintln!("Failed to remove mod: {}", e);
            }
            self.inactive.push(r#mod.clone());
            self.active.remove(index);
        } else {
            let mods_path = if r#mod.mod_id != 2400 {
                self.game_path.join("mods")
            } else {
                self.game_path.clone()
            };
            let file = File::open(self.download_path.join(&r#mod.zip_name)).unwrap();
            let mut archive = zip::ZipArchive::new(file).unwrap();
            // Get the folder name for the mod
            {
                let first = archive.by_index(0).unwrap();
                if first.is_dir() {
                    r#mod.folder_name = first.enclosed_name().unwrap().display().to_string();
                } else {
                    r#mod.folder_name = first
                        .enclosed_name()
                        .unwrap()
                        .parent()
                        .unwrap()
                        .display()
                        .to_string();
                }
            }
            // FIXME: Take this off thread
            for i in 0..archive.len() {
                let mut file = archive.by_index(i).unwrap();
                let outpath = match file.enclosed_name() {
                    Some(path) => path.to_owned(),
                    None => continue,
                };
                let outpath = mods_path.join(outpath);

                if (*file.name()).ends_with('/') {
                    create_dir_all(&outpath).unwrap();
                } else {
                    if let Some(p) = outpath.parent() {
                        if !p.exists() {
                            create_dir_all(&p).unwrap();
                        }
                    }
                    let mut outfile = File::create(&outpath).unwrap();
                    io::copy(&mut file, &mut outfile).unwrap();
                }
            }
            // TODO: Handle installing smapi and updating
            if r#mod.mod_id == 2400 {
                #[cfg(target_os = "windows")]
                let executable = mods_path
                    .join(&r#mod.folder_name)
                    .join("internal\\windows\\SMAPI.Installer.exe")
                    .display()
                    .to_string();
                #[cfg(target_os = "linux")]
                let executable = mods_path
                    .join(&r#mod.folder_name)
                    .join("internal/linux/SMAPI.Installer")
                    .display()
                    .to_string();
                let installer = Command::new(executable)
                    .stdin(Stdio::piped())
                    .spawn()
                    .unwrap();
                let mut stdin = installer.stdin.unwrap();
                let mut writer = BufWriter::new(&mut stdin);
                writer.write_all(b"2\n").unwrap();
                writer
                    .write_all(self.game_path.display().to_string().as_bytes())
                    .unwrap();
                writer.write_all(b"\n").unwrap();
                writer.write_all(b"1\n").unwrap();
                writer.write_all(b"\n").unwrap();
            }
            self.active.push(r#mod.clone());
            self.inactive.remove(index);
        }
    }
}

impl eframe::App for SDMMApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let base_path = PathBuf::from(crate::download::BASE_URI);
        for (mod_name, downloaded, total, mod_id, file_id) in self.downloads_receiver.try_recv() {
            if !self.downloads.contains_key(&mod_name) {
                self.downloads
                    .insert(mod_name, (downloaded, total, mod_id, file_id, false));
            } else {
                let download = self.downloads.get_mut(&mod_name).unwrap();
                if download.0 < downloaded {
                    *download = (downloaded, total, mod_id, file_id, false)
                }
            }
        }
        for (mod_name, (downloaded, total, mod_id, file_id, saved)) in self.downloads.iter_mut() {
            if downloaded == total && !saved.clone() {
                let resp = self
                    .web_client
                    .get(format!(
                        "https://{}/stardewvalley/mods/{mod_id}.json",
                        base_path.display()
                    ))
                    .header("apikey", self.api_key.clone())
                    .send();
                let mod_details: ModDetails = match resp {
                    Ok(res) => {
                        if let Ok(details) = res.json::<ModDetails>() {
                            details
                        } else {
                            ModDetails::default()
                        }
                    },
                    Err(e) => {
                        eprintln!("Error getting mod details: {e}");
                        continue;
                    },
                };
                let resp = self
                    .web_client
                    .get(format!(
                        "https://{}/stardewvalley/mods/{mod_id}/files/{file_id}.json",
                        base_path.display(), 
                    ))
                    .header("apikey", self.api_key.clone())
                    .send();
                match resp {
                    Ok(res) => match res.json::<crate::download::ModFileDetails>() {
                        Ok(json) => {
                            if self
                                .inactive
                                .iter()
                                .filter(|m| m.mod_id == *mod_id as u64 && m.version == json.version.clone().unwrap_or(String::from("0")))
                                .count()
                                > 0
                                || self
                                    .active
                                    .iter()
                                    .filter(|m| m.mod_id == *mod_id as u64 && m.version == json.version.clone().unwrap_or(String::from("0")))
                                    .count()
                                    > 0
                            {
                                continue;
                            }
                            self.inactive.push(GameMod {
                                name: mod_details.name,
                                zip_name: mod_name.clone(),
                                folder_name: "".into(),
                                version: json.version.unwrap(),
                                author: mod_details.author,
                                link: format!("https://www.nexusmods.com/stardewvalley/mods/{mod_id}"),
                                mod_id: *mod_id as u64,
                                file_id: *file_id as u64,
                            });
                        }
                        Err(e) => {
                            eprintln!("Response was not valid json: {e}");
                        }
                    },
                    Err(e) => {
                        eprintln!("Request failed: {e}");
                    }
                }
                *saved = true;
            }
        }
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::widgets::global_dark_light_mode_switch(ui);
                ui.selectable_value(&mut self.state, Menus::Settings, "");
                ui.separator();
                ui.selectable_value(&mut self.state, Menus::Browse, "Browse");
                ui.selectable_value(&mut self.state, Menus::Downloading, "Downloading");
                ui.selectable_value(&mut self.state, Menus::Mods, "Mods");
            });
        });
        if self.needs_key {
            egui::Window::new("API KEY REQUIRED").show(ctx, |ui| {
                ui.heading("A valid NexusMods API Key is currently needed to use this program, please provide one and restart.");
                ui.hyperlink_to("Can be found at the bottom of this page", "https://www.nexusmods.com/users/myaccount?tab=api+access");
                let _ = ui.add(egui::TextEdit::singleline(&mut self.api_key));
                if ui.button("Submit").clicked() || ui.input().key_pressed(egui::Key::Enter) {
                    if !self.api_key.is_empty() {
                        self.needs_key = false;
                        frame.quit();
                    }
                }
            });
        }
        match self.state {
            Menus::Browse => self.browse(ctx),
            Menus::Downloading => self.downloads_display(ctx),
            Menus::Mods => self.mods_display(ctx),
            Menus::Settings => {}
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "download_path", &self.download_path);
        eframe::set_value(storage, "active_mods", &self.active);
        eframe::set_value(storage, "inactive_mods", &self.inactive);
        eframe::set_value(storage, "game_path", &self.game_path);
        eframe::set_value(storage, "api_key", &self.api_key);
        eframe::set_value(storage, "last_download", &self.last_download);
    }
}

fn load_from_storage(
    storage: &dyn Storage,
    web_client: &Client
) -> (Vec<GameMod>, Vec<GameMod>, PathBuf, String, String) {
    let base_path = PathBuf::from(crate::download::BASE_URI);

    let mut active_mods: Vec<GameMod> = vec![];
    let mut inactive_mods: Vec<GameMod> = vec![];
    let mut game_path = PathBuf::new();
    let mut last_download = String::new();
    let mut api_key = String::new();
    if let Some(path) = eframe::get_value(storage, "game_path") {
        game_path = path;
    } else {
        let path = std::env::current_dir().unwrap();
        let res = rfd::FileDialog::new()
            .set_title("Stardew Valley Game Directory")
            .add_filter("StardewValley", &["exe"])
            .set_directory(&path)
            .pick_folder();
        if let Some(path) = res {
            game_path = path;
        }
    }
    if let Some(last) = eframe::get_value(storage, "last_download") {
        last_download = last;
    }
    if let Some(key) = eframe::get_value(storage, "api_key") {
        api_key = key;
    } else {
        // TODO: SOME KIND OF REQUEST FOR APIKEY
    }

    #[derive(Deserialize, Serialize)]
    struct ModFileDetailsVec{
        pub files: Vec<ModFileDetails>
    }

    // TODO: Make a request to check the status of the mods, check for different/newer version
    if let Some(active) = eframe::get_value(storage, "active_mods") {
        active_mods = active;
    }

    if let Some(active_old) = eframe::get_value::<Vec<DepGameMod>>(storage, "active_mods") {
        'main: for old in active_old {
            let resp = web_client
                .get(format!(
                    "https://{}/stardewvalley/mods/{}/files.json",
                    base_path.display(),
                    old.id,
                ))
                .header("apikey", api_key.clone())
                .send();

            match resp {
                Ok(res) => match res.json::<ModFileDetailsVec>() {
                    Ok(details) => {
                        for dets in details.files {
                            if dets.file_name.unwrap() == old.zip_name && dets.version.unwrap() == old.version {
                                active_mods.push(GameMod {
                                    name: old.name.clone(),
                                    zip_name: old.zip_name.clone(),
                                    folder_name: old.folder_name.clone(),
                                    version: old.version.clone(),
                                    author: old.author.clone(),
                                    link: old.link.clone(),
                                    mod_id: old.id,
                                    file_id: dets.file_id.unwrap_or(0),
                                });
                                continue 'main;
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to serialize Vec<ModFileDetails>: {e}");
                    },
                }, 
                Err(e) => {
                    eprintln!("Failed to get files: {e}");
                },
            }
            active_mods.push(GameMod {
                name: old.name,
                zip_name: old.zip_name,
                folder_name: old.folder_name,
                version: old.version,
                author: old.author,
                link: old.link,
                mod_id: old.id,
                file_id: 0,
            });
        }
    }
    if let Some(inactive) = eframe::get_value(storage, "inactive_mods") {
        inactive_mods = inactive;
    }
    if let Some(inactive_old) = eframe::get_value::<Vec<DepGameMod>>(storage, "inactive_mods") {
        'main: for old in inactive_old {
            let resp = web_client
                .get(format!(
                    "https://{}/stardewvalley/mods/{}/files.json",
                    base_path.display(),
                    old.id,
                ))
                .header("apikey", api_key.clone())
                .send();

            match resp {
                Ok(res) => 
                match res.json::<ModFileDetailsVec>() {
                    Ok(details) => {
                        for dets in details.files {
                            if dets.file_name.unwrap() == old.zip_name && dets.version.unwrap() == old.version {
                                inactive_mods.push(GameMod {
                                    name: old.name.clone(),
                                    zip_name: old.zip_name.clone(),
                                    folder_name: old.folder_name.clone(),
                                    version: old.version.clone(),
                                    author: old.author.clone(),
                                    link: old.link.clone(),
                                    mod_id: old.id,
                                    file_id: dets.file_id.unwrap_or(0),
                                });
                                continue 'main;
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to serialize Vec<ModFileDetails>: {e}")
                    },
                }, 
                Err(e) => {
                    eprintln!("Failed to get files: {e}")
                },
            }
            inactive_mods.push(GameMod {
                name: old.name,
                zip_name: old.zip_name,
                folder_name: old.folder_name,
                version: old.version,
                author: old.author,
                link: old.link,
                mod_id: old.id,
                file_id: 0,
            });
        }
    }

    (
        active_mods,
        inactive_mods,
        game_path,
        last_download,
        api_key,
    )
}

fn setup_download_path(context: &CreationContext<'_>) -> PathBuf {
    if let Some(storage) = context.storage {
        if let Some(dir) = eframe::get_value(storage, "download_path") {
            return dir;
        } else if let Some(proj_dirs) = ProjectDirs::from("", "", crate::PROJECT_NAME) {
            let dir = proj_dirs.data_dir();
            if let Ok(d) = read_dir(&dir) {
                let directories = d.filter(|d| d.as_ref().unwrap().file_name() == "mods");
                if directories.count() == 0 {
                    create_dir(dir.join("mods")).unwrap();
                }
            }
            return dir.join("mods");
        }
    } else if let Some(proj_dirs) = ProjectDirs::from("", "", crate::PROJECT_NAME) {
        let dir = proj_dirs.data_dir();
        if let Ok(d) = read_dir(&dir) {
            let directories = d.filter(|d| d.as_ref().unwrap().file_name() == "mods");
            if directories.count() == 0 {
                create_dir(dir.join("mods")).unwrap();
            }
        }
        return dir.join("mods");
    }
    panic!("Could not get or create the download path");
}
