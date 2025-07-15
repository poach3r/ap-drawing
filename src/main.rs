use core::f32;
use std::marker::PhantomData;
use std::usize;

use nannou::prelude::*;
use nannou::{
    event::{Update, WindowEvent},
    App, Frame,
};
use nannou_egui::egui::epaint::Shadow;
use nannou_egui::egui::{Vec2, Visuals};
use nannou_egui::{self, egui, Egui};

const OVERLAY: Rgba8 = Rgba8 {
    color: Rgb {
        red: 255,
        green: 255,
        blue: 255,
        standard: PhantomData,
    },
    alpha: 100,
};

enum Brush {
    Circle,
    Square,
}

#[derive(Clone)]
struct Pixel {
    color: Rgb8,
    x: f32,
    y: f32,
}

impl Default for Pixel {
    fn default() -> Self {
        Pixel {
            color: BLACK,
            x: 0.0,
            y: 0.0,
        }
    }
}

struct State {
    pixels: Vec<Vec<Pixel>>,
    drawing: bool,
    erasing: bool,
    should_reset: bool,
    should_exit: bool,
    should_calc_positions: bool,
}

struct Settings {
    brush: Brush,
    brush_size: usize,
    grid_size: usize,
    display_fps: bool,
    dark_mode: bool,
    primary_color: Rgb8,
    secondary_color: Rgb8,
    primary_color_buf: [u8; 3],
    secondary_color_buf: [u8; 3],
}

struct Model {
    egui: Egui,
    state: State,
    settings: Settings,
}

fn main() {
    nannou::app(model).update(update).run();
}

fn model(app: &App) -> Model {
    let window_id = app
        .new_window()
        .view(view)
        .event(event)
        .raw_event(raw_window_event)
        .build()
        .unwrap();
    let window = app.window(window_id).unwrap();

    let grid_size = 16usize;

    Model {
        egui: Egui::from_window(&window),
        settings: Settings {
            brush: Brush::Square,
            brush_size: 1,
            grid_size,
            display_fps: true,
            dark_mode: true,
            primary_color: WHITE,
            secondary_color: BLACK,
            primary_color_buf: [255; 3],
            secondary_color_buf: [0; 3],
        },
        state: State {
            pixels: vec![vec![Pixel::default(); grid_size]; grid_size],
            drawing: false,
            erasing: false,
            should_reset: false,
            should_exit: false,
            should_calc_positions: false,
        },
    }
}

fn raw_window_event(_app: &App, model: &mut Model, event: &nannou::winit::event::WindowEvent) {
    model.egui.handle_raw_event(event);
}

fn event(_app: &App, model: &mut Model, event: WindowEvent) {
    match event {
        Resized(_) => model.state.should_calc_positions = true,
        MousePressed(button) => {
            // Prevent drawing on the GUI
            if model.egui.ctx().is_pointer_over_area() {
                return;
            }

            // Enable drawing or erasing if the user
            // holds down left or right click respectively
            if let MouseButton::Left = button {
                model.state.drawing = true;
            } else if let MouseButton::Right = button {
                model.state.erasing = true;
            }
        }
        MouseReleased(button) => {
            // Disable drawing or erasing if the user
            // releases left or right click respectively
            if let MouseButton::Left = button {
                model.state.drawing = false;
            } else if let MouseButton::Right = button {
                model.state.erasing = false;
            }
        }
        KeyPressed(key) => match key {
            Key::Q => {
                model.state.should_exit = true;
            }
            Key::R => {
                model.state.should_reset = true;
            }
            _ => (),
        },
        _ => (),
    }
}

fn update(app: &App, model: &mut Model, update: Update) {
    let win = app.window_rect();
    let diff = win.w().min(win.h()) / model.settings.grid_size as f32;

    // Reset canvas
    if model.state.should_reset {
        model.state.should_reset = false;
        model.state.should_calc_positions = true;
        model.state.pixels =
            vec![vec![Pixel::default(); model.settings.grid_size]; model.settings.grid_size];
    }

    // Recalculate pixel positions
    if model.state.should_calc_positions {
        model.state.should_calc_positions = false;
        for (x, row) in model.state.pixels.iter_mut().enumerate() {
            for (y, pixel) in row.iter_mut().enumerate() {
                let h = (model.settings.grid_size / 2) as f32;
                let new_x = (x as f32 - (h - 0.5)) * diff;
                let new_y = (y as f32 - (h - 0.5)) * diff;
                pixel.x = new_x;
                pixel.y = new_y;
            }
        }
    }

    // Exit program
    if model.state.should_exit {
        std::process::exit(0);
    }

    if model.state.drawing || model.state.erasing {
        let h = (model.settings.grid_size / 2) as f32;
        let pos_x = (app.mouse.position().x / diff).floor() + h;
        let pos_y = (app.mouse.position().y / diff).floor() + h;
        let color = if model.state.drawing {
            model.settings.primary_color
        } else {
            model.settings.secondary_color
        };

        match model.settings.brush {
            Brush::Square => {
                for x in (pos_x - (model.settings.brush_size as f32 / 2.0))
                    .ceil()
                    .clamp(0.0, f32::MAX) as usize
                    ..(pos_x + (model.settings.brush_size as f32 / 2.0).ceil())
                        .clamp(0.0, model.settings.grid_size as f32) as usize
                {
                    for y in (pos_y - (model.settings.brush_size as f32 / 2.0))
                        .ceil()
                        .clamp(0.0, f32::MAX) as usize
                        ..(pos_y + (model.settings.brush_size as f32 / 2.0).ceil())
                            .clamp(0.0, model.settings.grid_size as f32)
                            as usize
                    {
                        model.state.pixels[x][y].color = color;
                    }
                }
            }
            Brush::Circle => {
                for (x, y) in calc_circle_pixels(model.settings.brush_size as i32) {
                    model.state.pixels[(x + pos_x as i32)
                        .clamp(0, model.settings.grid_size as i32 - 1)
                        as usize][(y + pos_y as i32)
                        .clamp(0, model.settings.grid_size as i32 - 1)
                        as usize]
                        .color = color;
                }
            }
        }
    }

    // Draw egui elements
    let egui = &mut model.egui;
    egui.set_elapsed_time(update.since_start);
    let ctx = egui.begin_frame();
    ctx.style_mut(|style| {
        style.visuals.window_shadow = Shadow::NONE;
        style.visuals = if model.settings.dark_mode {
            Visuals::dark()
        } else {
            Visuals::light()
        };
    });

    if model.settings.display_fps {
        egui::Window::new("fps")
            .title_bar(false)
            .interactable(false)
            .resizable(false)
            .anchor(egui::Align2::RIGHT_TOP, Vec2::new(0.0, 0.0))
            .show(&ctx, |ui| ui.label(app.fps().round().to_string()));
    }

    egui::Window::new("Actions").show(&ctx, |ui| {
        let reset_clicked = ui.button("Reset Canvas").clicked();
        if reset_clicked {
            model.state.should_reset = true;
        }

        let exit_clicked = ui.button("Exit").clicked();
        if exit_clicked {
            model.state.should_exit = true;
        }
    });

    egui::Window::new("Settings").show(&ctx, |ui| {
        ui.label("Primary Color");
        let primary_color_changed = ui
            .color_edit_button_srgb(&mut model.settings.primary_color_buf)
            .changed();
        if primary_color_changed {
            model.settings.primary_color = rgb8(
                model.settings.primary_color_buf[0],
                model.settings.primary_color_buf[1],
                model.settings.primary_color_buf[2],
            )
        }

        ui.label("Secondary Color");
        let secondary_color_changed = ui
            .color_edit_button_srgb(&mut model.settings.secondary_color_buf)
            .changed();
        if secondary_color_changed {
            model.settings.secondary_color = rgb8(
                model.settings.secondary_color_buf[0],
                model.settings.secondary_color_buf[1],
                model.settings.secondary_color_buf[2],
            );
        }

        ui.label("Grid Size");
        let grid_resized = ui
            .add(egui::Slider::new(&mut model.settings.grid_size, 1..=64))
            .changed();
        if grid_resized {
            model.state.should_reset = true;
        }

        ui.label("Brush Size");
        ui.add(egui::Slider::new(
            &mut model.settings.brush_size,
            1..=model.settings.grid_size,
        ));

        ui.label("Brush Type");
        ui.group(|ui| {
            let square_clicked = ui
                .add_enabled(
                    if let Brush::Square = model.settings.brush {
                        false
                    } else {
                        true
                    },
                    egui::Button::new("Square"),
                )
                .clicked();
            if square_clicked {
                model.settings.brush = Brush::Square;
            }

            let circle_clicked = ui
                .add_enabled(
                    if let Brush::Circle = model.settings.brush {
                        false
                    } else {
                        true
                    },
                    egui::Button::new("Circle"),
                )
                .clicked();
            if circle_clicked {
                model.settings.brush = Brush::Circle;
            }
        });

        ui.checkbox(&mut model.settings.display_fps, "Display FPS");

        ui.label("Theme");
        ui.group(|ui| {
            let light_clicked = ui
                .add_enabled(model.settings.dark_mode, egui::Button::new("Light"))
                .clicked();
            if light_clicked {
                model.settings.dark_mode = false;
            }

            let dark_clicked = ui
                .add_enabled(!model.settings.dark_mode, egui::Button::new("Dark"))
                .clicked();
            if dark_clicked {
                model.settings.dark_mode = true;
            }
        });
    });
}

fn view(app: &App, model: &Model, frame: Frame) {
    let win = app.window_rect();
    let draw = app.draw();
    let diff = win.w().min(win.h()) / model.settings.grid_size as f32;

    draw.background().color(LIGHTGRAY);

    // Draw grid
    for (x, row) in model.state.pixels.iter().enumerate() {
        let mut amt = 0.0;
        for (y, pixel) in row.iter().enumerate() {
            if y < amt as usize {
                continue;
            }

            amt = 0.0;
            for y_2 in y..model.state.pixels.len() {
                if pixel.color != model.state.pixels[x][y_2].color {
                    break;
                }

                amt += 1.0;
            }

            draw.rect()
                .w_h(diff, diff * amt)
                .x_y(pixel.x, pixel.y + (diff * (amt - 1.0)) / 2.0)
                .color(pixel.color);
        }
    }

    // Draw pixels over mouse
    let mut mouse_pos = Point2::new(
        ((app.mouse.position().x / diff).floor() + 0.5) * diff,
        ((app.mouse.position().y / diff).floor() + 0.5) * diff,
    );

    match model.settings.brush {
        Brush::Square => {
            // We need to manually align the pixels to the
            // mouse if its even since theres no center
            if model.settings.brush_size % 2 == 0 {
                mouse_pos.x -= diff / 2.0;
                mouse_pos.y -= diff / 2.0;
            }

            draw.rect().xy(mouse_pos).color(OVERLAY).w_h(
                diff * model.settings.brush_size as f32,
                diff * model.settings.brush_size as f32,
            );
        }
        Brush::Circle => {
            for (x, y) in calc_circle_pixels(model.settings.brush_size as i32) {
                draw.rect()
                    .color(OVERLAY)
                    .x_y(x as f32 * diff + mouse_pos.x, y as f32 * diff + mouse_pos.y)
                    .w_h(diff, diff);
            }
        }
    }

    // Finish drawing
    draw.to_frame(app, &frame).unwrap();
    model.egui.draw_to_frame(&frame).unwrap()
}

/// Implementation of Friedrich Gauss' solution
/// to the Gauss circle problem.
fn calc_circle_pixels(diameter: i32) -> Vec<(i32, i32)> {
    let radius_f32 = diameter as f32 / 2.0;
    let radius = radius_f32 as i32;
    let mut points = Vec::new();

    for x in -radius..=radius {
        for y in -radius..=radius {
            let x_f32 = x as f32;
            let y_f32 = y as f32;
            if x_f32 * x_f32 + y_f32 * y_f32 <= radius_f32 * radius_f32 {
                points.push((x, y));
            }
        }
    }

    points
}
