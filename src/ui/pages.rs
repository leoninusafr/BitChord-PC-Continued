//! Page bodies. Every page fades and slides in when navigated to.

use std::collections::HashSet;
use std::sync::atomic::Ordering;

use egui::{Align2, Color32, Rect, RichText, Sense, Ui, Vec2, pos2, vec2};

use super::widgets::{self, RowOpts, card, chip, h_scroll, named, paint_cover, pill, shelf_header, track_row};
use super::{Action, App, LibTab, Page, Selection, glyphs};
use crate::backend::compact;
use crate::model::{Card, ShelfKind, Target, Track};
use crate::theme::{self, icon};
use crate::transfer::{Account, spotify::{Entry, Source}};

const CARD: f32 = 180.0;

/// How many tiles Apple fits across a feed row at this content width. The site
/// measures from its own viewport, which includes the navigation column this page
/// already sits past.
fn feed_columns(width: f32, square: bool) -> usize {
    let viewport = width + 260.0;
    match square {
        true if viewport < 1260.0 => 4,
        true if viewport < 1580.0 => 5,
        true => 6,
        false if viewport < 1260.0 => 2,
        false if viewport < 1580.0 => 3,
        false => 4,
    }
}

/// One tile of an Apple feed row: a share of the content on wide windows, the site's
/// own narrow width below its breakpoint.
fn feed_tile(ui: &Ui, square: bool) -> f32 {
    let width = ui.available_width();
    let min = if square { 144.0 } else { 270.0 };
    if width < 1024.0 {
        return min;
    }
    let cols = feed_columns(width, square);
    ((width - 20.0 * (cols - 1) as f32) / cols as f32).max(min)
}

pub fn page(app: &mut App, ui: &mut Ui, now: f64) {
    let t = theme::ease_out(((now - app.page_changed) / 0.32) as f32);
    if t < 1.0 {
        // The fade-in finishes even when nothing else is drawing frames.
        ui.ctx().request_repaint();
    }
    ui.set_opacity(0.15 + 0.85 * t);
    let page = app.page.clone();
    // Each page (each album, each search) keeps its own scroll position.
    // Stable scrollbar visibility prevents layout shifts and horizontal jitter when shelves load.
    let mut area = egui::ScrollArea::vertical()
        .id_salt(("page", &page))
        .auto_shrink(false)
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible);
    // Dev hook: YTM_SCROLL=<px> pins the page scroll, for screenshots of lower sections.
    if let Some(y) = super::dev_var("YTM_SCROLL").and_then(|v| v.parse::<f32>().ok()) {
        area = area.vertical_scroll_offset(y);
    }
    let out = area.show(ui, |ui| {
        ui.set_max_width((ui.available_width() - 20.0).max(0.0));
        ui.add_space(52.0);
        match &page {
            Page::Home => home(app, ui, now),
            Page::New => new(app, ui, now),
            Page::Search(q) => search(app, ui, q, now),
            Page::Album(id) | Page::Playlist(id) => collection(app, ui, id, now),
            Page::Artist(id) => artist(app, ui, id, now),
            Page::Library(tab) => library(app, ui, *tab, now),
            Page::Downloads => downloads(app, ui, now),
            Page::Settings => settings(app, ui),
            Page::Transfer => super::transfer::page(app, ui, now),
            Page::Jam => super::jam::page(app, ui),
        }
        // Extra bottom spacing so content doesn't end abruptly at the bottom edge.
        if page == Page::Settings {
            ui.add_space(16.0);
        } else {
            ui.add_space(40.0);
        }
    });
    selection_bar(app, ui.ctx(), &page, out.inner_rect);
    // Infinite scroll on home: fetch the next shelves when nearing the bottom.
    if page == Page::Home {
        let remaining = out.content_size.y - (out.state.clamped_offset().y + out.inner_rect.height());
        if let Some(h) = &mut app.home {
            // A failed fetch reports nothing (it's a quiet request): allow a retry later.
            if h.loading_more.is_some_and(|t| now - t > 20.0) {
                h.loading_more = None;
            }
            if remaining < 700.0 && h.loading_more.is_none()
                && let Some(token) = h.continuation.clone()
            {
                h.loading_more = Some(now);
                app.backend.request(crate::backend::Req::HomeMore(token));
            }
        }
    }
}

/// What every song row needs from the app, gathered once per frame.
pub(super) struct Rows {
    current: Option<String>,
    playing: bool,
    accent: Color32,
    now: f64,
}

impl Rows {
    pub(super) fn new(app: &App, now: f64) -> Self {
        Self {
            current: app.current_id().map(str::to_owned),
            playing: app.audio.status.playing.load(Ordering::Relaxed),
            accent: app.accent_color(),
            now,
        }
    }

    pub(super) fn opts(&self, liked: &HashSet<String>, t: &Track, number: Option<usize>, show_album: bool) -> RowOpts {
        RowOpts {
            number,
            show_album,
            current: self.current.as_deref() == Some(t.id.as_str()),
            playing: self.playing,
            accent: self.accent,
            time: self.now,
            liked: liked.contains(&t.id),
            added_by: None,
            pick: false,
            selected: None,
            remove: None,
            queue_remove: None,
            hide_duration_hover: false,
            drag: false,
        }
    }
}

/// A page's data hasn't arrived: shimmering placeholders while it loads, a retry if the
/// request failed.
fn placeholder(app: &mut App, ui: &mut Ui, now: f64, rows: usize, card: f32) {
    if app.load_failed {
        ui.add_space(40.0);
        ui.label(RichText::new("Couldn't load this page").color(theme::text_dim()));
        ui.add_space(12.0);
        if pill(ui, "Try again", None, false).clicked() {
            app.refresh();
        }
    } else {
        widgets::skeleton(ui, now, rows, card);
    }
}

/// What a collection header's round buttons draw: a font glyph, or the three dots.
enum CircleGlyph {
    Char(char),
    More,
}

/// A round glass action on a collection header (shuffle, search, more).
fn circle_action(ui: &mut Ui, glyph: CircleGlyph, name: &str, active: bool) -> egui::Response {
    // Same 38 px height as the Play pill beside it, so the row shares one centre line.
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(38.0), Sense::click());
    let resp = named(resp, name).on_hover_text(name).on_hover_cursor(egui::CursorIcon::PointingHand);
    let open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&resp));
    let h = theme::anim_bool(ui.ctx(), resp.id.with("h"), resp.hovered() || open, 0.15);
    let press = if resp.is_pointer_button_down_on() { 18.3 } else { 19.0 };
    ui.painter().circle_filled(rect.center(), press, theme::glass(0.10 + 0.09 * h));
    let color = theme::lerp_color(theme::text_dim(), theme::text(), h.max(active as u8 as f32));
    match glyph {
        CircleGlyph::Char(g) => widgets::icon_at(ui, rect.center(), g, 16.0, color),
        CircleGlyph::More => {
            for i in -1..=1 {
                ui.painter().circle_filled(rect.center() + vec2(i as f32 * 5.2, 0.0), 1.8, color);
            }
        }
    }
    resp
}

/// A page's main Play button. Shows Pause while this page's album / playlist / artist
/// is playing, and toggles instead of restarting when it's already loaded.
fn play_pill(ui: &mut Ui, acts: &mut Vec<Action>, target: &Target, tracks: &[Track]) {
    let active = widgets::active_state(ui, target);
    let (label, glyph) = if active == Some(true) { ("Pause", icon::PAUSE) } else { ("Play", icon::PLAY) };
    if pill(ui, label, Some(glyph), true).clicked() {
        if active.is_some() {
            acts.push(Action::TogglePlay);
        } else if !tracks.is_empty() {
            acts.push(Action::Play(tracks.to_vec(), 0));
        }
    }
}

fn card_row(ui: &mut Ui, id: &str, title: &str, cards: &[Card], acts: &mut Vec<Action>) {
    if cards.is_empty() {
        return;
    }
    let (l, r) = shelf_header(ui, title, None, None, true);
    let nudge = r as i32 - l as i32;
    let tile = feed_tile(ui, true);
    h_scroll(ui, id, nudge, cards.len(), tile, 20.0, |ui| {
        ui.spacing_mut().item_spacing.x = 20.0;
        for c in cards {
            card(ui, c, tile, false, acts);
        }
    });
}

fn card_grid(ui: &mut Ui, cards: &[Card], acts: &mut Vec<Action>) {
    let tile = feed_tile(ui, true);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(20.0, 22.0);
        for c in cards {
            card(ui, c, tile, false, acts);
        }
    });
}

/// Songs laid out in columns of `rows`, scrolling sideways (Quick picks). Clicking a
/// song starts its radio. Returns whether the arrows reached the end.
#[allow(clippy::too_many_arguments)]
fn song_grid(ui: &mut Ui, rows_ctx: &Rows, liked: &HashSet<String>, acts: &mut Vec<Action>, id: &str, title: &str, strap: Option<&str>, tracks: &[Track], rows: usize) -> bool {
    let (l, r) = shelf_header(ui, title, strap, None, true);
    let nudge = r as i32 - l as i32;
    let col_w = feed_tile(ui, false);
    let mut picked = None;
    let (_, reached_end) = h_scroll(ui, id, nudge, tracks.len().div_ceil(rows.max(1)), col_w, 20.0, |ui| {
        ui.spacing_mut().item_spacing.x = 20.0;
        for (c, col) in tracks.chunks(rows.max(1)).enumerate() {
            ui.allocate_ui(vec2(col_w, rows as f32 * 60.0), |ui| {
                ui.set_width(col_w);
                ui.vertical(|ui| {
                    for (i, t) in col.iter().enumerate() {
                        let o = rows_ctx.opts(liked, t, None, false);
                        if track_row(ui, t, &o, acts).clicked() {
                            picked = Some(c * rows + i);
                        }
                    }
                });
            });
        }
    });
    if let Some(i) = picked {
        acts.push(Action::Radio(tracks[i].clone()));
    }
    reached_end
}

/// Where a song list lives: the page its selection belongs to, and whether it's your own
/// playlist (so songs can be taken out of it).
struct ListCtx<'a> {
    sel: &'a mut Selection,
    page: &'a Page,
    own: bool,
}

/// A long song list; only the rows on screen are built. Clicking one plays `tracks` from it,
/// or picks it while selecting: Ctrl-click picks one song, Shift-click a run of them.
#[allow(clippy::too_many_arguments)]
fn list(ui: &mut Ui, rows: &Rows, liked: &HashSet<String>, tracks: &[Track], show_album: bool, number: impl Fn(usize, &Track) -> Option<usize>, cx: ListCtx, acts: &mut Vec<Action>) {
    let ListCtx { sel, page, own } = cx;
    let selecting = sel.on(page);
    let mods = ui.input(|i| i.modifiers);
    widgets::visible_rows(ui, tracks.len(), |ui, i| {
        let t = &tracks[i];
        let o = RowOpts {
            selected: selecting.then(|| sel.rows.contains(&i)),
            remove: own.then_some(i),
            ..rows.opts(liked, t, number(i, t), show_album)
        };
        if track_row(ui, t, &o, acts).clicked() {
            if selecting || mods.command || mods.shift {
                sel.pick(page, i, mods.shift);
            } else {
                acts.push(Action::Play(tracks.to_vec(), i));
            }
        }
    });
}

/// A list page's "Select" button: turns select mode on and off.
fn select_pill(ui: &mut Ui, sel: &mut Selection, page: &Page) {
    let on = sel.on(page);
    if pill(ui, if on { "Done" } else { "Select" }, Some(icon::LIST_CHECKS), false).clicked() {
        if on { sel.clear() } else { sel.start(page) }
    }
}

/// The songs a page lists, and whether they're your own playlist's.
fn selectable<'a>(app: &'a App, page: &Page) -> Option<(&'a [Track], bool)> {
    match page {
        Page::Album(id) | Page::Playlist(id) => {
            app.opened(id).map(|o| &o.c).map(|c| (c.tracks.as_slice(), app.logged_in && c.owned && !c.is_album))
        }
        Page::Library(LibTab::Songs) => app.opened(super::LIKED_MUSIC).map(|o| (o.c.tracks.as_slice(), false)),
        Page::Library(LibTab::History) => app.history.as_deref().map(|h| (h, false)),
        Page::Library(LibTab::Uploads) => app.uploads.as_deref().map(|u| (u, false)),
        _ => None,
    }
}

/// While selecting: a bar floating over the bottom of the page with what to do with the picks.
fn selection_bar(app: &mut App, ctx: &egui::Context, page: &Page, area: Rect) {
    if !app.selection.on(page) {
        return;
    }
    let Some((tracks, own)) = selectable(app, page) else { return };
    let n = tracks.len();
    let picked: Vec<Track> = app.selection.rows.iter().filter_map(|&i| tracks.get(i).cloned()).collect();
    let all = !picked.is_empty() && picked.len() == n;
    let all_liked = !picked.is_empty() && picked.iter().all(|t| app.liked.contains(&t.id));
    // Labelled buttons when there's room, icons with tooltips when the window is narrow.
    let wide = area.width() > 820.0;
    let button = |ui: &mut Ui, label: &str, glyph: char, primary: bool| {
        if wide { pill(ui, label, Some(glyph), primary) } else { widgets::icon_button(ui, glyph, 18.0, label, false) }
    };
    let mut acts = Vec::new();
    let mut toggle_all = false;
    let mut done = ctx.input(|i| i.key_pressed(egui::Key::Escape));
    egui::Area::new(egui::Id::unique("selection-bar"))
        .order(egui::Order::Foreground)
        .pivot(Align2::CENTER_BOTTOM)
        .fixed_pos(pos2(area.center().x, area.bottom() - 18.0))
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(theme::menu_fill())
                .corner_radius(14)
                .inner_margin(egui::Margin { left: 20, right: 10, top: 8, bottom: 8 })
                .shadow(egui::epaint::Shadow { offset: [0, 8], blur: 24, spread: 0, color: theme::shade(0.45) })
                .show(ui, |ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        let count = if picked.is_empty() { "Select songs".to_owned() } else { format!("{} selected", picked.len()) };
                        ui.label(RichText::new(count).font(theme::semibold(14.0)).color(theme::text()));
                        ui.add_space(10.0);
                        if !picked.is_empty() {
                            if button(ui, "Add to playlist", icon::LIST_PLUS, true).clicked() {
                                acts.push(Action::AddToPlaylist(picked.clone()));
                            }
                            if own && button(ui, "Remove from playlist", icon::LIST_MINUS, false).clicked() {
                                acts.push(Action::RemoveFromPlaylist(app.selection.rows.iter().copied().collect()));
                            }
                            if button(ui, if all_liked { "Remove like" } else { "Like" }, icon::HEART, false).clicked() {
                                acts.push(Action::LikeMany(picked.clone(), !all_liked));
                            }
                            if button(ui, "Add to queue", icon::LIST_MUSIC, false).clicked() {
                                acts.extend(picked.iter().cloned().map(Action::Enqueue));
                            }
                        }
                        toggle_all = button(ui, if all { "Select none" } else { "Select all" }, icon::LIST_CHECKS, false).clicked();
                        done |= widgets::icon_button(ui, icon::X, 18.0, "Done selecting", false).clicked();
                    });
                });
        });
    if toggle_all {
        let page = page.clone();
        app.selection.start(&page);
        if !all {
            app.selection.rows = (0..n).collect();
        }
    }
    if done || !acts.is_empty() {
        app.selection.clear();
    }
    app.actions.extend(acts);
}

// ------------------------------------------------------------------ home

fn home(app: &mut App, ui: &mut Ui, now: f64) {
    page_title(ui, "Home");
    // A refresh keeps showing the current feed; the skeleton is only for an empty one.
    if app.home.as_ref().is_none_or(|h| app.home_loading && h.shelves.is_empty()) {
        placeholder(app, ui, now, 3, CARD);
        return;
    }
    let rows = Rows::new(app, now);
    let Some(h) = &app.home else { return };
    if h.shelves.is_empty() {
        ui.add_space(40.0);
        ui.label(RichText::new("Nothing here yet").color(theme::text_dim()));
    }
    let mut more_picks = None;
    for (i, s) in h.shelves.iter().enumerate() {
        let id = format!("home-{i}-{}", s.title);
        match &s.kind {
            ShelfKind::Cards(cards) => {
                let (strap, thumb) = if s.title == "Listen again" { (None, None) } else { (s.strapline.as_deref(), s.strap_thumb.as_deref()) };
                let (l, r) = shelf_header(ui, &s.title, strap, thumb, true);
                let nudge = r as i32 - l as i32;
                let tile = feed_tile(ui, true);
                let acts = &mut app.actions;
                let liked = &app.liked;
                h_scroll(ui, &id, nudge, cards.len(), tile, 20.0, |ui| {
                    ui.spacing_mut().item_spacing.x = 20.0;
                    for c in cards {
                        let is_liked = matches!(&c.target, Target::Song(t) if liked.contains(&t.id));
                        card(ui, c, tile, is_liked, acts);
                    }
                });
            }
            ShelfKind::Songs { tracks, rows: n } => {
                let reached_end = song_grid(ui, &rows, &app.liked, &mut app.actions, &id, &s.title, s.strapline.as_deref(), tracks, *n);
                if s.title == "Quick picks" && reached_end && !h.picks_loading && !h.picks_expanded {
                    more_picks = tracks.last().map(|t| t.id.clone());
                }
            }
        }
    }
    let loading_more = h.loading_more.is_some();
    if let Some(seed) = more_picks
        && let Some(h) = &mut app.home
    {
        h.picks_loading = true;
        app.backend.request(crate::backend::Req::MoreQuickPicks(seed));
    }
    if loading_more {
        widgets::skeleton(ui, now, 1, CARD);
    }
}

// ------------------------------------------------------------------ new

struct GenreCard {
    title: &'static str,
    query: &'static str,
    color: Color32,
}

const GENRE_CARDS: &[GenreCard] = &[
    GenreCard { title: "Pop", query: "Pop", color: Color32::from_rgb(233, 30, 99) },
    GenreCard { title: "Hip-Hop", query: "Hip-Hop", color: Color32::from_rgb(255, 140, 0) },
    GenreCard { title: "Rock", query: "Rock", color: Color32::from_rgb(229, 57, 53) },
    GenreCard { title: "Indie", query: "Indie", color: Color32::from_rgb(142, 36, 170) },
    GenreCard { title: "Electronic", query: "Electronic Dance", color: Color32::from_rgb(0, 172, 193) },
    GenreCard { title: "Gaming", query: "Gaming Music", color: Color32::from_rgb(63, 81, 181) },
    GenreCard { title: "Chill & Lo-Fi", query: "Lo-Fi Chill", color: Color32::from_rgb(0, 150, 136) },
    GenreCard { title: "Workout", query: "Workout Energy", color: Color32::from_rgb(244, 81, 30) },
    GenreCard { title: "Focus & Study", query: "Focus Study Instrumental", color: Color32::from_rgb(92, 107, 192) },
    GenreCard { title: "Party", query: "Party Club Dance", color: Color32::from_rgb(123, 31, 162) },
    GenreCard { title: "R&B & Soul", query: "R&B Soul", color: Color32::from_rgb(173, 20, 87) },
    GenreCard { title: "Jazz & Blues", query: "Jazz Blues", color: Color32::from_rgb(110, 70, 50) },
];

fn new(app: &mut App, ui: &mut Ui, now: f64) {
    page_title(ui, "Discover");

    // 1. Genres & Moods Bento-Grid (Spotify / Apple Music style browsing tiles)
    let avail_w = ui.available_width();
    let cols = if avail_w < 500.0 {
        2
    } else if avail_w < 800.0 {
        3
    } else if avail_w < 1100.0 {
        4
    } else {
        6
    };
    let gap = 14.0;
    let card_w = (avail_w - gap * (cols - 1) as f32) / cols as f32;
    let card_h = 90.0;

    ui.add_space(6.0);
    ui.label(RichText::new("Genres & Moods").font(theme::bold(22.0)).color(theme::text()));
    ui.add_space(10.0);

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(gap, gap);
        for g in GENRE_CARDS {
            let (rect, resp) = ui.allocate_exact_size(vec2(card_w, card_h), Sense::click());
            let resp = named(resp, g.title).on_hover_cursor(egui::CursorIcon::PointingHand);
            let h = theme::anim_bool(ui.ctx(), resp.id.with("gh"), resp.hovered(), 0.15);
            if h > 0.0 {
                super::widgets::soft_shadow(ui, rect, 12.0, 12.0, 0.35 * h);
            }
            let fill_color = theme::lerp_color(g.color, Color32::WHITE, 0.08 * h);
            ui.painter().rect_filled(rect, 12.0, fill_color);
            ui.painter().rect_filled(
                Rect::from_min_max(pos2(rect.center().x, rect.center().y), rect.max),
                12.0,
                Color32::from_black_alpha(45),
            );
            ui.painter().rect_stroke(rect, 12.0, egui::Stroke::new(1.0, theme::glass(0.18 + 0.15 * h)), egui::StrokeKind::Inside);
            ui.painter().text(
                rect.left_top() + vec2(14.0, 14.0),
                Align2::LEFT_TOP,
                g.title,
                theme::bold(17.0),
                Color32::WHITE,
            );
            if resp.clicked() {
                app.actions.push(Action::Go(Page::Search(g.query.to_string())));
            }
        }
    });
    ui.add_space(24.0);

    // 2. Charts, New Releases & Top Songs Shelves
    if app.explore.is_none() {
        placeholder(app, ui, now, 2, CARD);
        return;
    }
    let rows = Rows::new(app, now);
    let Some(e) = &app.explore else { return };
    card_row(ui, "ex-charts", "Charts", &e.charts, &mut app.actions);
    if !e.top.is_empty() {
        song_grid(ui, &rows, &app.liked, &mut app.actions, "ex-top", "Top songs", None, &e.top, 4);
    }
    card_row(ui, "ex-new", if app.logged_in { "New releases for you" } else { "Popular new releases" }, &e.new_albums, &mut app.actions);
}

pub(super) fn page_title(ui: &mut Ui, title: &str) {
    // Left-aligned with the page's content, a little below the top edge.
    ui.add_space(18.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 42.0), Sense::hover());
    ui.painter().text(pos2(rect.left(), rect.center().y), Align2::LEFT_CENTER, title, theme::bold(34.0), theme::text());
}

// ------------------------------------------------------------------ search

fn search(app: &mut App, ui: &mut Ui, raw_q: &str, now: f64) {
    let pick = super::transfer::pick_banner(app, ui);
    page_title(ui, "Search");
    ui.add_space(8.0);
    let q = raw_q.trim();
    // Coming from a suggestion, the sidebar or a link, the field says what's shown.
    if app.search_synced != q {
        app.search_synced = q.to_owned();
        app.search_text = q.to_owned();
    }

    let filter_id = egui::Id::unique("search-category-filter");
    let mut filter = ui.data(|d| d.get_temp::<u8>(filter_id).unwrap_or(0));

    // Category filter pills: All, Songs, Playlists, Artists, Albums, Spotify
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let tabs = [(0, "All"), (1, "Songs"), (2, "Playlists"), (3, "Artists"), (4, "Albums"), (5, "Spotify")];
        for (idx, label) in tabs {
            if widgets::chip(ui, label, filter == idx).clicked() {
                filter = idx;
                ui.data_mut(|d| d.insert_temp(filter_id, filter));
            }
        }
    });
    ui.add_space(14.0);

    if q.is_empty() {
        ui.add_space(10.0);
        recent_searches(ui, app);
        return;
    }

    let paint_spotify_track_row = |ui: &mut Ui, t: &crate::transfer::spotify::Track, id_salt: &str, acts: &mut Vec<Action>| {
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 52.0), Sense::click());
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        let h = theme::anim_bool(ui.ctx(), resp.id.with(id_salt), resp.hovered(), 0.12);
        if h > 0.0 {
            ui.painter().rect_filled(rect, 8.0, theme::glass(0.06 * h));
        }
        let cy = rect.center().y;
        let thumb_r = Rect::from_center_size(pos2(rect.left() + 28.0, cy), Vec2::splat(36.0));
        paint_cover(ui, thumb_r, &super::transfer::spotify_thumbs(&t.art), 6.0, 1.0);

        if h > 0.0 {
            ui.painter().rect_filled(thumb_r, 6.0, Color32::from_black_alpha((180.0 * h) as u8));
            widgets::icon_at(ui, thumb_r.center(), icon::PLAY, 14.0, Color32::WHITE);
        }

        let tx = thumb_r.right() + 14.0;
        let max_text_w = (rect.right() - tx - 110.0).max(80.0);
        let title_color = if h > 0.0 { super::transfer::SPOTIFY } else { theme::text() };
        widgets::text_at(ui, pos2(tx, cy - 17.0), &t.name, theme::semibold(14.5), title_color, max_text_w);
        widgets::text_at(ui, pos2(tx, cy + 2.0), &t.artist_line(), theme::regular(12.0), theme::text_dim(), max_text_w);

        let badge_r = Rect::from_center_size(pos2(rect.right() - 74.0, cy), Vec2::splat(16.0));
        super::transfer::badge(ui, badge_r);

        if let Some(dur) = t.duration {
            let dur_str = format!("{}:{:02}", dur / 60, dur % 60);
            ui.painter().text(pos2(rect.right() - 18.0, cy), egui::Align2::RIGHT_CENTER, dur_str, theme::regular(12.5), theme::text_dim());
        }

        if resp.clicked() {
            acts.push(Action::PlaySpotify(t.clone()));
        }
    };

    let sp_data = app.spotify_search.as_ref().filter(|(sq, _, _)| sq.trim() == q);

    // "Spotify" tab: show pure Spotify catalog songs and playlists
    if filter == 5 {
        let is_loading = app.spotify_search_loading;

        ui.horizontal(|ui| {
            let badge_r = Rect::from_center_size(pos2(ui.cursor().left() + 10.0, ui.cursor().top() + 12.0), Vec2::splat(22.0));
            super::transfer::badge(ui, badge_r);
            ui.add_space(26.0);
            ui.label(RichText::new("Spotify Catalog").font(theme::bold(22.0)).color(theme::text()));
            if is_loading {
                ui.add_space(12.0);
                ui.label(RichText::new("Searching Spotify…").font(theme::regular(14.0)).color(theme::text_dim()));
            }
        });
        ui.add_space(12.0);

        if let Some((_, sp_tracks, sp_pls)) = sp_data {
            if sp_tracks.is_empty() && sp_pls.is_empty() && !is_loading {
                ui.add_space(14.0);
                ui.label(RichText::new("No Spotify results found").font(theme::regular(16.0)).color(theme::text_dim()));
                return;
            }

            if !sp_tracks.is_empty() {
                ui.label(RichText::new("Tracks").font(theme::bold(18.0)).color(super::transfer::SPOTIFY));
                ui.add_space(8.0);
                for t in sp_tracks {
                    paint_spotify_track_row(ui, t, "sp_sh", &mut app.actions);
                }
            }

            if !sp_pls.is_empty() {
                ui.add_space(20.0);
                ui.label(RichText::new("Playlists").font(theme::bold(18.0)).color(super::transfer::SPOTIFY));
                ui.add_space(10.0);
                let card_w = 160.0;
                let count_per_row = ((ui.available_width() + 16.0) / (card_w + 16.0)).floor().max(1.0) as usize;
                for chunk in sp_pls.chunks(count_per_row) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = vec2(16.0, 16.0);
                        for pl in chunk {
                            let (cr, cresp) = ui.allocate_exact_size(vec2(card_w, 220.0), Sense::click());
                            let ch = theme::anim_bool(ui.ctx(), cresp.id.with("sp_pl_h"), cresp.hovered(), 0.12);
                            ui.painter().rect_filled(cr, 10.0, theme::glass(0.04 + 0.05 * ch));
                            ui.painter().rect_stroke(cr, 10.0, egui::Stroke::new(1.0, theme::glass(0.08 + 0.08 * ch)), egui::StrokeKind::Inside);
                            let c_inner = cr.shrink(10.0);
                            let art_r = Rect::from_min_size(c_inner.min, Vec2::splat(c_inner.width()));
                            paint_cover(ui, art_r, &super::transfer::spotify_thumbs(&pl.art), 8.0, 1.0);
                            if ch > 0.0 {
                                let play_circle = Rect::from_center_size(pos2(art_r.right() - 18.0, art_r.bottom() - 18.0), Vec2::splat(30.0));
                                ui.painter().circle_filled(play_circle.center(), 15.0, super::transfer::SPOTIFY);
                                widgets::icon_at(ui, play_circle.center(), icon::PLAY, 13.0, Color32::from_rgb(11, 11, 11));
                            }
                            let t_y = art_r.bottom() + 10.0;
                            let name_g = widgets::galley(ui, &pl.name, theme::semibold(13.0), theme::text(), c_inner.width());
                            ui.painter().galley(pos2(c_inner.left(), t_y), name_g, Color32::WHITE);
                            let sub = pl.count.map_or_else(|| pl.owner.clone(), |c| format!("{c} Songs"));
                            let sub_g = widgets::galley(ui, &sub, theme::regular(11.5), theme::text_dim(), c_inner.width());
                            ui.painter().galley(pos2(c_inner.left(), t_y + 18.0), sub_g, Color32::WHITE);
                            if cresp.clicked() {
                                app.actions.push(Action::OpenSpotifyPlaylist(pl.clone()));
                            }
                        }
                    });
                    ui.add_space(10.0);
                }
            }
        } else if is_loading {
            ui.add_space(10.0);
            widgets::skeleton_rows(ui, now, 5);
        } else {
            ui.label(RichText::new("No Spotify results yet.").font(theme::regular(14.0)).color(theme::text_dim()));
        }
        return;
    }

    let ytm_ready = app.search.as_ref().filter(|r| r.query.trim() == q);
    if ytm_ready.is_none() && sp_data.is_none() {
        ui.add_space(14.0);
        placeholder(app, ui, now, 2, CARD);
        return;
    }

    let rows = Rows::new(app, now);
    let empty_tracks: Vec<Track> = Vec::new();
    let empty_cards: Vec<Card> = Vec::new();
    let yt_tracks = ytm_ready.map_or(&empty_tracks[..], |r| &r.tracks[..]);
    let yt_artists = ytm_ready.map_or(&empty_cards[..], |r| &r.artists[..]);
    let yt_albums = ytm_ready.map_or(&empty_cards[..], |r| &r.albums[..]);
    let yt_playlists = ytm_ready.map_or(&empty_cards[..], |r| &r.playlists[..]);

    let empty_sp_tracks: Vec<crate::transfer::spotify::Track> = Vec::new();
    let empty_sp_pls: Vec<crate::transfer::spotify::Entry> = Vec::new();
    let sp_tracks = sp_data.map_or(&empty_sp_tracks[..], |(_, tr, _)| &tr[..]);
    let sp_pls = sp_data.map_or(&empty_sp_pls[..], |(_, _, pl)| &pl[..]);

    let clean_title = |s: &str| s.split(['(', '[']).next().unwrap_or(s).split(" - ").next().unwrap_or(s).trim().to_lowercase();
    let matches_yt = |yt: &Track, st: &crate::transfer::spotify::Track| {
        let st_name = clean_title(&st.name);
        let yt_name = clean_title(&yt.title);
        let st_artist = st.artists.first().map(|a| a.to_lowercase()).unwrap_or_default();
        !st_name.is_empty() && st_name == yt_name && !st_artist.is_empty() && yt.artist_line().to_lowercase().contains(&st_artist)
    };

    #[derive(Clone)]
    enum UnifiedSong<'a> {
        Ytm(usize, &'a Track),
        Spotify(&'a crate::transfer::spotify::Track),
    }

    // Build ONE unified combined song list:
    // Merge YouTube Music + Spotify hits without duplicates, then rank by query relevance
    // (handling joined words like "newjack" -> "New Jack", German "von", and artist+title matches)
    // while keeping original catalog rank for lyrics or broad queries.
    let norm_flat = |s: &str| -> String {
        s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
    };
    let q_tokens: Vec<String> = q
        .split_whitespace()
        .filter(|w| !w.eq_ignore_ascii_case("von") && !w.eq_ignore_ascii_case("by"))
        .map(|w| norm_flat(w))
        .filter(|w| !w.is_empty())
        .collect();
    let q_clean_flat: String = q_tokens.join("");
    let relevance = |title: &str, artist_line: &str, base_idx: usize, dual_hit: bool| -> (i32, usize) {
        let t_low = title.to_lowercase();
        let a_low = artist_line.to_lowercase();
        let t_flat = norm_flat(title);
        let a_flat = norm_flat(artist_line);
        let combined_flat = format!("{t_flat}{a_flat}");
        let mut s = if dual_hit { 6 } else { 0 };
        if !q_clean_flat.is_empty() {
            if t_flat == q_clean_flat {
                s += 30;
            } else if t_flat.contains(&q_clean_flat) {
                s += 18;
            } else if combined_flat.contains(&q_clean_flat) {
                s += 15;
            }
        }
        let mut title_hits = 0i32;
        let mut artist_hits = 0i32;
        for tok in &q_tokens {
            let in_t = t_flat.contains(tok) || t_low.contains(tok);
            let in_a = a_flat.contains(tok) || a_low.contains(tok);
            if in_t {
                title_hits += 1;
                s += 6;
            }
            if in_a {
                artist_hits += 1;
                s += 8;
            }
        }
        if title_hits > 0 && artist_hits > 0 {
            s += 24;
        }
        if !q_tokens.is_empty() && (title_hits + artist_hits) as usize >= q_tokens.len() {
            s += 14;
        }
        // Subtle boost for top Spotify hits on lyric/fuzzy queries when YTM didn't match well
        (s, base_idx)
    };

    let mut used_ytm = vec![false; yt_tracks.len()];
    let mut ranked: Vec<((i32, usize), UnifiedSong<'_>)> = Vec::new();
    let mut used_sp = vec![false; sp_tracks.len()];

    if !pick {
        for (si, st) in sp_tracks.iter().enumerate() {
            let a_line = st.artist_line();
            if let Some(yi) = yt_tracks.iter().position(|yt| matches_yt(yt, st)) {
                if !used_ytm[yi] {
                    used_ytm[yi] = true;
                    let rel = relevance(&yt_tracks[yi].title, &yt_tracks[yi].artist_line(), yi.min(si), true);
                    ranked.push((rel, UnifiedSong::Ytm(yi, &yt_tracks[yi])));
                }
            } else {
                let (mut s, idx) = relevance(&st.name, &a_line, si, false);
                if si < 5 {
                    s += (5 - si as i32) * 2;
                }
                ranked.push(((s, idx + 2), UnifiedSong::Spotify(st)));
            }
            used_sp[si] = true;
        }
    }
    for (yi, yt) in yt_tracks.iter().enumerate() {
        if !used_ytm[yi] {
            let rel = relevance(&yt.title, &yt.artist_line(), yi, false);
            ranked.push((rel, UnifiedSong::Ytm(yi, yt)));
        }
    }
    ranked.sort_by(|(a_sc, a_song), (b_sc, b_song)| {
        b_sc.0.cmp(&a_sc.0).then_with(|| a_sc.1.cmp(&b_sc.1)).then_with(|| match (a_song, b_song) {
            (UnifiedSong::Ytm(..), UnifiedSong::Spotify(..)) => std::cmp::Ordering::Less,
            (UnifiedSong::Spotify(..), UnifiedSong::Ytm(..)) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        })
    });
    let combined_songs: Vec<UnifiedSong<'_>> = ranked.into_iter().map(|(_, s)| s).collect();

    // Combine playlists from YTM and Spotify cleanly (deduplicated by lowercase name)
    let mut combined_playlists: Vec<Card> = yt_playlists.to_vec();
    for sp in sp_pls {
        if !combined_playlists.iter().any(|p| p.title.eq_ignore_ascii_case(&sp.name)) {
            combined_playlists.push(Card {
                title: sp.name.clone(),
                subtitle: if sp.owner.is_empty() { "Spotify Playlist".to_string() } else { format!("Spotify • {}", sp.owner) },
                thumbs: super::transfer::spotify_thumbs(&sp.art),
                target: Target::Playlist(format!("sp:{}", sp.source.id())),
                round: false,
                links: vec![crate::model::Link { name: sp.owner.clone(), id: None }],
            });
        }
    }

    let acts = &mut app.actions;
    let liked = &app.liked;

    if combined_songs.is_empty() && yt_artists.is_empty() && yt_albums.is_empty() && combined_playlists.is_empty() {
        ui.add_space(18.0);
        ui.label(RichText::new("No results found").font(theme::bold(20.0)).color(theme::text()));
        return;
    }

    let paint_unified_song = |ui: &mut Ui, song: &UnifiedSong<'_>, id_salt: &str, show_album: bool, acts: &mut Vec<Action>| {
        match song {
            UnifiedSong::Ytm(idx, t) => {
                let o = RowOpts { pick, ..rows.opts(liked, t, None, show_album) };
                if track_row(ui, t, &o, acts).clicked() {
                    acts.push(if pick { Action::TransferPick((*t).clone()) } else { Action::Play(yt_tracks.to_vec(), *idx) });
                }
            }
            UnifiedSong::Spotify(st) => {
                paint_spotify_track_row(ui, st, id_salt, acts);
            }
        }
    };

    match filter {
        1 => {
            // "Songs" tab: unified YouTube Music + Spotify tracks
            ui.horizontal(|ui| {
                ui.label(RichText::new("Songs").font(theme::bold(22.0)).color(theme::text()));
                if app.spotify_search_loading {
                    ui.add_space(10.0);
                    ui.label(RichText::new("Syncing Spotify…").font(theme::regular(13.0)).color(theme::text_dim()));
                }
            });
            ui.add_space(8.0);
            for song in &combined_songs {
                paint_unified_song(ui, song, "u_songs", false, acts);
            }
        }
        2 => {
            // "Playlists" tab: combined YouTube Music + Spotify playlists
            ui.label(RichText::new("Playlists").font(theme::bold(22.0)).color(theme::text()));
            ui.add_space(10.0);
            card_grid(ui, &combined_playlists, acts);
        }
        3 => {
            // "Artists" tab
            ui.label(RichText::new("Artists").font(theme::bold(22.0)).color(theme::text()));
            ui.add_space(10.0);
            card_grid(ui, yt_artists, acts);
        }
        4 => {
            // "Albums" tab
            ui.label(RichText::new("Albums").font(theme::bold(22.0)).color(theme::text()));
            ui.add_space(10.0);
            card_grid(ui, yt_albums, acts);
        }
        _ => {
            // "All" tab: Unified Top Result + Top 4 Unified Songs, then Artists, Albums, More songs, Playlists
            ui.horizontal_top(|ui| {
                if let Some(top) = yt_artists.first().filter(|a| a.title.eq_ignore_ascii_case(q)) {
                    top_result(ui, top, false, acts);
                } else if let Some(first_song) = combined_songs.first() {
                    let c = match first_song {
                        UnifiedSong::Ytm(_, t) => Card {
                            title: t.title.clone(),
                            subtitle: format!("Song • {}", t.artist_line()),
                            thumbs: t.thumbs.clone(),
                            target: Target::Song((*t).clone()),
                            round: false,
                            links: t.artists.clone(),
                        },
                        UnifiedSong::Spotify(st) => {
                            let synth = Track {
                                id: format!("sp:top:{}", st.name),
                                title: st.name.clone(),
                                artists: st.artists.iter().map(|a| crate::model::Link { name: a.clone(), id: None }).collect(),
                                album: None,
                                duration: st.duration,
                                thumbs: super::transfer::spotify_thumbs(&st.art),
                                track_nr: None,
                                plays: None,
                                music_video: false,
                            };
                            Card {
                                title: st.name.clone(),
                                subtitle: format!("Song • {}", st.artist_line()),
                                thumbs: super::transfer::spotify_thumbs(&st.art),
                                target: Target::Song(synth),
                                round: false,
                                links: st.artists.iter().map(|a| crate::model::Link { name: a.clone(), id: None }).collect(),
                            }
                        }
                    };
                    top_result(ui, &c, pick, acts);
                }
                ui.add_space(24.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Songs").font(theme::bold(24.0)));
                        if app.spotify_search_loading {
                            ui.add_space(10.0);
                            ui.label(RichText::new("Syncing Spotify…").font(theme::regular(12.5)).color(theme::text_dim()));
                        }
                    });
                    ui.add_space(8.0);
                    for song in combined_songs.iter().take(4) {
                        paint_unified_song(ui, song, "u_top4", false, acts);
                    }
                });
            });
            card_row(ui, "s-artists", "Artists", yt_artists, acts);
            card_row(ui, "s-albums", "Albums", yt_albums, acts);
            if combined_songs.len() > 4 {
                ui.add_space(26.0);
                ui.label(RichText::new("More songs").font(theme::bold(24.0)));
                ui.add_space(10.0);
                for song in combined_songs.iter().skip(4) {
                    paint_unified_song(ui, song, "u_more", true, acts);
                }
            }
            card_row(ui, "s-pl", "Playlists", &combined_playlists, acts);
        }
    }
}

/// Spotify / Apple style search landing: recent searches + quick category tags.
fn recent_searches(ui: &mut Ui, app: &mut App) {
    let mut clear_all = false;
    if !app.recent_searches.is_empty() {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Recently Searched").font(theme::bold(22.0)).color(theme::text()));
            let avail_w = ui.available_width().min(540.0);
            ui.add_space((avail_w - 90.0).max(10.0));
            if widgets::chip(ui, "Clear all", false).clicked() {
                clear_all = true;
            }
        });
        ui.add_space(10.0);
        let w = ui.available_width().min(620.0);
        let recents = app.recent_searches.clone();
        let mut remove = None;
        for (i, r) in recents.iter().enumerate() {
            let (rect, resp) = ui.allocate_exact_size(vec2(w, 64.0), Sense::click());
            let resp = named(resp, &r.title).on_hover_cursor(egui::CursorIcon::PointingHand);
            let h = theme::anim_bool(ui.ctx(), resp.id.with("h"), resp.contains_pointer(), 0.12);
            ui.painter().rect_filled(rect, 10.0, theme::glass(0.05 + 0.06 * h));
            let art = Rect::from_min_size(rect.left_top() + vec2(8.0, 8.0), Vec2::splat(48.0));
            let radius = if r.round { 24.0 } else { 6.0 };
            match r.thumb.as_deref().filter(|u| !u.is_empty()) {
                Some(url) => {
                    let thumbs = [crate::model::Thumb { url: url.to_owned(), width: 300, height: 300 }];
                    paint_cover(ui, art, &thumbs, radius, 1.0);
                }
                None => {
                    ui.painter().rect_filled(art, radius, theme::glass(0.1));
                }
            }
            let x = art.right() + 14.0;
            widgets::text_at(ui, pos2(x, rect.center().y - 17.0), &r.title, theme::semibold(15.0), theme::text(), rect.right() - x - 12.0);
            widgets::text_at(ui, pos2(x, rect.center().y + 4.0), &r.subtitle, theme::regular(13.0), theme::text_dim(), rect.right() - x - 12.0);
            egui::Popup::context_menu(&resp).show(|ui| {
                ui.set_min_width(200.0);
                if ui.button("Remove from Recently Searched").clicked() {
                    remove = Some(i);
                    ui.close();
                }
            });
            if resp.clicked() {
                app.actions.push(match &r.target {
                    Some(t) => Action::Open(t.clone()),
                    None => Action::Go(Page::Search(r.query.clone())),
                });
            }
            ui.add_space(6.0);
        }
        if let Some(i) = remove {
            app.recent_searches.remove(i);
        }
        if clear_all {
            app.recent_searches.clear();
        }
        ui.add_space(20.0);
    }

    // Browse Categories (Spotify style discover tiles)
    ui.label(RichText::new("Browse Categories").font(theme::bold(22.0)).color(theme::text()));
    ui.add_space(12.0);

    let categories: &[(&str, Color32, Color32)] = &[
        ("Pop", Color32::from_rgb(141, 103, 171), Color32::from_rgb(90, 48, 128)),
        ("Hip-Hop", Color32::from_rgb(186, 93, 7), Color32::from_rgb(140, 50, 5)),
        ("Rock", Color32::from_rgb(224, 53, 53), Color32::from_rgb(150, 20, 20)),
        ("Indie", Color32::from_rgb(230, 158, 48), Color32::from_rgb(180, 100, 20)),
        ("Electronic", Color32::from_rgb(30, 150, 180), Color32::from_rgb(15, 90, 120)),
        ("Gaming", Color32::from_rgb(80, 55, 175), Color32::from_rgb(45, 25, 120)),
        ("Chill & Lo-Fi", Color32::from_rgb(70, 120, 140), Color32::from_rgb(35, 75, 90)),
        ("Workout", Color32::from_rgb(119, 161, 58), Color32::from_rgb(70, 110, 30)),
    ];

    let avail_width = ui.available_width().max(300.0);
    let cols = if avail_width > 900.0 { 4 } else if avail_width > 560.0 { 3 } else { 2 };
    let gap = 12.0;
    let card_width = ((avail_width - (cols - 1) as f32 * gap) / cols as f32).max(120.0);
    let card_height = 76.0;

    egui::Grid::new(egui::Id::unique("search-categories-grid"))
        .spacing(vec2(gap, gap))
        .show(ui, |ui| {
            for (idx, (cat_name, color_top, color_bottom)) in categories.iter().enumerate() {
                let (rect, resp) = ui.allocate_exact_size(vec2(card_width, card_height), Sense::click());
                let resp = named(resp, cat_name).on_hover_cursor(egui::CursorIcon::PointingHand);
                let h = theme::anim_bool(ui.ctx(), resp.id.with("h"), resp.hovered(), 0.15);

                let top_c = theme::lerp_color(*color_top, Color32::WHITE, h * 0.15);
                let btm_c = theme::lerp_color(*color_bottom, Color32::BLACK, 0.1);

                widgets::soft_shadow(ui, rect, 10.0, 10.0 + 8.0 * h, 0.35 + 0.15 * h);
                ui.painter().rect_filled(rect, 10.0, top_c);

                // Subtle dark bottom gradient
                let bottom_shade = Rect::from_min_max(pos2(rect.left(), rect.center().y), rect.right_bottom());
                ui.painter().rect_filled(bottom_shade, 10.0, theme::with_alpha(btm_c, 0.5));

                ui.painter().text(
                    rect.left_top() + vec2(14.0, 14.0),
                    Align2::LEFT_TOP,
                    *cat_name,
                    theme::bold(17.0),
                    Color32::WHITE,
                );

                if resp.clicked() {
                    app.actions.push(Action::Go(Page::Search(cat_name.to_string())));
                }

                if (idx + 1) % cols == 0 {
                    ui.end_row();
                }
            }
        });
}

/// `pick`: choosing a transfer match, so clicking a song picks it.
fn top_result(ui: &mut Ui, c: &Card, pick: bool, acts: &mut Vec<Action>) {
    ui.vertical(|ui| {
        ui.label(RichText::new("Top result").font(theme::bold(24.0)));
        ui.add_space(8.0);
        let card_w = (ui.available_width() * 0.42).clamp(260.0, 400.0);
        let (rect, resp) = ui.allocate_exact_size(vec2(card_w, 236.0), Sense::click());
        let resp = named(resp, &c.title).on_hover_cursor(egui::CursorIcon::PointingHand);
        let h = theme::anim_bool(ui.ctx(), resp.id.with("h"), resp.contains_pointer(), 0.2);
        ui.painter().rect_filled(rect, 14.0, theme::glass(0.07 + 0.05 * h));
        let img = Rect::from_min_size(rect.min + vec2(20.0, 20.0), Vec2::splat(110.0));
        paint_cover(ui, img, &c.thumbs, if c.round { 55.0 } else { theme::card_radius() }, 1.0);
        let x = rect.left() + 20.0;
        widgets::text_at(ui, pos2(x, img.bottom() + 16.0), &c.title, theme::bold(26.0), theme::text(), rect.width() - 40.0);
        widgets::linked_text(ui, pos2(x, img.bottom() + 52.0), &c.subtitle, &c.links, theme::regular(14.0), theme::text_dim(), rect.width() - 40.0, acts);
        // Floating play button
        let pc = rect.right_bottom() + vec2(-44.0, -44.0 + 6.0 * (1.0 - h));
        let pr = Rect::from_center_size(pc, Vec2::splat(52.0));
        let active = widgets::active_state(ui, &c.target);
        let playing = active == Some(true);
        let label = format!("{} {}", if playing { "Pause" } else { "Play" }, c.title);
        let p = named(ui.interact(pr, resp.id.with("play"), Sense::click()), &label).on_hover_cursor(egui::CursorIcon::PointingHand);
        ui.painter().circle_filled(pc, 26.0, theme::with_alpha(Color32::WHITE, 0.4 + 0.6 * h));
        let nudge = if playing { 0.0 } else { 1.5 };
        widgets::play_pause_glyph(ui, pc + vec2(nudge, 0.0), 10.0, Color32::from_gray(15), playing);
        if p.clicked() {
            acts.push(match &c.target {
                _ if active.is_some() => Action::TogglePlay,
                Target::Song(t) => Action::Radio(t.clone()),
                t => Action::PlayCollection(t.clone(), false),
            });
        } else if resp.clicked() {
            acts.push(match &c.target {
                Target::Song(t) if pick => Action::TransferPick(t.clone()),
                t => Action::Open(t.clone()),
            });
        }
    });
}

// ------------------------------------------------------------------ album / playlist

fn collection(app: &mut App, ui: &mut Ui, id: &str, now: f64) {
    if app.opened(id).is_none() {
        placeholder(app, ui, now, 1, 232.0);
        return;
    }
    let rows = Rows::new(app, now);
    // The header's motion sleeve, asked for before the release is borrowed below.
    let (release, artist) = match app.opened(id) {
        Some(o) => (o.c.title.clone(), o.c.artists.first().map(|a| a.name.clone()).unwrap_or_else(|| o.c.byline())),
        None => (String::new(), String::new()),
    };
    app.apple_cover(&release, &artist, None);
    let px = (232.0 * ui.ctx().pixels_per_point()).round().max(256.0) as u32;
    let sprite = app.apple_sprite(ui.ctx(), &release, &artist, px);
    let Some(o) = app.lists.get(id) else { return };
    let (c, more) = (&o.c, app.receiving(id));
    let acts = &mut app.actions;
    let tracks = &c.tracks;
    let is_album = c.is_album;
    let filter_on = app.coll_search.as_ref().is_some_and(|(fid, _)| fid == id);
    let selecting = app.selection.on(&app.page);
    let mut toggle_search = false;
    let mut start_select = false;
    let mut stop_select = false;
    ui.add_space(18.0);
    ui.horizontal_top(|ui| {
        let (img, _) = ui.allocate_exact_size(Vec2::splat(232.0), Sense::hover());
        widgets::soft_shadow(ui, img, 10.0, 40.0, 0.5);
        let is_liked = id == "LM" || id == super::lists::LIKED_MUSIC || c.title.eq_ignore_ascii_case("Liked Songs") || c.title.eq_ignore_ascii_case("Liked Music") || c.title.eq_ignore_ascii_case("Lieblingssongs");
        if is_liked {
            widgets::paint_liked_cover(ui, img, theme::card_radius() + 2.0);
        } else {
            paint_cover(ui, img, &c.thumbs, theme::card_radius() + 2.0, 1.0);
        }
        if let Some(s) = &sprite {
            widgets::paint_motion(ui, img, theme::card_radius() + 2.0, s);
        }
        ui.add_space(26.0);
        ui.vertical(|ui| {
            ui.set_max_width(ui.available_width().min(760.0));
            ui.label(RichText::new(if is_album { "ALBUM" } else { "PLAYLIST" }).font(theme::semibold(12.0)).color(theme::text_dim()));
            ui.label(RichText::new(&c.title).font(theme::bold(38.0)).color(theme::text()));
            ui.add_space(2.0);
            let owner_line = if !c.subtitle.is_empty() {
                c.subtitle.as_str()
            } else if let Some(a) = c.artists.first() {
                a.name.as_str()
            } else {
                ""
            };
            if !owner_line.is_empty() {
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
                widgets::linked_text(ui, r.min, owner_line, &c.artists, theme::regular(15.0), theme::text_dim(), r.width(), acts);
            }
            let total: u32 = tracks.iter().filter_map(|t| t.duration).sum();
            let count = tracks.len();
            if count > 0 {
                let time_str = if total >= 3600 {
                    format!("{} hr {} min", total / 3600, (total % 3600) / 60)
                } else {
                    format!("{} min", total / 60)
                };
                let s_suffix = if count == 1 { "song" } else { "songs" };
                let sync_excluded = !is_album && app.spotify_excluded_playlists.contains(id);
                let sync_tag = if sync_excluded { " • Spotify: Excluded from sync" } else { "" };
                ui.label(
                    RichText::new(format!("{count} {s_suffix} • {time_str}{sync_tag}")).font(theme::regular(13.5)).color(theme::text_faint()),
                );
            }
            if let Some(d) = c.description.as_deref().filter(|d| !d.trim().is_empty()) {
                ui.add_space(4.0);
                let g = widgets::galley_wrapped(ui, d.trim(), theme::regular(13.5), theme::text_dim(), ui.available_width(), 2);
                ui.label(g);
            }
            ui.add_space(14.0);
            let target = if is_album { Target::Album(id.to_owned()) } else { Target::Playlist(id.to_owned()) };
            // Liked Music and Episodes for later are the account's own lists: they can't be
            // saved, removed, edited or deleted.
            let auto = matches!(id, "LM" | "SE");
            let saved = app.library.as_ref().is_some_and(|l| {
                let cards = if is_album { &l.albums } else { &l.playlists };
                cards.iter().any(|card| card.target == target)
            });
            // Saving only applies to someone else's playlist or an album.
            let save_id = if is_album { c.save_id.clone() } else { (!c.owned).then(|| id.to_owned()) };
            let owned = !is_album && c.owned;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(10.0, 0.0);
                play_pill(ui, acts, &target, tracks);
                if circle_action(ui, CircleGlyph::Char(icon::SHUFFLE), "Shuffle", false).clicked() && !tracks.is_empty() {
                    let mut t = tracks.clone();
                    super::shuffle(&mut t);
                    acts.push(Action::Play(t, 0));
                }
                if circle_action(ui, CircleGlyph::Char(icon::SEARCH), "Search in this list", filter_on).clicked() {
                    toggle_search = true;
                }
                let more = circle_action(ui, CircleGlyph::More, "More", false);
                egui::Popup::menu(&more).show(|ui| {
                    ui.set_width(226.0);
                    ui.add_space(3.0);
                    if let Some(first) = tracks.first()
                        && widgets::menu_action(ui, Some(icon::RADIO), "Start radio")
                    {
                        acts.push(Action::Radio(first.clone()));
                    }
                    if let Some(save_id) = save_id.clone()
                        && widgets::menu_action(ui, Some(icon::LIBRARY), if saved { "Remove from library" } else { "Save to library" })
                    {
                        acts.push(Action::SaveCollection(save_id, !saved));
                    }
                    if app.logged_in && !auto && !tracks.is_empty() {
                        if widgets::menu_action(ui, Some(icon::LIST_CHECKS), if selecting { "Done selecting" } else { "Select" }) {
                            if selecting { stop_select = true } else { start_select = true }
                        }
                    }
                    if app.logged_in && !auto && (save_id.is_some() || owned) {
                        ui.separator();
                    }
                    if owned {
                        if widgets::menu_action(ui, Some(icon::MUSIC), "Edit playlist") {
                            acts.push(Action::EditPlaylist);
                        }
                        if widgets::menu_action(ui, Some(icon::USERS), "Collaborate") {
                            acts.push(Action::Collaborate);
                        }
                        if widgets::menu_action(ui, Some(icon::TRASH_2), "Delete playlist") {
                            acts.push(Action::DeletePlaylist);
                        }
                    }
                    if app.logged_in && !auto && !owned && save_id.is_none() {
                        ui.separator();
                    }
                    if widgets::menu_action(ui, Some(icon::LINK), "Share") {
                        acts.push(Action::Share(target.clone()));
                    }
                    if !is_album && !auto {
                        ui.separator();
                        let is_excluded = app.spotify_excluded_playlists.contains(id);
                        let label = if is_excluded {
                            "Include in Spotify Sync"
                        } else {
                            "Exclude from Spotify Sync"
                        };
                        let sym = if is_excluded { icon::CHECK } else { icon::X };
                        if widgets::menu_action(ui, Some(sym), label) {
                            if is_excluded {
                                app.spotify_excluded_playlists.remove(id);
                            } else {
                                app.spotify_excluded_playlists.insert(id.to_string());
                            }
                        }
                    }
                    ui.add_space(3.0);
                });
            });
            if toggle_search {
                let closing = app.coll_search.as_ref().is_some_and(|(fid, _)| fid == id);
                app.coll_search = (!closing).then(|| (id.to_owned(), String::new()));
            }
            if start_select {
                app.selection.start(&app.page);
            }
            if stop_select {
                app.selection.clear();
            }

        });
    });
    let mut dismiss_search = false;
    if let Some((fid, query)) = &mut app.coll_search
        && fid == id
    {
        ui.add_space(14.0);
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width().min(420.0), 40.0), Sense::hover());
        ui.painter().rect_filled(rect, 20.0, theme::glass(0.09));
        widgets::icon_at(ui, pos2(rect.left() + 20.0, rect.center().y), icon::SEARCH, 16.0, theme::text_dim());
        let clearable = !query.is_empty();
        let inner = Rect::from_min_max(pos2(rect.left() + 40.0, rect.top() + 4.0), pos2(rect.right() - if clearable { 40.0 } else { 16.0 }, rect.bottom() - 4.0));
        let edit = ui.put(
            inner,
            egui::TextEdit::singleline(query)
                .hint_text(egui::RichText::new("Search in this list").color(theme::text_faint()))
                .font(theme::regular(14.0))
                .align(egui::Align2::LEFT_CENTER)
                .frame(egui::Frame::NONE)
                .desired_width(inner.width()),
        );
        if toggle_search {
            edit.request_focus();
        } else if !edit.has_focus() && query.is_empty() {
            // Left alone empty, the field goes away instead of sitting over the list.
            dismiss_search = true;
        }
        if clearable {
            let x = Rect::from_center_size(pos2(rect.right() - 20.0, rect.center().y), Vec2::splat(26.0));
            let resp = named(ui.interact(x, ui.scope_id().with("coll-clear"), Sense::click()), "Clear search").on_hover_cursor(egui::CursorIcon::PointingHand);
            widgets::icon_at(ui, x.center(), icon::X, 14.0, theme::text_dim());
            if resp.clicked() {
                query.clear();
            }
        }
        ui.add_space(18.0);
    } else {
        ui.add_space(26.0);
    }
    if dismiss_search {
        app.coll_search = None;
    }
    let needle = app.coll_search.as_ref().filter(|(fid, _)| fid == id).map(|(_, q)| q.trim().to_lowercase()).unwrap_or_default();
    let filtered: Vec<Track> = if needle.is_empty() {
        Vec::new()
    } else {
        c.tracks
            .iter()
            .filter(|t| {
                t.title.to_lowercase().contains(&needle)
                    || t.artist_line().to_lowercase().contains(&needle)
                    || t.album.as_ref().is_some_and(|a| a.name.to_lowercase().contains(&needle))
            })
            .cloned()
            .collect()
    };
    let shown: &[Track] = if needle.is_empty() { tracks } else { &filtered };
    let number = |i: usize, t: &Track| is_album.then(|| t.track_nr.map_or(i + 1, |n| n as usize));
    let own = app.logged_in && c.owned && !is_album;
    list(ui, &rows, &app.liked, shown, !is_album, number, ListCtx { sel: &mut app.selection, page: &app.page, own }, acts);
    if more {
        widgets::skeleton_rows(ui, now, 3);
    }
}

// ------------------------------------------------------------------ artist

/// Apple's wordmark for the artist, bottom-left at `base`, no wider than `max_w`.
/// False until the image is ready, so the caller can draw the name instead.
fn wordmark(ui: &Ui, base: egui::Pos2, logo: &crate::apple::LogoSrc, max_w: f32) -> bool {
    let aspect = logo.w.max(1) as f32 / logo.h.max(1) as f32;
    let mut size = vec2(72.0 * aspect, 72.0);
    if size.x > max_w {
        size = vec2(max_w, max_w / aspect);
    }
    let url = logo.at_height((size.y * 2.0) as u32);
    let ready = matches!(egui::Image::new(url.as_str()).load_for_size(ui.ctx(), size), Ok(egui::load::TexturePoll::Ready { .. }));
    if ready {
        let rect = Rect::from_min_size(pos2(base.x, base.y - 40.0 - size.y), size);
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        widgets::paint_remote(ui, &url, rect, 0.0, uv, Color32::WHITE);
    }
    ready
}

fn artist(app: &mut App, ui: &mut Ui, id: &str, now: f64) {
    if app.artist.as_ref().is_none_or(|a| a.id != id) {
        placeholder(app, ui, now, 2, CARD);
        return;
    }
    let rows = Rows::new(app, now);
    let accent = app.accent_color();
    // Apple's own page for this artist: their display name and the hero clip. Artists
    // Apple has no clip for (Charli xcx) keep the still banner.
    let apple = app.apple_artists.get(id).and_then(|o| o.clone());
    let name = apple.as_ref().map(|x| x.name.clone()).unwrap_or_else(|| app.artist.as_ref().map(|a| a.name.clone()).unwrap_or_default());
    // Apple's search misses once now and then. While the page is open, ask again after
    // the cooldown rather than waiting for the listener to come back to it.
    if apple.is_none() {
        let key = format!("artist:{id}");
        let cooling = app.apple_missed.get(&key).is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(120));
        if !cooling && app.apple_waiting.insert(key) {
            app.backend.request(crate::backend::Req::AppleArtist { id: id.to_owned(), name: name.clone() });
        }
    }
    // Apple's wordmark wins the hero; failing one, the listener's signature service;
    // failing that, the name in type. A lettered artist still has their signature
    // asked for, because it signs the About section below instead of the hero.
    let signature = app.signature_texture(ui.ctx(), &name);
    let is_blocked = app.blocked_artists.contains_key(id);
    let Some(a) = &app.artist else { return };
    let acts = &mut app.actions;
    let top = &a.top;
    // Hero banner fading into the page.
    let w = ui.available_width();
    let hero_h = (w * 0.34).clamp(240.0, 380.0);
    let (hero, _) = ui.allocate_exact_size(vec2(w, hero_h), Sense::hover());
    if let Some(t) = a.banner.iter().max_by_key(|t| t.width) {
        let img_ratio = if t.height > 0 { t.width as f32 / t.height as f32 } else { 16.0 / 9.0 };
        // Cover-fit: crop the image's top band to the hero's aspect.
        let vis_h = (img_ratio / (w / hero_h)).min(1.0);
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, vis_h));
        ui.painter().rect_filled(hero, 14.0, theme::glass(0.07));
        widgets::paint_remote(ui, &t.url, hero, 14.0, uv, Color32::WHITE);
    } else {
        ui.painter().rect_filled(hero, 14.0, theme::with_alpha(accent, 0.25));
    }
    super::shell::vgradient(ui.painter(), Rect::from_min_max(pos2(hero.left(), hero.center().y - 40.0), hero.max), Color32::TRANSPARENT, theme::shade(0.85));
    let base = hero.left_bottom() + vec2(28.0, -30.0);
    // Apple letters some artists a wordmark image instead of setting their name in type.
    // Until it downloads (or when there's none) the typed name stands in.
    let drawn = apple.as_ref().and_then(|x| x.logo.as_ref()).is_some_and(|logo| wordmark(ui, base, logo, w * 0.62));
    if !drawn {
        match &signature {
            // The artist's own hand, credited above it the way a page signs itself off.
            Some(tex) => {
                let tex_size = tex.size_vec2();
                let aspect = tex_size.x / tex_size.y.max(1.0);
                let mut size = vec2(96.0 * aspect, 96.0);
                let max_w = (w * 0.42).min(340.0);
                if size.x > max_w {
                    size = vec2(max_w, max_w / aspect);
                }
                let rect = Rect::from_min_size(pos2(base.x, base.y - 40.0 - size.y), size);
                ui.painter().text(pos2(base.x, rect.top() - 8.0), Align2::LEFT_BOTTOM, format!("Signed by {name}"), theme::regular(12.0), Color32::from_white_alpha(199));
                let tex = egui::load::SizedTexture::from_handle(tex);
                egui::Image::from_texture(tex).uv(Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0))).paint_at(ui, rect);
            }
            None => {
                ui.painter().text(base - vec2(0.0, 44.0), Align2::LEFT_BOTTOM, &name, theme::bold(58.0), Color32::WHITE);
            }
        }
    }
    if let Some(s) = a.subscribers {
        ui.painter().text(base - vec2(0.0, 18.0), Align2::LEFT_BOTTOM, format!("{} subscribers", compact(s)), theme::regular(14.0), theme::text_dim());
    }
    ui.add_space(18.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        play_pill(ui, acts, &Target::Artist(id.to_owned()), top);
        if pill(ui, "Shuffle", Some(icon::SHUFFLE), false).clicked() && !top.is_empty() {
            let mut t = top.clone();
            super::shuffle(&mut t);
            acts.push(Action::Play(t, 0));
        }
        if let Some(first) = top.first()
            && pill(ui, "Radio", Some(icon::RADIO), false).clicked() {
                acts.push(Action::Radio(first.clone()));
            }
        if is_blocked {
            if pill(ui, "Unblock", None, false).clicked() {
                acts.push(Action::UnblockArtist(id.to_owned()));
            }
        } else if pill(ui, "Block", None, false).clicked() {
            acts.push(Action::BlockArtist(id.to_owned(), name.clone()));
        }
    });
    if !top.is_empty() {
        ui.add_space(26.0);
        let filter_key = egui::Id::unique(("artist-top-filter", id));
        let mut art_filter = ui.data(|d| d.get_temp::<String>(filter_key).unwrap_or_default());
        ui.horizontal(|ui| {
            ui.label(RichText::new("Top songs").font(theme::bold(24.0)).color(theme::text()));
            ui.add_space(12.0);
            ui.allocate_ui_with_layout(vec2(190.0, 30.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut art_filter)
                        .hint_text("Filter songs…")
                        .desired_width(175.0),
                );
            });
            ui.data_mut(|d| d.insert_temp(filter_key, art_filter.clone()));
            if let Some(pl) = &a.top_playlist {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if pill(ui, "Show all", None, false).clicked() {
                        acts.push(Action::Open(Target::Playlist(pl.clone())));
                    }
                });
            }
        });
        ui.add_space(12.0);
        let expand_id = egui::Id::unique(("artist-top-expanded", id));
        let mut expanded = ui.data(|d| d.get_temp::<bool>(expand_id).unwrap_or(false));
        let needle = art_filter.trim().to_lowercase();
        let shown_songs: Vec<(usize, &Track)> = top
            .iter()
            .enumerate()
            .filter(|(_, t)| needle.is_empty() || t.title.to_lowercase().contains(&needle) || t.artist_line().to_lowercase().contains(&needle))
            .collect();
        let visible_limit = if expanded || !needle.is_empty() { shown_songs.len() } else { 8 };
        for &(i, t) in shown_songs.iter().take(visible_limit) {
            let o = rows.opts(&app.liked, t, None, true);
            if track_row(ui, t, &o, acts).clicked() {
                acts.push(Action::Play(top.clone(), i));
            }
        }
        if needle.is_empty() && top.len() > 8 {
            ui.add_space(8.0);
            let label = if expanded {
                "Show less".to_string()
            } else {
                format!("Show more ({})", top.len() - 8)
            };
            if pill(ui, &label, None, false).clicked() {
                expanded = !expanded;
                ui.data_mut(|d| d.insert_temp(expand_id, expanded));
            }
        }
    }
    card_row(ui, "a-albums", "Albums & singles", &a.albums, acts);
    card_row(ui, "a-pl", "Featured on", &a.playlists, acts);
    card_row(ui, "a-sim", "Fans might also like", &a.similar, acts);
    if let Some(d) = a.description.as_deref().filter(|d| !d.trim().is_empty()) {
        shelf_header(ui, "About", None, None, false);
        let full = ui.available_width().min(820.0);
        // A lettered artist's signature can't be the hero, so it signs the About
        // section beside the prose the way a page signs itself off.
        let beside = drawn.then_some(signature.as_ref()).flatten();
        if let Some(tex) = beside {
            ui.horizontal_top(|ui| {
                let sig_w = (full * 0.28).clamp(120.0, 210.0);
                let text_w = (full - sig_w - 32.0).max(220.0);
                let g = widgets::galley_wrapped(ui, d.trim(), theme::regular(14.5), theme::text_dim(), text_w, 8);
                ui.label(g);
                ui.add_space(32.0);
                let tex_size = tex.size_vec2();
                let aspect = tex_size.x / tex_size.y.max(1.0);
                let mut size = vec2(sig_w, sig_w / aspect);
                if size.y > 140.0 {
                    size = vec2(140.0 * aspect, 140.0);
                }
                let (rect, _) = ui.allocate_exact_size(vec2(size.x, size.y + 22.0), Sense::hover());
                ui.painter().text(rect.left_top(), Align2::LEFT_TOP, format!("Signed by {name}"), theme::regular(12.0), Color32::from_white_alpha(199));
                let img = Rect::from_min_size(pos2(rect.left(), rect.top() + 22.0), size);
                let tex = egui::load::SizedTexture::from_handle(tex);
                egui::Image::from_texture(tex).uv(Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0))).paint_at(ui, img);
            });
        } else {
            let g = widgets::galley_wrapped(ui, d.trim(), theme::regular(14.5), theme::text_dim(), full, 8);
            ui.label(g);
        }
    }
}

// ------------------------------------------------------------------ library

fn library(app: &mut App, ui: &mut Ui, tab: LibTab, now: f64) {
    page_title(ui, "Library");
    if !app.logged_in && tab != LibTab::Spotify {
        ui.add_space(10.0);
        ui.label(RichText::new("Sign in to see your playlists, liked songs and history.").color(theme::text_dim()));
        ui.add_space(12.0);
        if pill(ui, "Sign in with Google", Some(icon::CIRCLE_USER), true).clicked() {
            app.start_sign_in(ui.ctx());
        }
        return;
    }
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        for (t, label) in [
            (LibTab::Playlists, "Playlists"),
            (LibTab::Songs, "Liked songs"),
            (LibTab::Albums, "Albums"),
            (LibTab::History, "History"),
            (LibTab::Uploads, "Uploads"),
        ] {
            if chip(ui, label, tab == t).clicked() {
                // Tabs swap in place (no history entry); the current one refreshes.
                if tab == t {
                    app.refresh();
                } else {
                    app.page = Page::Library(t);
                    app.page_loaded();
                }
            }
        }
    });
    ui.add_space(12.0);
    let grid = card_grid;
    let rows = Rows::new(app, now);
    let plain = |_: usize, _: &Track| None;
    match tab {
        LibTab::History => {
            if app.history.is_none() {
                placeholder(app, ui, now, 1, 48.0);
                return;
            }
            let Some(h) = &app.history else { return };
            list(ui, &rows, &app.liked, h, true, plain, ListCtx { sel: &mut app.selection, page: &app.page, own: false }, &mut app.actions);
        }
        LibTab::Uploads => {
            ui.horizontal(|ui| {
                if pill(ui, "Upload music", Some(icon::UPLOAD), true).clicked() {
                    app.pick_uploads(ui.ctx());
                }
                if app.uploads.as_ref().is_some_and(|u| !u.is_empty()) && pill(ui, "Shuffle", Some(icon::SHUFFLE), false).clicked() {
                    let mut t = app.uploads.clone().unwrap_or_default();
                    super::shuffle(&mut t);
                    app.actions.push(Action::Play(t, 0));
                }
            });
            ui.add_space(6.0);
            ui.label(RichText::new("Or drop audio files anywhere on the window. MP3, M4A, FLAC, OGG and WMA.").font(theme::regular(13.0)).color(theme::text_dim()));
            ui.add_space(12.0);
            if app.uploads.is_none() {
                placeholder(app, ui, now, 1, 48.0);
                return;
            }
            let Some(up) = &app.uploads else { return };
            if up.is_empty() {
                ui.add_space(20.0);
                ui.label(RichText::new("No uploads yet").color(theme::text_dim()));
            }
            list(ui, &rows, &app.liked, up, true, plain, ListCtx { sel: &mut app.selection, page: &app.page, own: false }, &mut app.actions);
        }
        // Liked Music, kept like any playlist.
        LibTab::Songs => {
            if app.opened(super::LIKED_MUSIC).is_none() {
                placeholder(app, ui, now, 1, 48.0);
                return;
            }
            let more = app.receiving(super::LIKED_MUSIC);
            let Some(o) = app.lists.get(super::LIKED_MUSIC) else { return };
            let liked = &o.c.tracks;
            if !liked.is_empty() {
                ui.horizontal(|ui| {
                    if pill(ui, "Play", Some(icon::PLAY), true).clicked() {
                        app.actions.push(Action::Play(liked.clone(), 0));
                    }
                    if pill(ui, "Shuffle", Some(icon::SHUFFLE), false).clicked() {
                        let mut t = liked.clone();
                        super::shuffle(&mut t);
                        app.actions.push(Action::Play(t, 0));
                    }
                    if app.transfer.cookie().is_some() {
                        let label = if app.two_way_syncing { "Syncing with Spotify…" } else { "2-Way Sync with Spotify" };
                        if pill(ui, label, Some(icon::ROTATE_CW), false).clicked() && !app.two_way_syncing {
                            app.actions.push(Action::StartTwoWayLikedSync);
                        }
                    }
                    select_pill(ui, &mut app.selection, &app.page);
                });
                ui.add_space(10.0);
            } else {
                ui.add_space(32.0);
                ui.label(RichText::new("No liked songs yet").font(theme::semibold(16.0)).color(theme::text_dim()));
                ui.add_space(4.0);
                ui.label(RichText::new("Songs you like will appear here.").font(theme::regular(13.5)).color(theme::text_faint()));
                if app.transfer.cookie().is_some() {
                    ui.add_space(12.0);
                    let label = if app.two_way_syncing { "Syncing with Spotify…" } else { "Import Liked Songs from Spotify (2-Way)" };
                    if pill(ui, label, Some(icon::ROTATE_CW), true).clicked() && !app.two_way_syncing {
                        app.actions.push(Action::StartTwoWayLikedSync);
                    }
                }
            }
            list(ui, &rows, &app.liked, liked, true, plain, ListCtx { sel: &mut app.selection, page: &app.page, own: false }, &mut app.actions);
            if more {
                widgets::skeleton_rows(ui, now, 3);
            }
        }
        LibTab::Spotify => spotify_sync_tab(app, ui, now),
        _ => {
            let loaded = app.spotify_loaded_playlist.lock().unwrap().take();
            if let Some((src, res)) = loaded {
                if let Some((entry, playlist, loading)) = &mut app.spotify_selected {
                    if entry.source == src {
                        *loading = false;
                        match res {
                            Ok(p) => *playlist = Some(p),
                            Err(e) => {
                                app.toast(format!("Error loading Spotify: {e}"));
                            }
                        }
                    }
                }
            }

            if app.spotify_selected.is_some() {
                spotify_sync_tab(app, ui, now);
                return;
            }

            if app.library.is_none() {
                placeholder(app, ui, now, 2, CARD);
                return;
            }
            let Some(lib) = &app.library else { return };
            match tab {
                LibTab::Playlists => {
                    let ytm_playlists = &lib.playlists;

                    // Spotify playlists
                    app.transfer.ensure_account(&app.backend);
                    let spotify_playlists: Vec<crate::transfer::spotify::Entry> = {
                        let acc = app.transfer.account.lock().unwrap();
                        if let crate::transfer::Account::SignedIn { library, .. } = &*acc {
                            library.iter().filter(|e| !matches!(e.source, crate::transfer::spotify::Source::Liked)).cloned().collect()
                        } else {
                            Vec::new()
                        }
                    };

                    let (conflicts, unified_cards) = super::playlist_sync::analyze_playlists(
                        ytm_playlists,
                        &spotify_playlists,
                        &app.playlist_decisions,
                    );

                    let filter_id = egui::Id::unique("lib-pl-filter");
                    let mut filter = ui.data(|d| d.get_temp::<u8>(filter_id).unwrap_or(0));

                    ui.horizontal(|ui| {
                        if pill(ui, "New playlist", None, false).clicked() {
                            app.actions.push(Action::NewPlaylist(Vec::new()));
                        }
                        ui.add_space(14.0);

                        let count_all = unified_cards.len();
                        let count_ytm = unified_cards.iter().filter(|uc| uc.sources.contains(&super::playlist_sync::SourceKind::YouTubeMusic)).count();
                        let count_spotify = unified_cards.iter().filter(|uc| uc.sources.contains(&super::playlist_sync::SourceKind::Spotify)).count();

                        for (idx, label) in [
                            (0, format!("All Playlists ({count_all})")),
                            (1, format!("YouTube Music ({count_ytm})")),
                            (2, format!("Spotify ({count_spotify})")),
                        ] {
                            if chip(ui, &label, filter == idx).clicked() {
                                filter = idx;
                                ui.data_mut(|d| d.insert_temp(filter_id, filter));
                            }
                        }

                        if !app.playlist_decisions.is_empty() {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if pill(ui, "Reset decisions", Some(icon::ROTATE_CW), false).clicked() {
                                    app.clear_playlist_decisions();
                                    app.toast("Playlist decisions reset.");
                                }
                            });
                        }
                    });
                    ui.add_space(14.0);

                    // Render any unresolved conflict resolution banners
                    for conflict in &conflicts {
                        super::playlist_sync::render_conflict_banner(ui, conflict, app);
                        ui.add_space(10.0);
                    }
                    if !conflicts.is_empty() {
                        ui.add_space(6.0);
                    }

                    // Filter cards
                    let display_cards: Vec<&super::playlist_sync::UnifiedCard> = match filter {
                        1 => unified_cards.iter().filter(|uc| uc.sources.contains(&super::playlist_sync::SourceKind::YouTubeMusic)).collect(),
                        2 => unified_cards.iter().filter(|uc| uc.sources.contains(&super::playlist_sync::SourceKind::Spotify)).collect(),
                        _ => unified_cards.iter().collect(),
                    };

                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(20.0, 22.0);
                        for uc in display_cards {
                            card(ui, &uc.card, CARD, false, &mut app.actions);
                        }
                    });
                }
                LibTab::Albums => grid(ui, &lib.albums, &mut app.actions),
                LibTab::Songs | LibTab::Spotify | LibTab::History | LibTab::Uploads => unreachable!(),
            }
        }
    }
}

fn spotify_sync_tab(app: &mut App, ui: &mut Ui, _now: f64) {
    let loaded = app.spotify_loaded_playlist.lock().unwrap().take();
    if let Some((src, res)) = loaded {
        if let Some((entry, playlist, loading)) = &mut app.spotify_selected {
            if entry.source == src {
                *loading = false;
                match res {
                    Ok(p) => *playlist = Some(p),
                    Err(e) => {
                        app.toast(format!("Error loading Spotify: {e}"));
                    }
                }
            }
        }
    }

    app.transfer.ensure_account(&app.backend);
    let account = app.transfer.account.lock().unwrap().clone();

    if let Some((entry, playlist, loading)) = app.spotify_selected.clone() {
        // Back Button
        if pill(ui, "Back to library", Some(icon::CHEVRON_LEFT), false).clicked() {
            app.spotify_selected = None;
            return;
        }
        ui.add_space(18.0);

        // Header Section matching collection() styling
        let target_id = format!("sp:{}", entry.source.id());
        let target = Target::Playlist(target_id.clone());
        let mut toggle_search = false;

        ui.horizontal_top(|ui| {
            let cover_size = 232.0;
            let (img, _) = ui.allocate_exact_size(Vec2::splat(cover_size), Sense::hover());
            widgets::soft_shadow(ui, img, 10.0, 40.0, 0.5);
            paint_cover(ui, img, &super::transfer::spotify_thumbs(&entry.art), theme::card_radius() + 2.0, 1.0);

            ui.add_space(26.0);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let badge_rect = Rect::from_min_size(ui.cursor().min, vec2(16.0, 16.0));
                    super::transfer::badge(ui, badge_rect);
                    ui.add_space(22.0);
                    ui.label(RichText::new("SPOTIFY PLAYLIST").font(theme::semibold(12.0)).color(super::transfer::SPOTIFY));
                });
                ui.add_space(4.0);

                ui.label(RichText::new(&entry.name).font(theme::bold(38.0)).color(theme::text()));
                ui.add_space(4.0);

                let count_str = match playlist.as_ref() {
                    Some(p) => {
                        let c = p.tracks.len();
                        if c == 1 { "1 song".to_string() } else { format!("{c} songs") }
                    }
                    None => entry.count.map_or("Loading…".to_string(), |c| if c == 1 { "1 song".to_string() } else { format!("{c} songs") }),
                };

                let dur_str = playlist.as_ref().map(|p| {
                    let total_sec: u32 = p.tracks.iter().filter_map(|t| t.duration).sum();
                    if total_sec >= 3600 {
                        format!(" • {} hr {} min", total_sec / 3600, (total_sec % 3600) / 60)
                    } else if total_sec > 0 {
                        format!(" • {} min", total_sec / 60)
                    } else {
                        String::new()
                    }
                }).unwrap_or_default();

                let owner = if entry.owner.is_empty() { "Spotify".to_string() } else { entry.owner.clone() };
                ui.label(RichText::new(format!("{owner} • {count_str}{dur_str} • Spotify Sync (Ad-free audio)")).font(theme::regular(14.0)).color(theme::text_dim()));

                ui.add_space(16.0);

                let filter_on = app.coll_search.as_ref().is_some_and(|(fid, _)| fid == &target_id);
                let is_active = widgets::active_state(ui, &target) == Some(true);

                // Action buttons: exactly matching collection() - never clipped!
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = vec2(10.0, 0.0);
                    let label = if is_active { "Pause" } else { "Play" };
                    let glyph = if is_active { icon::PAUSE } else { icon::PLAY };
                    if pill(ui, label, Some(glyph), true).clicked() {
                        if is_active {
                            app.actions.push(Action::TogglePlay);
                        } else if let Some(p) = &playlist {
                            if !p.tracks.is_empty() {
                                app.actions.push(Action::PlaySpotifyPlaylist(p.tracks.clone(), 0));
                            }
                        }
                    }
                    if circle_action(ui, CircleGlyph::Char(icon::SHUFFLE), "Shuffle", false).clicked() {
                        if let Some(p) = &playlist {
                            if !p.tracks.is_empty() {
                                let mut t = p.tracks.clone();
                                super::shuffle(&mut t);
                                app.actions.push(Action::PlaySpotifyPlaylist(t, 0));
                            }
                        }
                    }
                    if circle_action(ui, CircleGlyph::Char(icon::SEARCH), "Search in this list", filter_on).clicked() {
                        let closing = app.coll_search.as_ref().is_some_and(|(fid, _)| fid == &target_id);
                        app.coll_search = (!closing).then(|| (target_id.clone(), String::new()));
                        toggle_search = !closing;
                    }
                    let more = circle_action(ui, CircleGlyph::More, "More", false);
                    egui::Popup::menu(&more).show(|ui| {
                        ui.set_width(220.0);
                        ui.add_space(3.0);
                        if widgets::menu_action(ui, Some(icon::DOWNLOAD), "Import to YouTube Music") {
                            app.xfer.picked = Some(entry.source.clone());
                            app.actions.push(Action::Go(Page::Transfer));
                        }
                        if widgets::menu_action(ui, Some(icon::ROTATE_CW), "Refresh tracks") {
                            let xfer = app.transfer.clone();
                            let b = app.backend.clone();
                            let slot = app.spotify_loaded_playlist.clone();
                            let src = entry.source.clone();
                            app.spotify_selected = Some((entry.clone(), None, true));
                            xfer.fetch_spotify_playlist(&b, src.clone(), move |res| {
                                *slot.lock().unwrap() = Some((src, res));
                            });
                        }
                        ui.add_space(3.0);
                    });
                });
            });
        });

        ui.add_space(20.0);

        if loading {
            ui.add_space(30.0);
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new("Loading songs from Spotify…").font(theme::regular(14.0)).color(theme::text_dim()));
            });
            return;
        }

        let Some(p) = playlist else {
            ui.label(RichText::new("Could not load playlist tracks.").color(theme::text_dim()));
            return;
        };

        if p.tracks.is_empty() {
            ui.label(RichText::new("This playlist contains no songs.").color(theme::text_dim()));
            return;
        }

        // Inline search bar if search button clicked
        let mut dismiss_search = false;
        if let Some((fid, query)) = &mut app.coll_search && fid == &target_id {
            ui.add_space(10.0);
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width().min(420.0), 40.0), Sense::hover());
            ui.painter().rect_filled(rect, 20.0, theme::glass(0.09));
            widgets::icon_at(ui, pos2(rect.left() + 20.0, rect.center().y), icon::SEARCH, 16.0, theme::text_dim());
            let clearable = !query.is_empty();
            let inner = Rect::from_min_max(pos2(rect.left() + 40.0, rect.top() + 4.0), pos2(rect.right() - if clearable { 40.0 } else { 16.0 }, rect.bottom() - 4.0));
            let edit = ui.put(
                inner,
                egui::TextEdit::singleline(query)
                    .hint_text(egui::RichText::new("Search in this list").color(theme::text_faint()))
                    .font(theme::regular(14.0))
                    .align(egui::Align2::LEFT_CENTER)
                    .frame(egui::Frame::NONE)
                    .desired_width(inner.width()),
            );
            if toggle_search {
                edit.request_focus();
            } else if !edit.has_focus() && query.is_empty() {
                dismiss_search = true;
            }
            if clearable {
                let x = Rect::from_center_size(pos2(rect.right() - 20.0, rect.center().y), Vec2::splat(26.0));
                let resp = named(ui.interact(x, ui.scope_id().with("coll-clear"), Sense::click()), "Clear search").on_hover_cursor(egui::CursorIcon::PointingHand);
                widgets::icon_at(ui, x.center(), icon::X, 14.0, theme::text_dim());
                if resp.clicked() {
                    query.clear();
                }
            }
            ui.add_space(14.0);
        }
        if dismiss_search {
            app.coll_search = None;
        }

        let needle = app.coll_search.as_ref().filter(|(fid, _)| fid == &target_id).map(|(_, q)| q.trim().to_lowercase()).unwrap_or_default();
        let model_tracks: Vec<Track> = p.tracks.iter().enumerate().map(|(i, st)| {
            Track {
                id: format!("sp:{}:{}", entry.source.id(), i),
                title: st.name.clone(),
                artists: st.artists.iter().map(|a| crate::model::Link { name: a.clone(), id: None }).collect(),
                album: None,
                duration: st.duration,
                thumbs: st.art.as_ref().map(|u| vec![crate::model::Thumb { url: u.clone(), width: 64, height: 64 }]).unwrap_or_default(),
                track_nr: Some((i + 1) as u16),
                plays: None,
                music_video: false,
            }
        }).filter(|t| needle.is_empty() || t.title.to_lowercase().contains(&needle) || t.artist_line().to_lowercase().contains(&needle)).collect();

        let rows = Rows::new(app, _now);
        list(ui, &rows, &app.liked, &model_tracks, false, |i, _| Some(i + 1), ListCtx { sel: &mut app.selection, page: &app.page, own: false }, &mut app.actions);
        return;
    }

    // Dashboard Overview
    match &account {
        Account::SignedIn { name, library } => {
            let (c_rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 70.0), Sense::hover());
            ui.painter().rect_filled(c_rect, 12.0, theme::panel(0.35));
            ui.painter().rect_stroke(c_rect, 12.0, egui::Stroke::new(1.0, theme::glass(0.1)), egui::StrokeKind::Inside);
            let inner = c_rect.shrink(14.0);
            let cy = inner.center().y;

            let badge_r = Rect::from_center_size(pos2(inner.left() + 20.0, cy), Vec2::splat(32.0));
            super::transfer::badge(ui, badge_r);

            let text_x = badge_r.right() + 14.0;
            let who = if name.is_empty() { "Spotify Connected".to_string() } else { format!("{}'s Spotify", name) };
            ui.painter().text(pos2(text_x, cy - 8.0), egui::Align2::LEFT_CENTER, &who, theme::bold(15.0), theme::text());
            ui.painter().text(pos2(text_x, cy + 8.0), egui::Align2::LEFT_CENTER, "Spotify Sync active · Streaming via YouTube Music", theme::regular(12.0), theme::text_dim());

            // Right action buttons: Refresh, Sign out
            let btn_area = Rect::from_min_max(pos2(inner.right() - 260.0, inner.top()), inner.max);
            ui.scope_builder(egui::UiBuilder::new().max_rect(btn_area), |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if pill(ui, "Sign out", None, false).clicked() {
                        app.transfer.set_cookie(None, &app.backend);
                    }
                    ui.add_space(8.0);
                    if pill(ui, "Refresh", Some(icon::HISTORY), false).clicked() {
                        app.transfer.load_account(&app.backend);
                    }
                });
            });

            ui.add_space(16.0);

            // Liked songs Hero card if available
            if let Some(liked_entry) = library.iter().find(|e| e.source == Source::Liked) {
                let (h_rect, h_resp) = ui.allocate_exact_size(vec2(ui.available_width(), 90.0), Sense::click());
                let h_resp = h_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                let h_hover = theme::anim_bool(ui.ctx(), h_resp.id.with("hh"), h_resp.hovered(), 0.15);
                ui.painter().rect_filled(h_rect, 14.0, theme::lerp_color(Color32::from_rgb(20, 48, 30), Color32::from_rgb(26, 64, 40), h_hover));
                ui.painter().rect_stroke(h_rect, 14.0, egui::Stroke::new(1.0, theme::lerp_color(theme::glass(0.15), super::transfer::SPOTIFY, h_hover * 0.5)), egui::StrokeKind::Inside);

                let hi = h_rect.shrink(16.0);
                let heart_r = Rect::from_center_size(pos2(hi.left() + 25.0, hi.center().y), Vec2::splat(36.0));
                ui.painter().rect_filled(heart_r, 8.0, super::transfer::SPOTIFY);
                glyphs::draw(ui, heart_r.center(), 22.0, glyphs::HEART_FILLED, Color32::WHITE);

                let tx = heart_r.right() + 16.0;
                ui.painter().text(pos2(tx, hi.center().y - 9.0), egui::Align2::LEFT_CENTER, "Spotify Liked Songs", theme::bold(17.0), Color32::WHITE);
                let count_text = liked_entry.count.map_or("Your liked songs".to_string(), |c| format!("{c} synced songs"));
                ui.painter().text(pos2(tx, hi.center().y + 9.0), egui::Align2::LEFT_CENTER, &count_text, theme::regular(13.0), Color32::from_rgb(200, 225, 205));

                let btns_r = Rect::from_min_max(pos2(hi.right() - 320.0, hi.top()), hi.max);
                ui.scope_builder(egui::UiBuilder::new().max_rect(btns_r), |ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if pill(ui, "Open", Some(icon::CHEVRON_RIGHT), false).clicked() || h_resp.clicked() {
                            app.spotify_selected = Some((liked_entry.clone(), None, true));
                            let xfer = app.transfer.clone();
                            let b = app.backend.clone();
                            let slot = app.spotify_loaded_playlist.clone();
                            xfer.fetch_spotify_playlist(&b, Source::Liked, move |res| {
                                *slot.lock().unwrap() = Some((Source::Liked, res));
                            });
                        }
                        ui.add_space(8.0);
                        let sync_label = if app.two_way_syncing { "Syncing…" } else { "2-Way Sync" };
                        if pill(ui, sync_label, Some(icon::ROTATE_CW), false).clicked() && !app.two_way_syncing {
                            app.actions.push(Action::StartTwoWayLikedSync);
                        }
                    });
                });
                ui.add_space(18.0);
            }

            // Playlists Grid Section
            ui.horizontal(|ui| {
                ui.label(RichText::new("Your Spotify Playlists").font(theme::bold(17.0)).color(theme::text()));
                ui.add_space(8.0);
                if pill(ui, "Refresh", Some(icon::ROTATE_CW), false).clicked() {
                    *app.transfer.account.lock().unwrap() = crate::transfer::Account::Unknown;
                    app.transfer.ensure_account(&app.backend);
                }
            });
            ui.add_space(8.0);

            let playlists: Vec<&Entry> = library.iter().filter(|e| e.source != Source::Liked).collect();
            if playlists.is_empty() {
                ui.label(RichText::new("No playlists found.").color(theme::text_dim()));
            } else {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(20.0, 22.0);
                    for pl in playlists {
                        let subtitle = pl.count.map_or_else(|| format!("Spotify • {}", pl.owner), |c| format!("Spotify • {c} songs"));
                        let c = Card {
                            title: pl.name.clone(),
                            subtitle,
                            thumbs: super::transfer::spotify_thumbs(&pl.art),
                            target: Target::Playlist(format!("sp:{}", pl.source.id())),
                            round: false,
                            links: vec![crate::model::Link { name: pl.owner.clone(), id: None }],
                        };
                        card(ui, &c, CARD, false, &mut app.actions);
                    }
                });
            }
        }
        Account::Unknown | Account::Loading => {
            let (c_rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 120.0), Sense::hover());
            ui.painter().rect_filled(c_rect, 14.0, theme::panel(0.4));
            ui.painter().rect_stroke(c_rect, 14.0, egui::Stroke::new(1.0, theme::glass(0.12)), egui::StrokeKind::Inside);
            let inner = c_rect.shrink(18.0);
            ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Connecting to Spotify…").font(theme::bold(16.0)).color(theme::text()));
                        ui.label(RichText::new("Loading your Spotify playlists and Liked Songs.").font(theme::regular(13.0)).color(theme::text_dim()));
                    });
                });
            });
        }
        Account::Failed(err) => {
            let (c_rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 156.0), Sense::hover());
            ui.painter().rect_filled(c_rect, 14.0, theme::panel(0.4));
            ui.painter().rect_stroke(c_rect, 14.0, egui::Stroke::new(1.0, theme::glass(0.12)), egui::StrokeKind::Inside);
            let inner = c_rect.shrink(18.0);
            let badge_r = Rect::from_center_size(pos2(inner.left() + 26.0, inner.top() + 26.0), Vec2::splat(40.0));
            super::transfer::badge(ui, badge_r);
            let tx = badge_r.right() + 18.0;
            ui.painter().text(pos2(tx, inner.top() + 8.0), egui::Align2::LEFT_TOP, "Spotify Connection Issue", theme::bold(17.0), theme::text());
            let g = widgets::galley_wrapped(ui, err, theme::regular(13.0), theme::text_dim(), (inner.right() - tx).max(160.0), 2);
            ui.painter().galley(pos2(tx, inner.top() + 34.0), g, theme::text_dim());
            let btn_r = Rect::from_min_size(pos2(tx, inner.bottom() - 36.0), vec2(360.0, 34.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(btn_r), |ui| {
                ui.horizontal(|ui| {
                    if app.transfer.cookie().is_some() && pill(ui, "Retry Spotify", Some(icon::ROTATE_CW), true).clicked() {
                        app.transfer.retry_account(&app.backend);
                    }
                    if pill(ui, "Sign in again", Some(icon::CIRCLE_USER), false).clicked() {
                        app.start_spotify_sign_in(ui.ctx());
                    }
                });
            });
        }
        Account::SignedOut => {
            // Signed out onboarding card
            let (c_rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 160.0), Sense::hover());
            ui.painter().rect_filled(c_rect, 14.0, theme::panel(0.4));
            ui.painter().rect_stroke(c_rect, 14.0, egui::Stroke::new(1.0, theme::glass(0.12)), egui::StrokeKind::Inside);

            let inner = c_rect.shrink(18.0);
            let badge_r = Rect::from_center_size(pos2(inner.left() + 26.0, inner.top() + 26.0), Vec2::splat(40.0));
            super::transfer::badge(ui, badge_r);

            let tx = badge_r.right() + 18.0;
            ui.painter().text(pos2(tx, inner.top() + 8.0), egui::Align2::LEFT_TOP, "Spotify Sync", theme::bold(18.0), theme::text());
            ui.painter().text(pos2(tx, inner.top() + 32.0), egui::Align2::LEFT_TOP, "Connect your Spotify account to seamlessly browse your playlists and liked songs in BitChord.", theme::regular(13.0), theme::text_dim());
            ui.painter().text(pos2(tx, inner.top() + 50.0), egui::Align2::LEFT_TOP, "All tracks are streamed ad-free in best audio quality via YouTube Music (pure music, no podcasts).", theme::regular(12.5), theme::text_dim());

            let btn_r = Rect::from_min_size(pos2(tx, inner.bottom() - 36.0), vec2(200.0, 34.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(btn_r), |ui| {
                if pill(ui, "Connect with Spotify", Some(icon::CIRCLE_USER), true).clicked() {
                    app.start_spotify_sign_in(ui.ctx());
                }
            });
        }
    }
}

// ------------------------------------------------------------------ downloads

fn downloads(app: &mut App, ui: &mut Ui, now: f64) {
    page_title(ui, "Downloads");
    let songs = crate::downloads::list();
    ui.add_space(4.0);
    ui.label(
        RichText::new(format!("{} songs \u{2022} {}", songs.len(), crate::downloads::folder().display()))
            .font(theme::regular(13.0))
            .color(theme::text_dim()),
    );
    ui.add_space(12.0);
    if songs.is_empty() {
        ui.label(RichText::new("No downloads yet").font(theme::semibold(16.0)).color(theme::text()));
        ui.add_space(4.0);
        ui.label(
            RichText::new("Right-click a song and choose Download to keep it on this computer, playable offline.")
                .font(theme::regular(13.0))
                .color(theme::text_dim()),
        );
        ui.add_space(10.0);
        if pill(ui, "Open folder", Some(icon::FOLDER_OPEN), false).clicked() {
            crate::downloads::open_folder();
        }
        return;
    }
    ui.horizontal(|ui| {
        if pill(ui, "Play", Some(icon::PLAY), true).clicked() {
            app.actions.push(Action::Play(songs.clone(), 0));
        }
        if pill(ui, "Shuffle", Some(icon::SHUFFLE), false).clicked() {
            let mut t = songs.clone();
            super::shuffle(&mut t);
            app.actions.push(Action::Play(t, 0));
        }
        if pill(ui, "Open folder", Some(icon::FOLDER_OPEN), false).clicked() {
            crate::downloads::open_folder();
        }
    });
    ui.add_space(12.0);
    let rows = Rows::new(app, now);
    list(ui, &rows, &app.liked, &songs, true, |_, _| None, ListCtx { sel: &mut app.selection, page: &app.page, own: false }, &mut app.actions);
}

// ------------------------------------------------------------------ settings

fn settings(app: &mut App, ui: &mut Ui) {
    // One centred column; every section shares its width.
    let avail = ui.available_width();
    let w = avail.min(920.0);
    ui.horizontal(|ui| {
        ui.add_space(((avail - w) / 2.0).max(0.0));
        ui.vertical(|ui| {
            ui.set_width(w);
            settings_body(app, ui);
            ui.add_space(16.0);
        });
    });
}

fn settings_body(app: &mut App, ui: &mut Ui) {
    page_title(ui, "Settings");
    settings_updates(app, ui);
    settings_account(app, ui);
    settings_playback(app, ui);
    settings_equalizer(app, ui);
    settings_downloads(ui);
    settings_lyrics(app, ui);
    settings_artists(app, ui);
    settings_appearance(app, ui);
    settings_window(app, ui);
    settings_connections(app, ui);
    section_label(ui, "Network", None);
    settings_card(ui, |ui| network(app, ui));
    super::transfer::settings_section(app, ui);
    settings_shortcuts(ui);
}

fn settings_updates(app: &mut App, ui: &mut Ui) {
    use crate::update::{State, current_version};
    let current = current_version();
    let updater = app.updater.clone();
    let state = updater.as_ref().map_or(State::Idle, |u| u.state());
    let (mut do_check, mut do_restart) = (false, false);
    let mut open_notes: Option<(String, bool)> = None;
    let mode = if crate::update::portable_dir().is_some() {
        "Portable"
    } else if crate::update::installed() {
        "Installed"
    } else {
        "Standalone"
    };
    section_label(ui, "Updates", Some(&format!("v{current} \u{2022} {mode}")));
    settings_card(ui, |ui| {
        let title = match &state {
            State::Ready { version, .. } => format!("Update ready: BitChord v{version}"),
            State::Downloading { version, percent } => format!("Downloading BitChord v{version} ({percent}%)"),
            _ => format!("BitChord v{current}"),
        };
        let desc = match &state {
            State::Idle => "Automatic in-app updates check for new releases cleanly in the background.",
            State::Checking => "Checking for a newer version…",
            State::UpToDate => "You are running the latest version.",
            State::Downloading { .. } => "Downloading the update in the background. You can keep listening.",
            State::Ready { .. } => "Click Update & Restart to apply the update automatically with no browser download.",
            State::Failed(_) => "Update check failed. Check your connection and try again.",
        };
        let err_tip = match &state {
            State::Failed(e) => Some(e.clone()),
            _ => None,
        };
        setting_row(ui, &title, Some(desc), |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                match &state {
                    State::Checking | State::Downloading { .. } => {
                        ui.spinner();
                    }
                    State::Ready { version, .. } => {
                        if pill(ui, "What's new", None, false).clicked() {
                            open_notes = Some((version.clone(), true));
                        }
                        if pill(ui, "Update & Restart", Some(icon::SPARKLES), true).clicked() {
                            do_restart = true;
                        }
                    }
                    State::Idle | State::UpToDate | State::Failed(_) => {
                        if pill(ui, "What's new", Some(icon::SPARKLES), false).clicked() {
                            open_notes = Some((current.clone(), false));
                        }
                        let mut chk = pill(ui, "Check for updates", Some(icon::ROTATE_CW), false);
                        if let Some(err) = &err_tip {
                            chk = chk.on_hover_text(err);
                        }
                        do_check = chk.clicked();
                    }
                }
            });
        });
    });
    if do_check && let Some(u) = updater {
        u.force_check();
    }
    if let Some((version, ready)) = open_notes {
        app.notes_dialog = Some(super::notes::NotesDialog { version, ready });
    }
    if do_restart {
        app.restart_to_update(ui.ctx());
    }
}

/// Settings › Account: signing in and out, and which channel to use.
fn settings_account(app: &mut App, ui: &mut Ui) {
    section_label(ui, "Account", None);
    let busy = app.signing_in.load(Ordering::Relaxed);
    let (mut sign_in, mut sign_out) = (false, false);
    // Asked again now and then while Settings shows: a failed ask, or a channel made since.
    let now = ui.input(|i| i.time);
    if app.logged_in && app.channels_asked.is_none_or(|at| now - at > 60.0) {
        app.channels_asked = Some(now);
        app.backend.request(crate::backend::Req::Channels);
    }
    settings_card(ui, |ui| {
        if app.logged_in {
            let name = app.account_name.clone().unwrap_or_else(|| "YouTube Music".to_owned());
            setting_row(ui, &name, Some("Signed in with Google. Your home feed, library and likes come from this account. Saved encrypted in Windows Credential Manager."), |ui| {
                sign_out = pill(ui, "Sign out", Some(icon::LOG_OUT), false).clicked();
            });
            // Also with one, when the one in use isn't it (a brand channel since deleted).
            if app.channels.as_ref().is_some_and(|c| c.list.len() > 1 || !c.list.iter().any(|l| l.page_id == c.current)) {
                divider(ui);
                setting_row(ui, "Channel", Some("Your Google account and each YouTube channel it manages have their own library, likes, history and recommendations."), |ui| {
                    channel_picker(app, ui);
                });
            }
        } else {
            setting_row(ui, "Not signed in", Some("Sign in to get your home feed, library and likes."), |ui| {
                if busy {
                    ui.spinner();
                } else {
                    sign_in = pill(ui, "Sign in with Google", Some(icon::CIRCLE_USER), true).clicked();
                }
            });
        }
    });
    if busy {
        ui.add_space(6.0);
        ui.label(RichText::new("Finish signing in in the window that opened.").font(theme::regular(12.5)).color(theme::text_dim()));
    }
    if sign_in {
        app.start_sign_in(ui.ctx());
    }
    if sign_out {
        app.backend.request(crate::backend::Req::Logout);
    }
}

fn channel_picker(app: &mut App, ui: &mut Ui) {
    let Some(crate::backend::Channels { list, current, epoch }) = &app.channels else { return };
    // A brand channel often has the account's name: the handle, or "No channel" (as
    // YouTube's switcher says), tells them apart.
    let label = |c: &crate::ytm::Channel| match (&c.handle, c.has_channel) {
        (Some(h), _) => format!("{} ({h})", c.name),
        (None, false) => format!("{} (No channel)", c.name),
        (None, true) => c.name.clone(),
    };
    // The button is narrower: a handle alone is enough there.
    let shown = match list.iter().find(|c| c.page_id == *current) {
        Some(c) => c.handle.clone().unwrap_or_else(|| label(c)),
        None => "Another channel".to_owned(),
    };
    // A transfer would go on as the new channel.
    if app.transfer.running() {
        ui.label(RichText::new("Not while a transfer runs").font(theme::regular(13.0)).color(theme::text_dim()));
        return;
    }
    let switching = app.switching_to.is_some();
    let btn = dropdown_button(ui, if switching { "Switching…" } else { &shown }, 220.0, "Channel");
    let mut pick = None;
    egui::Popup::menu(&btn).show(|ui| {
        ui.set_width(300.0);
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            ui.add_space(4.0);
            for c in list {
                if option_row(ui, &label(c), c.page_id == *current).clicked() {
                    pick = Some(c.clone());
                    ui.close();
                }
            }
            ui.add_space(4.0);
        });
    });
    if let Some(c) = pick.filter(|c| c.page_id != *current && !switching) {
        app.switching_to = Some(label(&c));
        app.backend.request(crate::backend::Req::SwitchChannel { page_id: c.page_id, epoch: *epoch });
    }
}

/// Settings › Playback.
fn settings_playback(app: &mut App, ui: &mut Ui) {
    section_label(ui, "Playback", None);
    settings_card(ui, |ui| {
        setting_row(ui, "Autoplay", Some("Keep playing similar songs when the queue ends."), |ui| {
            toggle(ui, &mut app.autoplay, "Autoplay");
        });
        divider(ui);
        setting_row(ui, "Volume normalization", Some("Turn loud songs down to the same level, like YouTube Music does."), |ui| {
            if toggle(ui, &mut app.normalize, "Volume normalization").changed() {
                app.audio.set_normalize(app.normalize);
            }
        });
        divider(ui);
        setting_row(ui, "Logarithmic volume", Some("Space the volume slider out the way ears hear loudness, so quiet levels aren't squeezed into its first few pixels."), |ui| {
            if toggle(ui, &mut app.log_volume, "Logarithmic volume").changed() {
                app.audio.set_log_volume(app.log_volume);
            }
        });
        divider(ui);
        setting_row(ui, "Play songs, not music videos", Some("An official music video plays as its song, the album recording, when YouTube Music has one: no intros, skits or video-only edits. Lists still show what they hold."), |ui| {
            toggle(ui, &mut app.play_songs, "Play songs, not music videos");
        });
        divider(ui);
        setting_row(ui, "Output device", Some("Where the music plays. A picked device that isn't connected leaves it to Windows' default until it's back."), |ui| {
            output_device_picker(app, ui);
        });
        divider(ui);
        setting_row(ui, "Automix", Some("Slide from one song into the next the way a DJ would: both hold their level for a moment, then swap in one smooth move, instead of a straight fade. The mix picks its own length, in phrases of 8 or 16 bars (16 on long songs), following the beat it hears."), |ui| {
            if toggle(ui, &mut app.automix, "Automix").changed() {
                app.set_crossfade();
            }
        });
        if !app.automix {
            divider(ui);
            setting_row(ui, "Crossfade songs", Some("Blend the end of each song into the start of the next."), |ui| {
                if toggle(ui, &mut app.crossfade, "Crossfade songs").changed() {
                    app.set_crossfade();
                }
            });
            if app.crossfade {
                divider(ui);
                setting_row(ui, "Crossfade length", None, |ui| {
                    if seconds_slider(ui, &mut app.crossfade_secs, 1..=super::CROSSFADE_MAX, "Crossfade length").changed() {
                        app.set_crossfade();
                    }
                });
            }
        }
        divider(ui);
        setting_row(ui, "Skip non-music parts", Some("Jump over talking, skits and sponsor breaks in music videos, as marked by the SponsorBlock community. Album songs don't have any."), |ui| {
            if toggle(ui, &mut app.sponsorblock, "Skip non-music parts").changed() {
                app.request_segments();
            }
        });
        divider(ui);
        setting_row(ui, "Hi-Res audio", Some("Addon servers that serve hi-res copies of songs. When one has the song, its copy plays instead of YouTube's, and its quality shows in the player."), |_| {});
        divider(ui);
        ui.add_space(12.0);
        hires_sources_editor(app, ui);
        ui.add_space(10.0);
        divider(ui);
        setting_row(ui, "Prefer Dolby Atmos", Some("Ask the hi-res sources for a Dolby Atmos copy when they offer one. Falls back to the best stereo copy when the stream's codec can't be decoded."), |ui| {
            if toggle(ui, &mut app.prefer_atmos, "Prefer Dolby Atmos").changed() {
                app.backend.set_prefer_atmos(app.prefer_atmos);
            }
        });
        divider(ui);
        setting_row(ui, "Blocked content", Some("Songs and artists you have blocked. They will be skipped automatically during playback."), |ui| {
            let count = app.blocked_tracks.len() + app.blocked_artists.len();
            ui.label(RichText::new(format!("{count} blocked")).color(theme::text_dim()));
        });
        if !app.blocked_tracks.is_empty() || !app.blocked_artists.is_empty() {
            ui.add_space(4.0);
            let mut unblock_t = None;
            for (id, t) in &app.blocked_tracks {
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    let (i_rect, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                    super::widgets::draw_minus_circle(ui, i_rect.center(), 7.0, Color32::from_rgb(239, 68, 68), 1.3);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&t.title).font(theme::semibold(13.0)).color(theme::text()));
                        ui.label(RichText::new(&t.artist).font(theme::regular(11.0)).color(theme::text_dim()));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("Unblock").font(theme::regular(12.0))).clicked() {
                            unblock_t = Some(id.clone());
                        }
                    });
                });
                ui.add_space(6.0);
            }
            if let Some(id) = unblock_t {
                app.unblock_track(&id);
            }

            let mut unblock_a = None;
            for (id, a) in &app.blocked_artists {
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    let (i_rect, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                    super::widgets::draw_minus_circle(ui, i_rect.center(), 7.0, Color32::from_rgb(239, 68, 68), 1.3);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&a.name).font(theme::semibold(13.0)).color(theme::text()));
                        ui.label(RichText::new("Artist blocked").font(theme::regular(11.0)).color(theme::text_dim()));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("Unblock").font(theme::regular(12.0))).clicked() {
                            unblock_a = Some(id.clone());
                        }
                    });
                });
                ui.add_space(6.0);
            }
            if let Some(id) = unblock_a {
                app.unblock_artist(&id);
            }
        }
    });
}

/// Settings › Equalizer.
fn settings_equalizer(app: &mut App, ui: &mut Ui) {
    use crate::audio::eq;
    section_label(ui, "Equalizer", None);
    settings_card(ui, |ui| {
        setting_row(ui, "Equalizer", Some("10-band graphic equalizer with smooth presets. Shapes only what you hear locally."), |ui| {
            if toggle(ui, &mut app.eq_on, "Equalizer").changed() {
                app.set_eq();
            }
        });
        ui.add_enabled_ui(app.eq_on, |ui| {
            divider(ui);
            setting_row(ui, "Preset", None, |ui| {
                let btn = dropdown_button(ui, eq::preset_name(&app.eq_gains).unwrap_or("Custom"), 180.0, "Equalizer preset");
                let mut pick = None;
                egui::Popup::menu(&btn).show(|ui| {
                    ui.set_width(200.0);
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    ui.add_space(4.0);
                    for (name, gains) in &eq::PRESETS {
                        if option_row(ui, name, app.eq_gains == *gains).clicked() {
                            pick = Some(*gains);
                            ui.close();
                        }
                    }
                });
                if let Some(g) = pick {
                    app.eq_gains = g;
                    app.set_eq();
                }
            });
            divider(ui);
            if eq_bands(ui, &mut app.eq_gains) {
                app.set_eq();
            }
            divider(ui);
            setting_row(ui, "Prevent clipping", Some("Attenuate by the highest band boost so loud tracks do not crackle."), |ui| {
                if toggle(ui, &mut app.eq_no_clip, "Prevent clipping").changed() {
                    app.set_eq();
                }
            });
        });
    });
}

/// The equalizer's bands as vertical sliders over the resulting frequency response curve.
fn eq_bands(ui: &mut Ui, gains: &mut crate::audio::eq::Gains) -> bool {
    use crate::audio::eq::{BANDS, MAX_DB, response};
    const LABELS: [&str; 10] = ["31", "62", "125", "250", "500", "1k", "2k", "4k", "8k", "16k"];
    const NAMES: [&str; 10] = ["31 Hz", "62 Hz", "125 Hz", "250 Hz", "500 Hz", "1 kHz", "2 kHz", "4 kHz", "8 kHz", "16 kHz"];
    const GUTTER: f32 = 34.0;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 210.0), Sense::hover());
    let fade = if ui.is_enabled() { 1.0 } else { 0.45 };
    let accent = ui.ctx().data(|d| d.get_temp::<Color32>(egui::Id::unique("accent"))).unwrap_or(theme::RED);
    let p = ui.painter();
    let top = rect.top() + 26.0;
    let bottom = rect.bottom() - 30.0;
    let mid = (top + bottom) / 2.0;
    let y_of = |db: f32| mid - db / MAX_DB * (bottom - top) / 2.0;
    let col_w = (rect.width() - GUTTER) / BANDS.len() as f32;
    let x_of = |i: f32| rect.left() + GUTTER + col_w * (i + 0.5);
    for (db, text) in [(MAX_DB, "+12"), (0.0, "0"), (-MAX_DB, "\u{2212}12")] {
        p.text(pos2(rect.left(), y_of(db)), Align2::LEFT_CENTER, text, theme::regular(11.5), theme::with_alpha(theme::text_dim(), fade));
    }
    p.line_segment([pos2(x_of(-0.5), mid), pos2(x_of(9.5), mid)], egui::Stroke::new(1.0, theme::glass(0.10 * fade)));
    let db_at = response(gains);
    let curve: Vec<egui::Pos2> = (0..=180)
        .map(|k| {
            let i = 9.0 * k as f32 / 180.0;
            pos2(x_of(i), y_of(db_at(BANDS[0] * 2f32.powf(i))))
        })
        .collect();
    ui.painter_at(rect).add(egui::Shape::line(curve, egui::Stroke::new(2.0, theme::with_alpha(accent, 0.55 * fade))));
    let mut changed = false;
    for (i, g) in gains.iter_mut().enumerate() {
        let x = x_of(i as f32);
        let col = Rect::from_min_max(pos2(x - col_w / 2.0, top - 8.0), pos2(x + col_w / 2.0, bottom + 8.0));
        let mut resp = ui.interact(col, ui.scope_id().with(("eq-band", i)), Sense::click_and_drag());
        let name = NAMES[i];
        let old = *g;
        if resp.double_clicked() {
            *g = 0.0;
        } else if let Some(pos) = resp.interact_pointer_pos()
            && (resp.dragged() || resp.clicked())
        {
            *g = (((mid - pos.y) / ((bottom - top) / 2.0) * MAX_DB * 2.0).round() / 2.0).clamp(-MAX_DB, MAX_DB);
        }
        if *g != old {
            changed = true;
            resp.mark_changed();
        }
        let value = *g;
        resp.widget_info(|| egui::WidgetInfo::slider(ui.is_enabled(), value as f64, name));
        let resp = resp.on_hover_cursor(egui::CursorIcon::ResizeVertical);
        let h = theme::anim_bool(ui.ctx(), resp.id.with("h"), resp.hovered() || resp.dragged() || resp.has_focus(), 0.12);
        let track = Rect::from_center_size(pos2(x, mid), vec2(4.0, bottom - top));
        p.rect_filled(track, 2.0, theme::glass(0.15 * fade));
        let knob = pos2(x, y_of(value));
        p.rect_filled(Rect::from_two_pos(pos2(x - 2.0, mid), pos2(x + 2.0, knob.y)), 2.0, theme::with_alpha(accent, fade));
        p.circle_filled(knob + vec2(0.0, 1.0), 7.0 + h, theme::shade(0.25 * fade));
        p.circle_filled(knob, 7.0 + h, theme::with_alpha(Color32::WHITE, fade));
        if h > 0.0 {
            let text = match value {
                0.0 => "0 dB".to_owned(),
                v if v > 0.0 => format!("+{v} dB"),
                v => format!("\u{2212}{} dB", -v),
            };
            p.text(pos2(x, rect.top() + 9.0), Align2::CENTER_CENTER, text, theme::semibold(12.0), theme::with_alpha(theme::text(), h * fade));
        }
        p.text(pos2(x, rect.bottom() - 12.0), Align2::CENTER_CENTER, LABELS[i], theme::regular(12.0), theme::with_alpha(theme::text_dim(), fade));
    }
    changed
}

/// The hi-res source list: one line per addon, and "+ Add a source" below it that
/// opens a fresh line for the next one.
fn hires_sources_editor(app: &mut App, ui: &mut Ui) {
    let mut edited = false;
    let mut remove = None;
    for i in 0..app.hires_sources.len() {
        ui.horizontal(|ui| {
            let field_w = (ui.available_width() - 34.0).max(120.0);
            ui.allocate_ui_with_layout(vec2(field_w, 42.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.set_width(field_w);
                if super::text_field(ui, "", &mut app.hires_sources[i], false, "https://your-addon.example").changed() {
                    edited = true;
                }
            });
            if widgets::icon_button(ui, icon::X, 14.0, "Remove source", false).clicked() {
                remove = Some(i);
            }
        });
        ui.add_space(6.0);
    }
    if let Some(i) = remove {
        app.hires_sources.remove(i);
        edited = true;
    }
    if edited {
        app.backend.set_hires_source(&live_sources(app));
    }
    if app.hires_adding {
        let resp = super::text_field(ui, "", &mut app.hires_draft, false, "https://your-addon.example");
        if app.hires_focus {
            resp.request_focus();
            app.hires_focus = false;
        }
        if resp.lost_focus() {
            let link = app.hires_draft.trim().to_owned();
            if !link.is_empty() && !app.hires_sources.iter().any(|s| s.trim() == link) {
                app.hires_sources.push(link);
                app.backend.set_hires_source(&live_sources(app));
            }
            app.hires_draft.clear();
            app.hires_adding = false;
        }
        ui.add_space(6.0);
    } else if pill(ui, "+  Add a source", None, false).clicked() {
        app.hires_adding = true;
        app.hires_focus = true;
        app.hires_draft.clear();
    }
}

/// The non-empty links as one newline-separated list, the way the backend reads them.
fn live_sources(app: &App) -> String {
    app.hires_sources.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n")
}

/// Settings › Downloads.
fn settings_downloads(ui: &mut Ui) {
    section_label(ui, "Downloads", None);
    ui.label(
        RichText::new("Download songs from their menu. They're saved at the best quality your account gets, tagged with their cover art, and play from the file, even offline.")
            .font(theme::regular(13.0))
            .color(theme::text_dim()),
    );
    ui.add_space(12.0);
    settings_card(ui, |ui| {
        let folder = crate::downloads::folder().display().to_string();
        setting_row(ui, "Download folder", Some(&folder), |ui| {
            if pill(ui, "Change", None, false).clicked() {
                crate::downloads::pick_folder(ui.ctx());
            }
            ui.add_space(6.0);
            if pill(ui, "Open", Some(icon::FOLDER_OPEN), false).clicked() {
                crate::downloads::open_folder();
            }
        });
    });
}

/// Settings › Lyrics: the better-lyrics extension's options.
fn settings_lyrics(app: &mut App, ui: &mut Ui) {
    section_label(ui, "Lyrics", None);
    settings_card(ui, |ui| {
        setting_row(ui, "Romanization", Some("Japanese, Korean, Chinese, Cyrillic and other scripts in Latin letters. Timed with the lyrics when the source has it."), |ui| {
            toggle(ui, &mut app.show_roman, "Romanization");
        });
        divider(ui);
        setting_row(ui, "Translation", Some("Translated by Unison, or Google when it can't. Lyrics that ship their own translation use it."), |ui| {
            toggle(ui, &mut app.show_trans, "Translation");
        });
        divider(ui);
        setting_row(ui, "Translate into", None, |ui| {
            let current = super::lyric_prefs::LANGUAGES.iter().find(|(c, _)| *c == app.lyric_prefs.target).map_or(app.lyric_prefs.target.as_str(), |(_, n)| n);
            let btn = dropdown_button(ui, current, 200.0, "Translate into");
            let just_opened = btn.clicked();
            egui::Popup::menu(&btn).show(|ui| {
                ui.set_width(200.0);
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                    ui.add_space(4.0);
                    for (code, name) in super::lyric_prefs::LANGUAGES {
                        let on = app.lyric_prefs.target == *code;
                        let row = option_row(ui, name, on);
                        if on && just_opened {
                            row.scroll_to_me(Some(egui::Align::Center));
                        }
                        if row.clicked() {
                            app.lyric_prefs.target = (*code).to_owned();
                            ui.close();
                        }
                    }
                    ui.add_space(4.0);
                });
            });
        });
        divider(ui);
        setting_row(ui, "Text size", Some("Make the lyrics larger or smaller. Works on top of the theme's own size."), |ui| {
            let (min, max, step) = super::lyric_prefs::TEXT_SCALE;
            let v = &mut app.lyric_prefs.text_scale;
            if ui.add_enabled(*v < max - 0.01, egui::Button::new("+").min_size(vec2(28.0, 26.0))).on_hover_text("Larger").clicked() {
                *v = ((*v + step) * 10.0).round() / 10.0;
            }
            ui.label(egui::RichText::new(format!("{:.0}%", *v * 100.0)).font(theme::semibold(13.0)));
            if ui.add_enabled(*v > min + 0.01, egui::Button::new("−").min_size(vec2(28.0, 26.0))).on_hover_text("Smaller").clicked() {
                *v = ((*v - step) * 10.0).round() / 10.0;
            }
        });
        divider(ui);
        // Offsets: seconds taken off the lyrics clock, as the extension's dock sets them.
        for (title, desc, which) in [
            ("Timing offset", "Shift all lyrics later (+) or earlier (−).", 0),
            ("Word-synced trim", "Extra shift for syllable and word-synced lyrics.", 1),
            ("Line-synced trim", "Extra shift for line-synced lyrics.", 2),
        ] {
            setting_row(ui, title, Some(desc), |ui| {
                let v = match which {
                    0 => &mut app.lyric_prefs.global_offset,
                    1 => &mut app.lyric_prefs.word_trim,
                    _ => &mut app.lyric_prefs.line_trim,
                };
                if ui.add(egui::Button::new("+").min_size(vec2(28.0, 26.0))).on_hover_text("Later").clicked() {
                    *v = ((*v + 0.1) * 10.0).round() / 10.0;
                }
                ui.label(egui::RichText::new(format!("{:+.1} s", *v).replace("+0.0", "0.0")).font(theme::semibold(13.0)));
                if ui.add(egui::Button::new("−").min_size(vec2(28.0, 26.0))).on_hover_text("Earlier").clicked() {
                    *v = ((*v - 0.1) * 10.0).round() / 10.0;
                }
            });
            divider(ui);
        }
        // Source order: the first enabled source with lyrics for a song is shown; the
        // lyrics toolbar can pick another for that song.
        setting_row(ui, "Lyrics sources", Some("Tried in this order. Switch one off to skip it; the toolbar under the lyrics picks a different one for a single song."), |_| {});
        let n = app.lyric_prefs.order.len();
        let mut swap: Option<(usize, usize)> = None;
        for i in 0..n {
            let (p, on) = app.lyric_prefs.order[i];
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::hover());
            let mut row = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(egui::Layout::left_to_right(egui::Align::Center)));
            let mut enabled = on;
            if toggle(&mut row, &mut enabled, p.name()).changed() {
                app.lyric_prefs.order[i].1 = enabled;
            }
            row.add_space(10.0);
            let kind = match p.sync() {
                crate::lyrics::SyncKind::Syllable => "syllable",
                crate::lyrics::SyncKind::Word => "word",
                crate::lyrics::SyncKind::Line => "line",
                crate::lyrics::SyncKind::Unsynced => "plain",
            };
            row.label(egui::RichText::new(p.name()).font(theme::semibold(13.5)).color(if on { theme::text() } else { theme::text_dim() }));
            row.label(egui::RichText::new(kind).font(theme::regular(12.0)).color(theme::text_dim()));
            row.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add_enabled(i + 1 < n, egui::Button::new("↓").min_size(vec2(28.0, 24.0))).on_hover_text("Lower").clicked() {
                    swap = Some((i, i + 1));
                }
                if ui.add_enabled(i > 0, egui::Button::new("↑").min_size(vec2(28.0, 24.0))).on_hover_text("Higher").clicked() {
                    swap = Some((i, i - 1));
                }
            });
        }
        if let Some((a, b)) = swap {
            app.lyric_prefs.order.swap(a, b);
        }
        ui.add_space(6.0);
    });
}

/// Settings › Artist pages: the listener's own signature service.
fn settings_artists(app: &mut App, ui: &mut Ui) {
    section_label(ui, "Artist pages", None);
    settings_card(ui, |ui| {
        setting_row(ui, "Signature service", Some("A link to your own signature API. Artists get their handwritten signature, with “Signed by …” above it — beside About when Apple also letters them a wordmark; without a signature, their name is set in type."), |_| {});
        divider(ui);
        ui.add_space(12.0);
        if super::text_field(ui, "Link", &mut app.signature_api, false, "https://your-signature-service.example").changed() {
            app.signature_reset();
        }
        ui.add_space(12.0);
    });
}

/// Settings › Appearance.
fn settings_appearance(app: &mut App, ui: &mut Ui) {
    section_label(ui, "Appearance", None);
    settings_card(ui, |ui| {
        let desc = if cfg!(windows) {
            "Light or dark pages. System follows Windows' app mode. Now playing stays dark, over the album art."
        } else {
            "Light or dark pages. System follows your system's setting. Now playing stays dark, over the album art."
        };
        let mut pick = None;
        setting_row(ui, "Mode", Some(desc), |ui| {
            let btn = dropdown_button(ui, app.appearance.label(), 160.0, "Mode");
            egui::Popup::menu(&btn).show(|ui| {
                ui.set_width(160.0);
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                ui.add_space(4.0);
                for a in theme::Appearance::ALL {
                    if option_row(ui, a.label(), a == app.appearance).clicked() {
                        pick = Some(a);
                        ui.close();
                    }
                }
                ui.add_space(4.0);
            });
        });
        if let Some(a) = pick {
            app.appearance = a;
        }
        divider(ui);
        setting_row(ui, "Animated background", Some("The album art slowly drifts behind everything."), |ui| {
            toggle(ui, &mut app.anim_bg, "Animated background");
        });
        divider(ui);
        setting_row(
            ui,
            "Liquid Glass (rust-glass)",
            Some("GPU-rendered refraction, backdrop blur and specular rim lighting on floating bars (inspired by MasterKPG/rust-glass). Turn off to save GPU power."),
            |ui| {
                toggle(ui, &mut app.liquid_glass, "Liquid Glass");
            },
        );
        divider(ui);
        setting_row(ui, "Pulse with the music", Some("The background swells with the beat."), |ui| {
            toggle(ui, &mut app.reactive_bg, "Pulse with the music");
        });
        divider(ui);
        setting_row(ui, "Album cover glow", Some("A soft, ambient light glow radiating from the album artwork in full view."), |ui| {
            toggle(ui, &mut app.cover_glow, "Album cover glow");
        });
        divider(ui);
        setting_row(ui, "Font", Some("For the whole app and the lyrics. Any font installed on this PC works, e.g. one made for your language."), |ui| {
            if let Some(name) = font_picker(ui) {
                theme::set_font_choice(&name);
                app.chrome_applied = None;
            }
        });
        divider(ui);
        let hidden = app.hidden_playlists.len();
        let desc = match hidden {
            0 => "Nothing hidden. Right-click a playlist in the sidebar to hide it from there.".to_owned(),
            1 => "1 playlist is hidden from the sidebar. Right-click a playlist there to hide or show it.".to_owned(),
            n => format!("{} playlists are hidden from the sidebar. Right-click a playlist there to hide or show it.", thousands(n as u64)),
        };
        setting_row(ui, "Hidden playlists", Some(&desc), |ui| {
            if hidden > 0 && pill(ui, "Show all", None, false).clicked() {
                app.hidden_playlists.clear();
            }
        });
    });
}

/// Settings › Window.
fn settings_window(app: &mut App, ui: &mut Ui) {
    section_label(ui, "Window", None);
    settings_card(ui, |ui| {
        setting_row(ui, "Start with Windows", Some("Launch BitChord automatically when you sign in to Windows."), |ui| {
            if toggle(ui, &mut app.autostart, "Start with Windows").changed() {
                crate::autostart::set_enabled(app.autostart);
            }
        });
        divider(ui);
        setting_row(ui, "Close to tray", Some("Clicking × hides the window and keeps the music playing. Click the tray icon to bring it back, or right-click it to quit."), |ui| {
            toggle(ui, &mut app.close_to_tray, "Close to tray");
        });
        divider(ui);
        setting_row(ui, "Collapse sidebar", Some("Show only icons in the sidebar, for narrow windows. The menu button at its top does the same."), |ui| {
            toggle(ui, &mut app.sidebar_collapsed, "Collapse sidebar");
        });
        divider(ui);
        let api = crate::renderer::api_name();
        let title = format!("Draw with {api}");
        let desc = format!(
            "Use {api} (wgpu) instead of OpenGL. It isn't faster for this app and takes about three times the memory; try it if OpenGL draws wrong on this PC. If {api} won't start, the app goes back to OpenGL."
        );
        setting_row(ui, &title, Some(&desc), |ui| {
            if toggle(ui, &mut app.wgpu_renderer, &title).changed() {
                use crate::renderer::Renderer;
                let r = if app.wgpu_renderer { Renderer::Wgpu } else { Renderer::OpenGl };
                if let Err(e) = crate::renderer::save(app.backend.data_dir(), r) {
                    log::warn!("renderer: couldn't save the setting: {e}");
                }
            }
        });
        if app.wgpu_renderer != (crate::renderer::active() == crate::renderer::Renderer::Wgpu) {
            divider(ui);
            let mut restart = false;
            setting_row(ui, "Restart to apply", Some("The app draws with the new renderer from the next start."), |ui| {
                restart = pill(ui, "Restart now", None, true).clicked();
            });
            if restart {
                app.restart(ui.ctx());
            }
        }
    });
}

/// Settings › Connections.
fn settings_connections(app: &mut App, ui: &mut Ui) {
    let mut discord_enabled = app.discord_enabled;
    section_label(ui, "Connections", None);
    settings_card(ui, |ui| {
        setting_row(ui, "Discord", Some("Show what you're playing on your Discord profile."), |ui| {
            toggle(ui, &mut discord_enabled, "Discord");
        });
        divider(ui);
        let busy = app.lastfm_busy.load(Ordering::Relaxed);
        let desc = match &app.lastfm_user {
            Some(name) => format!("Scrobbling what you play to {name}'s profile."),
            None if busy => "Allow BitChord on the Last.fm page that opened in your browser.".to_owned(),
            None => "Scrobble what you play to your Last.fm profile.".to_owned(),
        };
        let (mut connect, mut disconnect) = (false, false);
        setting_row(ui, "Last.fm", Some(&desc), |ui| {
            if app.lastfm_user.is_some() {
                disconnect = pill(ui, "Disconnect", None, false).clicked();
            } else if busy {
                ui.spinner();
            } else {
                connect = pill(ui, "Connect", None, false).clicked();
            }
        });
        if connect {
            app.lastfm_sign_in();
        }
        if disconnect {
            app.lastfm_sign_out();
        }
    });
    if discord_enabled != app.discord_enabled {
        app.set_discord_enabled(discord_enabled);
    }
}

/// Settings › Keyboard shortcuts.
fn settings_shortcuts(ui: &mut Ui) {
    section_label(ui, "Keyboard shortcuts", None);
    settings_card(ui, |ui| {
        let keys: &[(&str, &[&str])] = &[
            ("Play / pause", &["Space"]),
            ("Seek 5 s back / forward", &["← / →"]),
            ("Next / previous song", &["Ctrl", "→ / ←"]),
            ("Volume up / down", &["Ctrl", "↑ / ↓"]),
            ("Search", &["Ctrl", "F"]),
            ("Lyrics", &["L"]),
            ("Queue", &["Q"]),
            ("Full-screen lyrics (in now playing)", &["F"]),
            ("Back / forward", &["Alt", "← / →"]),
            ("Close now playing", &["Esc"]),
        ];
        for (i, (what, combo)) in keys.iter().enumerate() {
            if i > 0 {
                divider(ui);
            }
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::hover());
            ui.painter().text(pos2(rect.left(), rect.center().y), Align2::LEFT_CENTER, *what, theme::regular(14.0), theme::text());
            let mut x = rect.right();
            for (j, k) in combo.iter().enumerate().rev() {
                x = keycap(ui, pos2(x, rect.center().y), k) - 6.0;
                if j > 0 {
                    let plus = ui.painter().text(pos2(x, rect.center().y), Align2::RIGHT_CENTER, "+", theme::regular(13.0), theme::text_dim());
                    x = plus.left() - 6.0;
                }
            }
        }
    });
}

/// Settings › Network: the proxy everything goes through, a connection test, and a
/// restart when the setting changed.
fn network(app: &mut App, ui: &mut Ui) {
    use crate::proxy::{self, Setting};
    let same = |a: &Setting, b: &Setting| std::mem::discriminant(a) == std::mem::discriminant(b);
    let mut pick = None;
    setting_row(ui, "Proxy", Some("Send everything BitChord fetches through a proxy: songs, lyrics, sign-in and Listen together."), |ui| {
        let btn = dropdown_button(ui, app.proxy_mode.label(), 220.0, "Proxy");
        egui::Popup::menu(&btn).show(|ui| {
            ui.set_width(220.0);
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            ui.add_space(4.0);
            for s in [Setting::System, Setting::Off, Setting::Custom(String::new())] {
                if option_row(ui, s.label(), same(&s, &app.proxy_mode)).clicked() {
                    pick = Some(s);
                    ui.close();
                }
            }
            ui.add_space(4.0);
        });
    });
    if let Some(s) = pick {
        app.proxy_error = None;
        *app.proxy_test.lock().unwrap() = None;
        app.proxy_mode = s.clone();
        // System and Off take effect as picked; a custom address once it's saved.
        if !matches!(s, Setting::Custom(_)) {
            match proxy::save(app.backend.data_dir(), &s) {
                Ok(()) => app.proxy_saved = s,
                Err(e) => app.proxy_error = Some(format!("{e:#}")),
            }
        }
    }
    match &app.proxy_mode {
        Setting::System => {
            divider(ui);
            let desc = match proxy::system() {
                Some(p) => format!("Windows is set to use {}.", proxy::redact(&p)),
                None => "Windows isn't set to use a proxy, so BitChord connects directly.".to_owned(),
            };
            setting_row(ui, "Windows proxy", Some(&desc), |_| {});
        }
        Setting::Custom(_) => {
            divider(ui);
            ui.add_space(12.0);
            super::text_field(ui, "Address", &mut app.proxy_url, false, "http://127.0.0.1:8080 or socks5://user:pass@host:1080");
            ui.add_space(10.0);
            let custom = Setting::Custom(app.proxy_url.trim().to_owned());
            let saved = app.proxy_saved == custom;
            let mut save = false;
            ui.horizontal(|ui| {
                save = pill(ui, if saved { "Saved" } else { "Save" }, None, !saved).clicked() && !saved;
                ui.add_space(8.0);
                ui.label(RichText::new("A password in the address is kept in Windows Credential Manager.").font(theme::regular(12.5)).color(theme::text_dim()));
            });
            if save {
                match proxy::save(app.backend.data_dir(), &custom) {
                    Ok(()) => {
                        app.proxy_saved = custom;
                        app.proxy_error = None;
                    }
                    Err(e) => app.proxy_error = Some(format!("{e:#}")),
                }
            }
            ui.add_space(12.0);
        }
        Setting::Off => {}
    }
    if let Some(e) = &app.proxy_error {
        ui.label(RichText::new(e).font(theme::regular(12.5)).color(theme::RED));
        ui.add_space(8.0);
    }
    divider(ui);
    let result = app.proxy_test.lock().unwrap().clone();
    let desc = match &result {
        Some(Some((_, msg))) => msg.clone(),
        Some(None) => "Checking…".to_owned(),
        None => "Checks that YouTube Music answers through this setting.".to_owned(),
    };
    setting_row(ui, "Test connection", Some(&desc), |ui| {
        if matches!(result, Some(None)) {
            ui.spinner();
        } else if pill(ui, "Test", None, false).clicked() {
            let s = match &app.proxy_mode {
                Setting::Custom(_) => Setting::Custom(app.proxy_url.trim().to_owned()),
                other => other.clone(),
            };
            let (slot, b) = (app.proxy_test.clone(), app.backend.clone());
            *slot.lock().unwrap() = Some(None);
            app.backend.spawn(async move {
                let out = match proxy::test(&s).await {
                    Ok(t) => (true, format!("Working: YouTube Music answered in {} ms.", t.as_millis())),
                    Err(e) => (false, format!("Didn't connect: {e:#}")),
                };
                *slot.lock().unwrap() = Some(Some(out));
                b.wake();
            });
        }
    });
    if app.proxy_saved != *proxy::active() {
        divider(ui);
        let mut restart = false;
        setting_row(ui, "Restart to apply", Some("The new proxy setting is used from the next start."), |ui| {
            restart = pill(ui, "Restart now", None, true).clicked();
        });
        if restart {
            app.restart(ui.ctx());
        }
    }
}

/// Paint `v` into `rect`, from the ICO entry that fits its size in physical pixels.
#[allow(dead_code)]
pub(super) fn paint_app_icon(ui: &Ui, v: &crate::appicon::Variant, rect: Rect) {
    let px = (rect.width() * ui.ctx().pixels_per_point()).ceil() as u32;
    let png = v.png(px);
    egui::Image::from_bytes(format!("bytes://app-icon/{}-{}.png", v.id, png.len()), png).paint_at(ui, rect);
}

/// The font setting: the theme's own, a bundled face, or one installed on this PC.
/// Returns the new choice ("" for the theme's).
fn font_picker(ui: &mut Ui) -> Option<String> {
    let current = theme::font_choice();
    let btn = dropdown_button(ui, if current.is_empty() { "Theme's font" } else { &current }, 220.0, "Font");
    let just_opened = btn.clicked();
    let mut picked = None;
    egui::Popup::menu(&btn).show(|ui| {
        ui.set_width(220.0);
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
            ui.add_space(4.0);
            let mut row = |ui: &mut Ui, label: &str, value: &str| {
                let on = current.eq_ignore_ascii_case(value);
                let r = option_row(ui, label, on);
                if on && just_opened {
                    r.scroll_to_me(Some(egui::Align::Center));
                }
                if r.clicked() {
                    picked = Some(value.to_owned());
                    ui.close();
                }
            };
            row(ui, "Theme's font", "");
            for name in theme::BUNDLED_FONTS {
                row(ui, name, name);
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                ui.label(RichText::new("Installed on this PC").font(theme::semibold(12.0)).color(theme::text_faint()));
            });
            ui.add_space(4.0);
            let ctx = ui.ctx().clone();
            match theme::installed_fonts(move || ctx.request_repaint()) {
                Some(fonts) => {
                    for f in fonts {
                        row(ui, &f.name, &f.name);
                    }
                }
                None => {
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        ui.label(RichText::new("Looking for fonts…").font(theme::regular(13.0)).color(theme::text_dim()));
                    });
                }
            }
            ui.add_space(4.0);
        });
    });
    picked.filter(|p| *p != current)
}

const SYSTEM_DEFAULT: &str = "Windows default";

fn output_device_picker(app: &mut App, ui: &mut Ui) {
    let current = match &app.output_device {
        None => SYSTEM_DEFAULT.to_owned(),
        Some((id, name)) => match app.output_devices.iter().find(|d| d.id == *id) {
            Some(d) => d.name.clone(),
            None if app.output_devices.is_empty() => name.clone(),
            None => format!("{name} (not connected)"),
        },
    };
    let btn = dropdown_button(ui, &current, 210.0, "Output device");
    // Listed when opened, so a device that just connected shows up.
    if btn.clicked() || app.output_devices.is_empty() {
        app.output_devices = crate::audio::output_devices();
    }
    let mut pick: Option<Option<(String, String)>> = None;
    egui::Popup::menu(&btn).show(|ui| {
        ui.set_width(340.0);
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            ui.add_space(4.0);
            if option_row(ui, SYSTEM_DEFAULT, app.output_device.is_none()).clicked() {
                pick = Some(None);
                ui.close();
            }
            for d in &app.output_devices {
                if option_row(ui, &d.name, app.output_device.as_ref().is_some_and(|(id, _)| *id == d.id)).clicked() {
                    pick = Some(Some((d.id.clone(), d.name.clone())));
                    ui.close();
                }
            }
            ui.add_space(4.0);
        });
    });
    if let Some(pick) = pick {
        app.audio.send(crate::audio::AudioCmd::SetDevice(pick.as_ref().map(|(id, _)| id.clone())));
        app.output_device = pick;
    }
}

/// A dropdown's closed state: the chosen value and a chevron on a glass pill.
fn dropdown_button(ui: &mut Ui, value: &str, width: f32, name: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 36.0), Sense::click());
    let resp = widgets::named(resp, name).on_hover_cursor(egui::CursorIcon::PointingHand);
    let open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&resp));
    let h = theme::anim_bool(ui.ctx(), resp.id.with("h"), resp.hovered() || open, 0.12);
    ui.painter().rect_filled(rect, 10.0, theme::glass(0.08 + 0.05 * h));
    widgets::text_at(ui, pos2(rect.left() + 14.0, rect.center().y - 9.0), value, theme::regular(14.0), theme::text(), rect.width() - 44.0);
    let glyph = if open { theme::icon::CHEVRON_UP } else { theme::icon::CHEVRON_DOWN };
    widgets::icon_at(ui, pos2(rect.right() - 18.0, rect.center().y), glyph, 15.0, theme::lerp_color(theme::text_dim(), theme::text(), h));
    resp
}

/// A dropdown option: hover glass, and a check on the chosen one.
fn option_row(ui: &mut Ui, label: &str, selected: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click());
    let resp = widgets::named(resp, label).on_hover_cursor(egui::CursorIcon::PointingHand);
    let h = theme::anim_bool(ui.ctx(), resp.id.with("h"), resp.hovered(), 0.12);
    if h > 0.0 || selected {
        ui.painter().rect_filled(rect, 8.0, theme::glass(0.05 * selected as u8 as f32 + 0.08 * h));
    }
    let font = if selected { theme::semibold(14.0) } else { theme::regular(14.0) };
    widgets::text_at(ui, pos2(rect.left() + 12.0, rect.center().y - 9.0), label, font, theme::text(), rect.width() - 44.0);
    if selected {
        widgets::icon_at(ui, pos2(rect.right() - 16.0, rect.center().y), theme::icon::CHECK, 15.0, theme::text());
    }
    resp
}

/// 12345 -> "12,345".
pub(super) fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Small heading above a settings card, with an optional value on the right.
pub(super) fn section_label(ui: &mut Ui, title: &str, value: Option<&str>) {
    ui.add_space(30.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
    ui.painter().text(pos2(rect.left() + 2.0, rect.center().y), Align2::LEFT_CENTER, title, theme::bold(17.0), theme::text());
    if let Some(v) = value {
        ui.painter().text(pos2(rect.right() - 2.0, rect.center().y), Align2::RIGHT_CENTER, v, theme::semibold(13.0), theme::text_dim());
    }
    ui.add_space(10.0);
}

pub(super) fn settings_card(ui: &mut Ui, body: impl FnOnce(&mut Ui)) {
    egui::Frame::new()
        .fill(theme::glass(0.06))
        .stroke(egui::Stroke::new(1.0, theme::glass(0.05)))
        .corner_radius(theme::panel_radius())
        .inner_margin(egui::Margin::symmetric(20, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            body(ui);
        });
}

/// A settings row: title and description on the left, the control on the right.
pub(super) fn setting_row(ui: &mut Ui, title: &str, desc: Option<&str>, control: impl FnOnce(&mut Ui)) {
    let w = ui.available_width();
    let text_w = (w - 230.0).max(200.0);
    let title_g = widgets::galley(ui, title, theme::semibold(14.5), theme::text(), text_w);
    let desc_g = desc.map(|d| widgets::galley_wrapped(ui, d, theme::regular(12.5), theme::text_dim(), text_w, 3));
    let text_h = title_g.size().y + desc_g.as_ref().map_or(0.0, |g| g.size().y + 3.0);
    let h = (text_h + 26.0).max(58.0);
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let fade = if ui.is_enabled() { 1.0 } else { 0.45 };
    let top = rect.center().y - text_h / 2.0;
    ui.painter().galley(pos2(rect.left(), top), title_g.clone(), theme::with_alpha(theme::text(), fade));
    if let Some(g) = desc_g {
        ui.painter().galley(pos2(rect.left(), top + title_g.size().y + 3.0), g, theme::with_alpha(theme::text_dim(), fade));
    }
    let ctl = Rect::from_min_max(pos2(rect.left() + text_w + 20.0, rect.top()), rect.max);
    let mut cui = ui.new_child(egui::UiBuilder::new().max_rect(ctl).layout(egui::Layout::right_to_left(egui::Align::Center)));
    control(&mut cui);
}

pub(super) fn divider(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, theme::glass(0.07));
}

/// Modern elongated mobile switch with Material 3 / iOS liquid physics, size-morphing knob and accent glow.
pub(super) fn toggle(ui: &mut Ui, on: &mut bool, name: &str) -> egui::Response {
    let (rect, mut resp) = ui.allocate_exact_size(vec2(50.0, 28.0), Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let value = *on;
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, ui.is_enabled(), value, name));
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let t = theme::anim_bool(ui.ctx(), resp.id.with("t"), value, 0.22);
    let h = theme::anim_bool(ui.ctx(), resp.id.with("h"), resp.hovered(), 0.12);
    let accent = ui.ctx().data(|d| d.get_temp::<Color32>(egui::Id::unique("accent"))).unwrap_or(theme::RED);
    let fade = if ui.is_enabled() { 1.0 } else { 0.4 };

    // Subtly glow when active
    if t > 0.05 {
        super::widgets::soft_shadow(ui, rect, 14.0, 10.0, 0.25 * t * fade);
    }

    let off = theme::glass(0.12 + 0.06 * h);
    ui.painter().rect_filled(rect, 14.0, theme::with_alpha(theme::lerp_color(off, accent, t), fade));
    if t < 0.95 {
        let border_alpha = (1.0 - t) * 0.20;
        ui.painter().rect_stroke(rect, 14.0, egui::Stroke::new(1.2, theme::glass(border_alpha)), egui::StrokeKind::Inside);
    }

    // Modern mobile size-morphing knob: 16px off -> 22px on, with liquid stretch when pressed
    let pressing = resp.is_pointer_button_down_on();
    let stretch_x = if pressing { 1.25 } else { 1.0 };
    let knob_size = egui::lerp(16.0..=22.0, t);
    let knob_w = knob_size * stretch_x;
    let knob_h = knob_size;
    let knob_center_x = egui::lerp((rect.left() + 14.0)..=(rect.right() - 14.0), t);
    let knob_rect = Rect::from_center_size(pos2(knob_center_x, rect.center().y), vec2(knob_w, knob_h));
    let knob_color = theme::lerp_color(theme::text_dim(), Color32::WHITE, t);

    // Knob drop shadow & surface
    ui.painter().rect_filled(knob_rect.translate(vec2(0.0, 1.2)), knob_h / 2.0, theme::shade(0.30 * fade));
    ui.painter().rect_filled(knob_rect, knob_h / 2.0, theme::with_alpha(knob_color, fade));
    resp
}

/// Whole seconds on a track like the volume slider's, with the ends labelled and the
/// value beside it: click or drag to set (scrolling over it scrolls the page).
fn seconds_slider(ui: &mut Ui, v: &mut u32, range: std::ops::RangeInclusive<u32>, name: &str) -> egui::Response {
    let (lo, hi) = (*range.start(), *range.end());
    let label = |ui: &mut Ui, text: String, w: f32, font: egui::FontId, color: Color32| {
        let (rect, _) = ui.allocate_exact_size(vec2(w, 26.0), Sense::hover());
        let g = widgets::galley(ui, &text, font, color, w);
        ui.painter().galley(pos2(rect.right() - g.size().x, rect.center().y - g.size().y / 2.0), g, color);
    };
    label(ui, format!("{v} s"), 34.0, theme::semibold(13.0), theme::text());
    ui.add_space(6.0);
    label(ui, format!("{hi} s"), 26.0, theme::regular(12.0), theme::text_dim());
    let (rect, mut resp) = ui.allocate_exact_size(vec2(170.0, 26.0), Sense::click_and_drag());
    label(ui, format!("{lo} s"), 22.0, theme::regular(12.0), theme::text_dim());

    let old = *v;
    let track_w = rect.width() - 16.0;
    let track_x = Rect::from_center_size(rect.center(), vec2(track_w, 4.0));
    if let Some(p) = resp.interact_pointer_pos()
        && (resp.dragged() || resp.clicked())
    {
        let t = ((p.x - track_x.left()) / track_x.width()).clamp(0.0, 1.0);
        *v = (lo as f32 + t * (hi - lo) as f32).round() as u32;
    }
    if *v != old {
        resp.mark_changed();
    }
    let value = *v;
    resp.widget_info(|| egui::WidgetInfo::slider(ui.is_enabled(), value as f64, name));
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let h = theme::anim_bool(ui.ctx(), resp.id.with("h"), resp.hovered() || resp.dragged(), 0.15);
    let track_h = egui::lerp(4.0..=8.0, h);
    let track = Rect::from_center_size(rect.center(), vec2(track_w, track_h));

    let accent = ui.ctx().data(|d| d.get_temp::<Color32>(egui::Id::unique("accent"))).unwrap_or(theme::RED);
    let t = (value - lo) as f32 / (hi - lo) as f32;
    let knob = pos2(egui::lerp(track.left()..=track.right(), t), track.center().y);

    // Background track & filled accent section
    ui.painter().rect_filled(track, track_h / 2.0, theme::glass(0.15));
    if knob.x > track.left() {
        ui.painter().rect_filled(Rect::from_min_max(track.min, pos2(knob.x, track.max.y)), track_h / 2.0, accent);
    }
    let knob_r = 7.0 + 1.5 * h;
    ui.painter().circle_filled(knob + vec2(0.0, 1.0), knob_r, theme::shade(0.28));
    ui.painter().circle_filled(knob, knob_r, Color32::WHITE);
    resp
}

/// Draw a key cap whose right edge is at `right`; returns its left edge.
fn keycap(ui: &Ui, right: egui::Pos2, key: &str) -> f32 {
    let g = widgets::galley(ui, key, theme::semibold(12.5), theme::text(), 200.0);
    let size = vec2((g.size().x + 18.0).max(30.0), 26.0);
    let rect = Rect::from_min_size(pos2(right.x - size.x, right.y - size.y / 2.0), size);
    ui.painter().rect_filled(rect, 7.0, theme::glass(0.08));
    ui.painter().rect_stroke(rect, 7.0, egui::Stroke::new(1.0, theme::glass(0.12)), egui::StrokeKind::Inside);
    ui.painter().rect_filled(Rect::from_min_max(pos2(rect.left() + 2.0, rect.bottom() - 2.0), pos2(rect.right() - 2.0, rect.bottom())), 2.0, theme::shade(0.25));
    ui.painter().galley(rect.center() - g.size() / 2.0, g, theme::text());
    rect.left()
}

