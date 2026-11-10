//! Now Playing Details (Spotify-style right side panel).
//! Displays high-res album cover, track & artist details, artist follow/view card,
//! queue peek ("Next in queue"), and live synchronized lyrics preview.

use egui::{Align2, Color32, Pos2, Rect, Sense, Ui, Vec2, pos2, vec2};

use super::widgets::{named, paint_cover};
use super::{Action, App, LibTab, NpTab, Page, RightTab, glyphs};
use crate::model::Target;
use crate::theme;

pub fn details_panel(app: &mut App, ui: &mut Ui) {
    // 1. Header with Segmented Tab Switcher (Details vs Freunde) and Close button (✕)
    ui.horizontal(|ui| {
        let width = ui.available_width();
        let close_size = 28.0;
        let tabs_w = (width - close_size - 12.0).max(80.0).min(230.0);

        // Segmented pill control
        let (seg_rect, _) = ui.allocate_exact_size(vec2(tabs_w, 30.0), Sense::hover());
        ui.painter().rect_filled(seg_rect, 15.0, theme::glass(0.08));

        let half_w = seg_rect.width() / 2.0;
        let det_rect = Rect::from_min_size(seg_rect.min, vec2(half_w, seg_rect.height()));
        let fri_rect = Rect::from_min_size(pos2(seg_rect.left() + half_w, seg_rect.top()), vec2(half_w, seg_rect.height()));

        let is_det = app.right_tab == RightTab::Details;
        let is_fri = app.right_tab == RightTab::Friends;

        let det_resp = ui.interact(det_rect, ui.scope_id().with("tab-details"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let fri_resp = ui.interact(fri_rect, ui.scope_id().with("tab-friends"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);

        if det_resp.clicked() && !is_det {
            app.right_tab = RightTab::Details;
        }
        if fri_resp.clicked() && !is_fri {
            app.right_tab = RightTab::Friends;
            if app.friend_activity_last_fetch.is_none() || app.friend_activity_last_fetch.unwrap().elapsed() > std::time::Duration::from_secs(15) {
                app.refresh_friend_activity();
            }
        }

        // Active tab background pill
        let anim_det = theme::anim_bool(ui.ctx(), "tab-det-active", is_det, 0.18);
        if anim_det > 0.001 {
            let r = det_rect.shrink(2.0);
            ui.painter().rect_filled(r, 13.0, theme::glass(0.18 * anim_det));
            ui.painter().rect_stroke(r, 13.0, egui::Stroke::new(1.0, theme::glass(0.20 * anim_det)), egui::StrokeKind::Inside);
        }
        let anim_fri = theme::anim_bool(ui.ctx(), "tab-fri-active", is_fri, 0.18);
        if anim_fri > 0.001 {
            let r = fri_rect.shrink(2.0);
            ui.painter().rect_filled(r, 13.0, theme::glass(0.18 * anim_fri));
            ui.painter().rect_stroke(r, 13.0, egui::Stroke::new(1.0, theme::glass(0.20 * anim_fri)), egui::StrokeKind::Inside);
        }

        // Details text
        let det_fg = if is_det { Color32::WHITE } else { theme::text_dim() };
        ui.painter().text(det_rect.center(), Align2::CENTER_CENTER, "Details", if is_det { theme::bold(13.0) } else { theme::semibold(12.5) }, det_fg);

        // Friends text & count badge
        let fri_fg = if is_fri { Color32::WHITE } else { theme::text_dim() };
        let fri_label = if !app.friend_activity.is_empty() {
            format!("Friends ({})", app.friend_activity.len())
        } else {
            "Friends".to_string()
        };
        ui.painter().text(fri_rect.center(), Align2::CENTER_CENTER, &fri_label, if is_fri { theme::bold(13.0) } else { theme::semibold(12.5) }, fri_fg);

        // Spacer to push close button to the far right
        let remaining_w = ui.available_width() - close_size;
        if remaining_w > 0.0 {
            ui.add_space(remaining_w);
        }

        // Close button (✕)
        let (close_rect, close_resp) = ui.allocate_exact_size(Vec2::splat(close_size), Sense::click());
        let close_resp = close_resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Close");
        let ch = theme::anim_bool(ui.ctx(), close_resp.id.with("ch"), close_resp.hovered(), 0.12);
        if ch > 0.0 {
            ui.painter().circle_filled(close_rect.center(), close_size / 2.0, theme::glass(0.12 * ch));
        }
        super::widgets::icon_at(ui, close_rect.center(), crate::theme::icon::X, 15.0, theme::lerp_color(theme::text_dim(), theme::text(), ch));
        if close_resp.clicked() {
            app.details_open = false;
        }
    });

    ui.add_space(10.0);

    match app.right_tab {
        RightTab::Details => details_content(app, ui),
        RightTab::Friends => friends_content(app, ui),
    }
}

fn details_content(app: &mut App, ui: &mut Ui) {
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

    // If lyrics aren't loaded or are for another song, trigger backend lyrics request immediately
    if !app.lyrics.as_ref().is_some_and(|(lid, _)| lid == &t.id) && !app.loading {
        app.backend.request(crate::backend::Req::Lyrics(t.clone()));
    }

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
                let action_w = 32.0;
                let title_w = (width - action_w - 6.0).max(10.0);

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
                if hh > 0.0 {
                    ui.painter().circle_filled(btn_r.center(), 14.0, theme::glass(0.08 * hh));
                }
                let icon_fg = if liked { accent } else { theme::lerp_color(theme::text_dim(), theme::text(), hh) };
                glyphs::draw(ui, btn_r.center(), 18.0 + pop * 2.0, if liked { glyphs::HEART_FILLED } else { glyphs::HEART }, icon_fg);
                if hresp.clicked() {
                    app.actions.push(Action::Like(t.clone(), !liked));
                }
            });

            // Artists Line directly below Title with clean hover illumination
            ui.add_space(2.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 4.0);
                for (i, a) in t.artists.iter().enumerate() {
                    let font = theme::regular(14.0);
                    let galley = super::widgets::galley(ui, &a.name, font.clone(), theme::text_dim(), width);
                    let (a_rect, a_resp) = ui.allocate_exact_size(galley.size(), Sense::click());
                    let a_resp = a_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                    let ah = theme::anim_bool(ui.ctx(), a_resp.id.with("ah"), a_resp.hovered(), 0.12);
                    let a_color = theme::lerp_color(theme::text_dim(), Color32::WHITE, ah);
                    ui.painter().galley(a_rect.left_top(), galley, a_color);
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

            let (card_rect, card_resp) = ui.allocate_exact_size(vec2(width, 126.0), Sense::click());
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

            // Artist name with hover illumination
            let name_font = theme::bold(17.0);
            let name_g = super::widgets::galley(ui, artist_name, name_font, theme::text(), text_rect.width());
            let name_r = Rect::from_min_size(text_rect.left_top() + vec2(0.0, 22.0), name_g.size());
            let an_resp = ui.interact(name_r, ui.scope_id().with("about-artist-name"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            let anh = theme::anim_bool(ui.ctx(), an_resp.id.with("anh"), an_resp.hovered(), 0.12);
            let an_color = theme::lerp_color(theme::text(), Color32::WHITE, anh);
            ui.painter().galley(name_r.left_top(), name_g, an_color);

            // Spotify-style "Follow" / "Following" pill button inside card (generously spaced below name)
            let is_following = app.followed_artists.contains(artist_name);
            let btn_size = vec2(96.0, 28.0);
            let btn_rect = Rect::from_min_size(pos2(text_rect.left(), text_rect.top() + 58.0), btn_size);
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

            let is_artist_blocked = artist_id.as_ref().is_some_and(|id| app.blocked_artists.contains_key(id));
            let block_btn_size = vec2(78.0, 28.0);
            let block_btn_rect = Rect::from_min_size(pos2(btn_rect.right() + 8.0, btn_rect.top()), block_btn_size);
            let blk_resp = ui.interact(block_btn_rect, ui.scope_id().with("details-block-artist-btn"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            let blk_h = theme::anim_bool(ui.ctx(), blk_resp.id.with("blk_h"), blk_resp.hovered(), 0.15);
            if is_artist_blocked {
                ui.painter().rect_filled(block_btn_rect, 14.0, theme::glass(0.20 + 0.08 * blk_h));
                ui.painter().rect_stroke(block_btn_rect, 14.0, egui::Stroke::new(1.0, Color32::from_rgb(239, 68, 68).gamma_multiply(0.6)), egui::StrokeKind::Inside);
                ui.painter().text(
                    block_btn_rect.center(),
                    Align2::CENTER_CENTER,
                    "Blocked",
                    theme::semibold(12.0),
                    Color32::from_rgb(239, 68, 68),
                );
            } else {
                ui.painter().rect_filled(block_btn_rect, 14.0, theme::glass(0.08 + 0.10 * blk_h));
                ui.painter().rect_stroke(block_btn_rect, 14.0, egui::Stroke::new(1.0, theme::glass(0.20)), egui::StrokeKind::Inside);
                ui.painter().text(
                    block_btn_rect.center(),
                    Align2::CENTER_CENTER,
                    "Block",
                    theme::semibold(12.0),
                    theme::lerp_color(theme::text_dim(), theme::text(), blk_h),
                );
            }

            if bresp.clicked() {
                if is_following {
                    app.followed_artists.remove(artist_name);
                } else {
                    app.followed_artists.insert(artist_name.to_string());
                }
            } else if blk_resp.clicked() && let Some(aid) = artist_id {
                if is_artist_blocked {
                    app.actions.push(Action::UnblockArtist(aid.to_string()));
                } else {
                    app.actions.push(Action::BlockArtist(aid.to_string(), artist_name.to_string()));
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

fn format_relative_time(timestamp_ms: u64) -> String {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let diff_secs = now_ms.saturating_sub(timestamp_ms) / 1000;
    if diff_secs < 120 {
        "Now".to_string()
    } else if diff_secs < 3600 {
        format!("{}m", diff_secs / 60)
    } else if diff_secs < 86400 {
        format!("{}h", diff_secs / 3600)
    } else {
        format!("{}d", diff_secs / 86400)
    }
}

fn draw_animated_equalizer(ui: &mut Ui, center: Pos2, time: f64) {
    let bar_w = 2.5;
    let gap = 2.0;
    let start_x = center.x - (3.0 * bar_w + 2.0 * gap) / 2.0;
    let green = Color32::from_rgb(29, 185, 84); // Spotify green

    // 3 animated bouncing bars
    let h1 = (4.0 + 8.0 * ((time * 8.0).sin().abs())).clamp(3.0, 14.0) as f32;
    let h2 = (4.0 + 10.0 * ((time * 6.5 + 1.2).sin().abs())).clamp(3.0, 15.0) as f32;
    let h3 = (4.0 + 7.0 * ((time * 9.0 + 2.5).sin().abs())).clamp(3.0, 13.0) as f32;

    let bars = [(0, h1), (1, h2), (2, h3)];
    for (i, h) in bars {
        let x = start_x + (i as f32) * (bar_w + gap);
        let bar_rect = Rect::from_center_size(pos2(x + bar_w / 2.0, center.y), vec2(bar_w, h));
        ui.painter().rect_filled(bar_rect, 1.2, green);
    }
}

fn friends_content(app: &mut App, ui: &mut Ui) {
    let is_signed_in = matches!(*app.transfer.account.lock().unwrap(), crate::transfer::Account::SignedIn { .. });

    if !is_signed_in {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            let (badge_box, _) = ui.allocate_exact_size(Vec2::splat(52.0), Sense::hover());
            super::transfer::badge(ui, badge_box);
            ui.add_space(18.0);
            ui.label(egui::RichText::new("Friend Activity").font(theme::bold(17.0)).color(theme::text()));
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Connect your Spotify account to see what your friends are listening to in real time.")
                .font(theme::regular(13.0))
                .color(theme::text_dim()));
            ui.add_space(20.0);
            if super::widgets::pill(ui, "Sign in to Spotify", Some(crate::theme::icon::USERS), true).clicked() {
                app.actions.push(Action::Go(crate::ui::Page::Settings));
            }
        });
        return;
    }

    // Sub-header with friend count and refresh button
    ui.horizontal(|ui| {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let active_count = app.friend_activity.iter().filter(|f| now_ms.saturating_sub(f.timestamp) < 300_000).count();
        let subtitle = if active_count > 0 {
            format!("{active_count} active now")
        } else if !app.friend_activity.is_empty() {
            format!("{} friends", app.friend_activity.len())
        } else {
            "Friend Activity".to_string()
        };
        ui.label(egui::RichText::new(subtitle).font(theme::semibold(12.5)).color(theme::text_dim()));

        let remaining = ui.available_width() - 24.0;
        if remaining > 0.0 {
            ui.add_space(remaining);
        }

        let (r, resp) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Refresh friend activity");
        let rh = theme::anim_bool(ui.ctx(), resp.id.with("rh"), resp.hovered() || app.friend_activity_loading, 0.15);
        if rh > 0.0 {
            ui.painter().circle_filled(r.center(), 12.0, theme::glass(0.10 * rh));
        }
        if app.friend_activity_loading {
            let mut cui = ui.new_child(egui::UiBuilder::new().max_rect(r));
            cui.spinner();
        } else {
            super::widgets::icon_at(ui, r.center(), crate::theme::icon::ROTATE_CW, 13.0, theme::lerp_color(theme::text_faint(), theme::text(), rh));
            if resp.clicked() {
                app.refresh_friend_activity();
            }
        }
    });

    ui.add_space(6.0);

    if app.friend_activity.is_empty() {
        let err_opt = app.friend_activity_error.clone();
        if app.friend_activity_loading {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.spinner();
                ui.add_space(12.0);
                ui.label(egui::RichText::new("Loading activity…").font(theme::regular(13.0)).color(theme::text_dim()));
            });
        } else if let Some(err) = err_opt {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(egui::RichText::new("Activity could not be loaded").font(theme::semibold(14.5)).color(theme::text()));
                ui.add_space(6.0);
                ui.label(egui::RichText::new(&err).font(theme::regular(12.0)).color(theme::text_faint()));
                ui.add_space(14.0);
                if super::widgets::pill(ui, "Retry", Some(crate::theme::icon::ROTATE_CW), false).clicked() {
                    app.refresh_friend_activity();
                }
            });
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                let (icon_box, _) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::hover());
                glyphs::draw(ui, icon_box.center(), 32.0, glyphs::USERS, theme::text_faint());
                ui.add_space(14.0);
                ui.label(egui::RichText::new("No active friends").font(theme::semibold(15.0)).color(theme::text()));
                ui.add_space(6.0);
                ui.label(egui::RichText::new("None of your friends are currently listening to music, or sharing is disabled in Spotify.")
                    .font(theme::regular(12.5))
                    .color(theme::text_faint()));
                ui.add_space(14.0);
                if super::widgets::pill(ui, "Refresh", Some(crate::theme::icon::ROTATE_CW), false).clicked() {
                    app.refresh_friend_activity();
                }
            });
        }
        return;
    }

    egui::ScrollArea::vertical()
        .id_salt("np-friends-scroll")
        .auto_shrink(false)
        .show(ui, |ui| {
            let width = ui.available_width();
            let now_sec = ui.input(|i| i.time);
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            let mut clicked_track: Option<crate::transfer::spotify::Track> = None;
            let mut clicked_search: Option<String> = None;
            let mut clicked_context: Option<(String, String)> = None;

            for (idx, friend) in app.friend_activity.iter().enumerate() {
                let card_h = 72.0;
                let (rect, resp) = ui.allocate_exact_size(vec2(width, card_h), Sense::click());
                let card_id = ui.scope_id().with(("friend-card", idx));
                let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text(format!("Play \"{}\" by {}", friend.track_title(), friend.artist_name()));

                let is_hover = resp.hovered();
                let h = theme::anim_bool(ui.ctx(), card_id.with("h"), is_hover, 0.15);

                // Card background and smooth hover
                ui.painter().rect_filled(rect, 10.0, theme::glass(0.04 + 0.08 * h));
                if h > 0.0 {
                    ui.painter().rect_stroke(rect, 10.0, egui::Stroke::new(1.0, theme::glass(0.12 * h)), egui::StrokeKind::Inside);
                }

                // 1. Avatar (left, 40px circular)
                let avatar_size = 40.0;
                let avatar_rect = Rect::from_center_size(pos2(rect.left() + 26.0, rect.center().y), Vec2::splat(avatar_size));
                if let Some(url) = friend.user_avatar() {
                    super::widgets::paint_remote(ui, url, avatar_rect, avatar_size / 2.0, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                } else {
                    ui.painter().circle_filled(avatar_rect.center(), avatar_size / 2.0, theme::glass(0.18));
                    let initial = friend.user_name().chars().next().unwrap_or('?');
                    ui.painter().text(avatar_rect.center(), Align2::CENTER_CENTER, initial, theme::bold(15.0), Color32::WHITE);
                }

                let diff_ms = now_ms.saturating_sub(friend.timestamp);
                let is_active = diff_ms < 300_000;

                // Online green indicator dot on avatar
                if is_active {
                    let dot_pos = pos2(avatar_rect.right() - 3.0, avatar_rect.bottom() - 3.0);
                    ui.painter().circle_filled(dot_pos, 5.5, Color32::from_rgb(18, 18, 18));
                    ui.painter().circle_filled(dot_pos, 4.0, Color32::from_rgb(29, 185, 84));
                }

                // 2. Info layout
                let text_left = avatar_rect.right() + 10.0;
                let text_right = rect.right() - 36.0;
                let text_w = (text_right - text_left).max(10.0);

                // Row 1: Friend Name + Relative Time
                let name_y = rect.top() + 14.0;
                let time_str = format_relative_time(friend.timestamp);
                let time_g = super::widgets::galley(ui, &time_str, theme::regular(11.0), theme::text_faint(), 60.0);
                let time_w = time_g.size().x;
                ui.painter().galley(pos2(rect.right() - 10.0 - time_w, name_y), time_g, theme::text_faint());

                let friend_name_w = (text_w - time_w - 4.0).max(10.0);
                let name_g = super::widgets::galley(ui, friend.user_name(), theme::semibold(13.5), theme::text(), friend_name_w);
                ui.painter().galley(pos2(text_left, name_y), name_g, theme::text());

                // Row 2: Track Title + Live Equalizer (if active)
                let track_y = rect.top() + 32.0;
                let mut title_x = text_left;
                if is_active {
                    draw_animated_equalizer(ui, pos2(title_x + 6.0, track_y + 7.0), now_sec + idx as f64 * 0.4);
                    title_x += 18.0;
                }
                let title_w = (text_right - title_x).max(10.0);
                let track_fg = if is_hover {
                    Color32::WHITE
                } else {
                    theme::lerp_color(theme::text_dim(), theme::text(), 0.7)
                };
                let track_g = super::widgets::galley(ui, friend.track_title(), theme::semibold(13.0), track_fg, title_w);
                ui.painter().galley(pos2(title_x, track_y), track_g, track_fg);

                // Row 3: Artist Name + Context (playlist/album)
                let meta_y = rect.top() + 50.0;
                let mut meta_text = friend.artist_name().to_string();
                if let Some(ctx) = friend.context_name() {
                    meta_text.push_str(" · ");
                    meta_text.push_str(ctx);
                }
                let meta_g = super::widgets::galley(ui, &meta_text, theme::regular(11.5), theme::text_dim(), text_w);
                ui.painter().galley(pos2(text_left, meta_y), meta_g, theme::text_dim());

                // 3. Hover play button overlay on the right
                if h > 0.0 {
                    let play_center = pos2(rect.right() - 18.0, rect.center().y);
                    let play_size = 28.0;
                    ui.painter().circle_filled(play_center, play_size / 2.0, Color32::from_rgb(29, 185, 84).gamma_multiply(h));
                    glyphs::draw(ui, play_center + vec2(1.0, 0.0), 14.0, glyphs::PLAY, Color32::BLACK.gamma_multiply(h));
                }

                let sp_track = crate::transfer::spotify::Track {
                    name: friend.track_title().to_string(),
                    artists: vec![friend.artist_name().to_string()],
                    local: false,
                    art: friend.track_art().map(str::to_string),
                    duration: None,
                };

                let track_for_menu = sp_track.clone();
                let f_title = friend.track_title().to_string();
                let f_artist = friend.artist_name().to_string();
                let ctx_for_menu = friend.track.as_ref().and_then(|t| t.context.as_ref()).and_then(|c| {
                    c.uri.strip_prefix("spotify:playlist:").map(|id| (id.to_string(), c.name.clone()))
                });

                egui::Popup::context_menu(&resp).show(|ui| {
                    if ui.button(format!("♫ Play \"{}\"", f_title)).clicked() {
                        clicked_track = Some(track_for_menu.clone());
                        ui.close();
                    }
                    if let Some((pl_id, pl_name)) = &ctx_for_menu {
                        if ui.button(format!("📋 Open playlist \"{}\"", pl_name)).clicked() {
                            clicked_context = Some((pl_id.clone(), pl_name.clone()));
                            ui.close();
                        }
                    }
                    if ui.button(format!("🔍 Search \"{}\"", f_title)).clicked() {
                        clicked_search = Some(f_title.clone());
                        ui.close();
                    }
                    if ui.button(format!("👤 Search artist \"{}\"", f_artist)).clicked() {
                        clicked_search = Some(f_artist.clone());
                        ui.close();
                    }
                });

                if resp.clicked() {
                    clicked_track = Some(sp_track);
                }

                ui.add_space(4.0);
            }

            if let Some(track) = clicked_track {
                app.actions.push(Action::PlaySpotify(track));
            }
            if let Some(query) = clicked_search {
                app.actions.push(Action::Go(Page::Search(query)));
            }
            if let Some((pl_id, pl_name)) = clicked_context {
                let entry = crate::transfer::spotify::Entry {
                    source: crate::transfer::spotify::Source::Playlist(pl_id.clone()),
                    name: pl_name,
                    owner: String::new(),
                    count: None,
                    art: None,
                };
                app.page = Page::Library(LibTab::Playlists);
                app.spotify_selected = Some((entry.clone(), None, true));
                let xfer = app.transfer.clone();
                let b = app.backend.clone();
                let slot = app.spotify_loaded_playlist.clone();
                let src = entry.source.clone();
                xfer.fetch_spotify_playlist(&b, src.clone(), move |res| {
                    *slot.lock().unwrap() = Some((src, res));
                });
            }
        });
}
