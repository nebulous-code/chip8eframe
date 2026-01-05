use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use chip8sys::chip8::{Chip8KeyMask, Chip8Quirks, Chip8Sys, TimerMode, DISPLAY_WIDTH};
use chip8sys::chip8error::Chip8Error;
use egui::special_emojis;
use egui::text::LayoutJob;
use egui::{Color32, Key};
use egui_extras::{Column, TableBuilder};
use rodio::mixer::Mixer;
use rodio::source::{SineWave, Source};
// This import provides a native file dialog for ROM selection.
use rfd::FileDialog;

use crate::about::About;

/// This constant defines the default CPU speed in cycles per second.
const DEFAULT_CYCLES_PER_SECOND: f64 = 1600.0;
/// This constant caps CPU catch-up to keep the UI responsive.
const DEFAULT_MAX_CYCLES_PER_FRAME: u32 = 200;
/// This constant defines the timer tick rate in Hertz.
const TIMER_HZ: f64 = 60.0;

// if we add new fields, give them default values when deserializing old state
pub struct Chip8App {
    chip8: Chip8Sys,
    quirks: Chip8Quirks,
    zoom: f32,
    background_color: Color32,
    pixel_color: Color32,
    key_map: [Key; 16],
    sink: rodio::Sink,
    compute_info: ConfigWindow,
    screen_config: ConfigWindow,
    about: ConfigWindow,
    control_flow: ConfigWindow,
    run: bool,
    single_step: bool,
    rom_path: String,
    /// This field stores the last directory used for picking ROM files.
    last_rom_dir: Option<PathBuf>,
    /// This field stores the active cycles-per-second target for the emulator.
    cycles_per_second: f64,
    /// This field stores the maximum cycles allowed per frame.
    max_cycles_per_frame: u32,
    /// This field stores the last time the emulator loop ran.
    last_frame: Instant,
    /// This field accumulates fractional CPU cycles between frames.
    cpu_accumulator: f64,
    /// This field accumulates timer ticks between frames.
    timer_accumulator: f64,
}

impl Default for Chip8App {
    fn default() -> Self {
        // Setup and Handle Sound
        // let sink = rodio::Sink::connect_new(&stream_handle.mixer());
        let quirks = Chip8App::quirks_chip8();

        Self {
            // Example stuff:
            chip8: Chip8Sys::new_with_quirks(quirks),
            quirks,
            zoom: 15.0,
            background_color: Color32::BLACK,
            pixel_color: Color32::GREEN,
            key_map: [
                Key::X,
                Key::Num1,
                Key::Num2,
                Key::Num3,
                Key::Q,
                Key::W,
                Key::E,
                Key::A,
                Key::S,
                Key::D,
                Key::Z,
                Key::C,
                Key::Num4,
                Key::R,
                Key::F,
                Key::V,
            ],
            sink: rodio::Sink::new().0,
            screen_config: ConfigWindow {
                name: String::from("Screen Config"),
                show: false,
            },
            compute_info: ConfigWindow {
                name: String::from("Compute Info"),
                show: false,
            },
            about: ConfigWindow {
                name: String::from("About Chip-8"),
                // TODO: Make this true long term
                show: false,
            },
            control_flow: ConfigWindow {
                name: String::from("Control Flow"),
                show: false,
            },
            run: true,
            single_step: false,
            rom_path: String::new(),
            last_rom_dir: None,
            cycles_per_second: DEFAULT_CYCLES_PER_SECOND,
            max_cycles_per_frame: DEFAULT_MAX_CYCLES_PER_FRAME,
            last_frame: Instant::now(),
            cpu_accumulator: 0.0,
            timer_accumulator: 0.0,
        }
    }
}

impl Chip8App {
    /// Called once before the first frame.
    pub fn new(_cc: &eframe::CreationContext<'_>, mixer: Result<&Mixer, String>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        let mut result: Chip8App = Default::default();

        // Setup Sound
        // If I send a mixer in use it, otherwise ignore it
        // NOTE: WILL Cause problems if someone requests sound via chip8 and it's not there.
        match mixer {
            Ok(m) => result.sink = rodio::Sink::connect_new(m),
            Err(_) => (),
        }
        // This configures the emulator to use externally driven timers.
        result.chip8.set_timer_mode(TimerMode::External);

        // Load Chip-8 Roms
        // let rom_name = "1-chip8-logo.ch8";
        // let rom_name = "2-ibm-logo.ch8";
        // let rom_name = "3-corax+.ch8";
        // let rom_name = "5-quirks.ch8";
        // When running quirks rom hardcode this memory spot to auto run Chip-8
        // result.chip8.memory[0x1FF] = 1;
        let rom_name = "walking_man.ch8";
        // let rom_name = "7-beep.ch8";

        // This locates the shared ROM folder relative to this crate.
        let roms_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../roms");
        // This resolves the startup ROM path using the best available folder.
        let rom_path = if roms_dir.is_dir() {
            roms_dir.join(rom_name)
        } else if let Ok(current_dir) = std::env::current_dir() {
            current_dir.join(rom_name)
        } else {
            PathBuf::from(rom_name)
        };

        // This loads the ROM bytes into the emulator.
        result.load_rom_from_path(&rom_path);
        // result.chip8.load_chip8_logo();
        // result.chip8.load_sound_test();
        //

        /*
        println!("chip8-logo");
        println!("");
        println!("{:?}", result.chip8.memory);
        println!("");
        // */
        result
    }

    /// This function returns the Chip-8 preset quirk settings.
    /// Arguments: none.
    /// Returns: The Chip-8 preset quirks.
    fn quirks_chip8() -> Chip8Quirks {
        Chip8Quirks {
            increment_i_on_store: true,
            reset_vf_on_logic: true,
            wrap_draw: false,
            shift_uses_vx: false,
        }
    }

    /// This function returns the Super Chip-8 preset quirk settings.
    /// Arguments: none.
    /// Returns: The Super Chip-8 preset quirks.
    fn quirks_super_chip8() -> Chip8Quirks {
        Chip8Quirks {
            increment_i_on_store: false,
            reset_vf_on_logic: false,
            wrap_draw: false,
            shift_uses_vx: true,
        }
    }

    /// This function returns the XO-Chip preset quirk settings.
    /// Arguments: none.
    /// Returns: The XO-Chip preset quirks.
    fn quirks_xo_chip() -> Chip8Quirks {
        Chip8Quirks {
            increment_i_on_store: true,
            reset_vf_on_logic: false,
            wrap_draw: true,
            shift_uses_vx: false,
        }
    }

    /// This function returns the display label for the current quirk preset.
    /// Arguments:
    /// - quirks: The quirk configuration to label.
    /// Returns: The label for the matching preset, or "Custom".
    fn quirk_preset_label(quirks: &Chip8Quirks) -> &'static str {
        if *quirks == Self::quirks_chip8() {
            "Chip-8"
        } else if *quirks == Self::quirks_super_chip8() {
            "Super Chip-8"
        } else if *quirks == Self::quirks_xo_chip() {
            "XO-Chip"
        } else {
            "Custom"
        }
    }

    /// This function chooses the default directory for the ROM picker dialog.
    /// Arguments: none.
    /// Returns: The preferred directory if available, otherwise None.
    fn rom_picker_directory(&self) -> Option<PathBuf> {
        // This prioritizes the most recently used ROM directory.
        if let Some(last_dir) = &self.last_rom_dir {
            return Some(last_dir.clone());
        }

        // This falls back to the shared ROMs directory if it exists.
        let roms_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../roms");
        if roms_dir.is_dir() {
            return Some(roms_dir);
        }

        // This final fallback uses the current working directory.
        std::env::current_dir().ok()
    }

    /// This function resets the emulator state with the active quirks.
    /// Arguments: none.
    /// Returns: none.
    fn reset_emulator(&mut self) {
        let timer_mode = self.chip8.timer_mode();
        self.chip8 = Chip8Sys::new_with_quirks(self.quirks);
        self.chip8.set_timer_mode(timer_mode);
        self.cpu_accumulator = 0.0;
        self.timer_accumulator = 0.0;
    }

    /// This function restarts the current ROM with the active quirks.
    /// Arguments: none.
    /// Returns: none.
    fn restart_current_rom(&mut self) {
        if self.rom_path.is_empty() {
            return;
        }
        let rom_bytes = fs::read(&self.rom_path).expect("rom file should be readable");
        self.reset_emulator();
        self.chip8.load_rom_bytes(&rom_bytes);
    }

    /// This function loads a ROM file from disk and resets the emulator state.
    /// Arguments:
    /// - rom_path: The path to the ROM file.
    /// Returns: none.
    fn load_rom_from_path(&mut self, rom_path: &Path) {
        // This reads the ROM bytes from disk for the emulator to load.
        let rom_bytes = fs::read(rom_path).expect("rom file should be readable");
        // This resets the emulator while applying the active quirks.
        self.reset_emulator();
        // This stores the selected path for future restarts.
        self.rom_path = rom_path.to_string_lossy().to_string();
        // This stores the folder for the next file dialog.
        self.last_rom_dir = rom_path.parent().map(|parent| parent.to_path_buf());
        // This loads the ROM bytes into memory.
        self.chip8.load_rom_bytes(&rom_bytes);
    }
}

impl eframe::App for Chip8App {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // If we're restarting the chip-8 then reinitialize it with quirks
        // Scan the keys and if they're pressed tell the Chip-8
        // I think I'm missing an edge case quirk thing where chip-8 only acts if a key is released
        // But that might be beyond my scope of interest for this project
        ctx.input(|i| {
            // This mask stores the pressed keys for the boundary API.
            let mut key_mask: Chip8KeyMask = 0;
            for (n, k) in self.key_map.iter().enumerate() {
                if i.key_down(*k) {
                    key_mask |= 1u16 << n;
                }
            }
            // This updates the emulator keypad state.
            self.chip8.set_keys_mask(key_mask);
        });

        // Handle Sound
        if self.chip8.is_sound_playing() {
            self.sink.append(SineWave::new(440.0).repeat_infinite());
        } else {
            self.sink.stop();
        }

        // TODO: Not sure how I want to handle all these yet...
        // maybe log them in their own window?
        // This measures the time since the last update.
        let now = Instant::now();
        let delta_seconds = now.duration_since(self.last_frame).as_secs_f64();
        self.last_frame = now;

        if self.run {
            // This accumulates CPU work based on elapsed time.
            self.cpu_accumulator += delta_seconds * self.cycles_per_second;
            // This accumulates time toward the next timer tick.
            self.timer_accumulator += delta_seconds * TIMER_HZ;

            // This advances the delay and sound timers at 60Hz.
            let timer_ticks = self.timer_accumulator.floor() as u32;
            if timer_ticks > 0 {
                self.chip8.tick_timers(timer_ticks);
                self.timer_accumulator -= timer_ticks as f64;
            }

            // This advances the emulator state using the accumulated cycles.
            let cycles_to_run = self.cpu_accumulator.floor() as u32;
            if cycles_to_run > 0 {
                let capped_cycles = cycles_to_run.min(self.max_cycles_per_frame);
                match self.chip8.tick(capped_cycles) {
                    Ok(_) => (),
                    Err(e) => match e {
                        // if the N of 0xN___ is invalid it will return this and the N provided
                        Chip8Error::InvalidFirstByte(_) => (),
                        // If the X register should be <= 0xF
                        Chip8Error::InvalidRegisterX(_) => (),
                        // if the N in 0x8XYN is invalid it will return this and the N provided
                        Chip8Error::Invalid0x8XYN(_) => (),
                        // if the N in 0x8XYN is invalid it will return this and the N provided
                        Chip8Error::Invalid0xENNN(_, _) => (),
                        // if the N in 0x8XYN is invalid it will return this and the N provided
                        Chip8Error::Invalid0xFNNN(_, _) => (),
                        // If the register we're waiting for is somehow > 0xF
                        Chip8Error::InvalidWaitRegister(_) => (),
                        Chip8Error::IssueGeneratingRandomNum(_) => (),
                    },
                }
                self.cpu_accumulator -= capped_cycles as f64;
            }
        } else if self.single_step {
            match self.chip8.tick(1) {
                Ok(_) => (),
                Err(e) => match e {
                    // if the N of 0xN___ is invalid it will return this and the N provided
                    Chip8Error::InvalidFirstByte(_) => (),
                    // If the X register should be <= 0xF
                    Chip8Error::InvalidRegisterX(_) => (),
                    // if the N in 0x8XYN is invalid it will return this and the N provided
                    Chip8Error::Invalid0x8XYN(_) => (),
                    // if the N in 0x8XYN is invalid it will return this and the N provided
                    Chip8Error::Invalid0xENNN(_, _) => (),
                    // if the N in 0x8XYN is invalid it will return this and the N provided
                    Chip8Error::Invalid0xFNNN(_, _) => (),
                    // If the register we're waiting for is somehow > 0xF
                    Chip8Error::InvalidWaitRegister(_) => (),
                    Chip8Error::IssueGeneratingRandomNum(_) => (),
                },
            }
            self.single_step = false;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Chip-8 Display".to_string());
            let painter = ui.painter();

            let width = self.zoom;
            let mut row = 0.0;
            let mut col = 0.0;
            let x_off = 50.0;
            let y_off = 45.0;
            // This value represents the packed bytes per row of pixels.
            let col_count = DISPLAY_WIDTH / 8;

            for (n, px) in self.chip8.framebuffer_packed().iter().enumerate() {
                if n % col_count == 0 {
                    row += width;
                    col = 0.0;
                }
                let mut bit_stream: Vec<bool> = Vec::new();

                for b in 0..8 {
                    bit_stream.push(((px << b) & 0b1000_0000) == 0b1000_0000);
                }
                for cell in bit_stream {
                    let x_start = x_off + (col * width);
                    let y_start = y_off + row;
                    let color: Color32;
                    if cell {
                        color = self.pixel_color;
                    } else {
                        color = self.background_color;
                    };
                    painter.rect_filled(
                        egui::Rect {
                            min: egui::Pos2 {
                                x: x_start,
                                y: y_start,
                            },
                            max: egui::Pos2 {
                                x: x_start + width,
                                y: y_start + width,
                            },
                        },
                        0.0,
                        color,
                    );
                    col += 1.0;
                }
            }
            ctx.request_repaint();
        });

        egui::SidePanel::right("Config Toggle").show(ctx, |ui| {
            ui.heading("Chip-8 Toolbox");
            ui.separator();
            ui.toggle_value(&mut self.about.show, self.about.name.clone());
            ui.toggle_value(&mut self.compute_info.show, self.compute_info.name.clone());
            ui.toggle_value(
                &mut self.screen_config.show,
                self.screen_config.name.clone(),
            );
            ui.toggle_value(&mut self.control_flow.show, self.control_flow.name.clone());
            ui.separator();
            // This label shows the currently loaded ROM path.
            if !self.rom_path.is_empty() {
                ui.label(format!("ROM: {}", self.rom_path));
            }
            // This button opens a file dialog so a ROM can be selected at runtime.
            if ui.button("Load ROM").clicked() {
                // This dialog filters to common Chip-8 ROM extensions.
                let mut dialog = FileDialog::new().add_filter("Chip-8 ROM", &["ch8"]);
                // This sets the starting directory based on recent usage.
                if let Some(default_dir) = self.rom_picker_directory() {
                    dialog = dialog.set_directory(default_dir);
                }
                let rom_file = dialog.pick_file();
                if let Some(path) = rom_file {
                    // This loads the selected ROM and updates the emulator state.
                    self.load_rom_from_path(&path);
                }
            }
            ui.separator();
            // This label introduces the control flow buttons in the toolbox.
            ui.label("Control Flow");
            // This row toggles run and pause without opening the detail window.
            ui.horizontal_wrapped(|ui| {
                let state = if self.run { "Pause" } else { "Run" };
                ui.toggle_value(&mut self.run, state);
                // This button advances the emulator by one instruction while paused.
                if ui.add_enabled(!self.run, egui::Button::new("Single Step")).clicked() {
                    self.single_step = true;
                }
                // This button reloads the current ROM from disk.
                if ui.button("Restart").clicked() {
                    self.restart_current_rom();
                }
            });
            ui.separator();
            ui.label("Quirks (requires restart)");
            egui::ComboBox::from_id_salt("quirk_presets")
                .selected_text(Self::quirk_preset_label(&self.quirks))
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(
                            self.quirks == Self::quirks_chip8(),
                            "Chip-8",
                        )
                        .clicked()
                    {
                        self.quirks = Self::quirks_chip8();
                    }
                    if ui
                        .selectable_label(
                            self.quirks == Self::quirks_super_chip8(),
                            "Super Chip-8",
                        )
                        .clicked()
                    {
                        self.quirks = Self::quirks_super_chip8();
                    }
                    if ui
                        .selectable_label(
                            self.quirks == Self::quirks_xo_chip(),
                            "XO-Chip",
                        )
                        .clicked()
                    {
                        self.quirks = Self::quirks_xo_chip();
                    }
                });
            ui.checkbox(
                &mut self.quirks.increment_i_on_store,
                "Increment I on store",
            );
            ui.checkbox(&mut self.quirks.reset_vf_on_logic, "Reset VF on logic");
            ui.checkbox(&mut self.quirks.wrap_draw, "Wrap sprite draw");
            ui.checkbox(&mut self.quirks.shift_uses_vx, "Shift uses VX");
            ui.separator();
            ui.label("Tuning");
            ui.add(
                egui::Slider::new(&mut self.cycles_per_second, 100.0..=2000.0)
                    .text("Flicker Adjustment"),
            );
            ui.add(
                egui::Slider::new(&mut self.max_cycles_per_frame, 50..=1000)
                    .text("Frame Limit"),
            );
            // This section anchors the Quit button to the bottom of the toolbox.
            ui.allocate_ui_with_layout(
                ui.available_size(),
                egui::Layout::bottom_up(egui::Align::LEFT),
                |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    ui.separator();
                },
            );
        });

        egui::Window::new(self.compute_info.name.clone())
            .open(&mut self.compute_info.show)
            .show(ctx, |ui| {
                ctx.set_pixels_per_point(2.0);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // The central panel the region left after adding TopPanel's and SidePanel's
                    ui.horizontal(|ui| {
                        ui.label(format!("Program Counter: {}", &self.chip8.program_counter));
                    });
                    ui.horizontal(|ui| {
                        ui.label(format!("Register I: {}", &self.chip8.register_i));
                    });
                    ui.horizontal(|ui| {
                        ui.label(format!("Stack Pointer: {}", &self.chip8.stack_pointer));
                    });
                    ui.separator();
                    let available_height = ui.available_height();
                    let table = TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(Column::auto())
                        .column(Column::auto())
                        .column(Column::auto())
                        .column(Column::auto())
                        .min_scrolled_height(0.0)
                        .max_scroll_height(available_height);
                    // table = table.sense(egui::Sense::click());

                    table
                        .header(20.0, |mut header| {
                            header.col(|ui| {
                                ui.strong("Index");
                            });
                            header.col(|ui| {
                                ui.strong("Register Value");
                            });
                            header.col(|ui| {
                                ui.strong("Stack Value");
                            });
                            header.col(|ui| {
                                ui.strong("Keys");
                            });
                        })
                        .body(|mut body| {
                            for row_index in 0..self.chip8.register.len() {
                                body.row(30.0, |mut row| {
                                    row.col(|ui| {
                                        ui.label(format!("0x{:X}", row_index));
                                    });
                                    row.col(|ui| {
                                        ui.label(format!(
                                            "0x{:02X}",
                                            self.chip8.register[row_index]
                                        ));
                                    });
                                    row.col(|ui| {
                                        ui.label(format!("0x{:04X}", self.chip8.stack[row_index]));
                                    });
                                    row.col(|ui| {
                                        if self.chip8.keys[row_index] {
                                            ui.label("Pressed");
                                        } else {
                                            ui.label("");
                                        }
                                    });
                                });
                            }
                        });
                    // self.table(ui);
                    ui.separator();
                });
            });
        egui::Window::new(self.screen_config.name.clone())
            .open(&mut self.screen_config.show)
            .show(ctx, |ui| {
                ctx.set_pixels_per_point(2.0);
                ui.add(egui::Slider::new(&mut self.zoom, 0.0..=25.0).text("Zoom: "));
                ui.label("Pixel: ");
                ui.color_edit_button_srgba(&mut self.pixel_color);
                ui.label("Background: ");
                ui.color_edit_button_srgba(&mut self.background_color);
                egui::widgets::global_theme_preference_buttons(ui);
            });

        egui::Window::new(self.about.name.clone())
            .open(&mut self.about.show)
            .show(ctx, |ui| {
                ui.heading("Chip-8 Emulator");
                ui.label("By Nicholas Licalsi");
                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink(true)
                    .show(ui, |ui| {
                        let mut job = LayoutJob::single_section(
                            crate::about::About::about(),
                            egui::TextFormat {
                                ..Default::default()
                            },
                        );
                        job.wrap = egui::text::TextWrapping {
                            max_rows: 10,
                            ..Default::default()
                        };

                        // NOTE: `Label` overrides some of the wrapping settings, e.g. wrap width
                        ui.label(job);
                    });
                ui.separator();
                ui.label(format!("{} Source Code", special_emojis::GITHUB));
                ui.hyperlink("https://github.com/licalsinj/chip-8");
            });
        let mut control_flow_open = self.control_flow.show;
        egui::Window::new(self.control_flow.name.clone())
            .open(&mut control_flow_open)
            .show(ctx, |ui| {
                ui.heading("Chip-8 Control Flow");
                ui.label("Pause or run the emulator. When paused you can use Single Step to walk through one command at a time.");
                ui.separator();
                egui::Grid::new("control_flow_controls").show(ui,|ui|{
                    let state = if self.run { "Pause" } else { "Run" };
                    ui.toggle_value(&mut self.run, state);
                    if ui.add_enabled(!self.run, egui::Button::new("Single Step")).clicked() {
                        self.single_step = true;
                    }
                    if ui.button("Restart").clicked() {
                        self.restart_current_rom();
                    }
                    ui.end_row();
                });
                ui.separator();
                egui::Grid::new("instruction_output").show(ui, |ui| {
                    ui.label(format!("Program Counter: 0x{:04X}", self.chip8.program_counter));
                    ui.end_row();

                    let prev_instruction_high = self.chip8.memory[self.chip8.program_counter as usize - 2];
                    let prev_instruction_low = self.chip8.memory[self.chip8.program_counter as usize - 1];
                    ui.label(format!("Previous Instruction: 0x{:02X}{:02X}", prev_instruction_high, prev_instruction_low));
                    ui.label(About::chip_8_decode(prev_instruction_high, prev_instruction_low));
                    ui.end_row();

                    let next_instruction_high = self.chip8.memory[self.chip8.program_counter as usize];
                    let next_instruction_low = self.chip8.memory[self.chip8.program_counter as usize + 1];
                    ui.label(format!("Next Instruction: 0x{:02X}{:02X}", next_instruction_high, next_instruction_low));
                    ui.label(About::chip_8_decode(next_instruction_high,next_instruction_low));
                    ui.end_row();
                });
            });
        self.control_flow.show = control_flow_open;
    }
}

struct ConfigWindow {
    name: String,
    show: bool,
}
