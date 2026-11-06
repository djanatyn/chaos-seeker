#![feature(default_field_values)]

use chaos_seeker::state::{GameProgress, GameState};
use eframe::egui;
use std::time::{Duration, Instant};
use tetanes_core::prelude::*;
use tetanes_core::video::VideoFilter;

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
    emulator_texture: Option<egui::TextureHandle> = None,
    pathfind_x: u8 = 0,
    pathfind_y: u8 = 0,
    playing: bool = false,
    speed: f64 = 0.1,
    last_update: Instant,
    accumulated_time: Duration = Duration::ZERO,
}

impl DebuggerApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut control_deck = ControlDeck::new();
        let rom = include_bytes!("../../ff1.nes");
        control_deck
            .load_rom("ff1.nes", &mut &rom[..])
            .expect("failed to load ROM");
        control_deck.set_filter(VideoFilter::Pixellate);

        DebuggerApp {
            game: GameState::new(control_deck),
            last_update: Instant::now(),
            ..
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
                egui::TextureOptions {
                    wrap_mode: egui::TextureWrapMode::ClampToEdge,
                    ..egui::TextureOptions::NEAREST
                },
            ));
        }
    }
}

impl eframe::App for DebuggerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        const FRAME_RATE: f64 = 60.0988;
        const MAX_FRAMES_PER_UPDATE: usize = 8;

        let frame_time: Duration = Duration::from_secs_f64(1.0 / FRAME_RATE);

        let now = Instant::now();
        let elapsed = now
            .duration_since(self.last_update)
            .min(Duration::from_millis(250));
        self.last_update = now;

        if self.game.progress == GameProgress::Pause {
            self.playing = false;
        }

        if self.playing {
            self.accumulated_time += elapsed.mul_f64(self.speed);

            let mut frames = 0;

            while self.accumulated_time >= frame_time && frames < MAX_FRAMES_PER_UPDATE {
                self.game.auto_advance().expect("oh no game failed"); // TODO
                self.update_emulator_texture(ui.ctx());
                self.accumulated_time -= frame_time;
                frames += 1;
            }

            if frames == MAX_FRAMES_PER_UPDATE {
                self.accumulated_time = Duration::ZERO;
            }

            ui.ctx()
                .request_repaint_after(frame_time.saturating_sub(self.accumulated_time));
        }

        egui::Panel::left("tools")
            .min_size(32.0)
            .resizable(true)
            .show(ui, |ui| {
                ui.vertical_centered_justified(|ui| {
                    ui.vertical(|ui| {
                        ui.label("world map pathfinding coordinates");
                        ui.add(egui::Slider::new(&mut self.pathfind_x, 0..=u8::MAX).text("x"));
                        ui.add(egui::Slider::new(&mut self.pathfind_y, 0..=u8::MAX).text("y"));
                    });
                    ui.vertical(|ui| {
                        ui.label("game speed");
                        ui.add(egui::Slider::new(&mut self.speed, 0.0..=8.0));
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label("game state");
                        ui.monospace(format!("{:#?}", self.game.progress));
                    });
                })
            });

        egui::Panel::top("control")
            .min_size(32.0)
            .resizable(false)
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    let _ = ui.button("world map"); // TODO
                    let _ = ui.button("emulator"); // TODO
                    ui.separator();
                    if ui.button("play").clicked() {
                        self.playing = true;
                    }
                    if ui.button("tick").clicked() {
                        if let Err(error) = self.game.auto_advance() {
                            eprintln!("emulator tick failed: {error}");
                        }

                        self.update_emulator_texture(ui.ctx());
                    }
                    if ui.button("pause").clicked() {
                        self.playing = false;
                    }
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                if let Some(texture) = &self.emulator_texture {
                    let native_size = texture.size_vec2();
                    let available_size = ui.available_size();

                    let fit_scale =
                        (available_size.x / native_size.x).min(available_size.y / native_size.y);

                    let scale = if fit_scale >= 1.0 {
                        fit_scale.floor()
                    } else {
                        fit_scale
                    };

                    let display_size = native_size * scale;

                    // reserve central panel
                    let (viewport_rect, _) =
                        ui.allocate_exact_size(available_size, egui::Sense::hover());

                    // center emulator inside central panel
                    let image_rect =
                        egui::Rect::from_center_size(viewport_rect.center(), display_size);

                    // background around viewport
                    ui.painter()
                        .rect_filled(viewport_rect, 0.0, egui::Color32::BLACK);

                    // paint texture appropriately-sized
                    ui.painter().image(
                        texture.id(),
                        image_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
            });
    }
}
