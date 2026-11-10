
use eframe::egui;
use std::path::PathBuf;

const GREEN: egui::Color32 =
    egui::Color32::from_rgb(16, 200, 80);

struct Game {
    name: String,
    path: PathBuf,
}

#[derive(Default)]
struct X360Nexus {
    games: Vec<Game>,
    selected: Option<usize>,
    status: String,
}

impl X360Nexus {
    fn import_game(&mut self) {
        let files = rfd::FileDialog::new()
            .add_filter("Xbox 360", &["iso", "xex", "god"])
            .pick_files();

        if let Some(files) = files {
            for path in files {
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                self.games.push(Game { name, path });
            }

            self.status = "Jogos importados!".into();
        }
    }
}

impl eframe::App for X360Nexus {
    fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) {
        ctx.set_visuals(egui::Visuals::dark());

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(
                egui::RichText::new("X360 NEXUS")
                    .size(36.0)
                    .color(GREEN)
                    .strong(),
            );

            ui.label("Xbox 360 Gaming Hub");
            ui.separator();

            if ui.button("IMPORTAR JOGO").clicked() {
                self.import_game();
            }

            ui.add_space(20.0);
            ui.heading("Biblioteca de Jogos");

            for (i, game) in self.games.iter().enumerate() {
                if ui
                    .selectable_label(
                        self.selected == Some(i),
                        &game.name,
                    )
                    .clicked()
                {
                    self.selected = Some(i);
                }
            }

            if let Some(i) = self.selected {
                if let Some(game) = self.games.get(i) {
                    ui.label(game.path.display().to_string());

                    if ui.button("INICIAR JOGO").clicked() {
                        self.status =
                            "Emulacao ainda nao implementada."
                                .into();
                    }
                }
            }

            ui.separator();
            ui.label(&self.status);
            ui.label("X360 Nexus v0.1");
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("X360 Nexus")
            .with_inner_size([1100.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native(
        "X360 Nexus",
        options,
        Box::new(|_cc| Ok(Box::new(X360Nexus::default()))),
    )
}
