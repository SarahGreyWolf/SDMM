use directories_next::ProjectDirs;
use eframe::{egui, CreationContext};
use egui_extras::{Size, TableBuilder};
use core::panic;
use std::fs::{create_dir, read_dir};
use std::path::{PathBuf};
use std::sync::mpsc::{sync_channel, Receiver};
use std::collections::HashMap;
use crate::download::handle_download_requests;


#[derive(Default, PartialEq)]
enum Menus {
    Browse,
    Downloading,
    #[default]
    Mods,
    Settings,
}

pub struct SDMMApp {
    downloads_receiver: Receiver<(String, usize, usize)>,
    state: Menus,
    download_path: PathBuf,
    downloads: HashMap<String, (usize, usize)>,
    inactive: Vec<crate::GameMod>,
    active: Vec<crate::GameMod>,
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

        let (sync_sender, receiver) = sync_channel::<(String, usize, usize)>(1);
        let download_path_clone = download_path.clone();
        handle_download_requests(sync_sender, download_path);

        // Load all the currently downloaded mods into loaded vec
        SDMMApp {
            downloads_receiver: receiver,
            state: Menus::default(),
            download_path: download_path_clone,
            downloads: HashMap::new(),
            inactive: vec![crate::GameMod {
                name: "Content Patcher".into(),
                version: "1.27.2".into(),
                author: "Pathoschild".into(),
                link: "https://www.nexusmods.com/stardewvalley/mods/1915".into(),
                active: false,
            }],
            active: vec![],
        }
    }

    fn mods_display(&mut self, ctx: &egui::Context) {
    }

    fn downloads_display(&mut self, ctx: &egui::Context) {
        for (mod_name, downloaded, total) in self.downloads_receiver.try_recv() {
            if !self.downloads.contains_key(&mod_name) {
                self.downloads.insert(mod_name, (downloaded, total));
            } else {
                let download = self.downloads.get_mut(&mod_name).unwrap();
                if download.0 < downloaded {
                    *download = (downloaded, total)
                }
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Downloads");
            ui.separator();
            for (mod_name, (downloaded, total)) in self.downloads.iter() {
                ui.heading(mod_name);
                ui.add(egui::ProgressBar::new(*downloaded as f32 / *total as f32).animate(true).show_percentage());
            }
        });
    }
}



impl eframe::App for SDMMApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::widgets::global_dark_light_mode_switch(ui);
                if ui.button("").clicked() {
                    self.state = Menus::Settings;
                }
                ui.separator();
                ui.selectable_value(&mut self.state, Menus::Browse, "Browse");
                ui.selectable_value(&mut self.state, Menus::Downloading, "Downloading");
                ui.selectable_value(&mut self.state, Menus::Mods, "Mods");
            });
        });
        match self.state {
            Menus::Browse => {
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
            Menus::Downloading => {
                self.downloads_display(ctx);
            }
            Menus::Mods => {
                self.mods_display(ctx);
            }
            Menus::Settings => {}
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(
            storage,
            "download_dir",
            &self.download_path,
        );
    }
}

fn setup_download_path(context: &CreationContext<'_>) -> PathBuf {
        if let Some(storage) = context.storage {
            if let Some(dir) = storage.get_string("download_dir") {
                let mut dir = dir.replace("\\\\", "\\");
                dir = dir.replace('"', "");
                return PathBuf::from(dir);
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
