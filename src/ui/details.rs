//! Now Playing Details ("Ansicht aktueller Titel" - Spotify-style right side panel).
//! Displays high-res album cover, track & artist details, artist follow/view card,
//! queue peek ("Als Nächstes"), and live synchronized lyrics preview.

use egui::{Align2, Rect, Sense, Ui, Vec2, pos2, vec2};

use super::widgets::{icon_button, named, paint_cover};
use super::{Action, App, NpTab, glyphs};
use crate::model::Target;
use crate::theme::{self, icon};

pub fn details_panel(app: &mut App, ui: &mut Ui) {
    let track = app.playing_track().cloned().or_else(|| app.current().cloned());
    let accent = app.accent_color();

    // Top Header: "Aktueller Titel" + Close button
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Aktueller Titel").font(theme::bold(18.0)).color(theme::text()));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(super::titlebar::right_gap(ui.ctx(), ui.max_rect(), 0.0));
            if icon_button(ui, icon::X, 16.0, "Schließen", false).clicked() {
                app.details_open = false;
            }
        });
    });
    ui.add_space(10.0);

    let Some(t) = track else {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(egui::RichText::new("Kein Titel aktiv").font(theme::semibold(15.0)).color(theme::text_dim()));
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Wähle einen Song aus, um Details anzuzeigen").font(theme::regular(13.0)).color(theme::text_faint()));
        });
        return;
    };

    egui::ScrollArea::vertical()
        .id_salt("np-details-scroll")
        .auto_shrink(false)
        .show(ui, |ui| {
            let width = ui.available_width();

            // 1. Large Cover Art
            let cover_size = width.min(300.0);
            let (cover_rect, _) = ui.allocate_exact_size(Vec2::splat(cover_size), Sense::click());
            super::widgets::soft_shadow(ui, cover_rect, 10.0, 16.0, 0.4);
            paint_cover(ui, cover_rect, &t.thumbs, 10.0, 1.0);
            ui.add_space(14.0);

            // 2. Title, Artists & Favorite Heart
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.add_sized([width - 44.0, 22.0], egui::Label::new(
                        egui::RichText::new(&t.title).font(theme::bold(18.0)).color(theme::text())
                    ).truncate());

                    ui.horizontal_wrapped(|ui| {
                        for (i, a) in t.artists.iter().enumerate() {
                            let comma = if i + 1 < t.artists.len() { ", " } else { "" };
                            let label = format!("{}{}", a.name, comma);
                            let artist_resp = ui.link(egui::RichText::new(label).font(theme::regular(14.0)).color(theme::text_dim()));
                            if let Some(id) = &a.id {
                                if artist_resp.clicked() {
                                    app.actions.push(Action::Open(Target::Artist(id.clone())));
                                }
                            }
                        }
                    });
                });

                // Favorite Heart Button
                let liked = app.liked.contains(&t.id);
                let btn_r = Rect::from_center_size(pos2(cover_rect.right() - 16.0, ui.cursor().top() - 16.0), Vec2::splat(28.0));
                let hresp = ui.interact(btn_r, ui.scope_id().with("details-like"), Sense::click());
                let hresp = named(hresp, if liked { "Aus Favoriten entfernen" } else { "Zu Favoriten hinzufügen" })
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                let pop = theme::anim_bool(ui.ctx(), ("liked", &t.id), liked, 0.25);
                let icon_fg = if liked { accent } else { theme::text_dim() };
                glyphs::draw(ui, btn_r.center(), 22.0 + pop * 2.0, if liked { glyphs::HEART_FILLED } else { glyphs::HEART }, icon_fg);
                if hresp.clicked() {
                    app.actions.push(Action::Like(t.clone(), !liked));
                }
            });
            ui.add_space(16.0);

            // 3. "Über den Künstler" (About the Artist) card
            let artist_name = t.artists.first().map(|a| a.name.as_str()).unwrap_or("Künstler");
            let artist_id = t.artists.first().and_then(|a| a.id.clone());

            let (card_rect, card_resp) = ui.allocate_exact_size(vec2(width, 100.0), Sense::click());
            let ch = theme::anim_bool(ui.ctx(), card_resp.id.with("ch"), card_resp.hovered(), 0.15);
            ui.painter().rect_filled(card_rect, 10.0, theme::panel(0.5 + 0.15 * ch));
            ui.painter().rect_stroke(card_rect, 10.0, egui::Stroke::new(1.0, theme::glass(0.12)), egui::StrokeKind::Inside);

            let text_rect = card_rect.shrink(12.0);
            ui.painter().text(
                text_rect.left_top(),
                Align2::LEFT_TOP,
                "Über den Künstler",
                theme::bold(12.0),
                theme::text_dim(),
            );
            ui.painter().text(
                text_rect.left_top() + vec2(0.0, 20.0),
                Align2::LEFT_TOP,
                artist_name,
                theme::bold(16.0),
                theme::text(),
            );

            // "Künstlerprofil öffnen" button inside card
            let btn_size = vec2(150.0, 28.0);
            let btn_rect = Rect::from_min_size(pos2(text_rect.left(), text_rect.bottom() - btn_size.y), btn_size);
            let bresp = ui.interact(btn_rect, ui.scope_id().with("details-artist-btn"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            let bh = theme::anim_bool(ui.ctx(), bresp.id.with("bh"), bresp.hovered(), 0.15);
            ui.painter().rect_filled(btn_rect, 14.0, theme::glass(0.15 + 0.1 * bh));
            ui.painter().rect_stroke(btn_rect, 14.0, egui::Stroke::new(1.0, theme::glass(0.25)), egui::StrokeKind::Inside);
            ui.painter().text(
                btn_rect.center(),
                Align2::CENTER_CENTER,
                "Künstlerprofil",
                theme::semibold(12.0),
                theme::text(),
            );

            if (card_resp.clicked() || bresp.clicked()) && let Some(aid) = artist_id {
                app.actions.push(Action::Open(Target::Artist(aid)));
            }
            ui.add_space(14.0);

            // 4. "Als Nächstes in der Warteschlange" (Up Next Peek)
            if let Some(next_t) = app.queue.upcoming() {
                ui.label(egui::RichText::new("Als Nächstes").font(theme::bold(14.0)).color(theme::text()));
                ui.add_space(6.0);

                let (peek_rect, peek_resp) = ui.allocate_exact_size(vec2(width, 56.0), Sense::click());
                let ph = theme::anim_bool(ui.ctx(), peek_resp.id.with("ph"), peek_resp.hovered(), 0.15);
                ui.painter().rect_filled(peek_rect, 8.0, theme::glass(0.08 + 0.08 * ph));

                let thumb_rect = Rect::from_min_size(peek_rect.left_top() + vec2(8.0, 8.0), Vec2::splat(40.0));
                paint_cover(ui, thumb_rect, &next_t.thumbs, 6.0, 1.0);

                let info_left = thumb_rect.right() + 10.0;
                ui.painter().text(
                    pos2(info_left, peek_rect.top() + 12.0),
                    Align2::LEFT_TOP,
                    &next_t.title,
                    theme::semibold(13.0),
                    theme::text(),
                );
                ui.painter().text(
                    pos2(info_left, peek_rect.top() + 29.0),
                    Align2::LEFT_TOP,
                    next_t.artist_line(),
                    theme::regular(12.0),
                    theme::text_dim(),
                );

                if peek_resp.clicked() {
                    let next_idx = app.queue.index.map_or(0, |i| i + 1);
                    app.actions.push(Action::QueueJump(next_idx));
                }
                ui.add_space(14.0);
            }

            // 5. "Songtext" Preview card
            ui.label(egui::RichText::new("Songtext").font(theme::bold(14.0)).color(theme::text()));
            ui.add_space(6.0);

            let (lyric_rect, lyric_resp) = ui.allocate_exact_size(vec2(width, 110.0), Sense::click());
            let lh = theme::anim_bool(ui.ctx(), lyric_resp.id.with("lh"), lyric_resp.hovered(), 0.15);
            ui.painter().rect_filled(lyric_rect, 10.0, theme::panel(0.4 + 0.1 * lh));
            ui.painter().rect_stroke(lyric_rect, 10.0, egui::Stroke::new(1.0, theme::glass(0.12)), egui::StrokeKind::Inside);

            let inner_l = lyric_rect.shrink(12.0);
            let sample_lyrics = if let Some((_, lset)) = &app.lyrics {
                lset.sources.first().and_then(|src| src.lines.first().map(|l| l.text.clone()))
                    .unwrap_or_else(|| "Tippe hier für den vollständigen Songtext…".to_string())
            } else {
                "Songtext wird geladen oder ist verfügbar…".to_string()
            };

            ui.painter().text(
                inner_l.left_top(),
                Align2::LEFT_TOP,
                &sample_lyrics,
                theme::semibold(14.0),
                theme::text(),
            );

            let expand_btn = Rect::from_min_size(pos2(inner_l.left(), inner_l.bottom() - 26.0), vec2(140.0, 24.0));
            ui.painter().rect_filled(expand_btn, 12.0, theme::glass(0.15));
            ui.painter().text(
                expand_btn.center(),
                Align2::CENTER_CENTER,
                "Ganz anzeigen ›",
                theme::semibold(11.5),
                accent,
            );

            if lyric_resp.clicked() {
                app.np_tab = NpTab::Lyrics;
                app.actions.push(Action::OpenNowPlaying(true));
            }

            ui.add_space(20.0);
        });
}
