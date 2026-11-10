//! App chrome: background, sidebar, top bar, player bar, queue drawer, toasts.

use egui::{Align2, Color32, Pos2, Rect, Sense, Ui, Vec2, pos2, vec2};

use super::widgets::{self, icon_at, icon_button, named, paint_cover, text_at};
use super::{Action, App, LibTab, NpTab, Page, Repeat, fmt_time, glyphs};
use crate::model::Target;
use crate::shader::Params;
use crate::theme::{self, icon};

// ------------------------------------------------------------------ background

/// Vertical gradient (top colour → bottom colour) over `rect`.
pub fn vgradient(painter: &egui::Painter, rect: Rect, top: Color32, bottom: Color32) {
    gradient(painter, rect, [top, top, bottom, bottom]);
}

/// Horizontal gradient (left colour → right colour) over `rect`.
pub fn hgradient(painter: &egui::Painter, rect: Rect, left: Color32, right: Color32) {
    gradient(painter, rect, [left, right, left, right]);
}

/// Corner colours: left top, right top, left bottom, right bottom.
fn gradient(painter: &egui::Painter, rect: Rect, c: [Color32; 4]) {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), c[0]);
    mesh.colored_vertex(rect.right_top(), c[1]);
    mesh.colored_vertex(rect.left_bottom(), c[2]);
    mesh.colored_vertex(rect.right_bottom(), c[3]);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    painter.add(egui::Shape::mesh(mesh));
}

/// The background mode browse pages use: the theme's own choice, else whatever now
/// playing uses.
fn page_mode(look: &crate::preset::Preset) -> crate::preset::BgMode {
    use crate::preset::{BgMode, PageBg};
    match look.app.page_bg {
        PageBg::Same => look.background.mode,
        PageBg::Static => BgMode::CoverStatic,
        PageBg::Accent => BgMode::AccentGradient,
        PageBg::Solid => BgMode::Solid,
    }
}

/// Whether the pages' background moves while music plays (static themes need no frames).
pub fn pages_bg_moves(look: &crate::preset::Preset) -> bool {
    use crate::preset::BgMode;
    let b = &look.background;
    matches!(page_mode(look), BgMode::Kawarp | BgMode::CoverFullbleed) && b.speed > 0.0 && b.intensity > 0.0
}

/// `only`: paint just this part (the rest is covered by now playing).
/// The background's flat colour, under its art and gradients.
pub fn base_color(app: &App) -> Color32 {
    app.look().solid()
}

pub fn background(app: &mut App, ui: &mut Ui, screen: Rect, now: f64, only: Option<Rect>) {
    use crate::preset::BgMode;
    let dt = (now - app.last_frame).clamp(0.0, 0.1);
    app.last_frame = now;
    let look = app.look();
    let b = &look.background;
    let mode = page_mode(look);
    // Light pages are the art at full strength under a white wash, a pale tint of the cover.
    let light = theme::light();
    let dim = if light { 1.0 } else { look.app.page_dim.unwrap_or(b.page_dim).clamp(0.02, 1.0) };
    // Now playing animates the shader itself, so keep the clock running for it too.
    let animated = matches!(b.mode, BgMode::Kawarp | BgMode::CoverFullbleed) && b.speed > 0.0;
    let moving = app.anim_bg && animated && app.playing();
    if moving {
        app.bg_time += dt * b.speed as f64;
    }
    let painter = match only {
        Some(r) => ui.painter().with_clip_rect(r),
        None => ui.painter().clone(),
    };
    painter.rect_filled(screen, 0.0, base_color(app));
    let art = matches!(mode, BgMode::Kawarp | BgMode::CoverStatic | BgMode::CoverFullbleed);
    if let Some(bg) = app.bg.as_ref().filter(|_| art) {
        let blend = theme::ease_in_out(((now - app.art_changed) / 1.4) as f32);
        if blend < 1.0 {
            // The crossfade to new art finishes even while paused.
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
        }
        let level = if app.reactive_bg && app.playing() { app.audio.status.level() * b.reactive } else { 0.0 };
        let still = mode == BgMode::CoverStatic;
        bg.paint(
            &painter,
            screen,
            Params {
                time: app.bg_time as f32,
                blend,
                intensity: if still { 0.0 } else { b.intensity * 0.9 },
                saturation: b.saturation * 0.9,
                dim,
                scale: (b.scale - 0.08).max(1.0) + if still { 0.0 } else { level.min(0.5) * 0.08 },
            },
        );
    } else if mode == BgMode::Solid {
        // Plain colour: nothing over it.
    } else {
        vgradient(&painter, screen, theme::with_alpha(app.accent_color(), 0.18 * dim / 0.5), look.solid());
    }
    if light {
        painter.rect_filled(screen, 0.0, Color32::from_white_alpha(212));
    } else if mode != BgMode::Solid {
        // Keep text readable: darken toward the bottom and under the sidebar.
        vgradient(&painter, screen, theme::shade(0.12), theme::shade(0.55));
    }
}

// ------------------------------------------------------------------ sidebar

/// How the theme draws nav items.
#[derive(Clone, Copy)]
struct NavLook {
    style: crate::preset::NavStyle,
    radius: f32,
    accent: Color32,
}

impl NavLook {
    fn of(app: &App) -> Self {
        Self { style: app.look().app.nav, radius: (app.look().chrome.radius * 0.6).clamp(4.0, 16.0), accent: app.accent_color() }
    }
}

/// A nav item's background in the theme's nav style; returns its icon colour.
/// `h` and `s` are the hover and selected animations (0..1).
fn nav_bg(ui: &Ui, look: NavLook, rect: Rect, h: f32, s: f32) -> Color32 {
    use crate::preset::NavStyle;
    let (nav_r, accent) = (look.radius, look.accent);
    let icon_fg = theme::lerp_color(theme::text_dim(), theme::text(), (h + s).min(1.0));
    match look.style {
        NavStyle::Pill => {
            ui.painter().rect_filled(rect, nav_r, theme::glass(0.05 * h + 0.10 * s));
            icon_fg
        }
        NavStyle::Accent => {
            ui.painter().rect_filled(rect, nav_r, theme::glass(0.05 * h));
            ui.painter().rect_filled(rect, nav_r, theme::with_alpha(accent, 0.22 * s));
            theme::lerp_color(icon_fg, accent, s)
        }
        NavStyle::Bar => {
            ui.painter().rect_filled(rect, nav_r, theme::glass(0.05 * h));
            let bar = Rect::from_center_size(pos2(rect.left() + 2.0, rect.center().y), vec2(3.0, 20.0 * s));
            ui.painter().rect_filled(bar, 2.0, accent);
            icon_fg
        }
    }
}

/// Sidebar width expanded and collapsed to an icon rail (points).
pub const SIDEBAR_W: f32 = 236.0;
pub const RAIL_W: f32 = 76.0;

/// How collapsed the sidebar is, 0 (full) to 1 (icon rail), eased over the toggle.
pub fn sidebar_collapse(app: &App, ctx: &egui::Context) -> f32 {
    let t = ctx.animate_bool_with_time(egui::Id::unique("sidebar-collapse"), app.sidebar_collapsed, 0.28 * theme::SPEED);
    // Ease in-out: starts and lands gently in both directions.
    t * t * (3.0 - 2.0 * t)
}

/// The small chevron pair that walks the page history, on the logo's line at the rail's
/// top right.
#[allow(dead_code)]
fn history_button(ui: &mut Ui, glyph: char, name: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::click());
    let resp = named(resp, name).on_hover_text(name).on_hover_cursor(egui::CursorIcon::PointingHand);
    let h = theme::anim_bool(ui.ctx(), resp.id.with("h"), resp.hovered(), 0.15);
    if h > 0.0 {
        ui.painter().circle_filled(rect.center(), 11.0, theme::glass(0.10 * h));
    }
    icon_at(ui, rect.center(), glyph, 16.0, theme::lerp_color(theme::text_dim(), theme::text(), h));
    resp.clicked()
}

/// One layout for both states, so collapsing animates: the panel narrows while icons
/// and covers keep their column (24 pt from the left) and the labels fade and clip.
pub fn sidebar(app: &mut App, ui: &mut Ui) {
    const ICON_X: f32 = 24.0;
    let t = sidebar_collapse(app, ui.ctx());
    let collapsed = app.sidebar_collapsed;
    // Labels fade out over the first two thirds of collapsing (and back in late).
    let a = ((1.0 - t) * 1.5 - 0.5).clamp(0.0, 1.0);
    // Content width when expanded: text keeps its truncation while the panel narrows.
    let _full_w = ui.available_width() + (SIDEBAR_W - RAIL_W) * t;

    // Logo, with the page history and collapse toggle on the same line.
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space(8.0);
        let (r, resp) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::click());
        let logo_alpha = if collapsed { 0.85 } else { a };
        mark(ui, Rect::from_center_size(r.center(), vec2(30.0, 20.0)), theme::with_alpha(theme::text(), logo_alpha));
        let resp = named(resp, if collapsed { "Expand sidebar" } else { "Home" }).on_hover_cursor(egui::CursorIcon::PointingHand);
        if resp.clicked() {
            if collapsed {
                app.sidebar_collapsed = false;
            } else {
                app.actions.push(Action::Go(Page::Home));
            }
        }
        if collapsed {
            resp.on_hover_text("Click to expand sidebar");
        }
        // Drifts to the rail's edge as it collapses.
        ui.add_space(((ui.available_width() - 84.0) * (1.0 - t)).max(0.0));
    });
    ui.add_space(14.0);

    let mut go_to: Option<Page> = None;
    let look = NavLook::of(app);
    let app_accent = app.accent_color();
    let nav = |ui: &mut Ui, glyph: &'static [u8], size: f32, label: &str, page: Page, selected: bool, is_active: bool, go_to: &mut Option<Page>| {
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 42.0), Sense::click());
        let mut resp = named(resp, label).on_hover_cursor(egui::CursorIcon::PointingHand);
        if collapsed {
            resp = resp.on_hover_text(label);
        }
        let h = theme::anim_bool(ui.ctx(), resp.id.with("h"), resp.hovered(), 0.15);
        let s = theme::anim_bool(ui.ctx(), resp.id.with("s"), selected, 0.2);
        let icon_fg = nav_bg(ui, look, rect, h, s);
        let icon_col = if is_active { app_accent } else if selected { Color32::WHITE } else { icon_fg };
        glyphs::draw(ui, pos2(rect.left() + ICON_X, rect.center().y), size, glyph, icon_col);
        if a > 0.0 {
            let fg = if is_active {
                app_accent
            } else if selected {
                Color32::WHITE
            } else {
                theme::lerp_color(theme::text_dim(), theme::text(), h)
            };
            let font = if selected || is_active { theme::semibold(15.0) } else { theme::regular(15.0) };
            ui.painter().text(pos2(rect.left() + 48.0, rect.center().y), Align2::LEFT_CENTER, label, font, theme::with_alpha(fg, a));

            if is_active {
                let spk_center = pos2(rect.right() - 16.0, rect.center().y);
                icon_at(ui, spk_center, icon::VOLUME_2, 14.0, app_accent);
            }
        }
        if resp.clicked() {
            *go_to = Some(page);
        }
    };
    let p = app.page.clone();
    let liked_active = widgets::active_state(ui, &Target::Playlist(super::lists::LIKED_MUSIC.into())).is_some();
    const APPLE: f32 = glyphs::APPLE_GLYPH;
    const LUCIDE: f32 = glyphs::LUCIDE_GLYPH;
    nav(ui, glyphs::LIBRARY, APPLE, "Library", Page::Library(LibTab::Playlists), matches!(p, Page::Library(LibTab::Playlists)), false, &mut go_to);
    nav(ui, glyphs::HEART, APPLE, "Liked Songs", Page::Library(LibTab::Songs), matches!(p, Page::Library(LibTab::Songs)), liked_active, &mut go_to);
    nav(ui, glyphs::DOWNLOAD, APPLE, "Downloads", Page::Downloads, p == Page::Downloads, false, &mut go_to);

    ui.add_space(8.0);
    let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(line, 0.0, theme::glass(0.08));
    let sep_y = line.center().y;
    ui.add_space(8.0 - 2.0 * t);

    // Playlists that appeared since the order was last changed join it at the end, so a
    // drag can place any of them.
    if let Some(l) = &app.library {
        for c in &l.playlists {
            if let Target::Playlist(id) = &c.target
                && id != "LM"
                && id != "SE"
                && !c.title.eq_ignore_ascii_case("Liked Songs")
                && !c.title.eq_ignore_ascii_case("Lieblingssongs")
                && !c.title.eq_ignore_ascii_case("Liked Music")
                && !app.playlist_order.contains(id)
            {
                app.playlist_order.push(id.clone());
            }
        }
    }
    // Drag-to-reorder and right-click-to-hide results, applied once the rows are done.
    let mut reorder: Option<(String, String)> = None;
    let mut hide: Option<String> = None;
    let transferring = super::transfer::sidebar_visible(app, ui.input(|i| i.time));
    let ready = app.updater.as_ref().and_then(|u| match u.state() {
        crate::update::State::Ready { version, .. } => Some(version),
        _ => None,
    });
    // Room under the list: transfer or updater card if active
    let bottom_h = if transferring { 88.0 } else if ready.is_some() { 60.0 } else { 8.0 };
    let list_h = (ui.available_height() - bottom_h).max(0.0);
    ui.allocate_ui(vec2(ui.available_width(), list_h), |ui| {
        if !app.logged_in {
            // Too wide to squeeze: cross-fade the message and the rail's icon.
            if t < 0.5 {
                ui.multiply_opacity(1.0 - 2.0 * t);
                ui.label(egui::RichText::new("Sign in to see your playlists").color(theme::text_dim()));
                ui.add_space(8.0);
                if widgets::pill(ui, "Sign in", None, false).clicked() {
                    app.start_sign_in(ui.ctx());
                }
            } else {
                ui.multiply_opacity(2.0 * t - 1.0);
                nav(ui, glyphs::PERSON_CIRCLE, LUCIDE, "Sign in", Page::Settings, false, false, &mut go_to);
            }
            return;
        }
        // The heading folds away with the labels.
        let (head, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0 * (1.0 - t)), Sense::hover());
        if a > 0.0 {
            ui.painter().text(head.left_top() + vec2(8.0, 0.0), Align2::LEFT_TOP, "PLAYLISTS", theme::semibold(11.0), theme::with_alpha(theme::text_faint(), a));
        }
        let bars = if t > 0.0 { egui::scroll_area::ScrollBarVisibility::AlwaysHidden } else { egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded };
        egui::ScrollArea::vertical().id_salt("sidebar-pl").auto_shrink(false).scroll_bar_visibility(bars).show(ui, |ui| {
            // Last launch's list until the library arrives.
            let Some(playlists) = app.library.as_ref().map(|l| &l.playlists).or(app.sidebar_cache.as_ref()) else {
                // The spinner animates every frame: give up on a request that never answered.
                if app.library_requested.elapsed() < std::time::Duration::from_secs(20) {
                    ui.horizontal(|ui| {
                        ui.add_space(ICON_X - 8.0);
                        ui.spinner();
                    });
                } else if t < 0.5 {
                    ui.multiply_opacity(1.0 - 2.0 * t);
                    ui.label(egui::RichText::new("Couldn't load your playlists").color(theme::text_dim()));
                    ui.add_space(8.0);
                    if widgets::pill(ui, "Try again", None, false).clicked() {
                        app.request_library();
                    }
                }
                return;
            };
            let mut visible: Vec<&crate::model::Card> = playlists
                .iter()
                .filter(|c| match &c.target {
                    Target::Playlist(id) => {
                        id != "LM"
                            && id != "SE"
                            && !c.title.eq_ignore_ascii_case("Liked Songs")
                            && !c.title.eq_ignore_ascii_case("Lieblingssongs")
                            && !c.title.eq_ignore_ascii_case("Liked Music")
                            && !app.hidden_playlists.contains(id)
                    }
                    _ => true,
                })
                .collect();
            visible.sort_by_key(|c| match &c.target {
                Target::Playlist(id) => app.playlist_order.iter().position(|x| x == id).unwrap_or(usize::MAX),
                _ => usize::MAX,
            });
            let drag_key = egui::Id::unique("sidebar-pl-drag");
            let dragging: Option<String> = ui.data(|d| d.get_temp(drag_key));
            let down = ui.input(|i| i.pointer.any_down());
            let pointer = ui.input(|i| i.pointer.interact_pos());
            for c in &visible {
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 48.0), Sense::click_and_drag());
                let id = match &c.target {
                    Target::Playlist(id) => id.clone(),
                    _ => continue,
                };
                if !ui.is_rect_visible(rect) {
                    continue;
                }
                let is_dragged = dragging.as_deref() == Some(id.as_str());
                let mut resp = named(resp, &c.title).on_hover_cursor(if is_dragged { egui::CursorIcon::Grabbing } else { egui::CursorIcon::PointingHand });
                if collapsed {
                    resp = resp.on_hover_text(&c.title);
                }
                if resp.drag_started() {
                    ui.data_mut(|d| d.insert_temp(drag_key, id.clone()));
                }
                if let Some(did) = &dragging
                    && did != &id
                    && down
                    && pointer.is_some_and(|p| rect.contains(p))
                {
                    reorder = Some((did.clone(), id.clone()));
                }
                let h = theme::anim_bool(ui.ctx(), resp.id.with("h"), resp.hovered(), 0.15);
                let selected = matches!(&c.target, Target::Playlist(id) if app.page == Page::Playlist(id.clone()));
                let active = widgets::active_state(ui, &c.target);
                let is_active = active.is_some();
                let is_playing = active == Some(true);
                let accent = app.accent_color();

                let lift = if is_dragged { 0.10 } else { 0.0 };
                let bg_alpha = if is_active {
                    0.09 + 0.04 * h
                } else if selected {
                    0.08 + 0.03 * h
                } else {
                    0.05 * h
                } + lift;
                ui.painter().rect_filled(rect, 8.0, theme::glass(bg_alpha));
                if is_active {
                    // Subtle left active accent bar (3px)
                    ui.painter().rect_filled(
                        Rect::from_min_size(pos2(rect.left(), rect.top() + 8.0), vec2(3.0, 32.0)),
                        1.5,
                        accent,
                    );
                }

                let img = Rect::from_center_size(pos2(rect.left() + ICON_X, rect.center().y), Vec2::splat(36.0));
                paint_cover(ui, img, &c.thumbs, theme::thumb_radius(), 1.0);

                // Spotify-style direct play on cover hover (only when expanded!)
                let cover_resp = ui.interact(img, resp.id.with("c_play"), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                let show_play = !collapsed && (h > 0.01 || is_active);
                if show_play {
                    let scrim = if is_playing { 0.45 } else { 0.38 * h };
                    ui.painter().rect_filled(img, theme::thumb_radius(), Color32::from_black_alpha((scrim * 255.0) as u8));
                    widgets::play_pause_glyph(ui, img.center(), 8.0, Color32::WHITE, is_playing);
                } else if collapsed && is_active {
                    // In collapsed icon-rail mode: accent indicator ring around cover so user sees it's playing
                    ui.painter().rect_stroke(img.expand(1.5), theme::thumb_radius() + 1.5, egui::Stroke::new(2.0, accent), egui::StrokeKind::Outside);
                }

                if a > 0.0 {
                    let right_pad = if is_active { 36.0 } else { 20.0 };
                    let max_w = (rect.right() - (img.right() + 10.0) - right_pad).max(10.0);
                    let title_col = if is_active {
                        accent
                    } else if selected {
                        Color32::WHITE
                    } else {
                        theme::text()
                    };
                    text_at(ui, pos2(img.right() + 10.0, rect.top() + 7.0), &c.title, if is_active || selected { theme::semibold(13.5) } else { theme::regular(13.5) }, theme::with_alpha(title_col, a), max_w);
                    text_at(ui, pos2(img.right() + 10.0, rect.top() + 26.0), &c.subtitle, theme::regular(12.0), theme::with_alpha(theme::text_dim(), a), max_w);

                    // Right speaker icon when this playlist is actively playing
                    if is_active {
                        let spk_center = pos2(rect.right() - 16.0, rect.center().y);
                        widgets::icon_at(ui, spk_center, icon::VOLUME_2, 14.0, accent);
                    }
                }
                egui::Popup::context_menu(&resp).show(|ui| {
                    ui.set_min_width(200.0);
                    let is_excluded = app.spotify_excluded_playlists.contains(&id);
                    let label = if is_excluded {
                        "Include in Spotify Sync"
                    } else {
                        "Exclude from Spotify Sync"
                    };
                    if ui.button(label).clicked() {
                        if is_excluded {
                            app.spotify_excluded_playlists.remove(&id);
                        } else {
                            app.spotify_excluded_playlists.insert(id.clone());
                        }
                        ui.close();
                    }
                    if ui.button("Hide from sidebar").clicked() {
                        hide = Some(id.clone());
                        ui.close();
                    }
                });

                if cover_resp.clicked() && !is_dragged {
                    if collapsed {
                        // In collapsed mode (Spotify style): clicking the cover opens the playlist in the main window!
                        app.actions.push(Action::Open(c.target.clone()));
                    } else if is_active {
                        app.actions.push(Action::TogglePlay);
                    } else {
                        app.actions.push(Action::PlayCollection(c.target.clone(), false));
                    }
                } else if resp.clicked() && !is_dragged {
                    app.actions.push(Action::Open(c.target.clone()));
                }
            }

            // Spotify playlists in sidebar (deduplicated against YouTube Music playlists)
            let spotify_sidebar: Vec<crate::transfer::spotify::Entry> = {
                let acc = app.transfer.account.lock().unwrap();
                if let crate::transfer::Account::SignedIn { library, .. } = &*acc {
                    library.iter()
                        .filter(|e| !matches!(e.source, crate::transfer::spotify::Source::Liked))
                        .filter(|e| !visible.iter().any(|c| c.title.trim().eq_ignore_ascii_case(e.name.trim())))
                        .cloned()
                        .collect()
                } else {
                    Vec::new()
                }
            };

            let mut clicked_sp: Option<crate::transfer::spotify::Entry> = None;
            let mut play_sp: Option<crate::transfer::spotify::Entry> = None;
            for sp in &spotify_sidebar {
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 48.0), Sense::click());
                if !ui.is_rect_visible(rect) {
                    continue;
                }
                let mut resp = named(resp, &sp.name).on_hover_cursor(egui::CursorIcon::PointingHand);
                if collapsed {
                    resp = resp.on_hover_text(&sp.name);
                }
                let hover = theme::anim_bool(ui.ctx(), resp.id.with("sp_h"), resp.hovered(), 0.12);
                let is_sp_selected = app.spotify_selected.as_ref().is_some_and(|(s, _, _)| s.name == sp.name);
                let accent = app.accent_color();
                let bg_alpha = if is_sp_selected { 0.08 + 0.04 * hover } else { 0.04 * hover };
                if bg_alpha > 0.0 {
                    ui.painter().rect_filled(rect, 8.0, theme::glass(bg_alpha));
                }
                let art_rect = Rect::from_center_size(pos2(rect.left() + ICON_X, rect.center().y), Vec2::splat(32.0));
                super::widgets::paint_cover(ui, art_rect, &super::transfer::spotify_thumbs(&sp.art), 6.0, 1.0);

                let cover_resp = ui.interact(art_rect, resp.id.with("sp_cover_play"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                let show_sp_play = !collapsed && hover > 0.01;
                if show_sp_play {
                    ui.painter().rect_filled(art_rect, 6.0, theme::shade(0.38 * hover));
                    widgets::play_triangle(ui, art_rect.center(), 7.0, Color32::WHITE);
                } else if collapsed && is_sp_selected {
                    ui.painter().rect_stroke(art_rect.expand(1.5), 7.5, egui::Stroke::new(2.0, accent), egui::StrokeKind::Outside);
                }

                if a > 0.0 {
                    let tx = art_rect.right() + 10.0;
                    let title_w = (rect.right() - tx - 20.0).max(10.0);
                    let title_col = if is_sp_selected { accent } else { theme::text() };
                    let title_g = widgets::galley(ui, &sp.name, theme::semibold(13.0), title_col, title_w);
                    ui.painter().galley(pos2(tx, rect.center().y - 12.0), title_g, title_col);
                    let sub_g = widgets::galley(ui, "Spotify Playlist", theme::regular(11.0), theme::text_dim(), title_w);
                    ui.painter().galley(pos2(tx, rect.center().y + 2.0), sub_g, theme::text_dim());
                }

                if cover_resp.clicked() {
                    if collapsed {
                        clicked_sp = Some(sp.clone());
                    } else {
                        play_sp = Some(sp.clone());
                    }
                } else if resp.clicked() {
                    clicked_sp = Some(sp.clone());
                }
            }

            if ui.input(|i| i.pointer.any_released()) {
                ui.data_mut(|d| d.remove_temp::<String>(drag_key));
            }

            if let Some(sp) = play_sp {
                app.page = Page::Library(LibTab::Playlists);
                app.spotify_selected = Some((sp.clone(), None, true));
                let xfer = app.transfer.clone();
                let b = app.backend.clone();
                let b_wake = b.clone();
                let bg_actions = app.bg_actions.clone();
                let slot = app.spotify_loaded_playlist.clone();
                let src = sp.source.clone();
                xfer.fetch_spotify_playlist(&b, src.clone(), move |res| {
                    if let Ok(pl) = &res && !pl.tracks.is_empty() {
                        bg_actions.lock().unwrap().push(Action::PlaySpotifyPlaylist(pl.tracks.clone(), 0));
                        b_wake.wake();
                    }
                    *slot.lock().unwrap() = Some((src, res));
                });
            } else if let Some(sp) = clicked_sp {
                app.page = Page::Library(LibTab::Playlists);
                app.spotify_selected = Some((sp.clone(), None, true));
                let xfer = app.transfer.clone();
                let b = app.backend.clone();
                let slot = app.spotify_loaded_playlist.clone();
                let src = sp.source.clone();
                xfer.fetch_spotify_playlist(&b, src.clone(), move |res| {
                    *slot.lock().unwrap() = Some((src, res));
                });
            }
        });
    });
    if let Some(id) = hide {
        app.hidden_playlists.insert(id);
    }
    if let Some((dragged, target)) = reorder {
        if let (Some(from), Some(to)) = (app.playlist_order.iter().position(|x| x == &dragged), app.playlist_order.iter().position(|x| x == &target)) {
            let id = app.playlist_order.remove(from);
            app.playlist_order.insert(to.min(app.playlist_order.len()), id);
        }
    }
    let mut restart = false;
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        if transferring {
            super::transfer::sidebar_card(app, ui, collapsed, a, ICON_X);
        }
        if let Some(v) = &ready {
            ui.add_space(6.0);
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 52.0), Sense::click());
            let mut resp = named(resp, "Restart to update").on_hover_cursor(egui::CursorIcon::PointingHand);
            if collapsed {
                resp = resp.on_hover_text(format!("Restart to update to BitChord {v}"));
            }
            let h = theme::anim_bool(ui.ctx(), resp.id.with("h"), resp.hovered(), 0.15);
            ui.painter().rect_filled(rect, 12.0, theme::with_alpha(look.accent, 0.22 + 0.12 * h));
            glyphs::draw(ui, pos2(rect.left() + ICON_X, rect.center().y), LUCIDE, glyphs::SPARKLES, theme::text());
            if a > 0.0 {
                ui.painter().text(pos2(rect.left() + 46.0, rect.top() + 17.0), Align2::LEFT_CENTER, "Restart to update", theme::semibold(13.5), theme::with_alpha(theme::text(), a));
                ui.painter().text(pos2(rect.left() + 46.0, rect.top() + 35.0), Align2::LEFT_CENTER, format!("BitChord {v}"), theme::regular(12.0), theme::with_alpha(theme::text(), 0.75 * a));
            }
            restart = resp.clicked();
        }
    });
    if restart {
        app.restart_to_update(ui.ctx());
    }
    if let Some(page) = go_to {
        if matches!(page, Page::Search(_)) {
            // Search lands on the search page with the caret in the box; already there,
            // it just brings the box back.
            app.search_refocus = true;
            if !matches!(app.page, Page::Search(_)) {
                app.actions.push(Action::Go(Page::Search(String::new())));
            }
        } else {
            app.actions.push(Action::Go(page));
        }
    }

    // Subtle frosted glass border separating the sidebar from main content ONLY below the horizontal separator
    let y_min = sep_y.max(ui.max_rect().top());
    ui.painter().vline(ui.max_rect().right(), y_min..=ui.max_rect().bottom(), egui::Stroke::new(1.0, theme::glass(0.08)));
}

// ------------------------------------------------------------------ top bar

pub fn top_bar(app: &mut App, ui: &mut Ui, top_rect: Rect) {
    app.search_rect = None;
    let panel = top_rect;
    let row = Rect::from_min_size(panel.min, vec2(panel.width(), 44.0));
    let right_gap = super::titlebar::right_gap(ui.ctx(), row, 20.0);
    let cy = panel.top() + 18.0;

    // Floating Frosted Glass Backdrop:
    // Smooth translucent liquid glass so content scrolling underneath remains softly visible
    let glass_rect = Rect::from_min_max(
        pos2(panel.left() - 28.0, panel.top() - 14.0),
        pos2(panel.right() + 28.0, row.bottom() + 6.0),
    );
    ui.painter().rect_filled(glass_rect, 0.0, theme::panel(0.82));
    ui.painter().rect_filled(glass_rect, 0.0, theme::glass(0.08));
    ui.painter().hline(glass_rect.x_range(), glass_rect.bottom(), egui::Stroke::new(1.0, theme::glass(0.12)));

    // 1. LEFT SECTION: Back, Forward, Spicetify Random Song Dice ("Würfel")
    let left_start = panel.left();
    let nav_btn = |ui: &mut Ui, x: f32, icon_char: char, key: &str, tip: &str, enabled: bool| -> egui::Response {
        let r = Rect::from_center_size(pos2(x, cy), Vec2::splat(28.0));
        let resp = ui.interact(r, ui.scope_id().with(("top-nav", key)), Sense::click());
        let resp = named(resp, tip).on_hover_text(tip);
        let resp = if enabled { resp.on_hover_cursor(egui::CursorIcon::PointingHand) } else { resp };
        let h = theme::anim_bool(ui.ctx(), resp.id.with("h"), resp.hovered() && enabled, 0.12);
        ui.painter().circle_filled(r.center(), 14.0, theme::glass(if enabled { 0.06 + 0.10 * h } else { 0.04 }));
        let fg = if enabled { theme::lerp_color(theme::text_dim(), theme::text(), h) } else { theme::with_alpha(theme::text(), 0.32) };
        icon_at(ui, r.center(), icon_char, 16.0, fg);
        resp
    };

    let b_resp = nav_btn(ui, left_start + 16.0, icon::CHEVRON_LEFT, "back", "Back", !app.back.is_empty());
    if b_resp.clicked() && !app.back.is_empty() {
        app.nav_back();
    }
    let f_resp = nav_btn(ui, left_start + 48.0, icon::CHEVRON_RIGHT, "fwd", "Forward", !app.forward.is_empty());
    if f_resp.clicked() && !app.forward.is_empty() {
        app.nav_forward();
    }

    // Spicetify Random Song Dice ("Würfel")
    let dice_r = Rect::from_center_size(pos2(left_start + 84.0, cy), Vec2::splat(28.0));
    let dice_resp = ui.interact(dice_r, ui.scope_id().with("top-dice"), Sense::click());
    let dice_resp = named(dice_resp, "Play a random song").on_hover_text("Play a random song").on_hover_cursor(egui::CursorIcon::PointingHand);
    let dh = theme::anim_bool(ui.ctx(), dice_resp.id.with("dh"), dice_resp.hovered(), 0.12);
    ui.painter().circle_filled(dice_r.center(), 14.0, theme::glass(0.08 + 0.12 * dh));
    let dice_fg = theme::lerp_color(theme::text_dim(), theme::text(), dh);
    glyphs::draw(ui, dice_r.center(), 18.0, glyphs::DICE, dice_fg);
    if dice_resp.clicked() {
        app.actions.push(Action::PlayRandomSong);
    }

    // 2. CENTER SECTION: Home Button (left of search), Search Pill, Discover/New
    let right = panel.right() - right_gap;

    // Minimum gap requested by user:
    // "mindestens so viel Abstand wie eben der Vor- und Zurück-Button zueinander haben oder vielleicht auch etwas mehr minimal"
    const HOME_SEARCH_GAP: f32 = 9.0;
    const BTN_RADIUS: f32 = 16.0; // 32px diameter circle buttons

    // Safe boundaries to prevent collisions with Dice/Nav on left or Account on right
    let left_boundary = left_start + 108.0;
    let right_boundary = right - 64.0;
    let cx = panel.center().x;

    let clear_left = (cx - left_boundary).max(40.0);
    let clear_right = (right_boundary - cx).max(40.0);
    let clear_half = clear_left.min(clear_right);

    // Calculate dynamic search width so [Home] [Search Pill] [Discover] stays centered and never overlaps
    let wing_w = BTN_RADIUS * 2.0 + HOME_SEARCH_GAP;
    let max_search_w = ((clear_half - wing_w) * 2.0).clamp(34.0, 260.0);
    let search_w = max_search_w;

    let s_rect = Rect::from_center_size(pos2(cx, cy), vec2(search_w, 34.0));

    // Home Button: anchored directly to the left of the search pill
    let home_center_x = s_rect.left() - HOME_SEARCH_GAP - BTN_RADIUS;
    let home_r = Rect::from_center_size(pos2(home_center_x, cy), Vec2::splat(BTN_RADIUS * 2.0));
    let is_home = app.page == Page::Home;
    let home_resp = ui.interact(home_r, ui.scope_id().with("top-home"), Sense::click()).on_hover_text("Home").on_hover_cursor(egui::CursorIcon::PointingHand);
    let hh = theme::anim_bool(ui.ctx(), home_resp.id.with("hh"), home_resp.hovered(), 0.12);
    ui.painter().circle_filled(home_r.center(), BTN_RADIUS, theme::glass(if is_home { 0.16 } else { 0.06 + 0.08 * hh }));
    glyphs::draw(ui, home_r.center(), 18.0, glyphs::HOME, if is_home { Color32::WHITE } else { theme::lerp_color(theme::text_dim(), theme::text(), hh) });
    if home_resp.clicked() {
        app.actions.push(Action::Go(Page::Home));
    }

    // Centered Search Pill
    let s_field = ui.interact(s_rect, ui.scope_id().with("top-search-pill"), Sense::click()).on_hover_cursor(egui::CursorIcon::Text);
    app.search_rect = Some(s_rect);
    let sf = theme::anim_bool(ui.ctx(), "top-search-focus", app.search_focused, 0.2);
    ui.painter().rect_filled(s_rect, 17.0, theme::glass(0.09 + 0.05 * sf));
    if sf > 0.0 {
        ui.painter().rect_stroke(s_rect, 17.0, egui::Stroke::new(1.0, theme::glass(0.25 * sf)), egui::StrokeKind::Inside);
    }
    glyphs::draw(ui, pos2(s_rect.left() + 16.0, cy), 15.0, glyphs::SEARCH, if sf > 0.0 { theme::text() } else { theme::text_dim() });

    if search_w > 70.0 {
        let s_inner = Rect::from_min_max(pos2(s_rect.left() + 32.0, s_rect.top() + 2.0), pos2(s_rect.right() - 26.0, s_rect.bottom() - 2.0));
        let placeholder = if search_w > 150.0 { "What do you want to play?" } else { "Search..." };

        // Up and Down walk suggestions
        let fresh = app.suggestions.0 == app.search_text.trim();
        let n = if fresh { app.suggestions.1.len() } else { 0 };
        if app.search_focused && n > 0 && ui.input(|i| i.modifiers.is_none()) {
            ui.input_mut(|i| {
                if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
                    app.suggest_sel = match app.suggest_sel {
                        None => Some(0),
                        Some(s) if s + 1 < n => Some(s + 1),
                        Some(_) => None,
                    };
                }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
                    app.suggest_sel = match app.suggest_sel {
                        None => Some(n - 1),
                        Some(0) => None,
                        Some(s) => Some(s - 1),
                    };
                }
            });
        }

        let edit = ui.put(
            s_inner,
            egui::TextEdit::singleline(&mut app.search_text)
                .hint_text(egui::RichText::new(placeholder).color(theme::text_faint()))
                .font(theme::regular(13.0))
                .frame(egui::Frame::NONE)
                .align(egui::Align2::LEFT_CENTER)
                .desired_width(s_inner.width()),
        );

        if s_field.clicked() || (ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::F))) {
            edit.request_focus();
        }
        if std::mem::take(&mut app.search_refocus) {
            edit.request_focus();
        }
        app.search_focused = edit.has_focus();

        let mut cleared = false;
        if !app.search_text.is_empty() {
            let x_rect = Rect::from_center_size(pos2(s_rect.right() - 14.0, cy), Vec2::splat(20.0));
            let x_resp = ui.interact(x_rect, ui.scope_id().with("top-s-clear"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            icon_at(ui, x_rect.center(), icon::X, 14.0, theme::text_dim());
            if x_resp.clicked() {
                app.search_text.clear();
                app.search_synced = String::new();
                app.suggestions = Default::default();
                app.suggest_sel = None;
                cleared = true;
                edit.request_focus();
            }
        }

        if edit.changed() || edit.gained_focus() || cleared {
            app.suggest_sel = None;
            let now = ui.input(|i| i.time);
            let first = app.suggestions.1.is_empty();
            let query = app.search_text.trim();
            app.suggestions = app.suggest_cache.iter()
                .find(|(q, _)| q == query)
                .cloned()
                .unwrap_or_default();
            let delay = if first || cleared || edit.gained_focus() { 0.0 } else { 0.06 };
            app.suggest_due = Some(now + delay);
        }

        if (edit.has_focus() || app.search_focused) && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let picked = app.suggest_sel.filter(|_| fresh).and_then(|s| app.suggestions.1.get(s)).map(|s| s.text.clone());
            let q = picked.unwrap_or_else(|| app.search_text.trim().to_owned());
            if !q.is_empty() {
                app.suggestions = Default::default();
                app.search_focused = false;
                app.actions.push(Action::Go(Page::Search(q)));
            }
        }
    } else if s_field.clicked() {
        app.actions.push(Action::Go(Page::Search(String::new())));
    }

    // Discover / New Button: anchored symmetrically to the right of the search pill
    let disc_center_x = s_rect.right() + HOME_SEARCH_GAP + BTN_RADIUS;
    let disc_r = Rect::from_center_size(pos2(disc_center_x, cy), Vec2::splat(BTN_RADIUS * 2.0));
    let is_new = app.page == Page::New;
    let disc_resp = ui.interact(disc_r, ui.scope_id().with("top-disc"), Sense::click()).on_hover_text("Explore & New Releases").on_hover_cursor(egui::CursorIcon::PointingHand);
    let dh = theme::anim_bool(ui.ctx(), disc_resp.id.with("dh"), disc_resp.hovered(), 0.12);
    ui.painter().circle_filled(disc_r.center(), BTN_RADIUS, theme::glass(if is_new { 0.16 } else { 0.06 + 0.08 * dh }));
    glyphs::draw(ui, disc_r.center(), 18.0, glyphs::EXPLORE, if is_new { Color32::WHITE } else { theme::lerp_color(theme::text_dim(), theme::text(), dh) });
    if disc_resp.clicked() {
        app.actions.push(Action::Go(Page::New));
    }

    // 3. RIGHT SECTION: Friend Activity Button + Account Avatar
    let is_friends_open = app.details_open && app.right_tab == super::RightTab::Friends;
    let friends_r = Rect::from_center_size(pos2(right - 54.0, cy), Vec2::splat(32.0));
    let f_resp = ui.interact(friends_r, ui.scope_id().with("top-friends"), Sense::click());
    let f_resp = named(f_resp, "Friend Activity").on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Friend Activity");
    let fh = theme::anim_bool(ui.ctx(), f_resp.id.with("fh"), f_resp.hovered() || is_friends_open, 0.12);
    ui.painter().circle_filled(friends_r.center(), 16.0, theme::glass(if is_friends_open { 0.16 } else { 0.06 + 0.08 * fh }));
    let fg = if is_friends_open { app.accent_color() } else { theme::lerp_color(theme::text_dim(), theme::text(), fh) };
    glyphs::draw(ui, friends_r.center(), 18.0, glyphs::USERS, fg);
    if is_friends_open {
        ui.painter().circle_filled(pos2(friends_r.center().x, friends_r.bottom() + 1.0), 1.8, app.accent_color());
    }
    if f_resp.clicked() {
        app.actions.push(Action::ToggleFriends);
    }

    let acc_r = Rect::from_center_size(pos2(right - 18.0, cy), Vec2::splat(32.0));
    let resp = ui.interact(acc_r, ui.scope_id().with("account"), Sense::click());
    let resp = named(resp, "Account / Settings").on_hover_cursor(egui::CursorIcon::PointingHand);
    let initial = app.account_name.as_deref().and_then(|n| n.chars().next()).unwrap_or('?');
    if app.logged_in {
        ui.painter().circle_filled(acc_r.center(), 16.0, theme::lerp_color(app.accent_color(), Color32::BLACK, 0.35));
        if let Some(avatar) = &app.account_avatar {
            widgets::paint_remote(ui, avatar, acc_r, 16.0, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        } else {
            ui.painter().text(acc_r.center(), Align2::CENTER_CENTER, initial, theme::bold(14.0), Color32::WHITE);
        }
    } else {
        ui.painter().circle_filled(acc_r.center(), 16.0, theme::glass(0.12));
        icon_at(ui, acc_r.center(), icon::USER, 15.0, theme::text_dim());
    }
    if resp.clicked() {
        app.actions.push(Action::Go(Page::Settings));
    }

    if app.loading {
        let spin = Rect::from_min_size(pos2(friends_r.left() - 28.0, cy - 10.0), Vec2::splat(20.0));
        let mut cui = ui.new_child(egui::UiBuilder::new().max_rect(spin));
        cui.spinner();
    }
}

/// The search pill: typeahead, suggestions and Enter to search. The Search page draws
/// it; `app.search_rect` anchors the suggestions under it.
#[allow(dead_code)]
pub fn search_field(app: &mut App, ui: &mut Ui, width: f32) {
    // Anywhere on the pill focuses the box, not just on the text area inside it.
    let (rect, field) = ui.allocate_exact_size(vec2(width, 42.0), Sense::click());
    let field = field.on_hover_cursor(egui::CursorIcon::Text);
    let focus = theme::anim_bool(ui.ctx(), "search-focus", app.search_focused, 0.2);
    ui.painter().rect_filled(rect, 21.0, theme::glass(0.09 + 0.05 * focus));
    if focus > 0.0 {
        ui.painter().rect_stroke(rect, 21.0, egui::Stroke::new(1.0, theme::glass(0.25 * focus)), egui::StrokeKind::Inside);
    }
    icon_at(ui, pos2(rect.left() + 22.0, rect.center().y), icon::SEARCH, 17.0, theme::text_dim());
    let clearable = !app.search_text.is_empty();
    let inner = Rect::from_min_max(pos2(rect.left() + 42.0, rect.top() + 4.0), pos2(rect.right() - if clearable { 46.0 } else { 16.0 }, rect.bottom() - 4.0));
    // Up and Down walk the suggestions (past either end back to what was typed). Taken
    // before the box sees them, where they'd move the caret. Only bare arrows (Shift
    // selects text), and only suggestions for what's typed now, not ones still being replaced.
    let fresh = app.suggestions.0 == app.search_text.trim();
    let n = if fresh { app.suggestions.1.len() } else { 0 };
    if app.search_focused && n > 0 && ui.input(|i| i.modifiers.is_none()) {
        ui.input_mut(|i| {
            if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
                app.suggest_sel = match app.suggest_sel {
                    None => Some(0),
                    Some(s) if s + 1 < n => Some(s + 1),
                    Some(_) => None,
                };
            }
            if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
                app.suggest_sel = match app.suggest_sel {
                    None => Some(n - 1),
                    Some(0) => None,
                    Some(s) => Some(s - 1),
                };
            }
        });
    }
    let edit = ui.put(
        inner,
        egui::TextEdit::singleline(&mut app.search_text)
            .hint_text(egui::RichText::new("Search songs, albums, artists, podcasts").color(theme::text_faint()))
            .font(theme::regular(15.0))
            .frame(egui::Frame::NONE)
            .align(egui::Align2::LEFT_CENTER)
            .desired_width(inner.width()),
    );
    app.search_rect = Some(rect);
    if std::mem::take(&mut app.search_refocus) {
        edit.request_focus();
    }
    app.search_focused = edit.has_focus();
    let mut cleared = false;
    if clearable {
        let x = Rect::from_center_size(pos2(rect.right() - 25.0, rect.center().y), Vec2::splat(30.0));
        let resp = named(ui.interact(x, ui.scope_id().with("search-clear"), Sense::click()), "Clear search").on_hover_cursor(egui::CursorIcon::PointingHand);
        let h = theme::anim_bool(ui.ctx(), resp.id.with("h"), resp.hovered(), 0.15);
        if h > 0.0 {
            ui.painter().circle_filled(x.center(), 15.0, theme::glass(0.10 * h));
        }
        icon_at(ui, x.center(), icon::X, 16.0, theme::lerp_color(theme::text_dim(), theme::text(), h));
        if resp.clicked() {
            // Back to the empty search page: the old results go with the text.
            app.search_text.clear();
            app.search_synced = String::new();
            app.suggestions = Default::default();
            app.suggest_sel = None;
            cleared = true;
            app.actions.push(Action::Go(Page::Search(String::new())));
            edit.request_focus();
        }
    }
    if field.clicked() || ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::F)) {
        edit.request_focus();
    }
    if edit.changed() || edit.gained_focus() || cleared {
        app.suggest_sel = None;
        let now = ui.input(|i| i.time);
        // Show cached results immediately, then refresh after a short typing pause.
        let first = app.suggestions.1.is_empty();
        let query = app.search_text.trim();
        app.suggestions = app.suggest_cache.iter()
            .find(|(q, _)| q == query)
            .cloned()
            .unwrap_or_default();
        let delay = if first || cleared || edit.gained_focus() { 0.0 } else { 0.06 };
        app.suggest_due = Some(now + delay);
    }
    // The event loop can otherwise sleep past the debounce deadline.
    if let Some(due) = app.suggest_due {
        let now = ui.input(|i| i.time);
        ui.ctx().request_repaint_after(std::time::Duration::from_secs_f64((due - now).max(0.0)));
    }
    if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        let picked = app.suggest_sel.filter(|_| fresh).and_then(|s| app.suggestions.1.get(s)).map(|s| s.text.clone());
        let q = picked.unwrap_or_else(|| app.search_text.trim().to_owned());
        if !q.is_empty() {
            app.suggestions = Default::default();
            app.actions.push(Action::Go(Page::Search(q)));
        }
    }
}

/// Search suggestions dropdown under the search box.
pub fn suggestions(app: &mut App, ctx: &egui::Context) {
    let Some(rect) = app.search_rect else { return };
    // Through a press on a remove button too: its release briefly takes the box's focus.
    let show = (app.search_focused || app.forget_held) && !app.suggestions.1.is_empty() && !app.np_open;
    let t = theme::anim_bool(ctx, "suggest-open", show, 0.15);
    if t <= 0.0 {
        return;
    }
    egui::Area::new(egui::Id::unique("suggestions"))
        .order(egui::Order::Foreground)
        .fixed_pos(rect.left_bottom() + vec2(0.0, 6.0 - 6.0 * (1.0 - t)))
        .show(ctx, |ui| {
            ui.set_opacity(t);
            egui::Frame::new()
                .fill(theme::menu_fill())
                .corner_radius(14)
                .inner_margin(6)
                .shadow(egui::epaint::Shadow { offset: [0, 10], blur: 30, spread: 0, color: theme::shade(0.5) })
                .show(ui, |ui| {
                    ui.set_width(rect.width() - 12.0);
                    if !ui.input(|i| i.pointer.any_down() || i.pointer.any_released()) {
                        app.forget_held = false;
                    }
                    // Releasing that press is a click outside the box, which takes its focus
                    // (and this list) away: give it back until the press is over.
                    if app.forget_held {
                        app.search_refocus = true;
                    }
                    let now = ui.input(|i| i.time);
                    for (i, s) in app.suggestions.1.clone().into_iter().enumerate() {
                        let term = s.text;
                        // The removed one folds away, and the rows below slide up into its place.
                        if let Some((_, at)) = app.forgetting.as_ref().filter(|(t, _)| s.forget.as_deref() == Some(t.as_str())) {
                            let p = ((now - at) / 0.18).clamp(0.0, 1.0) as f32;
                            let e = 1.0 - (1.0 - p).powi(3);
                            let gap = ui.spacing().item_spacing.y;
                            // Its spacing folds away with it, so nothing jumps at the end.
                            ui.spacing_mut().item_spacing.y = 0.0;
                            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), (40.0 + gap) * (1.0 - e)), Sense::hover());
                            ui.spacing_mut().item_spacing.y = gap;
                            ui.scope(|ui| {
                                ui.set_clip_rect(r.intersect(ui.clip_rect()));
                                ui.set_opacity(1.0 - e);
                                let cy = r.top() + 20.0;
                                icon_at(ui, pos2(r.left() + 20.0, cy), icon::HISTORY, 15.0, theme::text_dim());
                                suggestion_text(ui, pos2(r.left() + 42.0, cy - 9.0), &term, &app.suggestions.0, r.width() - 88.0);
                            });
                            ui.ctx().request_repaint();
                            continue;
                        }
                        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
                        let resp = named(resp, &term).on_hover_cursor(egui::CursorIcon::PointingHand);
                        let h = theme::anim_bool(ui.ctx(), resp.id.with("h"), resp.contains_pointer() || app.suggest_sel == Some(i), 0.1);
                        ui.painter().rect_filled(r, 8.0, theme::glass(0.08 * h));
                        let glyph = if s.forget.is_some() { icon::HISTORY } else { icon::SEARCH };
                        icon_at(ui, pos2(r.left() + 20.0, r.center().y), glyph, 15.0, theme::text_dim());
                        let room = if s.forget.is_some() { 88.0 } else { 52.0 };
                        suggestion_text(ui, pos2(r.left() + 42.0, r.center().y - 9.0), &term, &app.suggestions.0, r.width() - room);
                        // A past search can be removed from the account's history.
                        if let Some(token) = s.forget {
                            let x = Rect::from_center_size(pos2(r.right() - 22.0, r.center().y), Vec2::splat(30.0));
                            let xr = named(ui.interact(x, resp.id.with("forget"), Sense::click()), "Remove from recent searches")
                                .on_hover_text("Remove")
                                .on_hover_cursor(egui::CursorIcon::PointingHand);
                            let xh = theme::anim_bool(ui.ctx(), xr.id.with("h"), xr.hovered(), 0.1);
                            if xh > 0.0 {
                                ui.painter().circle_filled(x.center(), 15.0, theme::glass(0.12 * xh));
                            }
                            icon_at(ui, x.center(), icon::TRASH_2, 15.0, theme::lerp_color(theme::text_dim(), theme::text(), xh));
                            // On the press, and one per press: the next row's button slides
                            // under the held pointer.
                            if (xr.is_pointer_button_down_on() || xr.clicked()) && !app.forget_held {
                                app.forget_held = true;
                                // One still folding away is done.
                                if let Some((t, _)) = app.forgetting.take() {
                                    app.suggestions.1.retain(|s| s.forget.as_deref() != Some(t.as_str()));
                                }
                                app.forgetting = Some((token.clone(), now));
                                app.suggest_sel = None;
                                app.suggest_round += 1;
                                app.suggest_cache.clear();
                                app.forgets_pending += 1;
                                app.search_refocus = true;
                                app.backend.request(crate::backend::Req::ForgetSearch(token));
                            }
                        }
                        // Pointer-down, not click: the text box loses focus (hiding this) first.
                        if resp.is_pointer_button_down_on() || resp.clicked() {
                            app.suggestions = Default::default();
                            app.search_focused = false;
                            app.actions.push(Action::Go(Page::Search(term)));
                            break;
                        }
                    }
                    if let Some((t, _)) = app.forgetting.take_if(|(_, at)| now - *at >= 0.18) {
                        app.suggestions.1.retain(|s| s.forget.as_deref() != Some(t.as_str()));
                    }
                });
        });
}

/// A suggestion with what's typed in bold and the rest dimmed, as on music.youtube.com.
fn suggestion_text(ui: &Ui, pos: Pos2, text: &str, typed: &str, width: f32) {
    let n = text.chars().zip(typed.chars()).take_while(|(a, b)| a.to_lowercase().eq(b.to_lowercase())).count();
    let split = if n == typed.chars().count() { text.char_indices().nth(n).map_or(text.len(), |(i, _)| i) } else { 0 };
    let (bold, rest) = text.split_at(split);
    let mut job = egui::text::LayoutJob::default();
    job.append(bold, 0.0, egui::TextFormat { font_id: theme::semibold(15.0), color: theme::text(), ..Default::default() });
    job.append(rest, 0.0, egui::TextFormat { font_id: theme::regular(15.0), color: theme::text_dim(), ..Default::default() });
    job.wrap = egui::text::TextWrapping::truncate_at_width(width.max(10.0));
    let g = ui.fonts_mut(|f| f.layout_job(job));
    ui.painter().galley(pos, g, theme::text());
}

// ------------------------------------------------------------------ player bar

/// The player: one frosted capsule floating over the bottom of the window, as in the
/// BitChord desktop app. Transport on the left, the sleeve with its titles and seek
/// The full-width modern player bar spanning the entire bottom of the window:
/// Left: Album art, Title, Artist, Heart Favorite Button
/// Center: Transport (Shuffle toggle, Prev, Play/Pause with tactile press, Next, Repeat) and Scrubber Seek Bar
/// Right: Always-visible Apple-style expandable volume slider, Lyrics, Queue, Details, Devices
pub fn player_bar(app: &mut App, ui: &mut Ui, now: f64, _frosted: bool) {
    let full = ui.max_rect();
    let ctx = ui.ctx().clone();

    // Frosted Liquid Glass surface across the full-width player bar
    ui.painter().rect_filled(full, 0.0, theme::panel(0.68));
    ui.painter().rect_filled(full, 0.0, theme::glass(0.06));
    ui.painter().hline(full.x_range(), full.top(), egui::Stroke::new(1.0, theme::glass(0.12)));

    let status = app.audio.status.clone();
    let hold = app.mix_hold().map(|h| (h.total, h.mix, app.mix_hold_pos(h)));
    let track = match &hold {
        Some(_) => app.shown().cloned(),
        None => app.current().cloned(),
    };
    let enabled = track.is_some();
    let playing = app.playing();
    let accent = app.accent_color();
    let cy = full.center().y;

    let pad = 16.0;
    let w = full.width();
    let compact = w < 650.0;
    let narrow = w < 880.0;

    let left_w = if compact { 160.0 } else if narrow { 220.0 } else { 280.0 };
    let right_w = if compact { 160.0 } else if narrow { 230.0 } else { 280.0 };

    let left_rect = Rect::from_min_max(pos2(full.left() + pad, full.top()), pos2(full.left() + pad + left_w, full.bottom()));
    let right_rect = Rect::from_min_max(pos2(full.right() - pad - right_w, full.top()), pos2(full.right() - pad, full.bottom()));
    let center_rect = Rect::from_min_max(pos2(left_rect.right() + 12.0, full.top()), pos2(right_rect.left() - 12.0, full.bottom()));

    let ink = |h: f32| {
        let c = theme::lerp_color(theme::text_dim(), theme::text(), h);
        if enabled { c } else { c.gamma_multiply(0.45) }
    };

    // =========================================================================
    // LEFT COLUMN: Album Art, Title, Artists, Heart Favorite Button
    // =========================================================================
    let heard = if hold.is_some() { track.clone() } else { app.playing_track().cloned().or_else(|| track.clone()) };
    let liked = heard.as_ref().is_some_and(|t| app.liked.contains(&t.id));
    let cover_size = 56.0;
    let cover = Rect::from_center_size(pos2(left_rect.left() + cover_size / 2.0, cy), Vec2::splat(cover_size));
    match &track {
        Some(t) => {
            widgets::paint_cover_xfade(ui, egui::Id::unique("player-bar-art"), cover, &t.thumbs, 6.0, 1.0);
            let ar = ui.interact(cover, ui.scope_id().with("bar-art"), Sense::click());
            let ar = named(ar, "Open now playing").on_hover_cursor(egui::CursorIcon::PointingHand);
            let ah = theme::anim_bool(&ctx, ar.id.with("h"), ar.hovered(), 0.15);
            if ah > 0.0 {
                ui.painter().rect_filled(cover, 6.0, theme::glass(0.16 * ah));
                ui.painter().rect_filled(cover, 6.0, theme::shade(0.3 * ah));
                glyphs::draw(ui, cover.center(), 14.0, glyphs::MAXIMIZE_2, theme::with_alpha(Color32::WHITE, ah));
            }
            if ar.clicked() {
                app.actions.push(Action::OpenNowPlaying(true));
            }
            egui::Popup::context_menu(&ar).show(|ui| {
                widgets::song_menu(ui, t, liked, None, None, &mut app.actions);
            });
        }
        None => {
            ui.painter().rect_filled(cover, 6.0, theme::glass(0.08));
            paint_mark(ui, cover.shrink(8.0));
        }
    }

    // Text between cover and heart (Spotify style)
    let text_l = cover.right() + 12.0;
    let max_title_w = (left_rect.right() - text_l - 30.0).max(20.0);
    let title = track.as_ref().map(|t| t.title.clone()).unwrap_or_else(|| "BitChord".to_owned());
    let title_font = theme::semibold(13.5);
    let title_rect = Rect::from_min_size(pos2(text_l, cy - 13.0), vec2(max_title_w, 18.0));
    let (tr, tre) = widgets::marquee_title(
        ui,
        ui.scope_id().with("bar-title"),
        title_rect,
        &title,
        title_font,
        theme::text(),
        app.playing(),
        true,
    );
    let title_w = tr.width();
    if let Some(t) = &track {
        let tre = named(tre, &t.title).on_hover_cursor(egui::CursorIcon::PointingHand);
        if tre.clicked() {
            app.actions.push(Action::OpenNowPlaying(true));
        }
        egui::Popup::context_menu(&tre).show(|ui| {
            widgets::song_menu(ui, t, liked, None, None, &mut app.actions);
        });
    }

    // Favorite Heart Button (Spotify style: aligned with song title line)
    let heart_x = (text_l + title_w + 14.0).min(left_rect.right() - 14.0);
    let heart_y = cy - 5.5;
    let heart_r = Rect::from_center_size(pos2(heart_x, heart_y), Vec2::splat(24.0));
    let hresp = ui.interact(heart_r, ui.scope_id().with("bar-heart"), Sense::click());
    let hresp = named(hresp, if liked { "Remove from favorites" } else { "Add to favorites" })
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let hh = theme::anim_bool(&ctx, hresp.id.with("h"), hresp.hovered(), 0.15);
    let pop = heard.as_ref().map_or(0.0, |t| theme::anim_bool(&ctx, ("liked", &t.id), liked, 0.3));
    let heart_color = if liked { accent } else { ink(hh) };
    glyphs::draw(ui, heart_r.center(), 17.0 + pop * 3.0, if liked { glyphs::HEART_FILLED } else { glyphs::HEART }, heart_color);
    if hresp.clicked() && let Some(t) = heard {
        app.actions.push(Action::Like(t, !liked));
    }

    let mut ax = text_l;
    let max_artist_w = (left_rect.right() - text_l - 10.0).max(10.0);
    if let Some(t) = &track {
        for (i, a) in t.artists.iter().enumerate() {
            let s = if i + 1 < t.artists.len() { format!("{}, ", a.name) } else { a.name.clone() };
            let r = text_at(ui, pos2(ax, cy + 4.0), &s, theme::regular(11.5), theme::text_dim(), (text_l + max_artist_w - ax).max(10.0));
            if let Some(id) = &a.id {
                let resp = ui.interact(r, ui.scope_id().with(("bar-artist", i)), Sense::click());
                let resp = named(resp, &a.name).on_hover_cursor(egui::CursorIcon::PointingHand);
                if resp.hovered() {
                    ui.painter().hline(r.x_range(), r.bottom() + 1.5, egui::Stroke::new(1.0, theme::text_dim()));
                }
                if resp.clicked() {
                    app.actions.push(Action::Open(Target::Artist(id.clone())));
                }
            }
            ax = r.right();
            if ax > text_l + max_artist_w {
                break;
            }
        }
    }

    // =========================================================================
    // CENTER COLUMN: Playback Controls & Scrubber Seek Bar
    // =========================================================================
    let mid = center_rect.center().x;
    let tbtn = |ui: &mut Ui, x: f32, y: f32, size: f32, key: &str, name: &str| -> egui::Response {
        let r = Rect::from_center_size(pos2(x, y), Vec2::splat(size));
        let resp = ui.interact(r, ui.scope_id().with(("bar-t", key)), Sense::click());
        named(resp, name).on_hover_text(name).on_hover_cursor(egui::CursorIcon::PointingHand)
    };

    let ty = full.top() + 27.0;

    // Shuffle Button (Spotify style: boolean toggle with indicator dot, enlarged to 22px)
    let shuffle_on = app.shuffle_mode();
    let r = tbtn(ui, mid - 84.0, ty, 30.0, "shuffle", if shuffle_on { "Disable shuffle" } else { "Enable shuffle" });
    let is_press = r.is_pointer_button_down_on();
    let scale = if is_press { 0.92 } else if r.hovered() { 1.05 } else { 1.0 };
    let h = theme::anim_bool(&ctx, r.id.with("h"), r.hovered(), 0.12);
    let fg = if shuffle_on { accent } else { ink(h) };
    glyphs::draw(ui, r.rect.center(), 22.0 * scale, glyphs::SHUFFLE, fg);
    if shuffle_on {
        ui.painter().circle_filled(pos2(r.rect.center().x, r.rect.bottom() + 1.0), 1.8, accent);
    }
    if r.clicked() {
        app.toggle_shuffle();
    }

    // Previous Button (enlarged 25px glyph with tactile press and subtle glass ripple)
    let r = tbtn(ui, mid - 42.0, ty, 32.0, "prev", "Previous");
    let is_press = r.is_pointer_button_down_on();
    let scale = if is_press { 0.92 } else if r.hovered() { 1.06 } else { 1.0 };
    let h = theme::anim_bool(&ctx, r.id.with("h"), r.hovered(), 0.12);
    if h > 0.0 {
        ui.painter().circle_filled(r.rect.center(), 16.0 * scale, theme::glass(0.08 * h));
    }
    glyphs::draw(ui, r.rect.center(), 25.0 * scale, glyphs::PREVIOUS, ink(h));
    if r.clicked() && enabled {
        app.prev();
    }

    // Play / Pause Button with tactile press feedback and smooth morph (enlarged to 40px)
    let pr = Rect::from_center_size(pos2(mid, ty), Vec2::splat(40.0));
    let presp = ui.interact(pr, ui.scope_id().with("bar-play"), Sense::click());
    let presp = named(presp, if playing { "Pause" } else { "Play" }).on_hover_cursor(egui::CursorIcon::PointingHand);
    let is_press = presp.is_pointer_button_down_on();
    let scale_factor = if is_press { 0.94 } else if presp.hovered() { 1.05 } else { 1.0 };
    let bg_circle = Rect::from_center_size(pr.center(), Vec2::splat(38.0 * scale_factor));
    ui.painter().circle_filled(bg_circle.center(), bg_circle.width() / 2.0, theme::text());

    let icon_c = theme::shade(0.95);
    let morph = theme::anim_bool(&ctx, "bar-play-morph", playing || app.buffering, 0.18);
    if app.buffering {
        let a = now as f32 * 6.0;
        let pts: Vec<Pos2> = (0..16).map(|i| bg_circle.center() + vec2((a + i as f32 * 0.24).cos(), (a + i as f32 * 0.24).sin()) * 8.0).collect();
        ui.painter().add(egui::Shape::Path(egui::epaint::PathShape::line(pts, egui::epaint::PathStroke::new(2.4, icon_c))));
        ctx.request_repaint_after(std::time::Duration::from_millis(33));
    } else {
        if morph < 1.0 {
            glyphs::draw(ui, bg_circle.center() + vec2(1.2, 0.0), 20.0 * scale_factor, glyphs::PLAY, theme::with_alpha(icon_c, 1.0 - morph));
        }
        if morph > 0.0 {
            glyphs::draw(ui, bg_circle.center(), 20.0 * scale_factor, glyphs::PAUSE, theme::with_alpha(icon_c, morph));
        }
    }
    if presp.clicked() && enabled {
        app.toggle();
    }

    // Next Button (enlarged 25px glyph with tactile press and subtle glass ripple)
    let r = tbtn(ui, mid + 42.0, ty, 32.0, "next", "Next");
    let is_press = r.is_pointer_button_down_on();
    let scale = if is_press { 0.92 } else if r.hovered() { 1.06 } else { 1.0 };
    let h = theme::anim_bool(&ctx, r.id.with("h"), r.hovered(), 0.12);
    if h > 0.0 {
        ui.painter().circle_filled(r.rect.center(), 16.0 * scale, theme::glass(0.08 * h));
    }
    glyphs::draw(ui, r.rect.center(), 25.0 * scale, glyphs::NEXT, ink(h));
    if r.clicked() && enabled {
        app.next();
    }

    // Repeat Button (enlarged 22px glyph with crisp vector "1" badge)
    let (is_repeat_one, on, tip) = match app.queue.repeat {
        Repeat::Off => (false, false, "Enable repeat (All)"),
        Repeat::All => (false, true, "Repeat current song (Repeat 1)"),
        Repeat::One => (true, true, "Disable repeat"),
    };
    let r = tbtn(ui, mid + 84.0, ty, 30.0, "repeat", tip);
    let is_press = r.is_pointer_button_down_on();
    let scale = if is_press { 0.92 } else if r.hovered() { 1.05 } else { 1.0 };
    let h = theme::anim_bool(&ctx, r.id.with("h"), r.hovered(), 0.12);
    let fg = if on { accent } else { ink(h) };
    glyphs::draw(ui, r.rect.center(), 22.0 * scale, glyphs::REPEAT, fg);
    if is_repeat_one {
        // Draw crisp vector "1" badge in the center of the repeat loop
        ui.painter().text(
            r.rect.center() + vec2(0.5, 0.0),
            Align2::CENTER_CENTER,
            "1",
            theme::bold(9.5 * scale),
            fg,
        );
    }
    if on {
        ui.painter().circle_filled(pos2(r.rect.center().x, r.rect.bottom() + 1.0), 1.8, accent);
    }
    if r.clicked() {
        app.queue.repeat = match app.queue.repeat {
            Repeat::Off => Repeat::All,
            Repeat::All => Repeat::One,
            Repeat::One => Repeat::Off,
        };
    }

    // Seek / Scrubber Bar
    let held = hold.is_some();
    let (total, mix) = match &hold {
        Some((total, mix, _)) => (*total, *mix),
        None => {
            let total = status
                .duration()
                .map(|d| d.as_secs_f32())
                .or(track.as_ref().and_then(|t| t.duration).map(|d| d as f32))
                .unwrap_or(0.0);
            let next = app.queue.upcoming().is_some() || app.autoplay;
            let mix = match (total > 0.0, next, app.queue.repeat) {
                (true, true, repeat) if repeat != Repeat::One => {
                    status.mix_secs(status.duration().or_else(|| track.as_ref().and_then(|t| t.duration).map(|s| std::time::Duration::from_secs(s as u64)))).min(total * 0.5)
                }
                _ => 0.0,
            };
            (total, mix)
        }
    };
    let smooth = match &hold {
        Some((_, _, pos)) => *pos,
        None => status.position_smooth().as_secs_f32(),
    };
    let pos = app.seek_drag.unwrap_or(smooth).min(total.max(0.0));

    let sy = full.bottom() - 20.0;
    let seek_w = (center_rect.width() - 96.0).clamp(100.0, 560.0);
    let sx0 = mid - seek_w / 2.0;
    let sx1 = mid + seek_w / 2.0;

    let seek_hit = Rect::from_min_max(pos2(sx0, sy - 8.0), pos2(sx1, sy + 8.0));
    let sresp = ui.interact(seek_hit, ui.scope_id().with("bar-seek"), if held { Sense::hover() } else { Sense::click_and_drag() });
    let sresp = if total > 0.0 && !held { sresp.on_hover_cursor(egui::CursorIcon::PointingHand) } else { sresp };
    sresp.widget_info(|| egui::WidgetInfo::slider(total > 0.0 && !held, pos as f64, "Seek"));
    let focus = theme::anim_bool(&ctx, "bar-seek-h", total > 0.0 && !held && (sresp.hovered() || sresp.dragged()), 0.18);
    let thick = 4.0 + 4.0 * focus;
    let line_r = Rect::from_min_max(pos2(sx0, sy - thick / 2.0), pos2(sx1, sy + thick / 2.0));
    ui.painter().rect_filled(line_r, thick / 2.0, Color32::from_white_alpha(38));

    // Automix: the handoff window tints to the song's colour with its start ticked, so
    // the blend has a place it visibly begins at.
    if mix > 0.5 {
        let edge = sx0 + (sx1 - sx0) * ((total - mix) / total);
        let band = Rect::from_min_max(pos2(edge, line_r.top()), line_r.max);
        ui.painter().rect_filled(band, thick / 2.0, theme::with_alpha(accent, 0.32));
        let tick = Rect::from_min_size(pos2(edge - 1.0, line_r.top()), vec2(2.0, thick));
        ui.painter().rect_filled(tick, 1.0, theme::with_alpha(accent, 0.9));
    }
    let frac = if total > 0.0 { (pos / total).clamp(0.0, 1.0) } else { 0.0 };
    if frac > 0.0 {
        let fill = theme::lerp_color(Color32::from_white_alpha(163), Color32::from_white_alpha(240), focus);
        ui.painter().rect_filled(Rect::from_min_size(line_r.min, vec2(line_r.width() * frac, thick)), thick / 2.0, fill);
    }

    // Timestamps matching fullscreen view (elapsed on left, remaining -X:XX on right)
    if total > 0.0 {
        let tc = theme::with_alpha(Color32::WHITE, 0.62);
        let left_str = fmt_time(pos as u32);
        let g_left = widgets::galley(ui, &left_str, theme::semibold(11.0), tc, 60.0);
        ui.painter().galley(pos2(sx0 - 10.0 - g_left.size().x, sy - g_left.size().y / 2.0), g_left, tc);

        let rem = fmt_time((total - pos).max(0.0) as u32);
        let right_str = format!("-{rem}");
        let g_right = widgets::galley(ui, &right_str, theme::semibold(11.0), tc, 60.0);
        ui.painter().galley(pos2(sx1 + 10.0, sy - g_right.size().y / 2.0), g_right, tc);
    }

    let at = |p: Pos2| (((p.x - sx0) / (sx1 - sx0)).clamp(0.0, 1.0) * total).max(0.0);
    if total > 0.0 && let Some(p) = sresp.interact_pointer_pos() {
        if sresp.dragged() {
            app.seek_drag = Some(at(p));
        }
        if sresp.drag_stopped() || sresp.clicked() {
            app.seek_drag = None;
            app.actions.push(Action::Seek(at(p)));
        }
    }

    // =========================================================================
    // RIGHT COLUMN: Tools & Apple-Style Expandable Volume Slider
    // =========================================================================
    let rbtn = |ui: &mut Ui, x: f32, key: &str, name: &str| -> egui::Response {
        let r = Rect::from_center_size(pos2(x, cy), Vec2::splat(28.0));
        let resp = ui.interact(r, ui.scope_id().with(("bar-r", key)), Sense::click());
        named(resp, name).on_hover_text(name).on_hover_cursor(egui::CursorIcon::PointingHand)
    };

    let vol_slider_w = if compact { 72.0 } else { 96.0 };
    let vol_slider_rect = Rect::from_min_size(pos2(right_rect.right() - vol_slider_w, cy - 12.0), vec2(vol_slider_w, 24.0));
    let speaker_rect = Rect::from_center_size(pos2(vol_slider_rect.left() - 16.0, cy), Vec2::splat(28.0));

    // Apple-style expandable volume slider (always visible!)
    volume_slider_apple(app, ui, vol_slider_rect, speaker_rect, "bar");

    let mut tool_x = speaker_rect.left() - 20.0;

    // Devices / Connect Button
    if !compact {
        let r = rbtn(ui, tool_x, "devices", "Connect to a device");
        let h = theme::anim_bool(&ctx, r.id.with("h"), r.hovered() || app.devices_open, 0.15);
        glyphs::draw(ui, r.rect.center(), 18.0, glyphs::DEVICES, if app.devices_open { accent } else { ink(h) });
        if r.clicked() {
            app.actions.push(Action::ToggleDevices);
        }
        tool_x -= 30.0;
    }

    // Friend Activity Button (Spotify Users icon)
    if !compact {
        let is_friends_open = app.details_open && app.right_tab == super::RightTab::Friends;
        let tip = if is_friends_open { "Hide friend activity" } else { "Friend activity" };
        let r = rbtn(ui, tool_x, "friends", tip);
        let h = theme::anim_bool(&ctx, r.id.with("h"), r.hovered() || is_friends_open, 0.15);
        let icon_fg = if is_friends_open { accent } else { ink(h) };
        glyphs::draw(ui, r.rect.center(), 18.0, glyphs::USERS, icon_fg);
        if is_friends_open {
            ui.painter().circle_filled(pos2(r.rect.center().x, r.rect.bottom() + 1.0), 1.8, accent);
        }
        if r.clicked() {
            app.actions.push(Action::ToggleFriends);
        }
        tool_x -= 30.0;
    }

    // Details ("Now playing view" - Spotify style)
    if !compact {
        let is_details_open = app.details_open && app.right_tab == super::RightTab::Details;
        let tip = if is_details_open { "Hide now playing view" } else { "Now playing view" };
        let r = rbtn(ui, tool_x, "details", tip);
        let h = theme::anim_bool(&ctx, r.id.with("h"), r.hovered() || is_details_open, 0.15);
        let icon_fg = if is_details_open { accent } else { ink(h) };

        // Authentic Spotify-style Now Playing View screen/sidebar icon
        let center = r.rect.center();
        let box_w = 16.0;
        let box_h = 13.0;
        let screen_box = Rect::from_center_size(center, vec2(box_w, box_h));
        ui.painter().rect_stroke(screen_box, 2.5, egui::Stroke::new(1.3, icon_fg), egui::StrokeKind::Inside);
        let split_x = screen_box.right() - 5.5;
        ui.painter().vline(split_x, (screen_box.top() + 1.0)..=(screen_box.bottom() - 1.0), egui::Stroke::new(1.3, icon_fg));
        if is_details_open {
            let right_panel = Rect::from_min_max(pos2(split_x + 1.0, screen_box.top() + 1.5), pos2(screen_box.right() - 1.5, screen_box.bottom() - 1.5));
            ui.painter().rect_filled(right_panel, 1.0, accent);
            // Spotify active indicator dot underneath
            ui.painter().circle_filled(pos2(center.x, r.rect.bottom() + 1.0), 1.8, accent);
        }

        if r.clicked() {
            app.actions.push(Action::ToggleDetails);
        }
        tool_x -= 30.0;
    }

    // Queue Button
    let queue_open = if app.np_open { app.np_tab == NpTab::UpNext } else { app.queue_open };
    let r = rbtn(ui, tool_x, "queue", "Queue");
    let h = theme::anim_bool(&ctx, r.id.with("h"), r.hovered(), 0.15);
    glyphs::draw(ui, r.rect.center(), 18.0, glyphs::QUEUE, if queue_open { accent } else { ink(h) });
    if r.clicked() {
        if app.np_open {
            app.np_tab = if queue_open { NpTab::Art } else { NpTab::UpNext };
        } else {
            app.queue_open = !app.queue_open;
            if app.queue_open {
                app.details_open = false;
            }
        }
    }
    tool_x -= 30.0;

    // Lyrics Button
    let lyrics_on = app.np_open && app.np_tab == NpTab::Lyrics && !app.no_lyrics();
    let r = rbtn(ui, tool_x, "lyrics", "Lyrics");
    let h = theme::anim_bool(&ctx, r.id.with("h"), r.hovered(), 0.15);
    glyphs::draw(ui, r.rect.center(), 18.0, if lyrics_on { glyphs::LYRICS } else { glyphs::LYRICS_OFF }, if lyrics_on { accent } else { ink(h) });
    if r.clicked() {
        if lyrics_on {
            app.np_open = false;
        } else {
            app.np_tab = NpTab::Lyrics;
            app.actions.push(Action::OpenNowPlaying(true));
        }
    }
}

/// The BitChord mark, white, fitted inside `r` (the sleeve slot when nothing plays).
fn paint_mark(ui: &Ui, r: Rect) {
    mark(ui, r, Color32::from_white_alpha(170));
}

/// The BitChord mark, fitted inside `r`, tinted `tint`.
fn mark(ui: &Ui, r: Rect, tint: Color32) {
    const MARK: &[u8] = include_bytes!("../../assets/icon/mark.png");
    const W: f32 = 256.0;
    const H: f32 = 171.0;
    let key = egui::Id::unique("bitchord-mark");
    let tex = match ui.ctx().data(|d| d.get_temp::<egui::TextureHandle>(key)) {
        Some(t) => t,
        None => {
            let Ok(img) = image::load_from_memory(MARK) else { return };
            let img = img.into_rgba8();
            let color = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
            let t = ui.ctx().load_texture("bitchord-mark", color, egui::TextureOptions::LINEAR);
            ui.ctx().data_mut(|d| d.insert_temp(key, t.clone()));
            t
        }
    };
    let k = (r.width() / W).min(r.height() / H);
    let dst = Rect::from_center_size(r.center(), vec2(W * k, H * k));
    ui.painter().image(tex.id(), dst, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), tint);
}

/// Where Mute keeps the volume to go back to (the player bar's and the mini player's).
pub fn unmute_id() -> egui::Id {
    egui::Id::unique("unmute")
}

/// Apple Control Center style expandable volume slider:
/// - Always visible, sleek capsule.
/// - Dynamic thickness expansion from 5px to 11px on hover or drag.
/// - Immediate volume adjustment on click or drag.
/// - Direct mouse wheel scrolling on speaker or slider adjusts volume smoothly.
fn volume_slider_apple(app: &mut App, ui: &mut Ui, slider: Rect, speaker: Rect, salt: &str) {
    let vol = app.audio.volume();
    let ctx = ui.ctx().clone();

    // Speaker interaction (click to toggle mute, wheel to adjust volume)
    let sresp = ui.interact(speaker, ui.scope_id().with((salt, "spk")), Sense::click());
    let sresp = named(sresp, "Mute / Volume").on_hover_cursor(egui::CursorIcon::PointingHand);
    let sh = theme::anim_bool(&ctx, sresp.id.with("h"), sresp.hovered(), 0.15);
    let step = glyphs::volume_step(vol);
    glyphs::draw(ui, speaker.center(), 18.0, glyphs::VOLUME[step], theme::lerp_color(theme::text_dim(), theme::text(), sh));

    if sresp.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            app.audio.set_volume((vol + scroll / 1050.0).clamp(0.0, 1.0));
        }
    }
    if sresp.clicked() {
        let stash = unmute_id();
        if vol > 0.0 {
            ui.data_mut(|d| d.insert_temp(stash, vol));
            app.audio.set_volume(0.0);
        } else {
            app.audio.set_volume(ui.data(|d| d.get_temp::<f32>(stash)).unwrap_or(0.6));
        }
    }

    // Slider interaction
    let vresp = ui.interact(slider, ui.scope_id().with(salt), Sense::click_and_drag()).on_hover_cursor(egui::CursorIcon::PointingHand);
    vresp.widget_info(|| egui::WidgetInfo::slider(true, vol as f64, "Volume"));
    let is_down = ui.input(|i| i.pointer.primary_down());
    let pointer_pos = ui.input(|i| i.pointer.latest_pos());
    let active = vresp.hovered() || vresp.dragged() || (is_down && vresp.contains_pointer());
    let vh = theme::anim_bool(&ctx, ui.scope_id().with((salt, "apple-h")), active, 0.18);

    // Volume slider geometry: 4.0px idle, 8.0px hover/drag (harmonized with seekbar!)
    let thick = 4.0 + 4.0 * vh;
    let track_r = Rect::from_center_size(slider.center(), vec2(slider.width(), thick));
    ui.painter().rect_filled(track_r, thick / 2.0, theme::glass(0.20));

    // Filled portion: flat junction towards the knob to prevent double-rounding (Option B)
    let fill_w = (track_r.width() * vol).clamp(0.0, track_r.width());
    if fill_w > 0.0 {
        let fill = Rect::from_min_size(track_r.min, vec2(fill_w, thick));
        let r_cap = (thick / 2.0).round() as u8;
        let fill_corners = if vh > 0.05 {
            egui::CornerRadius { nw: r_cap, sw: r_cap, ne: 0, se: 0 }
        } else {
            egui::CornerRadius::same(r_cap)
        };
        ui.painter().rect_filled(fill, fill_corners, app.accent_color());
    }
    // Subtle tactile knob when hovered / dragged (Option B: sits flush on the flat fill end)
    if vh > 0.05 {
        let knob_x = track_r.left() + fill_w;
        ui.painter().circle_filled(pos2(knob_x, track_r.center().y), thick / 2.0 + 1.5, theme::text());
    }

    // Direct click, hold or drag sets volume instantly
    if (vresp.dragged() || vresp.clicked() || (active && is_down)) && let Some(p) = pointer_pos {
        let new_vol = ((p.x - track_r.left()) / track_r.width()).clamp(0.0, 1.0);
        app.audio.set_volume(new_vol);
    }

    // Mouse wheel scrolling directly over the slider
    if vresp.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            app.audio.set_volume((vol + scroll / 1050.0).clamp(0.0, 1.0));
        }
    }
}

// ------------------------------------------------------------------ queue drawer

pub fn queue_drawer(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Up next").font(theme::bold(20.0)));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Left of the window buttons, which sit over the drawer's top right.
            ui.add_space(super::titlebar::right_gap(ui.ctx(), ui.max_rect(), 0.0));
            if icon_button(ui, icon::X, 16.0, "Close queue", false).clicked() {
                app.queue_open = false;
            }
        });
    });
    ui.add_space(8.0);
    queue_list(app, ui, "drawer");
}

/// Current track plus everything after it. Shared by the drawer and now playing.
pub fn queue_list(app: &mut App, ui: &mut Ui, salt: &str) {
    let rows = super::pages::Rows::new(app, ui.input(|i| i.time));
    egui::ScrollArea::vertical().id_salt(("queue", salt)).auto_shrink(false).show(ui, |ui| {
        if app.queue.tracks.is_empty() {
            ui.label(egui::RichText::new("Nothing queued yet").color(theme::text_dim()));
            return;
        }
        let start = app.queue.index.unwrap_or(0);
        ui.horizontal(|ui| {
            ui.checkbox(&mut app.autoplay, "Autoplay similar songs");
        });
        ui.add_space(8.0);
        let pitch = widgets::ROW_H + ui.spacing().item_spacing.y;
        let accent = app.accent_color();
        let drag_id = egui::Id::unique(("queue-drag", salt));
        let mut drag: Option<(usize, usize)> = ui.ctx().data(|d| d.get_temp(drag_id));
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let (queue, liked, acts, has_current, by) = (&app.queue.tracks, &app.liked, &mut app.actions, app.queue.index.is_some(), &app.jam.by);
        let mut drop_bar: Option<f32> = None;
        widgets::visible_rows(ui, queue.len() - start, |ui, k| {
            let i = start + k;
            let t = &queue[i];
            // The first row is the current song.
            let current = k == 0 && has_current;
            let dragging = drag.is_some_and(|(from, _)| from == i);
            let queue_remove = if !current { Some(i) } else { None };
            let o = widgets::RowOpts {
                current,
                queue_remove,
                added_by: by.get(&t.id).cloned(),
                hide_duration_hover: !current,
                drag: !current,
                ..rows.opts(liked, t, None, false)
            };
            let resp = widgets::track_row(ui, t, &o, acts);
            if !current && resp.drag_started() {
                drag = Some((i, i));
                ui.ctx().request_repaint();
            }
            if let Some((from, to)) = &mut drag
                && *from == i
                && resp.dragged()
                && let Some(p) = pointer
            {
                let moved = ((p.y - resp.rect.center().y) / pitch).round() as isize;
                *to = (i as isize + moved).clamp(start as isize, (queue.len() - 1) as isize) as usize;
                drop_bar = Some(resp.rect.top() + (*to as f32 - i as f32) * pitch);
            }
            if dragging {
                let r = resp.rect;
                let p = ui.painter();
                p.rect_filled(r, 8.0, theme::glass(0.16));
                p.rect_stroke(r, 8.0, egui::Stroke::new(1.0, theme::glass(0.14)), egui::StrokeKind::Inside);
            }
            if let Some((from, to)) = drag
                && from == i
                && resp.drag_stopped()
            {
                if to != from {
                    acts.push(Action::QueueMove(from, to));
                }
                drag = None;
            }
            if resp.clicked() {
                // The playing row pauses/resumes instead of doing nothing.
                acts.push(if current { Action::TogglePlay } else { Action::QueueJump(i) });
            }
            if !current && ui.rect_contains_pointer(Rect::from_min_max(resp.rect.min, pos2(resp.rect.right() + 42.0, resp.rect.bottom()))) {
                // Remove button over the duration column.
                let x = Rect::from_center_size(resp.rect.right_center() - vec2(26.0, 0.0), Vec2::splat(28.0));
                let xr = named(ui.interact(x, resp.id.with("rm"), Sense::click()), "Remove from queue").on_hover_cursor(egui::CursorIcon::PointingHand);
                ui.painter().circle_filled(x.center(), 14.0, theme::panel(0.6));
                icon_at(ui, x.center(), icon::X, 14.0, theme::text());
                if xr.clicked() {
                    acts.push(Action::QueueRemove(i));
                }
            }
        });
        if let Some(y) = drop_bar {
            let clip = ui.clip_rect();
            let y = y.clamp(clip.top(), clip.bottom() - 2.0);
            let left = ui.max_rect().left() + 2.0;
            let right = ui.max_rect().right() - 2.0;
            ui.painter().rect_filled(Rect::from_min_max(pos2(left, y - 1.0), pos2(right, y + 1.0)), 1.0, accent);
        }
        ui.ctx().data_mut(|d| {
            if let Some(d_val) = drag {
                d.insert_temp(drag_id, d_val);
            } else {
                d.remove_temp::<(usize, usize)>(drag_id);
            }
        });
    });
}

// ------------------------------------------------------------------ toast

pub fn toast(app: &mut App, ctx: &egui::Context, now: f64) {
    let Some((msg, shown)) = &app.toast else { return };
    if *shown < 0.0 {
        return;
    }
    let age = (now - shown) as f32;
    let t = theme::ease_out(age / 0.25) * (1.0 - ((age - 4.1) / 0.4).clamp(0.0, 1.0));
    let screen = ctx.content_rect();
    egui::Area::new(egui::Id::unique("toast"))
        .order(egui::Order::Tooltip)
        .pivot(Align2::CENTER_BOTTOM)
        // Above the player bar; in the mini player, over the strip's middle.
        .fixed_pos(Pos2::new(screen.center().x, if screen.height() < 200.0 { screen.center().y + 22.0 } else { screen.bottom() - 104.0 } + 14.0 * (1.0 - t)))
        .show(ctx, |ui| {
            ui.set_opacity(t);
            egui::Frame::new()
                .fill(theme::menu_fill())
                .corner_radius(12)
                .inner_margin(egui::Margin::symmetric(18, 12))
                .shadow(egui::epaint::Shadow { offset: [0, 8], blur: 24, spread: 0, color: theme::shade(0.45) })
                .show(ui, |ui| {
                    ui.set_max_width(520.0);
                    ui.label(egui::RichText::new(msg.as_str()).font(theme::regular(14.0)).color(theme::text()));
                });
        });
    // Frames only while it fades in or out; in between it just waits.
    if (0.25..4.1).contains(&age) {
        ctx.request_repaint_after(std::time::Duration::from_secs_f32(4.1 - age));
    } else {
        ctx.request_repaint();
    }
}
