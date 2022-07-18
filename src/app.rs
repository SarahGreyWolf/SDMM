use directories_next::ProjectDirs;
use eframe::{egui, CreationContext, Storage};
use egui_extras::{Size, TableBuilder};
use serde::{Serialize, Deserialize};
use core::panic;
use std::fs::{create_dir, read_dir};
use std::path::{PathBuf};
use std::sync::mpsc::{sync_channel, Receiver};
use std::collections::HashMap;
use crate::download::handle_download_requests;

#[derive(Serialize, Deserialize, Default)]
struct GameMod {
    name: String,
    file_name: String,
    version: String,
    author: String,
    link: String,
    active: bool,
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
    downloads_receiver: Receiver<(String, usize, usize, usize)>,
    state: Menus,
    download_path: PathBuf,
    game_path: PathBuf,
    api_key: String,
    needs_key: bool,
    downloads: HashMap<String, (usize, usize, usize)>,
    inactive: Vec<GameMod>,
    active: Vec<GameMod>,
}

impl SDMMApp {
    pub fn new(context: &eframe::CreationContext<'_>) -> SDMMApp {
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

        let mut active: Vec<GameMod> = vec![];
        let mut inactive: Vec<GameMod> = vec![];
        let mut game_path = PathBuf::new();
        let mut api_key = String::new();
        if let Some(storage) = context.storage {
            let loaded = load_from_storage(storage);
            active = loaded.0;
            inactive = loaded.1;
            game_path = loaded.2;
            api_key = loaded.3;
        }

        let (sync_sender, receiver) = sync_channel::<(String, usize, usize, usize)>(1);
        let download_path_clone = download_path.clone();
        handle_download_requests(sync_sender, download_path, api_key.clone());

        inactive.push(GameMod {
            name: "Content Patcher".into(),
            file_name: "Content Patcher 1.27.2-1915-1-27-2-1656983440.zip".into(),
            version: "1.27.2".into(),
            author: "Pathoschild".into(),
            link: "https://www.nexusmods.com/stardewvalley/mods/1915".into(),
            active: false,
        });
        let mut needs_key = true;
        if !api_key.is_empty() {
            needs_key = false;
        }

        // Load all the currently downloaded mods into loaded vec
        SDMMApp {
            downloads_receiver: receiver,
            state: Menus::default(),
            download_path: download_path_clone,
            game_path,
            api_key,
            needs_key,
            downloads: HashMap::new(),
            inactive,
            active,
        }
    }

    fn mods_display(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.columns(3, |columns| {
                if let [left_panel, center, right_panel] = &mut columns[0..3] {
                    // Left (inactive) Table
                    left_panel.heading("Inactive");
                    left_panel.separator();
                    left_panel.push_id(0, |ui| {
                        TableBuilder::new(ui)
                        .cell_layout(
                            egui::Layout::left_to_right().with_cross_align(egui::Align::Center),
                        )
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
                            for r#mod in &mut self.inactive {
                                body.row(20., |mut row| {
                                    row.col(|ui| {
                                        ui.hyperlink_to(&r#mod.name, &r#mod.link);
                                    });
                                    row.col(|ui| {
                                        ui.label(&r#mod.version);
                                    });
                                    row.col(|ui| {
                                        ui.label(&r#mod.author);
                                    });
                                });
                            }
                        });
                    });
                    // Center Buttons
                    if center.button("▲").clicked() {}
                    if center.button("►").clicked() {}
                    if center.button("◄").clicked() {}
                    if center.button("▼").clicked() {}
                    
                    // Right (active) Table
                    right_panel.heading("Active");
                    right_panel.separator();
                    right_panel.push_id(1, |ui| {
                        TableBuilder::new(ui)
                            .cell_layout(
                                egui::Layout::left_to_right().with_cross_align(egui::Align::Center),
                            )
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
                                for r#mod in &mut self.active {
                                    body.row(20., |mut row| {
                                        row.col(|ui| {
                                            ui.hyperlink_to(&r#mod.name, &r#mod.link);
                                        });
                                        row.col(|ui| {
                                            ui.label(&r#mod.version);
                                        });
                                        row.col(|ui| {
                                            ui.label(&r#mod.author);
                                        });
                                    });
                                }
                            });
                    });
                }
            });
        });
    }

    fn downloads_display(&mut self, ctx: &egui::Context) {
        for (mod_name, downloaded, total, id) in self.downloads_receiver.try_recv() {
            if !self.downloads.contains_key(&mod_name) {
                self.downloads.insert(mod_name, (downloaded, total, id));
            } else {
                let download = self.downloads.get_mut(&mod_name).unwrap();
                if download.0 < downloaded {
                    *download = (downloaded, total, id)
                }
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Downloads");
            ui.separator();
            for (mod_name, (downloaded, total, id)) in self.downloads.clone().iter() {
                ui.heading(mod_name);
                if ui.button("X").clicked() {
                    if downloaded == total {
                        self.downloads.remove(mod_name).unwrap();
                    }
                }
                ui.add(egui::ProgressBar::new(*downloaded as f32 / *total as f32).animate(true).show_percentage());
            }
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
}



impl eframe::App for SDMMApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
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
                ui.heading("A valid NexusMods API Key is needed to use this program, please provide one and restart.");
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
        eframe::set_value(
            storage,
            "download_dir",
            &self.download_path,
        );
        eframe::set_value(
            storage,
            "active_mods",
            &self.active
        );
        eframe::set_value(
            storage,
            "inactive_mods",
            &self.inactive
        );
        eframe::set_value(
            storage,
            "game_path",
            &self.game_path
        );
        eframe::set_value(
            storage,
            "api_key",
            &self.api_key
        );
    }
}

fn load_from_storage(storage: &dyn Storage) -> (Vec<GameMod>, Vec<GameMod>, PathBuf, String) {
    let mut active_mods: Vec<GameMod> = vec![];
    let mut inactive_mods: Vec<GameMod> = vec![];
    let mut game_path = PathBuf::new();
    let mut api_key = String::new();
    if let Some(active) = eframe::get_value(storage, "active_mods") {
        active_mods = active;
    }
    if let Some(inactive) = eframe::get_value(storage, "inactive_mods") {
        inactive_mods = inactive;
    }
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
    if let Some(key) = eframe::get_value(storage, "api_key") {
        api_key = key;
    } else {
        // TODO: SOME KIND OF REQUEST FOR APIKEY
    }

    (active_mods, inactive_mods, game_path, api_key)
}

fn setup_download_path(context: &CreationContext<'_>) -> PathBuf {
        if let Some(storage) = context.storage {
            if let Some(dir) = eframe::get_value(storage, "download_dir") {
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
