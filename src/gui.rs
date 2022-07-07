use eframe::egui;
use egui_extras::{Size, TableBuilder};

#[derive(Default, PartialEq)]
enum Menus {
    Browse,
    Downloading,
    #[default]
    Mods,
}

#[derive(Default)]
pub struct SDMMApp {
    state: Menus,
    // Maybe wants to be a string of paths
    loaded: Vec<crate::GameMod>,
}

impl SDMMApp {
    pub fn new(context: &eframe::CreationContext<'_>) -> SDMMApp {
        // Customization can be done here from the context
        // and loading persistance data
        context.egui_ctx.set_visuals(egui::Visuals::dark());
        // Load all the currently downloaded mods into loaded vec
        SDMMApp {
            loaded: vec![crate::GameMod {
                name: "Content Patcher".into(),
                version: "1.27.2".into(),
                author: "Pathoschild".into(),
                link: "https://www.nexusmods.com/stardewvalley/mods/1915".into(),
                active: false,
            }],
            ..Default::default()
        }
    }
}

impl eframe::App for SDMMApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::widgets::global_dark_light_mode_switch(ui);
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
                });
            }
            Menus::Downloading => {
                // TODO: Panel for currently Downloading.
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading("Downloads");
                    ui.separator();
                });
            }
            Menus::Mods => {
                // TODO: Panel for Downloaded and Active mods/tools.
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading("Mods");
                    ui.separator();
                    TableBuilder::new(ui)
                        .cell_layout(
                            egui::Layout::left_to_right().with_cross_align(egui::Align::Center),
                        )
                        .columns(Size::remainder().at_least(5.), 5)
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
                            header.col(|ui| {
                                ui.heading("Active");
                            });
                        })
                        .body(|mut body| {
                            for r#mod in &mut self.loaded {
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
                                    row.col(|ui| {
                                        let toggle_value = if r#mod.active {
                                            ui.toggle_value(&mut r#mod.active, "Active")
                                        } else {
                                            ui.toggle_value(&mut r#mod.active, "Inactive")
                                        };
                                        if toggle_value.clicked() {
                                            // Handle adding to and removing from the games mods
                                            // directory
                                            if r#mod.active {
                                                // Was just activated
                                            } else {
                                                // Was just deactivated
                                            }
                                        }
                                    });
                                    row.col(|ui| if ui.button("Delete").clicked() {});
                                });
                            }
                        });
                });
            }
        }
    }
}
