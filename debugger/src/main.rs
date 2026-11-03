use chaos_seeker::state::GameState;
use eframe::egui;
use tetanes_core::prelude::*;

fn main() {
    let _ = std::fs::create_dir("screenshots");

    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "chaos-seeker",
        native_options,
        Box::new(|cc| Ok(Box::new(DebuggerApp::new(cc)))),
    );
}

struct DebuggerApp {
    game: GameState,
    emulator_texture: Option<egui::TextureHandle>,
    pathfind_x: u8,
    pathfind_y: u8,
}

impl DebuggerApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut control_deck = ControlDeck::new();
        control_deck
            .load_rom_path("ff1.nes")
            .expect("failed to load ROM");

        let game = GameState::new(control_deck);
        let emulator_texture = None;
        let pathfind_x = 0;
        let pathfind_y = 0;
        DebuggerApp {
            game,
            emulator_texture,
            pathfind_x,
            pathfind_y,
        }
    }
}

impl DebuggerApp {
    fn update_emulator_texture(&mut self, ctx: &egui::Context) {
        let Some(image) = self.game.image() else {
            return;
        };

        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [image.width() as usize, image.height() as usize],
            image.as_raw(),
        );

        if let Some(texture) = self.emulator_texture.as_mut() {
            texture.set(color_image, egui::TextureOptions::NEAREST);
        } else {
            self.emulator_texture = Some(ctx.load_texture(
                "emulator-frame",
                color_image,
                egui::TextureOptions::NEAREST,
            ));
        }
    }
}

impl eframe::App for DebuggerApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.button("tick").clicked() {
                        if let Err(error) = self.game.tick() {
                            eprintln!("emulator tick failed: {error}");
                        }

                        self.update_emulator_texture(ui.ctx());
                    }

                    ui.separator();

                    ui.vertical(|ui| {
                        ui.add(egui::Slider::new(&mut self.pathfind_x, 0..=u8::MAX).text("x"));
                        ui.add(egui::Slider::new(&mut self.pathfind_y, 0..=u8::MAX).text("y"));
                    });
                });

                ui.separator();

                if let Some(texture) = &self.emulator_texture {
                    ui.add(egui::Image::new((texture.id(), texture.size_vec2())));
                }
            })
        });
    }
}
