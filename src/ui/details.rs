//! Now Playing Details ("Ansicht aktueller Titel" - Spotify-style right side panel).
//! Displays high-res album cover, track & artist details, artist follow/view card,
//! queue peek ("Als Nächstes"), and live synchronized lyrics preview.

use egui::{Align2, Color32, Rect, Sense, Ui, Vec2, pos2, vec2};

use super::widgets::{named, paint_cover};
use super::{Action, App, NpTab, glyphs};
use crate::model::Target;
use crate::theme;

pub fn details_panel(app: &mut App, ui: &mut Ui) {
    let track = app.shown().cloned().or_else(|| app.current().cloned());
    let accent = app.accent_color();

    // Context Header: Spotify-style Playlist/Album name (or "Now Playing" if none) + close button
    let context_info = app.playing_context();
    let (header_title, header_target) = match &context_info {
        Some((title, target)) => (title.as_str(), target.clone()),
        None => ("Now Playing", None),
    };

    ui.horizontal(|ui| {
        let avail_w = (ui.available_width() - 8.0).max(10.0);
        let (h_box, _) = ui.allocate_exact_size(vec2(avail_w, 24.0), Sense::hover());
        let is_link = header_target.is_some();
        let (_, h_resp) = super::widgets::marquee_title(
            ui,
            ui.scope_id().with("details-header-title"),
            h_box,
            header_title,
            theme::bold(17.0),
            theme::text(),
            false,
            is_link,
        );
        if let Some(target) = header_target {
            let h_resp = h_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
            if h_resp.clicked() {
                app.actions.push(Action::Open(target));
            }
        }
    });
    ui.add_space(10.0);

    let Some(t) = track else {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(egui::RichText::new("Nothing playing").font(theme::semibold(15.0)).color(theme::text_dim()));
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Select a song to display details").font(theme::regular(13.0)).color(theme::text_faint()));
        });
        return;
    };

    egui::ScrollArea::vertical()
        .id_salt("np-details-scroll")
        .auto_shrink(false)
        .show(ui, |ui| {
            let width = ui.available_width();

            // 1. Large Cover Art (10px rounded corners, deep soft shadow, crossfade on change)
            let cover_size = (width - 4.0).min(420.0).max(180.0);
            let cover_x = ui.cursor().left() + (width - cover_size) / 2.0;
            let cover_rect = Rect::from_min_size(pos2(cover_x, ui.cursor().top()), Vec2::splat(cover_size));
            let (_, cover_resp) = ui.allocate_exact_size(Vec2::splat(cover_size), Sense::click());
            let has_album = t.album.as_ref().and_then(|a| a.id.as_ref()).is_some();
            let cover_resp = if has_album {
                cover_resp.on_hover_cursor(egui::CursorIcon::PointingHand)
            } else {
                cover_resp
            };
            super::widgets::soft_shadow(ui, cover_rect, 10.0, 18.0, 0.45);
            super::widgets::paint_cover_xfade(ui, egui::Id::unique("details-cover"), cover_rect, &t.thumbs, 10.0, 1.0);
            let liked = app.liked.contains(&t.id);
            egui::Popup::context_menu(&cover_resp).show(|ui| {
                super::widgets::song_menu(ui, &t, liked, None, None, &mut app.actions);
            });
            if cover_resp.clicked() {
                if let Some(album) = &t.album && let Some(id) = &album.id {
                    app.actions.push(Action::Open(Target::Album(id.clone())));
                }
            }
            ui.add_space(14.0);

            // 2. Title & Favorite Heart on same horizontal baseline (Spotify style)
            ui.horizontal(|ui| {
                let action_w = 64.0;
                let title_w = (width - action_w - 8.0).max(10.0);

                let (title_box, _) = ui.allocate_exact_size(vec2(title_w, 24.0), Sense::hover());
                let (_, title_resp) = super::widgets::marquee_title(
                    ui,
                    ui.scope_id().with("details-title"),
                    title_box,
                    &t.title,
                    theme::bold(18.0),
                    theme::text(),
                    app.playing(),
                    true,
                );
                let title_resp = if has_album {
                    title_resp.on_hover_cursor(egui::CursorIcon::PointingHand)
                } else {
                    title_resp
                };
                egui::Popup::context_menu(&title_resp).show(|ui| {
                    super::widgets::song_menu(ui, &t, liked, None, None, &mut app.actions);
                });
                if title_resp.clicked() {
                    if let Some(album) = &t.album && let Some(id) = &album.id {
                        app.actions.push(Action::Open(Target::Album(id.clone())));
                    }
                }

                let (btn_r, hresp) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
                let hresp = named(hresp, if liked { "Remove from favorites" } else { "Add to favorites" })
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                let hh = theme::anim_bool(ui.ctx(), hresp.id.with("hh"), hresp.hovered(), 0.15);
                let pop = theme::anim_bool(ui.ctx(), ("liked", &t.id), liked, 0.25);
                let icon_fg = if liked { accent } else { theme::lerp_color(theme::text_dim(), theme::text(), hh) };
                glyphs::draw(ui, btn_r.center(), 20.0 + pop * 3.0, if liked { glyphs::HEART_FILLED } else { glyphs::HEART }, icon_fg);
                if hresp.clicked() {
                    app.actions.push(Action::Like(t.clone(), !liked));
                }

                // Block button (⊖ Minus Circle)
                let is_blocked = app.blocked_tracks.contains_key(&t.id);
                let (b_rect, b_resp) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
                let b_resp = named(b_resp, if is_blocked { "Song entblocken" } else { "Song blockieren" })
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                let bh = theme::anim_bool(ui.ctx(), b_resp.id.with("bh"), b_resp.hovered(), 0.15);
                let b_fg = if is_blocked { Color32::from_rgb(239, 68, 68) } else { theme::lerp_color(theme::text_dim(), theme::text(), bh) };
                let center = pos2(b_rect.center().x.round(), b_rect.center().y.round());
                super::widgets::draw_minus_circle(ui, center, 7.2, b_fg, 1.2);
                if b_resp.clicked() {
                    if is_blocked {
                        app.actions.push(Action::UnblockTrack(t.id.clone()));
                    } else {
                        app.actions.push(Action::BlockTrack(t.clone()));
                    }
                }
            });

            // Artists Line directly below Title with perfectly calibrated optical underline
            ui.add_space(2.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 4.0);
                for (i, a) in t.artists.iter().enumerate() {
                    let font = theme::regular(14.0);
                    let galley = super::widgets::galley(ui, &a.name, font.clone(), theme::text_dim(), width);
                    let (a_rect, a_resp) = ui.allocate_exact_size(galley.size(), Sense::click());
                    let a_resp = a_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                    let ah = theme::anim_bool(ui.ctx(), a_resp.id.with("ah"), a_resp.hovered(), 0.12);
                    let a_color = theme::lerp_color(theme::text_dim(), theme::text(), ah);
                    ui.painter().galley(a_rect.left_top(), galley, a_color);
                    // Underline only under the artist's name, snapped to text baseline
                    if ah > 0.0 {
                        ui.painter().hline(a_rect.left()..=a_rect.right(), (a_rect.bottom() + 0.5).round(), egui::Stroke::new(1.0, a_color));
                    }
                    if a_resp.clicked() && let Some(id) = &a.id {
                        app.actions.push(Action::Open(Target::Artist(id.clone())));
                    }
                    if i + 1 < t.artists.len() {
                        let comma_g = super::widgets::galley(ui, ", ", font, theme::text_dim(), width);
                        let (c_rect, _) = ui.allocate_exact_size(comma_g.size(), Sense::hover());
                        ui.painter().galley(c_rect.left_top(), comma_g, theme::text_dim());
                    }
                }
            });
            ui.add_space(16.0);

            // 3. "About the artist" Card with integrated "Follow" / "Following" button
            let artist_name = t.artists.first().map(|a| a.name.as_str()).unwrap_or("Artist");
            let artist_id = t.artists.first().and_then(|a| a.id.clone());

            let (card_rect, card_resp) = ui.allocate_exact_size(vec2(width, 114.0), Sense::click());
            let ch = theme::anim_bool(ui.ctx(), card_resp.id.with("ch"), card_resp.hovered(), 0.15);
            ui.painter().rect_filled(card_rect, 10.0, theme::panel(0.48 + 0.14 * ch));
            ui.painter().rect_stroke(card_rect, 10.0, egui::Stroke::new(1.0, theme::glass(0.12)), egui::StrokeKind::Inside);

            let text_rect = card_rect.shrink(14.0);
            ui.painter().text(
                text_rect.left_top(),
                Align2::LEFT_TOP,
                "About the artist",
                theme::bold(12.0),
                theme::text_dim(),
            );

            // Artist name with hover underline
            let name_font = theme::bold(16.5);
            let name_g = super::widgets::galley(ui, artist_name, name_font, theme::text(), text_rect.width());
            let name_r = Rect::from_min_size(text_rect.left_top() + vec2(0.0, 22.0), name_g.size());
            let an_resp = ui.interact(name_r, ui.scope_id().with("about-artist-name"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            let anh = theme::anim_bool(ui.ctx(), an_resp.id.with("anh"), an_resp.hovered(), 0.12);
            let an_color = theme::lerp_color(theme::text(), Color32::WHITE, anh);
            ui.painter().galley(name_r.left_top(), name_g, an_color);
            if anh > 0.0 {
                ui.painter().hline(name_r.left()..=name_r.right(), (name_r.bottom() + 0.5).round(), egui::Stroke::new(1.0, an_color));
            }

            // Spotify-style "Follow" / "Following" pill button inside card (generous 16px gap below name)
            let is_following = app.followed_artists.contains(artist_name);
            let btn_size = vec2(96.0, 28.0);
            let btn_rect = Rect::from_min_size(pos2(text_rect.left(), text_rect.bottom() - btn_size.y), btn_size);
            let bresp = ui.interact(btn_rect, ui.scope_id().with("details-follow-btn"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            let bh = theme::anim_bool(ui.ctx(), bresp.id.with("bh"), bresp.hovered(), 0.15);
            if is_following {
                ui.painter().rect_filled(btn_rect, 14.0, theme::glass(0.20 + 0.08 * bh));
                ui.painter().rect_stroke(btn_rect, 14.0, egui::Stroke::new(1.0, theme::glass(0.35)), egui::StrokeKind::Inside);
                ui.painter().text(
                    btn_rect.center(),
                    Align2::CENTER_CENTER,
                    "Following",
                    theme::semibold(12.0),
                    theme::text(),
                );
            } else {
                ui.painter().rect_filled(btn_rect, 14.0, theme::glass(0.10 + 0.10 * bh));
                ui.painter().rect_stroke(btn_rect, 14.0, egui::Stroke::new(1.0, theme::glass(0.25)), egui::StrokeKind::Inside);
                ui.painter().text(
                    btn_rect.center(),
                    Align2::CENTER_CENTER,
                    "Follow",
                    theme::semibold(12.0),
                    theme::text(),
                );
            }

            if bresp.clicked() {
                if is_following {
                    app.followed_artists.remove(artist_name);
                } else {
                    app.followed_artists.insert(artist_name.to_string());
                }
            } else if card_resp.clicked() && let Some(aid) = artist_id {
                app.actions.push(Action::Open(Target::Artist(aid)));
            }
            ui.add_space(14.0);

            // 4. "Next in queue" (Up Next Peek)
            let next_t = if app.mix_hold().is_some() {
                app.current()
            } else {
                app.queue.upcoming()
            };
            if let Some(next_t) = next_t {
                ui.label(egui::RichText::new("Next in queue").font(theme::bold(14.0)).color(theme::text()));
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
                    let next_idx = if app.mix_hold().is_some() {
                        app.queue.index.unwrap_or(0)
                    } else {
                        app.queue.index.map_or(0, |i| i + 1)
                    };
                    app.actions.push(Action::QueueJump(next_idx));
                }
                ui.add_space(14.0);
            }

            // 5. Live Synced Lyrics Preview card (Spotify-style)
            ui.label(egui::RichText::new("Lyrics").font(theme::bold(14.0)).color(theme::text()));
            ui.add_space(6.0);

            let (lyric_rect, lyric_resp) = ui.allocate_exact_size(vec2(width, 140.0), Sense::click());
            let lh = theme::anim_bool(ui.ctx(), lyric_resp.id.with("lh"), lyric_resp.hovered(), 0.15);
            ui.painter().rect_filled(lyric_rect, 10.0, theme::panel(0.42 + 0.12 * lh));
            ui.painter().rect_stroke(lyric_rect, 10.0, egui::Stroke::new(1.0, theme::glass(0.12)), egui::StrokeKind::Inside);

            let inner_l = lyric_rect.shrink(14.0);
            let curr_id = Some(t.id.as_str());

            if app.playing() {
                // Keep lyrics preview synced smoothly while song is playing
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
            }

            let mut showed_synced = false;
            if let Some((lid, lset)) = &app.lyrics
                && curr_id == Some(lid.as_str())
                && let Some(lyrics) = app.lyric_prefs.choose(lid, lset).and_then(|p| lset.get(p))
                && !lyrics.lines.is_empty()
            {
                let cur_ms = app.audio.status.position().as_millis() as i32;
                let active_idx = lyrics.lines.iter().rposition(|l| l.start <= cur_ms).unwrap_or(0);

                let mut y_cursor = inner_l.top();

                // If active line has a predecessor, show it subtly
                if active_idx > 0 && let Some(prev) = lyrics.lines.get(active_idx - 1) {
                    if !prev.text.trim().is_empty() {
                        let prev_g = super::widgets::galley(ui, &prev.text, theme::regular(12.5), theme::text_faint(), inner_l.width());
                        ui.painter().galley(pos2(inner_l.left(), y_cursor), prev_g, theme::text_faint());
                        y_cursor += 18.0;
                    }
                }

                // Active line (prominent, bold, illuminated in white)
                if let Some(curr) = lyrics.lines.get(active_idx) {
                    let text = if curr.text.trim().is_empty() { "♫" } else { &curr.text };
                    let curr_g = super::widgets::galley(ui, text, theme::bold(15.0), Color32::WHITE, inner_l.width());
                    ui.painter().galley(pos2(inner_l.left(), y_cursor), curr_g, Color32::WHITE);
                    y_cursor += 22.0;
                }

                // Next line (dim)
                if let Some(next) = lyrics.lines.get(active_idx + 1) {
                    if !next.text.trim().is_empty() {
                        let next_g = super::widgets::galley(ui, &next.text, theme::regular(13.0), theme::text_dim(), inner_l.width());
                        ui.painter().galley(pos2(inner_l.left(), y_cursor), next_g, theme::text_dim());
                    }
                }

                showed_synced = true;
            }

            if !showed_synced {
                let sample_lyrics = if app.loading {
                    "Loading lyrics…".to_string()
                } else {
                    "Click to view lyrics & sing along…".to_string()
                };
                ui.painter().text(
                    inner_l.left_top(),
                    Align2::LEFT_TOP,
                    &sample_lyrics,
                    theme::semibold(14.0),
                    theme::text_dim(),
                );
            }

            let expand_btn = Rect::from_min_size(pos2(inner_l.left(), inner_l.bottom() - 24.0), vec2(130.0, 24.0));
            let eh = theme::anim_bool(ui.ctx(), lyric_resp.id.with("eh"), lyric_resp.hovered(), 0.15);
            ui.painter().rect_filled(expand_btn, 12.0, theme::glass(0.14 + 0.10 * eh));
            ui.painter().text(
                expand_btn.center(),
                Align2::CENTER_CENTER,
                "Show all lyrics ›",
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
