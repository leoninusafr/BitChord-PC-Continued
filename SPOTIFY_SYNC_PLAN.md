# BitChord PC – Spotify Integration & Two-Way Sync Architektur

Dieses Dokument beschreibt die vollständige Architektur und das Konzept für den **Zwei-Wege-Sync** zwischen BitChord (YouTube Music Engine) und Spotify – inklusive kollaborativer Playlists, dynamischer Mixtapes ("Dein Mixtape 2"), Einladungslinks für Freunde, Metadaten-Sync (Titel & Cover) und Rechteverwaltung.

---

## 1. Leitprinzip: 100% Parität zu nativem Spotify

> **Ziel**: Wenn der Two-Way-Sync aktiviert ist, verhält sich BitChord für dein Spotify-Konto exakt so, als ob du die offizielle Spotify Desktop-App verwenden würdest.
> Jede Änderung (Song hinzufügen, löschen, umbenennen, Cover anpassen, Kollaborationen) wird nahtlos und in Echtzeit zwischen beiden Welten synchronisiert.

---

## 2. Playlists-Kategorien & Berechtigungsmodell

BitChord unterscheidet clientseitig sauber zwischen drei Playlist-Typen:

```
                          ┌────────────────────────────────────────┐
                          │          Spotify Playlists             │
                          └──────────────────┬─────────────────────┘
                                             │
      ┌──────────────────────────────────────┼──────────────────────────────────────┐
      ▼                                      ▼                                      ▼
[ 1. Eigene Playlists ]             [ 2. Kollaborativ ]                    [ 3. Fremd / Algorithmisch ]
• Ersteller: Du                     • Ersteller: Du oder Freund            • "Dein Mixtape 2", Top 50, etc.
• Vollzugriff: Songs, Titel,        • Jeder Mitwirkende darf Songs         • Nur in Bibliothek, wenn geliked (+)
  Beschreibung, Cover                 hinzufügen & entfernen               • Reiner Lesezugriff (Read-Only)
• Live-Sync in beide Richtungen     • 'Added by [Name]' für jeden Track    • Auto-Update, wenn Spotify ändert
```

---

## 3. Geteilte & Kollaborative Playlists (Collaborative Sync)

### A. Wie funktioniert die Zusammenarbeit?
- **Freund löscht Song auf Spotify** $\to$ Der Song verschwindet automatisch auch in deiner BitChord-Playlist.
- **Leon löscht Song in BitChord** $\to$ BitChord sendet den Löschbefehl atomar an die Spotify-API (`DELETE /v1/playlists/{id}/tracks` mit `snapshot_id`). Der Song ist sofort auch auf Spotify für alle Freunde weg.
- **Freund fügt Song auf Spotify hinzu** $\to$ BitChord erkennt den neuen Track, matcht ihn für YouTube Music und zeigt ihn mit dem Badge *"Added by [Freund]"* an.
- **Leon fügt Song in BitChord hinzu** $\to$ BitChord ermittelt die Spotify-ID und pusht den Track sofort auf Spotify (`POST /v1/playlists/{id}/tracks`).
- **Neuer Kollaborateur tritt bei** $\to$ Die Mitwirkenden-Liste in BitChord aktualisiert sich automatisch.

### B. Schutz vor Konflikten (Snapshot-ID Delta-Sync):
- Spotify vergibt bei jeder Playlist-Änderung eine neue `snapshot_id`.
- Vor jedem Schreibvorgang prüft BitChord, ob die lokale `snapshot_id` noch aktuell ist.
- Wurde die Playlist zwischenzeitlich von einem Freund geändert, zieht BitChord zuerst im Millisekundenbereich das Delta (die Änderungen des Freundes) ab und wendet erst danach deine Aktion an.
- **Ergebnis**: Kein Überschreiben oder versehentliches Löschen von fremden Beiträgen.

---

## 4. Algorithmische & Fremde Playlists ("Dein Mixtape 2" / "Daily Mix")

### A. Das Verhalten bei dynamischen Spotify-Mixes:
- **Speicher-Filter (Bibliotheks-Regel)**:
  - Nur Playlists, die du in deiner Spotify-Bibliothek mit dem **Plus (`+`)** markiert hast (z. B. *"Dein Mixtape 2"*), erscheinen in BitChord.
  - Mixes, die du nicht in deiner Bibliothek hast (z. B. *"Dein Mixtape 3"*), tauchen gar nicht erst auf.
- **Algorithmische Auto-Updates**:
  - Spotify verändert den Inhalt von *"Dein Mixtape"* regelmäßig automatisch.
  - BitChord aktualisiert den Inhalt im Hintergrund (One-Way Sync von Spotify zu BitChord), sobald Spotify neue Songs einsteuert.
- **Schreibschutz**:
  - Da Spotify bei algorithmischen Mixen keine manuellen Änderungen erlaubt, ist der "Hinzufügen"- und "Löschen"-Button in BitChord für diese Playlists ausgegraut, mit klarem Hinweis:
    *„Von Spotify für dich erstellt – keine manuelle Bearbeitung möglich.“*

---

## 5. Spotify-Teilen & Freunde einladen ("Spotify Kollaborateur hinzufügen")

### Das Szenario:
Du erstellst eine Playlist in BitChord und möchtest sie mit einem Freund teilen, der **nur die normale Spotify-App** (am Handy oder PC) nutzt:

1. **„Spotify Link teilen“**:
   - Im Teilen-Menü der Playlist wählst du: `[🔗 Spotify-Link kopieren]`.
   - Erzeugt die offizielle URL `https://open.spotify.com/playlist/{id}`.
2. **„Spotify-Kollaborateur einladen“**:
   - BitChord ruft die Spotify API auf und erzeugt einen sicheren Kollaborations-Einladungslink (`open.spotify.com/playlist/{id}?si=...&pt=...`).
   - Du schickst diesen Link deinem Freund (z. B. via WhatsApp, Discord oder Mail).
   - **Dein Freund klickt drauf**: Die normale Spotify-App öffnet sich, er tritt bei und kann sofort Songs beisteuern.
   - **Du nutzt weiterhin BitChord**: Alle Songs, die dein Freund hinzufügt, landen live bei dir in BitChord!

---

## 6. Metadaten-Sync: Titel, Beschreibung & Cover

### A. Titel & Beschreibung:
- **Änderung in BitChord**: Titel im Header anklicken, neuen Namen eintippen $\to$ BitChord sendet `PUT /v1/playlists/{id}` an Spotify.
- **Änderung auf Spotify**: Wird sofort beim nächsten Refresh in BitChord übernommen.
- **Rechte-Prüfung**: Bei fremden Playlists ist die Umbenennung gesperrt.

### B. Cover-Verhalten:
- **Neues Cover hochladen**:
  - Bild auswählen in BitChord $\to$ BitChord komprimiert auf max. 640×640 px JPEG und sendet es per `PUT /v1/playlists/{id}/images` (Base64) an Spotify.
  - Auf Spotify erscheint sofort dein neues Custom-Cover.
- **Bestehendes Cover ändern**:
  - Überschreibt das alte Cover auf Spotify und leert den lokalen Cache.
- **Cover löschen**:
  - Löscht das Custom-Cover auf Spotify $\to$ Spotify stellt automatisch das klassische 4er-Mosaik der ersten Tracks wieder her. BitChord übernimmt ebenfalls sofort das 4er-Mosaik.
- **Cover auf Spotify geändert**:
  - BitChord erkennt die geänderte Bild-URL in der API und blendet das neue Artwork flüssig ein.

---

## 7. Sicherheit, Warnhinweise & Opt-In

Um maximale Kontrolle und Datenschutz zu gewährleisten:

1. **Umschalter in Settings (`Settings > Spotify`)**:
   - `[ ] Read-Only Sync (Playlists nur anzeigen und anhören)` (Standard)
   - `[x] Full Two-Way Sync (Änderungen live mit Spotify synchronisieren)`
2. **Warnhinweis beim Einschalten**:
   > **Achtung – Vollständiger Zwei-Wege-Sync aktiv**:
   > Alle Änderungen an deinen eigenen und geteilten Playlists (Hinzufügen, Entfernen, Titel, Cover) werden direkt auf dein Spotify-Konto übertragen.
3. **Blacklist / Playlist-Ausschluss ("Nicht synchronisieren")**:
   - Per Rechtsklick auf eine beliebige Playlist:
     `[x] Diese Playlist nicht mit Spotify synchronisieren`.
   - Schützt private YouTube-Listen davor, jemals auf Spotify zu landen.

---

*Status: Vollständiges Gesamtkonzept dokumentiert. Zur späteren schrittweisen Implementierung vorgemerkt.*
