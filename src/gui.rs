use directories_next::ProjectDirs;
use eframe::{egui, CreationContext};
use egui_extras::{Size, TableBuilder};
use interprocess::local_socket::LocalSocketListener;
use serde::{Deserialize, Serialize};
use core::panic;
use std::fs::{create_dir, read_dir, File};
use std::io::{Write, Read};
use std::path::{Path, PathBuf};
use std::sync::mpsc::TrySendError::Disconnected;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;
use std::collections::HashMap;
use std::time::Instant;
use futures_util::StreamExt;


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
        handle_downloads(sync_sender, download_path);

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

fn handle_downloads(sync_sender: SyncSender<(String, usize, usize)>, download_path: PathBuf) {
    #[derive(Deserialize, Serialize, Debug, Clone)]
    struct Links(Vec<DownloadLink>);
    
    #[derive(Deserialize, Serialize, Debug, Clone)]
    struct DownloadLink {
        name: String,
        short_name: String,
        #[serde(rename = "URI")]
        uri: String,
    }
    let base_path = Path::new("api.nexusmods.com/v1/games/");
    let listener = LocalSocketListener::bind("/tmp/sdmm.sock").unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .thread_name("download_handler")
        .thread_stack_size(3 * 1024 * 1024)
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    thread::spawn(move || {
        for stream in listener.incoming() {
            if let Err(e) = &stream {
                eprintln!("Stream Error: {e}");
            }
            let mut stream = stream.unwrap();
            let download_path = download_path.clone();
            let sync_sender = sync_sender.clone();
            let mut buffer = vec![0u8; 1024];
            stream.read_exact(&mut buffer).unwrap();
            runtime.spawn(async move {
                let sync_sender = sync_sender.clone();
                let client = reqwest::Client::new();
                let mut string = String::from_utf8(buffer).unwrap();
                string = string.replace('\0', "");
                let url = string.split_at(6).1;
                let url: Vec<&str> = url.split('?').collect();
                let path = Path::new(&url[0]);
                let queries = url[1];
                // TODO: Handle Downloading here
                let link_request = format!(
                    "https://{}{}/download_link.json?{}",
                    base_path.display(),
                    path.display(),
                    &queries
                );
                // println!("{}", link_request);
                let response = client.get(link_request)
                // This needs to be less static and with a proper API key from Nexus for
                // the application
                .header("apikey", "UUhzVis4aGZqdnJjVHJLWlNQeHVRakVERmo4RmNaZTZ6VlNoN1I3QTg1dVNZeE50NFEyUUZ0UHpzVHBFQXVqQi0tYzdpTnNxcmtoRiszZGw3Z08wRUdBUT09--6daacd911a1c2ee0df51c0380157d51e34b53495")
                .send().await;
                match response {
                    Ok(resp) => {
                        let response_body = resp.json::<Links>().await.unwrap();
                        let download = response_body.0.first().unwrap();
                        println!("Beginning Download from {:?}", download.uri);
                        let download_clone = download.clone();
                        let download_path_clone = download_path.clone();
                                let client = client.clone();
                                let sync_sender = sync_sender.clone();
                                let res = client.get(&download_clone.uri).send().await;
                                if let Ok(resp) = res {
                                    let total_size =
                                        if let Some(size) = resp.content_length() {
                                            size as usize
                                        } else {
                                            eprintln!(
                                                "Failed to download from {}",
                                                &download_clone.uri
                                            );
                                            0
                                        };
                                    if total_size == 0 {
                                        eprintln!("File size is 0");
                                        return;
                                    }
                                    let split_uri = download_clone.uri
                                    .split('/')
                                    .collect::<Vec<&str>>();
                                    let file_name = if split_uri[2].contains("nexus-cdn") {
                                        split_uri[5]
                                    } else {
                                        split_uri[6]
                                    };
                                    let file_name =
                                        file_name.split('?').next().unwrap();
                                    if let Ok(mut file) =
                                        File::create(download_path_clone.join(file_name))
                                    {
                                        let mut downloaded: usize = 0;
                                        match sync_sender.try_send((file_name.to_string(), downloaded, total_size)) {
                                            Err(e) if e == Disconnected((file_name.to_string(), downloaded, total_size)) => {
                                                panic!("Error Occured when sending: {}", e);
                                            }
                                            _ => {}
                                        }
                                        let mut stream = resp.bytes_stream();
                                        while let Some(item) = stream.next().await {
                                            if let Ok(chunk) = item {
                                                let written = file.write(&chunk).unwrap();
                                                assert_eq!(written, chunk.len());
                                                downloaded += chunk.len();
                                                match sync_sender.try_send((file_name.to_string(), downloaded, total_size)) {
                                                    Err(e) if e == Disconnected((file_name.to_string(), downloaded, total_size)) => {
                                                        panic!("Error Occured when sending: {}", e);
                                                    }
                                                    _ => {}
                                                }
                                            } else {
                                                eprintln!("Failed to create file at {}", download_path_clone.join(file_name).display());
                                                return;
                                            }
                                        }
                                        sync_sender.send((file_name.to_string(), downloaded, total_size)).unwrap();
                                        println!("Finished Download of {}", file_name);
                                    } else {
                                        eprintln!("Failed to create file at {}", download_path_clone.join(file_name).display());
                                    }
                                } else {
                                    eprintln!(
                                        "Failed to download from {}",
                                        &download_clone.uri
                                    );
                                }
                    }
                    Err(e) => {
                        eprintln!("Error Occured: {}", e);
                    }
                }
            });
        }
    });
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
