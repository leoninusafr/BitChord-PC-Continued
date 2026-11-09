//! "Mit einem Gerät verbinden" / Spotify Connect style devices dialog.
//! Shows local playback outputs, active remote web controller sessions,
//! local network URL and authentic PNG QR code (NO ASCII/text QR code).

use egui::{Align2, Color32, Rect, Sense, pos2, vec2};

use super::widgets::icon_button;
use super::{App, glyphs};
use crate::theme::{self, icon};

pub fn devices_dialog(app: &mut App, ctx: &egui::Context) {
    if !app.devices_open {
        return;
    }

    let screen = ctx.content_rect();
    // Backdrop overlay
    egui::Area::new(egui::Id::unique("devices-modal-backdrop"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            let resp = ui.allocate_response(screen.size(), Sense::click());
            ui.painter().rect_filled(screen, 0.0, Color32::from_black_alpha(160));
            if resp.clicked() {
                app.devices_open = false;
            }
        });

    let w = 460.0f32.min(screen.width() - 32.0);
    let h = 540.0f32.min(screen.height() - 40.0);
    let modal_rect = Rect::from_center_size(screen.center(), vec2(w, h));

    egui::Area::new(egui::Id::unique("devices-modal-window"))
        .order(egui::Order::Tooltip)
        .fixed_pos(modal_rect.min)
        .show(ctx, |ui| {
            ui.scope_builder(egui::UiBuilder::new().max_rect(modal_rect), |ui| {
                // Acrylic dialog background with smooth rounded corners and subtle border
                ui.painter().rect_filled(modal_rect, 16.0, theme::panel(0.96));
                ui.painter().rect_stroke(modal_rect, 16.0, egui::Stroke::new(1.0, theme::glass(0.18)), egui::StrokeKind::Inside);
                super::widgets::soft_shadow(ui, modal_rect, 16.0, 32.0, 0.5);

                ui.set_clip_rect(modal_rect);
                let inner = modal_rect.shrink(20.0);

                ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
                    // Title Bar
                    ui.horizontal(|ui| {
                        glyphs::draw(ui, ui.cursor().left_top() + vec2(12.0, 12.0), 22.0, glyphs::DEVICES, app.accent_color());
                        ui.add_space(28.0);
                        ui.label(egui::RichText::new("Mit einem Gerät verbinden").font(theme::bold(18.0)).color(theme::text()));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if icon_button(ui, icon::X, 16.0, "Schließen", false).clicked() {
                                app.devices_open = false;
                            }
                        });
                    });
                    ui.add_space(14.0);

                    // 1. Current Playback Device: Desktop Windows
                    ui.label(egui::RichText::new("Aktuelles Wiedergabegerät").font(theme::bold(13.0)).color(theme::text_dim()));
                    ui.add_space(6.0);

                    let (cur_rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 52.0), Sense::hover());
                    ui.painter().rect_filled(cur_rect, 10.0, theme::glass(0.08));
                    ui.painter().rect_stroke(cur_rect, 10.0, egui::Stroke::new(1.5, app.accent_color()), egui::StrokeKind::Inside);

                    let icon_center = pos2(cur_rect.left() + 26.0, cur_rect.center().y);
                    glyphs::draw(ui, icon_center, 22.0, glyphs::DEVICES, app.accent_color());

                    ui.painter().text(
                        pos2(cur_rect.left() + 52.0, cur_rect.top() + 10.0),
                        Align2::LEFT_TOP,
                        "Dieser Computer (Windows)",
                        theme::bold(14.0),
                        theme::text(),
                    );
                    ui.painter().text(
                        pos2(cur_rect.left() + 52.0, cur_rect.top() + 28.0),
                        Align2::LEFT_TOP,
                        "Aktive BitChord Audioausgabe",
                        theme::regular(12.0),
                        app.accent_color(),
                    );

                    ui.add_space(16.0);

                    // 2. Remote Control / Spotify Connect section
                    ui.label(egui::RichText::new("Smartphone Remote-Steuerung").font(theme::bold(13.0)).color(theme::text_dim()));
                    ui.add_space(6.0);

                    let (remote_rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 260.0), Sense::hover());
                    ui.painter().rect_filled(remote_rect, 12.0, theme::panel(0.4));
                    ui.painter().rect_stroke(remote_rect, 12.0, egui::Stroke::new(1.0, theme::glass(0.1)), egui::StrokeKind::Inside);

                    let r_inner = remote_rect.shrink(14.0);
                    ui.scope_builder(egui::UiBuilder::new().max_rect(r_inner), |ui| {
                        ui.label(egui::RichText::new("Scanne den QR-Code mit der Handykamera, um die BitChord Remote-Steuerung im Browser zu öffnen:")
                            .font(theme::regular(12.5))
                            .color(theme::text_dim()));
                        ui.add_space(8.0);

                        let url = if let Some(srv) = &app.remote_server {
                            srv.local_url()
                        } else {
                            "http://127.0.0.1:8420".to_string()
                        };

                        // Real PNG QR Code rendering (Strictly NO text/ASCII QR)
                        let qr_bytes = if let Some(srv) = &app.remote_server {
                            srv.qr_png()
                        } else {
                            crate::remote::generate_qr_png_bytes(&url)
                        };

                        ui.horizontal(|ui| {
                            if let Some(png) = qr_bytes.as_ref() {
                                let key = egui::Id::unique(("remote-qr-tex", &url));
                                let tex = ui.ctx().data_mut(|d| {
                                    d.get_temp::<egui::TextureHandle>(key).unwrap_or_else(|| {
                                        let img = image::load_from_memory(png.as_slice()).unwrap().into_rgba8();
                                        let ci = egui::ColorImage::from_rgba_unmultiplied(
                                            [img.width() as usize, img.height() as usize],
                                            img.as_raw(),
                                        );
                                        let t = ui.ctx().load_texture("remote-qr", ci, egui::TextureOptions::LINEAR);
                                        d.insert_temp(key, t.clone());
                                        t
                                    })
                                });

                                let (img_rect, _) = ui.allocate_exact_size(vec2(130.0, 130.0), Sense::hover());
                                ui.painter().rect_filled(img_rect, 8.0, Color32::WHITE);
                                ui.painter().image(tex.id(), img_rect.shrink(4.0), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                            }

                            ui.add_space(10.0);
                            ui.vertical(|ui| {
                                ui.add_space(8.0);
                                ui.label(egui::RichText::new("WLAN-Direktlink:").font(theme::semibold(12.0)).color(theme::text()));
                                ui.add_space(2.0);
                                let link_resp = ui.link(egui::RichText::new(&url).font(egui::FontId::monospace(12.0)).color(app.accent_color()));
                                if link_resp.clicked() {
                                    ui.ctx().copy_text(url.clone());
                                    app.toast(format!("Link kopiert: {}", url));
                                }
                                ui.add_space(10.0);
                                ui.label(egui::RichText::new("Features am Smartphone:")
                                    .font(theme::semibold(11.5))
                                    .color(theme::text_dim()));
                                ui.label(egui::RichText::new("• Play / Pause / Skip / Zurück\n• Echtzeit-Scrubber (Timeline)\n• Lautstärkeregler\n• Shuffle & Repeat Toggle")
                                    .font(theme::regular(11.0))
                                    .color(theme::text_faint()));
                            });
                        });

                        ui.add_space(10.0);
                        if ui.button(egui::RichText::new("Link kopieren").font(theme::semibold(12.0))).clicked() {
                            ui.ctx().copy_text(url.clone());
                            app.toast("Remote-URL in Zwischenablage kopiert!".to_string());
                        }
                    });
                });
            });
        });
}
