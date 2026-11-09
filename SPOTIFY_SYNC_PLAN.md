# BitChord PC – Spotify Integration & Sync Architektur

Dieses Dokument beschreibt die genaue Funktionsweise des aktuellen Spotify-Syncs in BitChord PC und die geplanten Erweiterungen.

---

## 1. Aktueller Status: Was ist bereits synchronisiert?

### A. Gelikete Songs (Favoriten) – Live & Bidirektional
- **Verhalten**: Wenn du in BitChord auf das **Herz-Icon** klickst, wird der Titel automatisch im Hintergrund an die Spotify Web-API übermittelt (`sync_like_to_spotify`).
- **Ergebnis**: Der Song wird in deiner echten Spotify-Bibliothek unter *„Lieblingssongs“* gespeichert. Klickst du erneut drauf (Entfernen), wird der Like auch auf Spotify entfernt.

### B. Playlists von Spotify in BitChord anzeigen
- **Verhalten**: Deine auf Spotify erstellten Playlists werden ausgelesen und in der linken Seitenleiste mit dem Spotify-Symbol aufgelistet.
- **Ergebnis**: Du kannst deine Spotify-Playlists direkt in BitChord anhören; die Titel werden on-the-fly mit bester Audioqualität gematcht.

---

## 2. Antworten auf deine Fragen

### Frage 1: „Wenn ich bei Spotify eine neue Playlist erstelle, ist die dann auch in BitChord?“
- **Antwort**: **Ja!**
- **Wie es funktioniert**: Beim Start der App und über den Refresh-Button lädt BitChord deinen Account neu. 
- **Geplante Optimierung**: Ein automatischer Live-Check im Hintergrund (z. B. alle 60 Sekunden oder bei Fenster-Fokus), damit neu erstellte Playlists ohne manuellen Klick sofort in der Seitenleiste erscheinen.

### Frage 2: „Kann ich in die Spotify-Playlists einfach Songs reinpacken und auf Spotify sind sie dann auch drin?“
- **Aktueller Status**: Aktuell landen Songs beim Hinzufügen primär in der YouTube-Music-Datenbank.
- **Geplante Neuerung (Step 10 im Master-Plan)**:
  - Im Kontextmenü eines Titels (Rechtsklick / `...`-Button) kommt der Eintrag: `Zu Spotify-Playlist hinzufügen`.
  - BitChord nimmt den Song, ermittelt die Spotify-Track-ID und schickt den Befehl `POST /v1/playlists/{playlist_id}/tracks` direkt an deinen Spotify-Account.
  - **Ergebnis**: Der Song taucht wenige Sekunden später auch auf deinem Handy in der Spotify-App auf.

### Frage 3: „Sollten Spotify-Playlists einen anderen Stil haben als normale Playlists?“
- **Empfehlung**: **Ja, aber dezent und edel!**
- **Grund**: Es ist extrem praktisch, auf den ersten Blick zu wissen, wo eine Playlist gespeichert ist (Spotify vs. YouTube Music).
- **Design-Ansatz**:
  - Typografie, Abstände, Hover-Zustände und Animationen sind **100% identisch**.
  - Der einzige Unterschied: Ein feiner, dezenter Spotify-Akzentpunkt (oder ein kleines grünes Symbol), damit du sofort weißt: *„Das ist eine Spotify-Playlist“*.

---

## 3. Geplante technische Schritte für den vollen Playlist-Schreib-Sync

1. **Spotify-Client API-Erweiterung (`src/transfer/spotify.rs`)**:
   - `add_track_to_playlist(playlist_id: &str, track_uri: &str)`
   - `remove_track_from_playlist(playlist_id: &str, track_uri: &str)`
2. **UI-Dialog (`src/ui/mod.rs` & `src/ui/actions.rs`)**:
   - Beim Klick auf „Zu Playlist hinzufügen“ werden sowohl deine lokalen / YouTube-Playlists als auch deine Spotify-Playlists zur Auswahl angeboten.
3. **Auto-Polling / Live-Refresh**:
   - Ein sanfter Background-Worker, der deine Spotify-Playlists alle paar Minuten oder bei Fenster-Aktivierung auf neue Einträge prüft.
