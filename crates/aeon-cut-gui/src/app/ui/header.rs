use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use crate::app::icons::draw_pin_icon;
use crate::app::platform::set_linux_always_on_top;
use crate::app::{AeonCutApp, LogLevel};
use crate::i18n::{Language, get_strings};

pub fn render(app: &mut AeonCutApp, ui: &mut egui::Ui) {
    let t = get_strings(app.lang);
    let screen_w = ui.max_rect().width();
    let header_height = 56.0;
    let header_rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(screen_w, header_height));

    // 1. Paint subtle blackish-purple hardware gradient FIRST (behind widgets)
    let mut bg_mesh = egui::Mesh::default();
    let c_tl = Color32::from_rgb(36, 18, 48); // refined obsidian violet
    let c_tr = Color32::from_rgb(22, 14, 32); // deep charcoal purple
    let c_br = Color32::from_rgb(15, 14, 20); // dark obsidian charcoal
    let c_bl = Color32::from_rgb(22, 14, 30); // subtle dark violet

    let idx = bg_mesh.vertices.len() as u32;
    bg_mesh.vertices.push(egui::epaint::Vertex {
        pos: header_rect.left_top(),
        uv: Pos2::ZERO,
        color: c_tl,
    });
    bg_mesh.vertices.push(egui::epaint::Vertex {
        pos: header_rect.right_top(),
        uv: Pos2::ZERO,
        color: c_tr,
    });
    bg_mesh.vertices.push(egui::epaint::Vertex {
        pos: header_rect.right_bottom(),
        uv: Pos2::ZERO,
        color: c_br,
    });
    bg_mesh.vertices.push(egui::epaint::Vertex {
        pos: header_rect.left_bottom(),
        uv: Pos2::ZERO,
        color: c_bl,
    });
    bg_mesh.add_triangle(idx, idx + 1, idx + 2);
    bg_mesh.add_triangle(idx, idx + 2, idx + 3);
    ui.painter().add(egui::Shape::mesh(bg_mesh));

    // 1px hairline gradient accent divider at bottom of header
    let mut border_mesh = egui::Mesh::default();
    let b_tl = Pos2::new(0.0, header_rect.max.y - 1.0);
    let b_tr = Pos2::new(screen_w, header_rect.max.y - 1.0);
    let b_br = Pos2::new(screen_w, header_rect.max.y);
    let b_bl = Pos2::new(0.0, header_rect.max.y);

    let bc_left = Color32::from_rgba_unmultiplied(170, 50, 220, 85);
    let bc_right = Color32::from_rgba_unmultiplied(50, 38, 70, 40);

    let b_idx = border_mesh.vertices.len() as u32;
    border_mesh.vertices.push(egui::epaint::Vertex {
        pos: b_tl,
        uv: Pos2::ZERO,
        color: bc_left,
    });
    border_mesh.vertices.push(egui::epaint::Vertex {
        pos: b_tr,
        uv: Pos2::ZERO,
        color: bc_right,
    });
    border_mesh.vertices.push(egui::epaint::Vertex {
        pos: b_br,
        uv: Pos2::ZERO,
        color: bc_right,
    });
    border_mesh.vertices.push(egui::epaint::Vertex {
        pos: b_bl,
        uv: Pos2::ZERO,
        color: bc_left,
    });
    border_mesh.add_triangle(b_idx, b_idx + 1, b_idx + 2);
    border_mesh.add_triangle(b_idx, b_idx + 2, b_idx + 3);
    ui.painter().add(egui::Shape::mesh(border_mesh));

    // 2. ================= TOP HEADER WIDGETS =================
    egui::Frame::new()
        .inner_margin(egui::Margin {
            left: 18,
            right: 18,
            top: 10,
            bottom: 10,
        })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Left: Logo + App Titles
                ui.spacing_mut().item_spacing = Vec2::new(12.0, 0.0);
                if let Some(tex) = &app.logo_texture {
                    ui.add(egui::Image::new((tex.id(), Vec2::new(32.0, 32.0))));
                }

                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);
                    ui.horizontal(|ui| {
                        ui.heading(
                            egui::RichText::new("AEON-CUT")
                                .size(20.0)
                                .strong()
                                .color(Color32::from_rgb(245, 240, 255)),
                        );
                        ui.label(
                            egui::RichText::new(concat!("v", env!("CARGO_PKG_VERSION")))
                                .size(11.0)
                                .color(Color32::from_rgb(155, 130, 185)),
                        );
                    });

                    ui.label(
                        egui::RichText::new(t.app_subtitle)
                            .size(11.0)
                            .color(Color32::from_rgb(150, 145, 170)),
                    );
                });

                // Right side: Status Badge, Refresh Button, Language Selector, Pin Button
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);

                    // 1. Pin / Always-On-Top Window Button (Pure vector-drawn via egui Painter)
                    let (pin_rect, pin_resp) =
                        ui.allocate_exact_size(Vec2::new(28.0, 26.0), egui::Sense::click());
                    if pin_resp.clicked() {
                        app.always_on_top = !app.always_on_top;
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                                if app.always_on_top {
                                    egui::WindowLevel::AlwaysOnTop
                                } else {
                                    egui::WindowLevel::Normal
                                },
                            ));

                        #[cfg(target_os = "linux")]
                        set_linux_always_on_top(app.always_on_top);

                        let log_msg = if app.always_on_top {
                            "Window pinned to top (Always on Top: ON)."
                        } else {
                            "Window unpinned (Always on Top: OFF)."
                        };
                        app.add_log(LogLevel::Info, log_msg);
                    }

                    let pin_tooltip = match app.lang {
                        Language::English => {
                            if app.always_on_top {
                                "Unpin window (Always on Top is ON)"
                            } else {
                                "Pin window to front (Always on Top)"
                            }
                        }
                        Language::Turkish => {
                            if app.always_on_top {
                                "Pencere sabitlemesini kaldır (Her Zaman Üstte Açık)"
                            } else {
                                "Pencereyi ekrana sabitle (Her Zaman Üstte)"
                            }
                        }
                        Language::Chinese => {
                            if app.always_on_top {
                                "取消窗口置顶"
                            } else {
                                "窗口置顶 (始终在最前端)"
                            }
                        }
                        Language::Japanese => {
                            if app.always_on_top {
                                "最前面表示を解除"
                            } else {
                                "ウィンドウを最前面に固定"
                            }
                        }
                    };
                    let pin_resp = pin_resp.on_hover_text(pin_tooltip);

                    let is_hovered = pin_resp.hovered();
                    let (bg_fill, border_stroke, icon_color) = if app.always_on_top {
                        (
                            if is_hovered {
                                Color32::from_rgb(78, 32, 115)
                            } else {
                                Color32::from_rgb(62, 24, 92)
                            },
                            Stroke::new(1.0, Color32::from_rgb(195, 80, 255)),
                            Color32::from_rgb(255, 245, 255),
                        )
                    } else {
                        (
                            if is_hovered {
                                Color32::from_rgb(38, 28, 50)
                            } else {
                                Color32::from_rgb(26, 20, 36)
                            },
                            Stroke::new(
                                1.0,
                                if is_hovered {
                                    Color32::from_rgb(85, 65, 115)
                                } else {
                                    Color32::from_rgb(55, 42, 75)
                                },
                            ),
                            if is_hovered {
                                Color32::from_rgb(215, 205, 235)
                            } else {
                                Color32::from_rgb(140, 130, 160)
                            },
                        )
                    };

                    ui.painter().rect_filled(pin_rect, 6.0, bg_fill);
                    ui.painter().rect_stroke(
                        pin_rect,
                        6.0,
                        border_stroke,
                        egui::StrokeKind::Inside,
                    );
                    draw_pin_icon(ui.painter(), pin_rect, icon_color, app.always_on_top);

                    // Clean Language Selector ComboBox
                    egui::ComboBox::from_id_salt("language_selector")
                        .width(90.0)
                        .selected_text(app.lang.display_name())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut app.lang, Language::English, "English");
                            ui.selectable_value(&mut app.lang, Language::Turkish, "Türkçe");
                            ui.selectable_value(&mut app.lang, Language::Chinese, "简体中文");
                            ui.selectable_value(&mut app.lang, Language::Japanese, "日本語");
                        });

                    // Refresh Button (styled with subtle purple tone)
                    let refresh_btn = egui::Button::new(
                        egui::RichText::new(t.refresh_btn)
                            .size(12.0)
                            .color(Color32::from_rgb(220, 215, 235)),
                    )
                    .fill(Color32::from_rgb(32, 22, 44))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(68, 50, 92)))
                    .corner_radius(6.0);

                    if ui.add(refresh_btn).clicked() {
                        app.refresh_resolve_status();
                    }

                    // Connection Status Pill Badge
                    let is_connected = app.status.as_ref().is_some_and(|s| s.connected);
                    let (bg_color, stroke_color, dot_color, dot_char, text_color, status_text) =
                        if is_connected {
                            let project_name = app
                                .status
                                .as_ref()
                                .and_then(|s| s.project_name.as_deref())
                                .unwrap_or("Unknown Project");
                            (
                                Color32::from_rgb(18, 30, 24),
                                Color32::from_rgb(38, 80, 52),
                                Color32::from_rgb(46, 213, 115),
                                "●",
                                Color32::from_rgb(180, 245, 205),
                                format!("{} ({})", t.connected, project_name),
                            )
                        } else {
                            (
                                Color32::from_rgb(32, 26, 20),
                                Color32::from_rgb(85, 65, 32),
                                Color32::from_rgb(255, 171, 0),
                                "○",
                                Color32::from_rgb(255, 205, 120),
                                t.waiting_resolve.to_string(),
                            )
                        };

                    egui::Frame::new()
                        .fill(bg_color)
                        .stroke(Stroke::new(1.0, stroke_color))
                        .corner_radius(12.0)
                        .inner_margin(egui::Margin::symmetric(10, 4))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);
                                ui.label(egui::RichText::new(dot_char).size(10.0).color(dot_color));
                                ui.label(
                                    egui::RichText::new(status_text)
                                        .size(11.5)
                                        .color(text_color),
                                );
                            });
                        });
                });
            });
        });
}
