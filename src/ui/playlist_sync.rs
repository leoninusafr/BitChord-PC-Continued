//! Smart Playlist Deduplication & Conflict Resolution.
//!
//! Compares playlists across YouTube Music and Spotify (and within YouTube Music):
//! 1. Both Name AND Content match: Automatically unified into a single playlist card ("zack, wird das sozusagen zu einer gemacht").
//! 2. Only Name matches (different content): Conflict prompted to user ("Zusammenführen" vs "Getrennt halten").
//! 3. Only Content matches (different names): Conflict prompted to user ("Zusammenführen" vs "Getrennt halten").
//! User decisions are persisted in `app.playlist_decisions`.

use std::collections::HashMap;
use egui::{vec2, pos2, Rect, Sense, Ui, Vec2};
use crate::model::{Card, Target, Link};
use crate::theme;
use crate::theme::icon;
use crate::ui::widgets::pill;
use crate::ui::App;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceKind {
    YouTubeMusic,
    Spotify,
}

impl SourceKind {
    pub fn label(&self) -> &'static str {
        match self {
            SourceKind::YouTubeMusic => "YouTube Music",
            SourceKind::Spotify => "Spotify",
        }
    }
}

#[derive(Clone, Debug)]
pub struct PlaylistItem {
    pub id: String,
    pub title: String,
    pub norm_title: String,
    pub count: Option<usize>,
    pub source_kind: SourceKind,
    pub ytm_card: Option<Card>,
    pub spotify_entry: Option<crate::transfer::spotify::Entry>,
}

#[derive(Clone, Debug)]
pub enum ConflictKind {
    /// Same name, but different song count / content
    NameOnly {
        name: String,
        count_a: Option<usize>,
        source_a: SourceKind,
        count_b: Option<usize>,
        source_b: SourceKind,
    },
    /// Different name, but identical song count / content
    ContentOnly {
        name_a: String,
        source_a: SourceKind,
        name_b: String,
        source_b: SourceKind,
        count: usize,
    },
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct PlaylistConflict {
    pub key: String,
    pub item_a: PlaylistItem,
    pub item_b: PlaylistItem,
    pub kind: ConflictKind,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct UnifiedCard {
    pub card: Card,
    pub sources: Vec<SourceKind>,
    pub is_unified: bool,
    pub count: Option<usize>,
}

/// Parses the number of tracks from card subtitles such as "Playlist • 45 songs", "45 songs", "Leon • 12 tracks", etc.
pub fn parse_card_song_count(sub: &str) -> Option<usize> {
    for part in sub.split(['•', '·', '|', '-']) {
        let trimmed = part.trim();
        let words: Vec<&str> = trimmed.split_whitespace().collect();
        for (i, word) in words.iter().enumerate() {
            let num_str: String = word.chars().filter(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = num_str.parse::<usize>() {
                if n > 0 {
                    if i + 1 < words.len() {
                        let next = words[i + 1].to_lowercase();
                        if next.starts_with("song")
                            || next.starts_with("track")
                            || next.starts_with("titel")
                            || next.starts_with("lied")
                            || next.starts_with("item")
                        {
                            return Some(n);
                        }
                    } else if words.len() == 1 {
                        return Some(n);
                    }
                }
            }
        }
    }
    None
}

/// Normalizes playlist titles for comparison (lowercase, trimmed, stripping punctuation).
pub fn normalize_playlist_title(t: &str) -> String {
    t.trim()
        .to_lowercase()
        .replace(
            |c: char| {
                c == '-'
                    || c == '—'
                    || c == '–'
                    || c == '_'
                    || c == '|'
                    || c == ':'
                    || c == '.'
                    || c == '/'
                    || c == '('
                    || c == ')'
                    || c == '['
                    || c == ']'
            },
            " ",
        )
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Deterministic unique key for a pair of playlist IDs.
pub fn conflict_pair_key(id_a: &str, id_b: &str) -> String {
    if id_a < id_b {
        format!("{id_a}:{id_b}")
    } else {
        format!("{id_b}:{id_a}")
    }
}

/// Disjoint Set Union (DSU) helper for grouping unified playlists.
struct Dsu {
    parent: Vec<usize>,
}

impl Dsu {
    fn new(size: usize) -> Self {
        Self {
            parent: (0..size).collect(),
        }
    }

    fn find(&mut self, i: usize) -> usize {
        if self.parent[i] == i {
            i
        } else {
            let p = self.parent[i];
            self.parent[i] = self.find(p);
            self.parent[i]
        }
    }

    fn union(&mut self, i: usize, j: usize) {
        let root_i = self.find(i);
        let root_j = self.find(j);
        if root_i != root_j {
            self.parent[root_i] = root_j;
        }
    }
}

/// Analyzes all playlists from YouTube Music and Spotify, detecting:
/// 1. Both Match (Name + Content) -> Auto-unified into 1 card.
/// 2. Name-only or Content-only matches -> User decision checked or returned as unresolved conflict.
pub fn analyze_playlists(
    ytm_playlists: &[Card],
    spotify_playlists: &[crate::transfer::spotify::Entry],
    decisions: &HashMap<String, String>,
) -> (Vec<PlaylistConflict>, Vec<UnifiedCard>) {
    let mut items: Vec<PlaylistItem> = Vec::with_capacity(ytm_playlists.len() + spotify_playlists.len());

    // 1. YouTube Music candidates
    for c in ytm_playlists {
        let id = match &c.target {
            Target::Playlist(id) => id.clone(),
            _ => c.title.clone(),
        };
        let count = parse_card_song_count(&c.subtitle);
        let norm_title = normalize_playlist_title(&c.title);
        items.push(PlaylistItem {
            id,
            title: c.title.clone(),
            norm_title,
            count,
            source_kind: SourceKind::YouTubeMusic,
            ytm_card: Some(c.clone()),
            spotify_entry: None,
        });
    }

    // 2. Spotify candidates
    for sp in spotify_playlists {
        let id = format!("sp:{}", sp.source.id());
        let norm_title = normalize_playlist_title(&sp.name);
        items.push(PlaylistItem {
            id,
            title: sp.name.clone(),
            norm_title,
            count: sp.count,
            source_kind: SourceKind::Spotify,
            ytm_card: None,
            spotify_entry: Some(sp.clone()),
        });
    }

    let n = items.len();
    let mut dsu = Dsu::new(n);
    let mut unresolved_conflicts: Vec<PlaylistConflict> = Vec::new();

    // 3. Compare cross-platform pairs (i, j)
    for i in 0..n {
        for j in (i + 1)..n {
            if items[i].source_kind == items[j].source_kind {
                continue;
            }
            let pair_key = conflict_pair_key(&items[i].id, &items[j].id);
            let decision = decisions.get(&pair_key).map(String::as_str);

            let same_name = !items[i].norm_title.is_empty() && items[i].norm_title == items[j].norm_title;

            // Content match: identical song counts when both are known and > 0,
            // or if both counts are None but the names match.
            let same_content = match (items[i].count, items[j].count) {
                (Some(a), Some(b)) if a > 0 && b > 0 => a == b,
                (None, None) if same_name => true,
                _ => false,
            };

            if same_name && same_content {
                // CASE 1: Both Name AND Content match -> auto-unify immediately.
                dsu.union(i, j);
            } else if same_name && !same_content {
                // CASE 2: Name matches, but content differs.
                match decision {
                    Some("merge") => dsu.union(i, j),
                    Some("separate") => {} // User explicitly wants them separated
                    _ => {
                        unresolved_conflicts.push(PlaylistConflict {
                            key: pair_key,
                            item_a: items[i].clone(),
                            item_b: items[j].clone(),
                            kind: ConflictKind::NameOnly {
                                name: items[i].title.clone(),
                                count_a: items[i].count,
                                source_a: items[i].source_kind,
                                count_b: items[j].count,
                                source_b: items[j].source_kind,
                            },
                        });
                    }
                }
            } else if !same_name && same_content && items[i].count.is_some_and(|c| c >= 8) {
                // CASE 3: Larger playlists with matching counts but different names.
                match decision {
                    Some("merge") => dsu.union(i, j),
                    Some("separate") => {} // User explicitly wants them separated
                    _ => {
                        unresolved_conflicts.push(PlaylistConflict {
                            key: pair_key,
                            item_a: items[i].clone(),
                            item_b: items[j].clone(),
                            kind: ConflictKind::ContentOnly {
                                name_a: items[i].title.clone(),
                                source_a: items[i].source_kind,
                                name_b: items[j].title.clone(),
                                source_b: items[j].source_kind,
                                count: items[i].count.unwrap_or(0),
                            },
                        });
                    }
                }
            }
        }
    }

    // 4. Group unified items into UnifiedCards
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        let root = dsu.find(i);
        groups.entry(root).or_default().push(i);
    }

    let mut unified_cards: Vec<UnifiedCard> = Vec::with_capacity(groups.len());

    for group in groups.values() {
        let group_items: Vec<&PlaylistItem> = group.iter().map(|&idx| &items[idx]).collect();
        let is_unified = group_items.len() > 1;

        let has_ytm = group_items.iter().any(|it| it.source_kind == SourceKind::YouTubeMusic);
        let has_spotify = group_items.iter().any(|it| it.source_kind == SourceKind::Spotify);

        let mut sources = Vec::new();
        if has_ytm {
            sources.push(SourceKind::YouTubeMusic);
        }
        if has_spotify {
            sources.push(SourceKind::Spotify);
        }

        // Prefer YouTube Music card if available (opens natively), otherwise Spotify entry
        let ytm_item = group_items.iter().find(|it| it.source_kind == SourceKind::YouTubeMusic);
        let spotify_item = group_items.iter().find(|it| it.source_kind == SourceKind::Spotify);

        let count = group_items.iter().find_map(|it| it.count);

        let card = if let Some(yt) = ytm_item {
            let mut c = yt.ytm_card.clone().unwrap();
            if is_unified && has_spotify {
                let count_str = count.map_or_else(String::new, |n| format!(" • {n} songs"));
                c.subtitle = format!("Synced (YouTube Music & Spotify){count_str}");
            }
            c
        } else if let Some(sp) = spotify_item {
            let entry = sp.spotify_entry.as_ref().unwrap();
            let subtitle = if is_unified && has_ytm {
                let count_str = count.map_or_else(String::new, |n| format!(" • {n} songs"));
                format!("Synced (YouTube Music & Spotify){count_str}")
            } else {
                sp.count.map_or_else(
                    || format!("Spotify • {}", entry.owner),
                    |c| format!("Spotify • {c} songs"),
                )
            };
            let thumbs = crate::ui::transfer::spotify_thumbs(&entry.art);
            Card {
                title: entry.name.clone(),
                subtitle,
                thumbs,
                target: Target::Playlist(format!("sp:{}", entry.source.id())),
                round: false,
                links: vec![Link {
                    name: entry.owner.clone(),
                    id: None,
                }],
            }
        } else {
            continue;
        };

        unified_cards.push(UnifiedCard {
            card,
            sources,
            is_unified,
            count,
        });
    }

    (unresolved_conflicts, unified_cards)
}

/// Renders an interactive conflict resolution banner at the top of the playlists tab.
pub fn render_conflict_banner(ui: &mut Ui, conflict: &PlaylistConflict, app: &mut App) {
    let card_height = 82.0;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), card_height), Sense::hover());

    // Card styling with soft glass and border
    ui.painter().rect_filled(rect, 12.0, theme::glass(0.06));
    ui.painter().rect_stroke(
        rect,
        12.0,
        egui::Stroke::new(1.0, theme::glass(0.12)),
        egui::StrokeKind::Inside,
    );

    let inner = rect.shrink(14.0);
    let cy = inner.center().y;

    // Left Icon
    let icon_r = Rect::from_center_size(pos2(inner.left() + 20.0, cy), Vec2::splat(34.0));
    ui.painter().rect_filled(icon_r, 8.0, theme::glass(0.12));
    crate::ui::widgets::icon_at(ui, icon_r.center(), icon::ROTATE_CW, 17.0, theme::RED);

    // Right Action Buttons
    let btns_w = 240.0_f32.min((inner.width() * 0.45).max(180.0));
    let btns_r = Rect::from_min_max(pos2(inner.right() - btns_w, inner.top()), inner.max);

    // Title & Description Text (clipped cleanly before buttons)
    let tx = icon_r.right() + 14.0;
    let text_max_w = (btns_r.left() - tx - 14.0).max(80.0);
    match &conflict.kind {
        ConflictKind::NameOnly {
            name,
            count_a,
            source_a,
            count_b,
            source_b,
        } => {
            let ca_str = count_a.map_or_else(|| "?".to_string(), |c| c.to_string());
            let cb_str = count_b.map_or_else(|| "?".to_string(), |c| c.to_string());
            crate::ui::widgets::text_at(
                ui,
                pos2(tx, cy - 19.0),
                &format!("Playlist conflict: \"{name}\""),
                theme::semibold(14.5),
                theme::text(),
                text_max_w,
            );
            crate::ui::widgets::text_at(
                ui,
                pos2(tx, cy + 2.0),
                &format!(
                    "Same name, different sizes: {} ({ca_str} songs) vs. {} ({cb_str} songs).",
                    source_a.label(),
                    source_b.label()
                ),
                theme::regular(12.5),
                theme::text_dim(),
                text_max_w,
            );
        }
        ConflictKind::ContentOnly {
            name_a,
            source_a,
            name_b,
            source_b,
            count,
        } => {
            crate::ui::widgets::text_at(
                ui,
                pos2(tx, cy - 19.0),
                "Possible duplicate playlist",
                theme::semibold(14.5),
                theme::text(),
                text_max_w,
            );
            crate::ui::widgets::text_at(
                ui,
                pos2(tx, cy + 2.0),
                &format!(
                    "Matching count ({count} songs): \"{name_a}\" ({}) vs. \"{name_b}\" ({}).",
                    source_a.label(),
                    source_b.label()
                ),
                theme::regular(12.5),
                theme::text_dim(),
                text_max_w,
            );
        }
    }

    ui.scope_builder(egui::UiBuilder::new().max_rect(btns_r), |ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if pill(ui, "Keep separate", Some(icon::X), false).clicked() {
                app.set_playlist_decision(&conflict.key, "separate");
                app.toast("Playlists kept separate.");
            }
            ui.add_space(8.0);
            if pill(ui, "Merge", Some(icon::CHECK), true).clicked() {
                app.set_playlist_decision(&conflict.key, "merge");
                app.toast("Playlists merged.");
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transfer::spotify::{Entry, Source};

    fn ytm_card(id: &str, title: &str, songs: usize) -> Card {
        Card {
            title: title.into(),
            subtitle: format!("Playlist • {songs} songs"),
            thumbs: vec![],
            target: Target::Playlist(id.into()),
            round: false,
            links: vec![],
        }
    }

    fn sp_entry(id: &str, name: &str, songs: usize) -> Entry {
        Entry {
            source: Source::Playlist(id.into()),
            name: name.into(),
            owner: "User".into(),
            count: Some(songs),
            art: None,
        }
    }

    #[test]
    fn exact_match_unifies_and_same_platform_never_conflicts() {
        let ytm = vec![ytm_card("pl1", "Gym Mix", 20), ytm_card("pl2", "Gym Mix", 15)];
        let sp = vec![sp_entry("sp1", "Gym Mix", 20)];
        let decisions = HashMap::new();
        let (conflicts, cards) = analyze_playlists(&ytm, &sp, &decisions);
        // pl1 and sp1 unify; pl2 conflicts only cross-platform with sp1, not with pl1.
        assert_eq!(conflicts.len(), 1);
        assert_eq!(cards.len(), 2);
    }
}
