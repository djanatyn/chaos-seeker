#![feature(default_field_values)]

use chaos_seeker::overworld::WorldMap;
use chaos_seeker::pathfinding::{PathProblem, TravelCapabilities};
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

#[derive(Debug, PartialEq)]
enum DebuggerMode {
    Emulator,
    MapExplorer,
}

struct DebuggerApp {
    game: GameState,
    emulator_texture: Option<egui::TextureHandle> = None,
    map_explorer_texture: Option<egui::TextureHandle> = None,
    map_explorer_texture_needs_update: bool = true,
    map: WorldMap,
    hover_tile: Option<(i32, i32)> = None,
    map_source: Option<(u8, u8)> = None,
    map_target: Option<(u8, u8)> = None,
    playing: bool = false,
    speed: f64 = 0.25,
    last_update: Instant,
    accumulated_time: Duration = Duration::ZERO,
    mode: DebuggerMode = DebuggerMode::Emulator,
    scene_rect: egui::Rect = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(256.0, 256.0)),
}

impl DebuggerApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut control_deck = ControlDeck::new();
        let rom = include_bytes!("../../ff1.nes");
        control_deck
            .load_rom("ff1.nes", &mut &rom[..])
            .expect("failed to load ROM");
        control_deck.set_filter(VideoFilter::Pixellate);

        let map = WorldMap::parse(rom).expect("failed to load world map");
        DebuggerApp {
            game: GameState::new(control_deck),
            last_update: Instant::now(),
            map,
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

    fn update_map_explorer_texture(&mut self, ctx: &egui::Context) {
        let mut image = self.map.map.image(&self.map.properties);
        if let Some(source) = self.map_source
            && let Some(target) = self.map_target
        {
            let solution = PathProblem {
                starting_tile: source,
                goal_tile: target,
                map: &self.map,
                capabilities: TravelCapabilities {
                    canoe: true,
                    ship: false,
                    airship: false,
                },
            };

            if let Ok(path) = solution.solve() {
                image = self.map.map.pathfinding_image(&path, &self.map.properties)
            }
        }

        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [image.width() as usize, image.height() as usize],
            image.as_raw(),
        );

        if let Some(texture) = self.map_explorer_texture.as_mut() {
            texture.set(color_image, egui::TextureOptions::NEAREST);
        } else {
            self.map_explorer_texture = Some(ctx.load_texture(
                "map-explorer",
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

        if self.playing {
            self.accumulated_time += elapsed.mul_f64(self.speed);

            let mut frames = 0;

            while self.accumulated_time >= frame_time && frames < MAX_FRAMES_PER_UPDATE {
                self.game.auto_advance().expect("oh no game failed"); // TODO
                if self.game.progress == GameProgress::Pause {
                    self.playing = false;
                }
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

        if self.mode == DebuggerMode::MapExplorer && self.map_explorer_texture_needs_update {
            self.update_map_explorer_texture(ui.ctx());
            self.map_explorer_texture_needs_update = false;
        }

        egui::Panel::left("tools")
            .min_size(32.0)
            .resizable(true)
            .show(ui, |ui| {
                ui.vertical_centered_justified(|ui| {
                    // ui.vertical(|ui| {
                    //     ui.label("world map pathfinding coordinates");
                    //     ui.add(egui::Slider::new(&mut self.pathfind_x, 0..=u8::MAX).text("x"));
                    //     ui.add(egui::Slider::new(&mut self.pathfind_y, 0..=u8::MAX).text("y"));
                    // });
                    ui.vertical(|ui| {
                        ui.label("game speed");
                        ui.add(egui::Slider::new(&mut self.speed, 0.0..=4.0));
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label("game state");
                        ui.monospace(format!("{:#?}", self.game.progress));
                    });
                    ui.vertical(|ui| {
                        ui.label("map explorer");
                        ui.monospace(format!("hover_tile: {:#?}", self.hover_tile));
                        ui.monospace(format!("map_source: {:#?}", self.map_source));
                        ui.monospace(format!("map_target: {:#?}", self.map_target));
                    });
                })
            });

        egui::Panel::top("control")
            .min_size(32.0)
            .resizable(false)
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    if ui.button("map explorer").clicked() {
                        self.mode = DebuggerMode::MapExplorer;
                    }
                    if ui.button("emulator").clicked() {
                        self.mode = DebuggerMode::Emulator;
                    }

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

        match self.mode {
            DebuggerMode::Emulator => {
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(ui, |ui| {
                        if let Some(texture) = &self.emulator_texture {
                            let native_size = texture.size_vec2();
                            let available_size = ui.available_size();

                            let fit_scale = (available_size.x / native_size.x)
                                .min(available_size.y / native_size.y);

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
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                egui::Color32::WHITE,
                            );
                        }
                    });
            }
            DebuggerMode::MapExplorer => {
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(ui, |ui| {
                        egui::Scene::new()
                            .zoom_range(1.0..=16.0)
                            .scroll_zooms(true)
                            .drag_pan_buttons(egui::DragPanButtons::PRIMARY)
                            .show(ui, &mut self.scene_rect, |ui| {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::Vec2::splat(256.0),
                                    egui::Sense::hover(),
                                );

                                if let Some(pos) = response.hover_pos() {
                                    self.hover_tile =
                                        Some((pos.x.floor() as i32, pos.y.floor() as i32));
                                } else {
                                    self.hover_tile = None;
                                }

                                let click = ui.input(|i| {
                                    let pos = i.pointer.interact_pos();
                                    if i.pointer.button_clicked(egui::PointerButton::Primary) {
                                        Some((egui::PointerButton::Primary, pos))
                                    } else if i
                                        .pointer
                                        .button_clicked(egui::PointerButton::Secondary)
                                    {
                                        Some((egui::PointerButton::Secondary, pos))
                                    } else {
                                        None
                                    }
                                });

                                if let Some((btn, Some(pos_global))) = click {
                                    // convert mouse position from screen to scene coordinates
                                    let from_global = ui
                                        .ctx()
                                        .layer_transform_from_global(ui.layer_id())
                                        .unwrap_or(egui::emath::TSTransform::IDENTITY);
                                    let scene_pos = from_global * pos_global;

                                    let in_map = scene_pos.x >= 0.0
                                        && scene_pos.y >= 0.0
                                        && scene_pos.x < 256.0
                                        && scene_pos.y < 256.0;

                                    if in_map {
                                        let tile_x = scene_pos.x.floor() as u8;
                                        let tile_y = scene_pos.y.floor() as u8;

                                        match btn {
                                            egui::PointerButton::Primary => {
                                                self.map_explorer_texture_needs_update = true;
                                                self.map_source = Some((tile_x, tile_y))
                                            }
                                            egui::PointerButton::Secondary => {
                                                self.map_explorer_texture_needs_update = true;
                                                self.map_target = Some((tile_x, tile_y))
                                            }
                                            _ => {}
                                        }
                                    }
                                }

                                let full_uv = egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                );
                                let texture_id = self.map_explorer_texture.as_ref().map(|t| t.id());
                                if let Some(id) = texture_id {
                                    ui.painter().image(id, rect, full_uv, egui::Color32::WHITE);
                                }
                            });
                    });
            }
        }
    }
}
