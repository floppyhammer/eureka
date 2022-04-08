use std::borrow::Borrow;
use std::ops::Deref;
use epi;
use egui::widgets::TextEdit;
use egui::color::Color32;
use egui::containers::ScrollArea;
use std::process::Command;
use std::str;
use anyhow::Context;
use chrono;
use egui::TextStyle;
use image::GenericImageView;
use egui_extras::RetainedImage;

#[derive(PartialEq)]
enum Enum { First, Second }

enum BuildStatus { None, Success, Fail }

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
pub struct App {
    is_text_draw_on: bool,
    is_text_animation_on: bool,

    terminal_output: String,

    build_type: Enum,

    ndk_path: String,
    engine_path: String,
    aar_path: String,

    build_status: BuildStatus,

    image: Option<RetainedImage>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            is_text_draw_on: false,
            is_text_animation_on: false,

            terminal_output: String::new(),
            build_type: Enum::First,
            ndk_path: String::from("D:/Env/android-ndk-r10e-windows-x86_64/android-ndk-r10e"),
            engine_path: String::from("D:/Dev/QuVideo/ces_adk_v5_vivacut_17607"),
            aar_path: String::from("D:/Dev/QuVideo/cutVeSdk"),

            build_status: BuildStatus::None,
            image: None,
        }
    }
}

impl epi::App for App {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        let Self {
            is_text_draw_on,
            is_text_animation_on,
            terminal_output,
            build_type,
            ndk_path,
            engine_path,
            aar_path,
            build_status,
            ..
        } = self;

        // Examples of how to create different panels and windows.
        // Pick whichever suits you.
        // Tip: a good default choice is to just keep the `CentralPanel`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                egui::menu::menu_button(ui, "File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                });
            });
        });

        egui::SidePanel::left("left_side_panel").default_width(320.0).show(ctx, |ui| {
            ui.heading("Options");

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("Build type:");
                ui.radio_value(build_type, Enum::First, "all");
                ui.radio_value(build_type, Enum::Second, "sdk");
            });

            ui.add_space(8.0);

            ui.label("NDK directory:");
            if ui.text_edit_singleline(ndk_path).changed() {}

            ui.label("Engine directory:");
            if ui.text_edit_singleline(engine_path).changed() {}

            ui.label("AAR directory:");
            if ui.text_edit_singleline(aar_path).changed() {}

            ui.add_space(8.0);

            ui.checkbox(is_text_draw_on, "TextDraw");
            ui.checkbox(is_text_animation_on, "AnimationText");

            ui.add_space(8.0);

            if ui.button("Build").clicked() {
                terminal_output.clear();

                // Show current time.
                terminal_output.push_str(&*(chrono::offset::Local::now().to_string() + "\n"));

                let mut is_building_ok = true;

                *build_status = BuildStatus::None;

                // TextDraw.
                if *is_text_draw_on && is_building_ok {
                    let mut cmd_lines = vec![];

                    cmd_lines.push("$env:Path = ".to_owned() + "'" + ndk_path + "'");

                    cmd_lines.push("\n'Building TextDraw...'".to_owned());

                    cmd_lines.push("\nSet-Location '".to_owned() + engine_path + "/textdraw/makefile/android/jni'");

                    cmd_lines.push("\nndk-build -j4".to_owned());

                    cmd_lines.push("\nCopy-Item './../obj/local/arm64-v8a/*.a' '".to_owned() + engine_path + "/lib/android_arm64-v8a'");
                    cmd_lines.push("\nCopy-Item './../obj/local/armeabi-v7a/*.a' '".to_owned() + engine_path + "/lib/android'");

                    let (output_string, res) = run_powershell(&mut cmd_lines);

                    if res {
                        *build_status = BuildStatus::Success;
                        is_building_ok = true;
                    } else {
                        *build_status = BuildStatus::Fail;
                        is_building_ok = false;
                    }

                    terminal_output.push_str(&output_string);
                }

                // AnimationText.
                if *is_text_animation_on && is_building_ok {
                    let mut cmd_lines = vec![];

                    cmd_lines.push("$env:Path = ".to_owned() + "'" + ndk_path + "'");

                    cmd_lines.push("\n'Building AnimationText...'".to_owned());

                    cmd_lines.push("\nSet-Location '".to_owned() + engine_path + "/animationtext/makefile/android/jni'");

                    cmd_lines.push("\nndk-build -j4".to_owned());

                    let build_type_flag = if *build_type == Enum::First {
                        "all'"
                    } else {
                        "sdk'"
                    };

                    cmd_lines.push("\nCopy-Item './../obj/local/arm64-v8a/*.a' '".to_owned() + engine_path + "/lib/android_arm64-v8a/" + build_type_flag);
                    cmd_lines.push("\nCopy-Item './../obj/local/armeabi-v7a/*.a' '".to_owned() + engine_path + "/lib/android/" + build_type_flag);

                    let (output_string, res) = run_powershell(&mut cmd_lines);

                    if res {
                        *build_status = BuildStatus::Success;
                        is_building_ok = true;
                    } else {
                        *build_status = BuildStatus::Fail;
                        is_building_ok = false;
                    }

                    terminal_output.push_str(&output_string);
                }
            }

            match build_status {
                BuildStatus::None => { ui.colored_label(Color32::YELLOW, ""); }
                BuildStatus::Success => { ui.colored_label(Color32::GREEN, "Build succeeded."); }
                BuildStatus::Fail => { ui.colored_label(Color32::RED, "Build failed!"); }
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label("Powered by ");
                    ui.hyperlink_to("egui", "https://github.com/emilk/egui");
                    ui.label(" and ");
                    ui.hyperlink_to("eframe", "https://github.com/emilk/egui/tree/master/eframe");
                });
                ui.horizontal(|ui| {
                    ui.hyperlink_to("Contact author", "email://tannhauser_chen@outlook.com");
                    egui::warn_if_debug_build(ui);
                });
            });
        });

        if self.image.is_none() {
            // Load the image:
            let asset_dir = std::path::Path::new(env!("OUT_DIR")).join("assets");
            println!("Asset dir: {}", asset_dir.display());

            // let image_data = include_bytes!("rust-logo-256x256.png");
            // use image::GenericImageView;
            // let image = image::load_from_memory(image_data).expect("Failed to load image");
            //let image = image::open(asset_dir.join("happy-tree.png")).context("Invalid image path").unwrap();
            self.image = Some(RetainedImage::from_image_bytes("happy-tree", include_bytes!("../../assets/happy-tree.png")).unwrap());
        }

        // egui::CentralPanel::default().show(ctx, |ui| {
        //     if let Some(image) = self.image.as_ref() {
        //         ui.vertical_centered(|ui| {
        //             ui.heading("Scene Editor");
        //         });
        //         ui.add(
        //             egui::Image::new(
        //                 image.texture_id(ctx),
        //                 image.size_vec2()),
        //         );
        //     }
        // });

        // egui::SidePanel::right("right_panel")
        //     .resizable(true)
        //     .default_width(150.0)
        //     .width_range(80.0..=200.0)
        //     .show(ctx, |ui| {
        //         ui.vertical_centered(|ui| {
        //             ui.heading("Right Panel");
        //         });
        //         egui::ScrollArea::vertical().show(ui, |ui| {
        //         });
        //     });

        egui::SidePanel::right("right_side_panel")
            .default_width(320.0)
            .resizable(true)
            .width_range(80.0..=200.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Inspector");
                });
                egui::ScrollArea::vertical().show(ui, |ui| {
                });
            });
    }

    /// Called once before the first frame.
    fn setup(
        &mut self,
        _ctx: &egui::Context,
        _frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        #[cfg(feature = "persistence")]
        if let Some(storage) = _storage {
            *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
        }
    }

    /// Called by the frame work to save state before shutdown.
    /// Note that you must enable the `persistence` feature for this to work.
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    fn name(&self) -> &str {
        "NDK Powershell GUI"
    }
}

fn run_powershell(cmd_lines: &mut Vec<String>) -> (String, bool) {
    let output = if cfg!(target_os = "windows") {
        Command::new("powershell")
            .args(cmd_lines)
            .output()
            .expect("failed to execute process")
    } else {
        Command::new("sh")
            .arg("-c")
            .arg("echo hello")
            .output()
            .expect("failed to execute process")
    };

    let out = output.stdout;
    let err = output.stderr;

    let (output_str, res) = if err.is_empty() {
        (str::from_utf8(&out).unwrap(), true)
    } else {
        (str::from_utf8(&err).unwrap(), false)
    };

    (output_str.to_owned(), res)
}
