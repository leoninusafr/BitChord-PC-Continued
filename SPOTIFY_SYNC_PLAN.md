# BitChord PC – Spotify Integration & Two-Way Sync Architektur

Dieses Dokument beschreibt die genaue Architektur und das Konzept für die vollständige Zwei-Wege-Synchronisation zwischen BitChord (YouTube Music) und Spotify – inklusive kollaborativer Playlists, fremder Playlists, Metadaten-Sync (Titel & Cover) und Rechteverwaltung.

---

## 1. Aktueller Status: Was ist bereits synchronisiert?

### A. Gelikete Songs (Favoriten) – Live & Bidirektional
- **Verhalten**: Klick auf das **Herz-Icon** in BitChord übermittelt den Titel sofort im Hintergrund an die Spotify Web-API (`sync_like_to_spotify`).
- **Ergebnis**: Der Song wird in deiner echten Spotify-Bibliothek unter *„Lieblingssongs“* gespeichert. Bei erneutem Klick wird der Like auf Spotify wieder entfernt.

### B. Playlists von Spotify in BitChord anzeigen
- **Verhalten**: Auf Spotify erstellte und gefolgte Playlists werden ausgelesen und in der linken Seitenleiste mit dem Spotify-Symbol aufgelistet.
- **Ergebnis**: Playlists können direkt in BitChord gestreamt werden; Titel werden on-the-fly mit YouTube Music in bester Audioqualität gematcht.

---

## 2. Playlists-Kategorien & Berechtigungsmodell

In Spotify existieren drei grundlegend verschiedene Arten von Playlists, die BitChord sauber unterscheiden muss:

```
                  ┌────────────────────────────────────────┐
                  │          Spotify Playlists             │
                  └──────────────────┬─────────────────────┘
                                     │
           ┌─────────────────────────┼─────────────────────────┐
           ▼                         ▼                         ▼
   [ 1. Eigene Playlist ]   [ 2. Geteilte / Kollaborativ ]   [ 3. Fremde / Gefolgt ]
   owner.id == my_id         collaborative == true           owner.id != my_id
   Voller Lese- & Schreib-   Mehrere Personen können         Reiner Lesezugriff (Read-Only)
   zugriff (Titel, Cover,    Songs hinzufügen/löschen        Updates des Erstellers werden
   Songs add/remove)         Attribut: 'Added by [User]'     nur empfangen (One-Way)
```

---

## 3. Geteilte & Kollaborative Playlists (Collaborative Playlists)

### A. Das Szenario:
- Du hast eine Playlist auf Spotify, die du mit Freunden oder der Familie teilst.
- Freunde fügen auf Spotify neue Songs hinzu oder löschen Songs.
- Du fügst in BitChord Songs hinzu oder löschst Songs.

### B. Die technische Lösung (Delta-Sync & Snapshot-IDs):
1. **Kein blindes Überschreiben (Snapshot-ID Tracking)**:
   - Spotify nutzt für jede Playlist eine eindeutige `snapshot_id` (z. B. `MSwxMjM0NWFiY...`).
   - BitChord speichert die letzte bekannte `snapshot_id`.
   - Bei jeder Änderung vergleicht BitChord die lokale Snapshot-ID mit der von Spotify:
     - Hat sich die Snapshot-ID geändert, zieht BitChord zuerst die Änderungen der Freunde ab (Delta-Update).
     - Erst danach wird deine lokale Änderung (`add` oder `remove`) als atomare Operation an Spotify gesendet.
     - **Ergebnis**: Niemals werden Änderungen von Freunden versehentlich überschrieben oder gelöscht!
2. **„Added by [Name]“-Attributierung**:
   - Die Spotify Web-API liefert für jeden Track das Feld `added_by`:
     ```json
     "added_by": {
       "id": "friend_user_id",
       "display_name": "Sarah"
     }
     ```
   - In BitChord wird in der Songzeile dezent angezeigt:
     `Titel • Interpret • Added by Sarah`
     *(Das `added_by`-Feld ist in BitChords `RowOpts` bereits architektonisch vorbereitet!)*
3. **Song in geteilter Playlist löschen**:
   - Wenn du einen Song entfernst, sendet BitChord `DELETE /v1/playlists/{id}/tracks` mit `snapshot_id` an Spotify.
   - Der Song verschwindet sowohl in BitChord als auch bei deinen Freunden auf Spotify.

---

## 4. Fremde / Gelikte Playlists (Followed / Read-Only Playlists)

### A. Das Szenario:
- Du hast auf Spotify eine Playlist geliked (z. B. *„Today's Top Hits“* von Spotify oder eine Playlist von einem Freund/Künstler).
- Du hast auf Spotify **keine Schreibrechte** (`owner.id != my_id` und `collaborative == false`).

### B. Das Verhalten in BitChord:
1. **Schreibschutz & Deaktivierte Buttons**:
   - In BitChord wird die Playlist als schreibgeschützt markiert.
   - Der Button „Songs hinzufügen“ oder „Aus Playlist entfernen“ ist deaktiviert oder blendet einen informativen Tooltip ein:
     `„This is a followed playlist created by [Owner]. You cannot modify it on Spotify.“`
2. **Automatischer One-Way-Sync**:
   - Wenn der Ersteller auf Spotify neue Songs hinzufügt oder die Reihenfolge ändert, lädt BitChord diese Änderungen beim Sync/Start automatisch nach.
3. **Ersteller-Anzeige**:
   - Im Header steht klar: `By Spotify • 50 songs` oder `By Max • 34 songs`.

---

## 5. Playlist-Titel & Cover ändern (Metadaten-Synchronisation)

### A. Titel & Beschreibung ändern:
- **Eigene / Kollaborative Playlists**:
  - Wenn du in BitChord auf den Stift klickst und den Namen oder die Beschreibung änderst:
    BitChord sendet `PUT /v1/playlists/{playlist_id}` mit `{ "name": "Neuer Name", "description": "..." }` an Spotify.
    Wenige Sekunden später heißt die Playlist auch auf Spotify und bei allen Mitwirkenden so.
- **Fremde Playlists**:
  - Der Edit-Stift ist ausgegraut, da man auf Spotify fremde Playlists nicht umbenennen darf.
- **Umgekehrt (Änderung auf Spotify)**:
  - Wenn du oder ein Freund auf Spotify den Namen ändert, übernimmt BitChord beim nächsten automatischen Refresh den neuen Namen.

### B. Playlist-Cover ändern:
- **Eigene Playlists**:
  - Wenn du in BitChord ein neues Bild als Cover auswählst:
    BitChord skaliert das Bild auf maximal 640×640 px JPEG und sendet es per `PUT /v1/playlists/{playlist_id}/images` (Base64-kodiert) an die Spotify-API.
    Auf Spotify erscheint sofort dein neues Custom-Cover!
- **Fremde Playlists**:
  - Cover-Upload ist gesperrt; BitChord cached und zeigt immer das aktuelle offizielle Cover des Erstellers.
- **Umgekehrt (Cover auf Spotify geändert)**:
  - BitChord aktualisiert den Cover-Cache automatisch, sobald die Spotify-API eine neue Bild-URL meldet.

---

## 6. Sicherheit, Warnhinweise & Opt-In Schalter

Wie vom Nutzer gefordert, wird die Synchronisation niemals heimlich oder destruktiv scharfgeschaltet:

1. **Dedizierter Umschalter in Settings (`Settings > Spotify`)**:
   - Standardmäßig: `[ ] Read-Only Sync (Playlists nur anzeigen und abspielen)`.
   - Optional: `[x] Enable Full 2-Way Sync (Änderungen live mit Spotify abgleichen)`.
2. **Warnhinweis beim ersten Aktivieren**:
   > **Achtung – Vollständiger Zwei-Wege-Sync**:
   > Alle Änderungen, die du an deinen eigenen und kollaborativen Playlists in BitChord vornimmst (Songs hinzufügen, löschen, Titel ändern), werden direkt auf dein Spotify-Konto übertragen.
3. **Playlist-Ausschluss ("Nicht synchronisieren")**:
   - Für jede Playlist kann im Kontextmenü (Rechtsklick) festgelegt werden:
     `[x] Do not sync this playlist to Spotify`.
   - Damit lassen sich private YouTube-Listen führen, die niemals auf Spotify landen sollen.

---

*Status: Vollständige Konzeption und Architektur dokumentiert. Bleibt im Plan, bis zur Umsetzung.*
