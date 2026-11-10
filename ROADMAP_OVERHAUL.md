# BitChord PC – Master Implementation Plan & Roadmap

Dieser Plan fasst alle Anforderungen, Detail-Kritiken und Ideen strukturiert in priorisierte, modulare **Plan-Steps** zusammen. 
**Hinweis**: Es wurden noch keine Code-Änderungen durchgeführt; diese Datei dient als exakte Arbeitsgrundlage für die Umsetzung.

---

## Inhaltsverzeichnis

1. [Step 1: Spicetify „Play a Random Song“ (TechShivvy & Hitesh1090 Architektur)](#step-1-spicetify-play-a-random-song-techshivvy--hitesh1090-architektur)
2. [Step 2: Mobile App Toggle-Switches & Slider (BitChord Mobile V2.x Physik)](#step-2-mobile-app-toggle-switches--slider)
3. [Step 3: Player-Bar: Transport-Buttons, Taktiles Feedback & Cover-Kontextmenü](#step-3-player-bar-transport-buttons-taktiles-feedback--cover-kontextmenü)
4. [Step 4: Lautstärke-Leiste: Übergang, Knopflose Kapsel & Dicke](#step-4-lautstärke-leiste-übergang-knopflose-kapsel--dicke)
5. [Step 5: Player-Bar Tool-Icons: Einheitliche Linienstärke](#step-5-player-bar-tool-icons-einheitliche-linienstärke)
6. [Step 6: Rechte Seitenleiste („Aktueller Titel“ / Now Playing View)](#step-6-rechte-seitenleiste-aktueller-titel--now-playing-view)
7. [Step 7: Warteschlange & Song-Menüs: Herz-Icon statt Stern & Rechtsklick-Pop-up](#step-7-warteschlange--song-menüs-herz-icon-statt-stern--rechtsklick-pop-up)
8. [Step 8: Linke Seitenleiste: Bereinigung, Abstände, Vertikallinie & Einzeilige Playlists](#step-8-linke-seitenleiste-bereinigung-abstände-vertikallinie--einzeilige-playlists)
9. [Step 9: Bibliotheks-Grid: Symmetrie, Smooth Fading & Resize-Flackern beheben](#step-9-bibliotheks-grid-symmetrie-smooth-fading--resize-flackern-beheben)
10. [Step 10: Top-Bar: Layout-Architektur (Pfeil- & Home-Positionen), Responsive Lupe & Logo Schärfe](#step-10-top-bar-layout-architektur-pfeil---home-positionen-responsive-lupe--logo-schärfe)
11. [Step 11: Vollständige Lokalisierung auf Englisch](#step-11-vollständige-lokalisierung-auf-englisch)
12. [Step 12: Spotify Playlists: 100% gleiches Design, „[Name]’s Spotify“ & 2-Wege-Sync](#step-12-spotify-playlists-100-gleiches-design-names-spotify--2-wege-sync)
13. [Step 13: Remote-Steuerung & Audio-Output Routing (Cast / Phone)](#step-13-remote-steuerung--audio-output-routing-cast--phone)
14. [Step 14: Fullscreen-Modus vereinfachen](#step-14-fullscreen-modus-vereinfachen)
15. [Step 15: Zukünftiges UI-Card-Design (Vorerst beobachten)](#step-15-zukünftiges-ui-card-design)
16. [Step 16: Playlist-Header: Doppelte Song-Anzahl entfernen & Vertikale Cover-Bündigkeit](#step-16-playlist-header-doppelte-song-anzahl-entfernen--vertikale-cover-bündigkeit)
17. [Step 17: Such-System: Doppelte Suchleiste entfernen, Umrandung nur bei Fokus & Empty-State füllen](#step-17-such-system-doppelte-suchleiste-entfernen-umrandung-nur-bei-fokus--empty-state-füllen)
18. [Step 18: Schwebende Frosted-Glass Top-Bar (Transparenter Header beim Scrollen)](#step-18-schwebende-frosted-glass-top-bar-transparenter-header-beim-scrollen)
19. [Step 19: Echter Shuffle-Start bei Playlists (Zufälliger Song statt immer Track 1)](#step-19-echter-shuffle-start-bei-playlists-zufälliger-song-statt-immer-track-1)
20. [Step 20: Discover-Page Überholung (Genres, Moods & Bento-Cards)](#step-20-discover-page-überholung-genres-moods--bento-cards)
21. [Step 21: Repeat-Modi (Repeat 1 vs. All & „1“-Icon Schärfe) & Akzentfarben-Harmonisierung](#step-21-repeat-modi-repeat-1-vs-all--1-icon-schärfe--akzentfarben-harmonisierung)
22. [Step 22: Song-Wechsel Glitch-Fix (Cover & Lyrics Race Condition), Typo-Underline & Globales Kontextmenü](#step-22-song-wechsel-glitch-fix-cover--lyrics-race-condition-typo-underline--globales-kontextmenü)

---

## Step 1: Spicetify „Play a Random Song“ (TechShivvy & Hitesh1090 Architektur)

### Deep-Dive in die Spicetify-Extension `Play a Random Song`:
Die Extension von **TechShivvy** und **Hitesh1090** nutzt einen genialen Mechanismus:
1. **Target-Katalog `thesoundsofspotify` (Every Noise at Once)**:
   - Spotify hostet über den Account `thesoundsofspotify` über **6.000 kuratierte Genre-Playlists**, die praktisch jedes existierende Musik-Subgenre der Welt abbilden (von obskurem japanischem Shoegaze über Underground Dark Ambient bis zu winzigen Indie-Künstlern).
2. **2-Stufiger Zufalls-Algorithmus**:
   - Die Extension holt sich mit einem zufälligen `offset` eine zufällige Playlist aus den tausenden Genre-Listen.
   - Aus dieser Playlist wählt sie dann per Zufallsstichprobe (`sample`) einen spielbaren Track aus.
   - Dadurch entdeckt man echte Underground-Künstler mit 10 monatlichen Hörern, weil nicht aus den Top-Charts geschuffelt wird, sondern aus tiefen Nischen-Listen!
3. **Queue-Prefetching**:
   - Sobald ein Random Song abgespielt wird, holt die Extension im Hintergrund bereits den nächsten Überraschungs-Track und reiht ihn in die Warteschlange ein.

### Konkrete Umsetzung in BitChord PC:
- [ ] **Integration der „The Sounds of Spotify“ / Genre-Kataloge**:
  - Wir hinterlegen das Mapping der 6.000+ Subgenre-Listen bzw. nutzen die Spotify Web-API / YouTube Music Nischen-Kataloge.
  - Beim Klick auf den Würfel wird eine zufällige Nische ausgewählt und daraus ein Track gestartet.
- [ ] **Kombination mit dem globalen Silben-Cluster-Algorithmus**:
  - Umschaltbar oder gemischt: Einmal aus obskuren Subgenre-Listen, einmal aus weltweiten YouTube-Music-Tiefensuchen.
- [ ] **Prefetch in die Queue**:
  - Wenn der Würfel gedrückt wird, wird der Song gestartet und der nächste Random-Discovery-Track direkt als nächstes in die Queue gelegt.
- [ ] **Interaktiver Toast**:
  - `„Discovering from [Genre / Nische]: [Title] by [Artist]“`.

---

## Step 2: Mobile App Toggle-Switches & Slider

### Analyse des Mobile App Switch-Designs:
- **Pill-Form & Maße**: Kompakte Pille (ca. 42 × 24 px), `CornerRadius::same(12.0)`.
- **Interaktions-Physik**:
  - Beim Antippen/Klicken dehnt sich der Knopf leicht oval (`scaleX = 1.15`), bevor er auf die andere Seite schnappt (Liquid Stretch).
  - Dezent elastisches Einrasten (Spring-Physics) statt linearer Bewegung.
- **Farben & Schatten**:
  - **Aus**: Feines transluzentes Dunkelgrau (`theme::glass(0.16)` mit subtilem Rand).
  - **Ein**: Dynamische Song-Akzentfarbe oder Mobile-Korallenrot (`#FA2D48`), mit zartem farbigen Glow/Schatten (`0.25` Alpha).
  - **Knob**: Reines Weiß (`Color32::WHITE`) mit zartem Drop-Shadow nach unten.

### Konkrete Umsetzung in `src/ui/widgets.rs` & `src/ui/pages.rs`:
- [ ] Neuen einheitlichen Widget-Typ `mobile_switch(ui, &mut bool, "Label")` und `mobile_slider(ui, &mut val, range)` implementieren.
- [ ] Sanfte Easing-Kurven (`spring_ease_out` mit Nachfederung) für alle Toggles in den Einstellungen und im Player hinterlegen.

---

## Step 3: Player-Bar: Transport-Buttons, Taktiles Feedback & Cover-Kontextmenü

### Konkrete Umsetzung:
- [ ] **Buttons vergrößern & Abstände erweitern**:
  - **Play/Pause**: Durchmesser von 34 px auf **38–40 px** anheben (taktiler Klick mit `scale 0.94` bleibt erhalten).
  - **Vor / Zurück**: Glyph-Größe von 22 px auf **25 px** anheben.
  - **Shuffle & Repeat**: Icon-Größe von 18 px / 20 px auf **22 px** anheben.
  - **Abstände**: Horizontalen Abstand zwischen den Buttons um ca. 8–10 px vergrößern für ein luftiges, entspanntes Klickgefühl.
- [ ] **Taktiles visuelles Feedback für Skip & Zurück**:
  - Beim Klicken (Pointer down) schrumpft das Icon dezent (`scale(0.92)` Micro-Press).
  - Ein sanfter, runder Glass-Flash (`theme::glass(0.15)`) blinkt hinter dem Button auf und fadet in 150 ms sanft aus.
- [ ] **Cover- & Titel-Klick Pop-up (Aktionsmenü)**:
  - Klick oder Rechtsklick auf das Album-Cover oder den Songtitel links unten öffnet das gewohnte Aktions-Pop-up-Menü:
    - *Add to playlist*
    - *Share song*
    - *Go to artist*
    - *Like / Remove like*
- [ ] **Vertikale Position & Glass-Hintergrund**:
  - Die gesamte Transport-Reihe ca. 2–3 Pixel tiefer setzen, um eine perfekte visuelle Achse zwischen Cover und Seek-Bar zu schaffen.
  - Den Grundton im Screen-Shader unter der Bar neutralisieren, sodass das Frosted Liquid Glass sauber und wie auf iOS/macOS transparent wirkt.

---

## Step 4: Lautstärke-Leiste: Übergang, Knopflose Kapsel & Dicke

### Problemstellung:
- Der Lautstärkebalken wird beim Hovern aktuell dicker als die Seekbar (11 px vs. 8 px).
- Beim Verstellen ist der Füllbalken am rechten Ende rund gezeichnet und darauf sitzt der weiße Kreis -> erzeugt eine unschöne „Doppel-Wölbung“.

### Gewählte Umsetzung (Entscheidung: Option B zuerst, Option A für später vorgemerkt):
- [ ] **Dicken-Harmonisierung**:
  - Beide Bars (Seekbar und Lautstärkebalken) erhalten exakt dieselbe Geometrie:
    - **Idle**: `4.0 px` Höhe.
    - **Hover / Drag**: `8.0 px` Höhe mit identischer sanfter Easing-Animation.
- [ ] **Flacher, bündiger Knopfübergang (Option B)**:
  - Der weiße Kreis-Knob bleibt erhalten.
  - Der gefüllte weiße Balken endet vor dem Knopf rechtwinklig / plan, sodass Balken und Knopf nahtlos ohne überlappende Doppel-Rundung ineinander verschmelzen.
  - *Option A (reine Kapsel ohne Kreis-Knob)* wird für spätere Tests vermerkt.

---

## Step 5: Player-Bar Tool-Icons: Einheitliche Linienstärke

### Problemstellung:
- Rechts in der Player-Bar haben die Icons (Lyrics, Warteschlange, Aktueller Titel, Devices, Lautsprecher) aktuell leicht unterschiedliche Strichstärken (Strokes).

### Konkrete Umsetzung:
- [ ] **Einheitliche Stroke-Dicke**:
  - Alle Konturen auf eine einheitliche Linienstärke von `1.3 px` kalibrieren.
  - Das Monitor-/Details-Icon, das Warteschlangen-Icon und das Devices-Icon optisch perfekt aufeinander abstimmen.

---

## Step 6: Rechte Seitenleiste („Aktueller Titel“ / Now Playing View)

### Konkrete Umsetzung:
- [ ] **Resizable / Frei vergrößerbar (Drag-Handle)**:
  - An der linken Kante des `np-details`-Panels einen interaktiven 4-Pixel-Drag-Bereich mit `CursorIcon::ResizeHorizontal` einrichten.
  - Nutzer kann die Leiste stufenlos von `260 px` bis `520 px` ziehen; Breite wird persistent gespeichert.
  - Cover, Typografie und Widgets skalieren flüssig mit der Panelbreite.
- [ ] **Kopfzeile ohne „X“**:
  - Das generische `X`-Icon oben entfernen.
  - Stattdessen das dezente Spotify-Monitor/Panel-Icon aus der Player-Bar verwenden, das bei Hover eingeblendet wird.
- [ ] **Songtitel & Favoriten-Herz**:
  - Songtitel linksbündig verankern.
  - Favoriten-Herz auf exakt dieselbe Zeilen- und Baseline-Höhe wie den Songtitel setzen.
- [ ] **Album-Cover**:
  - Gleichmäßig abgerundete Ecken (`CornerRadius::same(10.0)`), identisch zum Stil der Player-Leiste.
- [ ] **Künstler & Neuer „Folgen“-Button**:
  - Den separaten Button `„Künstlerprofil ansehen“` entfernen.
  - Künstlername wird bei Hover dezent unterstrichen und führt per Klick direkt zur Künstlerseite.
  - Anstelle des alten Buttons wird ein nativer Pill-Button `„Follow“` bzw. `„Following“` integriert.
- [ ] **Künstler-Banner (Bild)**:
  - Alle vier Ecken symmetrisch abrunden.
  - Künstlername und Follower-Anzahl weiter nach unten positionieren, Schriftgröße harmonisieren.
  - Überflüssigen Leerraum vor der Überschrift `„About the Artist“` straffen.
- [ ] **Smartphone-Style Lyrics-Box**:
  - Eine kompakte, abgerundete Lyrics-Preview-Card einfügen, die synchron mitscrollende Textzeilen anzeigt.
  - Klick auf die Box wechselt sofort in die Vollbild-Lyrics-Ansicht.
- [ ] **„Next up“ / „Als Nächstes“ Play-Button bei Hover**:
  - In der Zeile des nächsten Songs blendet sich bei Hover ein Play-Button über dem Cover ein, um den Titel direkt zu starten.
  - Direkt auf Höhe der Zeile `„Next up“` ein Warteschlangen-Icon platzieren, um sofort zur vollen Queue zu wechseln.

---

## Step 7: Warteschlange & Song-Menüs: Herz-Icon statt Stern & Rechtsklick-Pop-up

### Problemstellung:
- In Kontextmenüs (z. B. Rechtsklick auf Songs oder im Queue-Menü) wird aktuell über `menu_action_star` ein Stern-Icon gezeichnet statt des Standard-Herzens.

### Konkrete Umsetzung:
- [ ] **Globaler Austausch: Stern -> Herz im Kontextmenü**:
  - `menu_action_star` in `src/ui/widgets.rs` auf `glyphs::HEART_FILLED` und `glyphs::HEART` umstellen.
  - Im gesamten Menü-System konsistent das Herz anzeigen.
- [ ] **Rechtsklick-Support für Queue-Tracks**:
  - Statt das Drei-Punkte-Menü treffen zu müssen, öffnet ein Rechtsklick auf den Song das vollständige Kontextmenü (Play next, Add to playlist, Remove, Like).
- [ ] **Erklär-Tooltip für Autoplay**:
  - Tooltip: `„Keep playing similar music automatically when queue ends“`.

---

## Step 8: Linke Seitenleiste: Bereinigung, Abstände, Vertikallinie & Einzeilige Playlists

### Konkrete Umsetzung:
- [ ] **Aktiver Menü-Eintrag leuchtend Weiß**:
  - Beim ausgewählten Menüpunkt (Library, Downloads) sind sowohl **Icon als auch Text** in reinem Weiß (`Color32::WHITE`).
- [ ] **Vertikale Trennlinie nur im unteren Bereich**:
  - Die vertikale Trennlinie rechts von der Sidebar wird im oberen Bereich (neben Library, Liked Songs, Downloads) ausgeblendet.
  - Sie beginnt erst ab der horizontalen Trennlinie unter Downloads und fungiert nach unten hin auch als visuelle Scroll-Begrenzung.
- [ ] **Abstand unter Downloads straffen**:
  - Den Leerraum unter Downloads um ca. 6–8 px verringern; die Playlists rücken ein kleines Stück nach oben. Die horizontale Trennlinie bleibt erhalten.
- [ ] **Einzeilige Playlist-Titel (Ellipsis `...`)**:
  - Playlists brechen nicht mehr in 2 Zeilen um, sondern werden sauber einzeilig abgeschnitten (`truncate_at_width`), damit alle Zeilen dieselbe Höhe haben.
- [ ] **Drei-Punkte-Menü in Playlists entfernen -> Rechtsklick**:
  - Das unruhige Drei-Punkte-Menü auf Playlist-Einträgen entfällt. Alle Aktionen (Play, Edit, Delete, Share) öffnen sich intuitiv per Rechtsklick.
- [ ] **Deduplizierung von „Liked Songs“**:
  - Aus der unteren Playlist-Liste den doppelten Eintrag „Liked Songs“ entfernen, da er bereits oben unter Library fest verankert ist.
- [ ] **Such-Icon (Lupe) in der Bibliothek (Spotify-Style)**:
  - Neben dem Playlists-Header ein Lupen-Icon platzieren; Klick klappt ein animiertes Filter-Suchfeld aus.

---

## Step 9: Fenster-Resizing & Bibliotheks-Grid: Symmetrie, Mindestgröße & Blackscreen-Fix

### Problemstellung:
- Beim Ziehen des Fensters (Größenänderung) flackert der Screen ab und zu kurz komplett schwarz auf.
- Wenn das Fenster schmal gezogen wird, darf das UI nicht zusammenbrechen oder unbenutzbar werden.
- Die Grid-Spalten springen beim Verkleinern teilweise ruckartig um.

### Konkrete Umsetzung:
- [ ] **Blackscreen / Resize-Flackern beheben**:
  - **Ursache**: Beim Vergrößern/Verkleinern empfängt Windows `WM_SIZE`-Events und die Grafik-Swapchain (OpenGL/WGPU) wird neu initialisiert. Wenn ein Frame gerendert wird, bevor die Shader-Callback-Textur fertig gebunden ist, cleared der Treiber auf Schwarz (`#000000`).
  - **Lösung**:
    - Vor dem Shader-Callback wird im egui-Frame ein stabiler, matter Hintergrund (`theme::bg()`) gezeichnet, sodass selbst bei Verzögerungen der GPU-Pipeline niemals ein schwarzer Frame sichtbar wird.
    - Zero-Size Viewport Handling absichern (wenn `width <= 0` oder `height <= 0`, vorherigen Frame halten statt Shader-Fehler zu werfen).
- [ ] **Mindestfenstergröße absichern (`min_window_size`)**:
  - In `src/main.rs` und im Windows-Fenstermanager eine strikte Mindestgröße (z. B. 760 × 480 px) fest verankern, damit das Fenster auch bei schnellem Ziehen nicht in unbrauchbare Miniatur-Dimensionen geschrumpft werden kann.
- [ ] **Symmetrische Randabstände & sanfte Grid-Transitionen**:
  - Spaltenanzahl erst reduzieren, wenn der Rand rechts vom rechtesten Cover kleiner wird als der Rand links zur Seitenleiste (`Margin_right == Margin_left`).
  - Ausblendende oder hinzukommende Spalten erhalten ein weiches 120 ms Alpha-Easing.

---

## Step 10: Top-Bar: Layout-Architektur (Home links an Suchleiste), Responsive Skalierung & Logo Schärfe

### Finale Layout-Entscheidung:
- **Home-Button links an der Suchleiste**:
  - Der runde Home-Button (Durchmesser 32 px) dockt direkt links an der Suchleiste an.
  - **Definierter Abstand (`HOME_SEARCH_GAP = 9.0 px`)**: 
    - Exakt wie vom Nutzer vorgegeben: Mindestens so viel Abstand wie zwischen dem Vor- und Zurück-Button (4 px Randabstand / 8 px zu Würfel) bzw. minimal mehr.
    - Mit 9 px Abstand hat der Home-Button eine klare optische Trennung zur Kapsel der Suchleiste, ohne verloren zu wirken.
- **Symmetrisches Discover-Icon rechts an der Suchleiste**:
  - Genau wie Home links sitzt der Discover-Button („Explore & New Releases“) mit exakt denselben 9 px Abstand rechts an der Suchleiste.
  - Das gesamte Trio `[ (Home) ]` -- 9px -- `[ Search Pill ]` -- 9px -- `[ (Discover) ]` bildet eine perfekt symmetrische, zentrierte Navigationseinheit.
- **Kollisionsfreie responsive Breitenberechnung**:
  - Die Suchleiste hat eine Maximalbreite von 260 px.
  - Wenn das Fenster schmal gezogen wird (oder das rechte Details-Panel geöffnet wird), berechnet sich die Breite dynamisch anhand des verbleibenden Abstands zu den linken Buttons (Back, Forward, Würfel) und den rechten Buttons (Account, Titlebar):
    `max_search_w = ((clear_half - wing_w) * 2.0).clamp(34.0, 260.0)`
  - **Ergebnis**: Kein Überlagern mehr! Der Home-Button und der Würfel stoßen niemals zusammen, selbst bei sehr kleinen Fensterbreiten.

### Weitere Top-Bar Optimierungen:
- [x] **Home-Button links an der Suchleiste mit 9 px Abstand verankert** (in `src/ui/shell.rs` umgesetzt).
- [x] **Responsive Breitenbegrenzung gegen Überlagerungen** (in `src/ui/shell.rs` umgesetzt).
- [ ] **Gestochen scharfes BitChord-Logo**:
  - Das alte 256×171 `mark.png` durch hochauflösende Retina-Assets (4x Skalierung) oder saubere Vektor-Pfade ersetzen, damit das Logo auf jedem Monitor gestochen scharf gerendert wird.

---

## Step 11: Vollständige Lokalisierung auf Englisch

### Konkrete Umsetzung:
- [ ] Durchgängige Konsistenz auf Englisch:
  - `Bibliothek` -> `Library`
  - `Lieblingssongs` -> `Liked Songs` (Empty State: `„No liked songs yet“`)
  - `Downloads` -> `Downloads`
  - `Aktueller Titel einblenden` -> `Now Playing View`
  - `Warteschlange` -> `Queue`
  - `Mit einem Gerät verbinden` -> `Connect to a Device`
  - `Entdecken & Neuheiten` -> `Explore & New Releases`
  - Alle Tooltips, Bestätigungsdialoge und Kontextmenüs auf Englisch polieren.

---

## Step 12: Spotify Playlists: 100% gleiches Design, „[Name]’s Spotify“ & 2-Wege-Sync

### Design & Identität:
- **Exakt gleiches Design**: Kein abweichendes, seltsames UI für Spotify-Playlists. Sie nutzen exakt denselben modernen Glaskasten, dieselbe Tabellen-/Grid-Ansicht und dieselben Schriften wie normale BitChord / YouTube Music Playlists.
- **Urheber-Klarheit**: 
  - YouTube Music Playlists zeigen den Account-Namen des Nutzers (z. B. `Leon`).
  - Spotify Playlists zeigen statt nur generisch „Spotify“ nun: `Leon’s Spotify` (bzw. `[Username]’s Spotify`).
- **HQ-Cover für Spotify Playlists**:
  - Immer das größte verfügbare Cover (`images[0].url`, 640×640 px) cachen und rendern.
- **Vollständiger 2-Wege-Sync (100% Spotify-Parität)**:
  - In den Einstellungen (`Settings › Spotify`) wird der Umschalter integriert:
    `[x] Enable Full 2-Way Sync with Spotify` (mit deutlichem Warnhinweis vor dem Scharfschalten).
  - **Kollaborative Playlists**:
    - Wenn Freunde auf Spotify Songs hinzufügen oder löschen, aktualisiert sich BitChord live.
    - Wenn der Nutzer in BitChord Songs hinzufügt oder löscht, wird dies sofort via Spotify-API auf Spotify übertragen.
    - Konfliktfreier Delta-Sync über Spotify `snapshot_id` (kein gegenseitiges Überschreiben).
    - Attributierung *"Added by [Freund]"* für jeden Mitwirkenden.
  - **Dynamische & Algorithmische Playlists ("Dein Mixtape 2" / "Daily Mix")**:
    - Nur in der Spotify-Bibliothek gespeicherte Mixes (`+`) tauchen in BitChord auf (ungespeicherte wie Mixtape 3 bleiben verborgen).
    - Automatische Aktualisierung, sobald Spotify den Inhalt ändert. Schreibschutz für algorithmische Playlists.
  - **Spotify-Kollaborationslinks ("Freunde einladen")**:
    - Im Teilen-Menü: `[🔗 Spotify-Link teilen]` und `[+] Spotify-Kollaborateur einladen`.
    - Freunde öffnen den Link in ihrer normalen Spotify-App und arbeiten mit, während der Nutzer in BitChord bleibt!
  - **Metadaten-Sync (Titel & Cover)**:
    - Titel-, Beschreibungs- und Cover-Änderungen (inkl. Erstellung, Upload, Löschung und Rückfall auf 4er-Mosaik) synchronisieren sich nahtlos bidirektional.
  - **Playlist-Ausschluss**:
    - Einzelne Playlists können per Rechtsklick von der Synchronisation ausgeschlossen werden (`Do not sync this playlist`).

---

## Step 13: Remote-Steuerung & Audio-Output Routing (Cast / Phone)

### Analyse & Konzeption:
- **Audio-Output & Device Pipeline**:
  - BitChord Desktop kann über die Audio-Engine Ausgabegeräte (z. B. Kopfhörer, Boxen, Virtual Audio Cable) wählen.
  - Für Cast/Chromecast und Remote-Wiedergabe auf z. B. „Leon's Phone“ kann das integrierte WebSocket-Connect-System genutzt werden.
- [ ] **Web-Remote / PWA Generalüberholung (`src/remote.rs`)**:
  - **HQ-Cover**: High-Resolution Artwork (`w544-h544`).
  - **CSS Flexbox Fix**: Play-Button pixelgenau im Kreis zentrieren.
  - **Stabiler Two-Way State**: WebSocket-Heartbeat und Auto-Reconnect bei Pause/Resume.

---

## Step 14: Fullscreen-Modus vereinfachen

### Konkrete Umsetzung:
- [ ] Zusätzliche intuitive Schließen-Interaktionen einbauen:
  - Klick in leere Hintergrundbereiche schließt den Fullscreen.
  - Ein dezenter, klar erkennbarer Schließen-Button oben rechts.
  - Schnelle Wisch- oder Scroll-Geste nach unten führt zurück zur Hauptansicht.

---

## Step 15: Zukünftiges UI-Card-Design (Vorerst beobachten)
- [ ] Die Idee mit abgerundeten Gesamt-Layout-Cards (schwebende Panels) stellen wir vorerst zurück, bis alle funktionalen Details und Verhaltensweisen perfekt sitzen.

---

## Step 16: Playlist-Header: Doppelte Song-Anzahl entfernen & Vertikale Cover-Bündigkeit

### Problemstellung:
- **Doppelte Song-Anzahl**:
  - Zeile 1 zeigt: `Leon • 54 Songs`
  - Zeile 2 zeigt: `54 Songs • 186 Minuten`
  - Das Wort „54 Songs“ ist redundant doppelt vorhanden.
- **Vertikaler Versatz zum Cover**:
  - Oben ist in `src/ui/pages.rs` ein festes `ui.add_space(24.0);` verbaut, wodurch das Wort `PLAYLIST` 24 Pixel unterhalb der oberen Kante des Album-Covers beginnt.
  - Dadurch rutscht der gesamte Text- und Button-Block nach unten ab: Die untere Kante des Album-Covers schneidet den Play-Button exakt horizontal in der Mitte durch (der Button ragt zur Hälfte nach unten heraus).

### Konkrete Umsetzung in `src/ui/pages.rs`:
- [ ] **Doppelte Angabe eliminieren**:
  - `c.byline()` und die Spieldauer werden harmonisiert:
    - Entweder sauber 2-zeilig:
      - Zeile 1: `Leon` (Name des Erstellers als anklickbarer Link)
      - Zeile 2: `54 songs • 3 hr 6 min` (oder `54 songs • 186 min`)
    - Oder auf einer zusammenhängenden Zeile im Spotify-Stil:
      - `Leon • 54 songs • 3 hr 6 min`
- [ ] **Perfekte vertikale Bündigkeit (Cover vs. Text & Play-Button)**:
  - Das künstliche `ui.add_space(24.0);` über „PLAYLIST“ entfernen.
  - **Oberes Alignment**: Die Oberkante des Texts `PLAYLIST` / `ALBUM` liegt exakt auf der oberen Kante des Album-Covers.
  - **Unteres Alignment**: Der Pill-Play-Button und die Buttons (Shuffle, Search, More) schließen mit ihrer Unterkante exakt bündig mit der Unterkante des Covers ab.
  - Der gesamte Header-Inhalt bildet ein visuell geschlossenes Rechteck mit dem Cover.

---

## Step 17: Such-System: Spotify-Style Pop-up, Filter-Pillen & Kontur nur bei Fokus

### Problemstellung & Bisheriges Verhalten:
1. **Sofortiges Verlassen der aktuellen Seite**:
   - Sobald man oben in die Suchleiste klickte, wechselte BitChord sofort unaufgefordert auf die `Search`-Page.
   - Dort wurde bisher eine *zweite* riesige Suchleiste (`search_field`) gerendert – zwei Suchleisten übereinander wirkten chaotisch und redundant.
2. **Dauerhafte weiße Umrandung in der Top-Bar**:
   - Die obere Suchleiste hatte im Ruhezustand dauerhaft eine weiße Kontur (`theme::glass(0.18)`), was unruhig wirkte.
3. **Fehlende Filterung auf der Ergebnisseite**:
   - Die Suchergebnisse zeigten bisher nur einen starren Mix aus Songs, Artists und Albums ohne gezielte Kategorisierung.

### Spotify-Standard Such-Workflow (Gewählte Umsetzung):
- [ ] **Top-Bar Klick & Dropdown-Pop-up (kein sofortiger Seitenwechsel)**:
  - Wenn man in die Top-Suchleiste klickt, bleibt man auf der aktuellen Seite; es öffnet sich ein elegantes **schwebendes Dropdown-Pop-up** direkt unter der Suchleiste:
    - **Ohne Texteingabe**: Zeigt die letzten Suchanfragen (`Recent searches`) mit Klick zum Wiederholen oder Entfernen.
    - **Mit Texteingabe**: Live-Typeahead-Vorschläge (Matching Songs, Playlists, Artists).
- [ ] **Erst bei Enter (oder Klick auf Vorschlag) öffnet sich die Ergebnisseite**:
  - Drückt der Nutzer `Enter`, wechselt BitChord auf die eigentliche Suchergebnisseite `Page::Search(query)`.
  - Die redundante zweite Suchleiste auf der Ergebnisseite entfällt komplett; die Top-Bar bleibt die einzige Eingabeleiste.
- [ ] **Kategorie-Filter-Pillen auf der Suchergebnisseite**:
  - Oben auf der Ergebnisseite sitzen moderne Filter-Pillen:
    - `[ All ]` (Aktuelle Übersicht: Top-Ergebnis, 4 Songs, Artists, Albums)
    - `[ Songs ]` (Vollständige scrollbare Song-Tabelle mit allen Treffern)
    - `[ Playlists ]` (Grid aller passenden Playlists)
    - `[ Artists ]` (Karten aller passenden Künstler)
    - `[ Albums ]` (Grid aller passenden Alben)
- [ ] **Top-Bar Suchleiste: Kontur nur bei Fokus**:
  - Im Idle-Zustand: Rein transluzente, weiche Glasfläche ohne harte weiße Kontur.
  - Bei Klick/Fokus (`sf > 0.0`): Sanftes, edles Aufleuchten der feinen weißen Kontur (`theme::glass(0.20 * sf)`).

---

## Step 18: Schwebende Frosted-Glass Top-Bar (Transparenter Header beim Scrollen)

### Problemstellung:
- Aktuell teilt sich die obere Top-Bar (mit Suchleiste, Home, Pfeilen etc.) einfach denselben flachen Hintergrund wie die jeweilige Seite darunter.
- Wenn man auf einer Seite nach unten scrollt, schneidet die obere Kante der Scroll-Area den Inhalt (Cover, Texte, Titel) mit einer harten, abrupten Kante ab.

### Ziel & Vision:
- Die obere Topleiste soll genau wie die untere Player-Leiste in einem modernen, edlen **Frosted Liquid Glass-Stil** schweben.
- Wenn man nach unten scrollt, gleiten die Album-Cover, Songzeilen und Bannerelemente **unter der Topleiste hindurch** und bleiben durch das Milchglas hindurch noch weich und unscharf/transluzent sichtbar, statt hart abgeschnitten zu werden.

### Technische Architektur & Konkrete Umsetzung:
- [ ] **Full-Height Scroll-Area unter der Top-Bar**:
  - In `src/ui/mod.rs` wird die `pages::page`-ScrollArea so erweitert, dass sie bis an den oberen Fensterrand reicht (`y = 0` bzw. oberer Fensterbereich).
  - Der Seiteninhalt erhält ein oberes Padding / Margin in der Höhe der Top-Bar (ca. 56–60 px), damit er im Ruhezustand sauber unterhalb der Top-Bar beginnt.
- [ ] **Top-Bar als schwebende Glas-Ebene (Floating Layer)**:
  - Die `top_bar` wird nach der Scroll-Area auf einer höheren visuellen Ebene (oder mit `ui.painter()`) über den scrollenden Inhalt gezeichnet.
  - **Hintergrund**: Ein weiches transluzentes Frosted-Glass-Panel:
    - Ruhezustand ganz oben (`scroll_offset == 0`): Nahezu transparent oder ganz dezenter Glas-Touch (`theme::glass(0.04)`).
    - Beim Scrollen (`scroll_offset > 0`): Sanfter Übergang zu echtem Liquid Frosted Glass (`rgba(16, 15, 20, 160)` mit Blur/Shader-Diffusion), identisch zur Player-Leiste.
- [ ] **Dezente untere Trennlinie**:
  - Eine hauchfeine horizontale Glas-Linie (`theme::glass(0.08)`) schließt die Topleiste nach unten hin ab, sobald gescrollt wird.
- [ ] **Ergebnis**:
  - Hochmoderner macOS-/iOS-Look: Content verschwindet nicht mehr an einer harten Kante, sondern gleitet organisch und weich durch den Milchglas-Header.

---

## Step 19: Echter Shuffle-Start bei Playlists (Zufälliger Song statt immer Track 1)

### Problemstellung:
- **Aktuelles fehlerhaftes Verhalten**:
  - Wenn in der Player-Bar Shuffle aktiviert ist (`self.shuffle == true`), und man auf einer Playlist auf den großen **Play-Button** klickt, ruft `play_pill` bisher fest `Action::Play(tracks, 0)` auf.
  - In `src/ui/queue.rs:65` setzt `replace` den aktuellen Track fest auf `index = 0` (den allerersten Track der Playlist).
  - Anschließend mischt `shuffle_upcoming()` nur den Bereich ab Index 1 (`&mut self.tracks[1..]`).
  - **Auswirkung**: Man hört beim Starten einer Playlist mit aktiviertem Shuffle **jedes Mal zwingend Song 1** der Playlist – erst ab Song 2 greift der Zufall!

### Gewünschtes Verhalten (Spotify-Standard):
- Wenn Shuffle aktiviert ist und eine Playlist gestartet wird, soll **sofort ein zufälliger Song aus der Playlist als Start-Track** gewählt werden.
- Die restlichen Tracks der Playlist werden darum herum gemischt.
- **Ausnahme**: Klickt der Nutzer gezielt auf eine bestimmte Song-Zeile (z. B. Track 7), startet exakt dieser Song und die restlichen Songs werden geshuffelt.

### Konkrete Umsetzung in `src/ui/queue.rs` & `src/ui/pages.rs`:
- [ ] **Zufallsauswahl beim Initial-Start in `Queue::replace`**:
  - In `Queue::replace(&mut self, tracks: Vec<Track>, start: usize, fill: Option<QueueFill>)`:
    ```rust
    let initial_index = if self.shuffle && start == 0 && tracks.len() > 1 {
        fastrand::usize(..tracks.len())
    } else {
        start.min(tracks.len() - 1)
    };
    ```
  - Der zufällig gewählte Song wird an den Anfang der Queue gesetzt (`self.tracks.swap(0, initial_index)`) oder `self.index = Some(initial_index)` gesetzt, und alle anderen Tracks werden nahtlos drumherum zufällig gemischt (`super::shuffle(&mut self.tracks[1..])`).
- [ ] **Ergebnis**:
  - Jedes Mal, wenn man mit aktiviertem Shuffle auf eine Playlist klickt, startet sofort ein echter Überraschungssong aus der Playlist!

---

## Step 20: Discover-Page Überholung (Genres, Moods & Bento-Cards)

### Problemstellung:
- Die aktuelle Entdecken-Seite (`Page::New` / Kompass-Button „Explore & New Releases“) ist sehr mager ausgestattet:
  - Sie besteht aktuell nur aus einer einzigen Leiste namens „Charts“ mit Playlists.
  - Es fehlen visuelle Genre- und Mood-Einstiege, die zum Entdecken von Musik einladen.

### Gewünschte Umsetzung (Spotify- & Apple Music-Standard):
- [ ] **Mehrere thematische Shelves (Regale)**:
  - Neben „Top Charts“ werden kuratierte Zeilen ergänzt:
    - *New Releases* (Neuerscheinungen)
    - *Trending Playlists*
    - *Moods & Activities* (Chill, Workout, Focus, Party, Sleep, Gaming)
- [ ] **Bunte Genre- & Mood-Bento-Kacheln**:
  - Am Anfang oder als markante Sektion: Farbige Kacheln im Spotify-Stil (Pop, Hip-Hop, Rock, Indie, R&B, Electronic, Metal, Jazz, Lo-Fi, Anime / Gaming).
  - Jede Karte hat einen satten Farbverlauf / Akzentfarbe und ein schräges Mini-Cover-Artwork.
  - Klick auf ein Genre öffnet die kuratierten Listen oder startet sofort eine thematische Radio-Session.
- [ ] **Optionale Übernahme von Spotify-Kategorien**:
  - Die kuratierten Kategorie-Kataloge können direkt über die Spotify Web-API (`/v1/browse/categories`) oder kuratierte YouTube Music Shelves bezogen werden.

---

## Step 21: Repeat-Modi (Repeat 1 vs. All & „1“-Icon Schärfe) & Akzentfarben-Harmonisierung

### 1. Repeat-Button: Was bedeuten die Modi?
- **Klick 0 (Aus / Repeat::Off)**:
  - Grau / Inaktiv. Nach dem letzten Titel der Queue/Playlist stoppt die Wiedergabe.
- **Klick 1 (Repeat All / Repeat::All)**:
  - Akzentfarbe aktiv + Indikator-Punkt unter dem Button.
  - Die **gesamte Queue/Playlist läuft in einer Dauerschleife**. Ist der letzte Track vorbei, beginnt Track 1 wieder von vorne.
- **Klick 2 (Repeat One / Repeat::One)**:
  - Akzentfarbe aktiv + kleine „1“ im Symbol.
  - Nur **dieser eine einzelne Song** wiederholt sich unendlich in Dauerschleife (Single-Track-Loop).

### 2. Warum sieht das Repeat-1 Symbol aktuell halb abgeschnitten aus?
- In `assets/icon/glyphs/repeat_one.png` ist die Ziffer „1“ winzig klein (nur ca. 8 Pixel in einem 256×256 Bild).
- Wenn das Icon in der Player-Bar auf 20×20 pt skaliert wird, ist die „1“ **weniger als 1 Pixel groß (0,78 px)**!
- Beim bilinearen Downsampling wird die Ziffer unscharf, flackert und sieht buchstäblich wie ein abgeschnittener Fleck aus.
- **Lösung**:
  - Wir zeichnen die kleine „1“ entweder als gestochen scharfen Vektor-Text (`theme::bold(9.0)`) direkt mittig in den `REPEAT`-Kreis, oder tauschen das Asset gegen ein zentriertes, fettes Badge wie bei Spotify/Apple Music aus.

### 3. Akzentfarben-Inkonsistenz in der Player-Bar (Farben-Harmonisierung):
- **Ursachen der unterschiedlichen Farbtöne**:
  1. **Volume-Slider Verwaschung**:
     - In `src/ui/shell.rs:1319` wird die Lautstärkeleiste mit `theme::lerp_color(theme::text_dim(), app.accent_color(), 0.85)` berechnet!
     - Durch die Mischung mit `text_dim()` (dunkelgrau) ist die Lautstärkeleiste viel matter und dunkler als die anderen Buttons.
  2. **PNG-Glyphen vs. Vektor-Strokes**:
     - Manche Buttons (Shuffle, Repeat, Queue) sind PNG-Texturen mit weichen Kantentransparenzen.
     - Der Details-Monitor-Button ist ein Vektor-Rechteck (`rect_stroke`) mit 100% Opacity.
     - Dadurch wirkt der Monitor-Button optisch greller als die PNG-Symbole.
  3. **Uneinheitliche Indikator-Punkte**:
     - Shuffle, Repeat und Details haben kleine Punkte darunter, Queue und Lyrics nicht.
- **Lösung**:
  - **Einheitliche Farbquelle**: Alle aktiven Player-Elemente (Shuffle, Repeat, Queue, Details, Lyrics) verwenden exakt denselben ungemischten Farbton `accent` (`app.accent_color()`).
  - **Lautstärkeregler-Farbe**: Der Volume-Balken wird entweder komplett neutral in Weiß/Hellgrau gehalten (wie Spotify & Seekbar) oder nutzt exakt die reine Akzentfarbe ohne trübes Grau-Lerp.
  - **Konsistente Indikatoren**: Jeder aktive Button erhält denselben 1.6 px Indikator-Punkt oder einen klaren Active-State.

---

## Step 22: Song-Wechsel Glitch-Fix (Cover & Lyrics Race Condition), Typo-Underline & Globales Kontextmenü

### 1. Klarstellung zu Cover-Ecken im rechten Panel:
- Die Cover im rechten Details-Panel besitzen bereits abgerundete Ecken; hier ist keine zusätzliche Geometrie-Änderung an der Basis-Rundung nötig.

### 2. Skipping & Song-Wechsel Race Condition (Blinken & Double-Update):
- **Problembeschreibung**:
  - Wenn man zum nächsten Song skippt, während das rechte Panel geöffnet ist, flackert/blinkt das Cover manchmal kurz unruhig auf.
  - Wenn zusätzlich die Lyrics (Songtexte) geöffnet sind, tritt ein auffälliger Glitch auf:
    1. Kurz wird der neue Song angezeigt,
    2. dann blitzt für einen Sekundenbruchteil noch einmal der Text/Zustand des vorherigen Songs auf,
    3. danach springt die Ansicht erst final auf den richtigen neuen Song.
- **Root Cause Analyse**:
  - Typische asynchrone **Race Condition / State Invalidation Lücke**:
    - Beim Skippen triggert `app.next()` den Trackwechsel. Die Audio-Pipeline und UI wechseln sofort auf den neuen Track.
    - Gleichzeitig laufen im Hintergrund noch ausstehende Callback-Futures oder Repaint-Frames für den vorherigen Track (z. B. `app.lyrics` oder Cover-Texture-Slots).
    - Wenn der alte Callback kurz nach dem Track-Wechsel auflöst oder der Cache nicht sofort atomar mit der neuen Track-ID invalidiert wird, überschreibt der veraltete State den Render-Buffer für genau 1 Frame.
- **Konkrete Lösungs-Architektur**:
  - **Sofortige atomare State-Invalidierung**: Sobald `track.id != last_rendered_id`, wird der aktive Lyrics- und Cover-Cache für den vorherigen Track synchron geleert (`app.lyrics = None;`), bevor neue Async-Tasks starten.
  - **Guard mit Request-Token / Generational ID**: Jeder Lyrics- und Artwork-Fetch erhält ein eindeutiges generationales Token (`u64` oder Track-ID-Matching). Trifft eine Antwort für eine veraltete Generation ein, wird sie still verworfen und überschreibt niemals den UI-Zustand.
  - **Sanfter Crossfade beim Cover-Wechsel**: Übergang zwischen Songs mit dem bestehenden `paint_cover_xfade`, um hartes Flackern/Blinken zu eliminieren.

### 3. Link-Unterstreichung bei Hover (Typo-Underline Baseline-Ausrichtung):
- **Problem**:
  - Im rechten Panel sitzt der Hover-Unterstrich bei Interpreten-Links aktuell zu weit oben, sodass er den Text optisch fast durchstreicht (Strikethrough-Effekt).
  - Unten links in der Player-Leiste sitzt der Unterstrich dagegen perfekt mit ca. 2–3 Pixeln optischem Abstand unter der Baseline.
- **Lösung**:
  - Die Baseline- und Y-Offset-Berechnung vereinheitlichen.
  - Einen global konsistenten Helfer für interaktive Text-Links mit sauber kalibriertem Unterstrich-Abstand verwenden (`pos.y + font_height + 2.0`), sodass alle Interpreten- und Album-Links app-weit identisch und elegant unterstrichen werden.

### 4. Hover-Unterstrich in der Künstler-Card:
- Wenn der Mauszeiger über den Künstlernamen in der „About the artist“-Karte im rechten Panel hovert, soll auch dieser denselben eleganten Hover-Unterstrich erhalten, um sofort haptisch zu signalisieren, dass er anklickbar ist und zur Künstlerseite führt.

### 5. Globales Kontextmenü (Rechtsklick) auf Cover & Titel:
- Rechtsklick auf das Album-Cover oder den Songtitel muss nicht nur auf der Hauptseite oder in der unteren Player-Bar funktionieren, sondern auch im rechten Details-Panel und in allen Detail-Ansichten:
  - Rechtsklick öffnet sofort das native Aktions-Pop-up-Menü (`song_menu`):
    - *Add to playlist*
    - *Like / Remove from favorites*
    - *Share song*
    - *Go to artist / Go to album*
    - *Play next / Add to queue*

---

## Step 23: Warteschlangen-Kontextmenü & Globale Artist/Album-Navigation

- **Warteschlange (Up Next)**:
  - Rechtsklick auf Songs in der Warteschlange (`queue_list`) öffnet das volle `song_menu`.
  - Bei anstehenden Songs erscheint die Option `Remove from queue` (`Action::QueueRemove`).
- **Globale Artist/Album Navigation**:
  - `song_menu` enthält nun überall `Go to {artist}` und `Go to album`.
  - Optischer Baseline-Offset für Hover-Unterstreichungen auf einheitliche `r.bottom() + 1.5` gesetzt, damit Unterstriche niemals durch Buchstaben durchgehen.

---

## Step 24: Web-Remote-Controller (PWA, Reconnect & Haptik)

- **Mobile PWA-Optimierung**:
  - `viewport-fit=cover`, `100dvh`, `apple-mobile-web-app-capable`, `theme-color: #121212`.
  - Hochauflösende Artwork-Auslieferung über `thumbs.last()`.
  - Optisch zentrierter Play-Button und taktiles Haptik-Feedback über die Web Vibration API (`navigator.vibrate(8)`).
- **Auto-Reconnect Loop**:
  - Bei Netzwerkwechsel oder Standby auf dem Smartphone verbindet sich der WebSocket automatisch alle 1,5s neu mit Live-Statusanzeige („Connected“ vs. „Reconnecting...“).
  - Vollständige Lokalisierung auf Englisch.

---

## Step 25: Search Page Empty-State & Browse Categories Bento-Grid

- **Kategorie-Entdeckung auf der Such-Seite**:
  - Wenn das Suchfeld leer ist oder geleert wird, erscheint ein farbenfrohes Bento-Grid von Musik-Genres (*Pop, Hip-Hop, Rock, Indie, Electronic, Gaming, Chill & Lo-Fi, Workout*) mit sanftem Glow bei Hover.
  - Ein Klick springt sofort in die jeweilige Kategoriensuche.
- **Recently Searched Polish**:
  - Button `Clear all` neben der Überschrift „Recently Searched“ zum schnellen Leeren des Verlaufs.
  - Veraltete `egui::Id::new` Aufrufe eliminiert.

---

## Step 26: Sanfte Akzentfarben-Interpolation (Ambient Lerp)

- **Fließende Farbübergänge**:
  - `App` hält `accent_curr` und lerpt in `App::logic` kontinuierlich mit 60 FPS (`theme::lerp_color`) auf das jeweilige `target_accent`.
  - Verhindert harte Farbsprünge beim Song-Wechsel oder Umschalten zwischen Themes – das UI fließt wie bei Apple Music organisch in die neue Song-Farbe.

---

## Step 27: Codebase-Hygiene & Null-Warnungen-Garantie

- Sämtliche 46 Compiler-Warnungen und Deprecations (`Id::new`, `vertical_align`, `ui.id()`, `dead_code`) vollständig bereinigt.
- Cross-Kompilierung mit `cargo check --target x86_64-pc-windows-gnu` schließt mit **0 Fehlern und 0 Warnungen** ab.

---

## Zukünftige Feature-Konzepte & Research

## Step 35: Echter Song- & Artist-Blocker ("Diesen Song/Künstler nie wieder spielen") — UMSETZUNG IN v2.1.0

- **Umsetzung**:
  - **Minus-Circle-Icon (`⊖`)**:
    - Mathematisch exakt per egui Painter gezeichnet (`draw_minus_circle`), gestochen scharf ohne Artefakte.
    - Im rechten Details-Panel direkt neben dem Favoriten-Herz auf der Titelzeile platziert.
    - In Kontextmenüs (Drei-Punkte-Menü `⋯` und Rechtsklick) für jeden Song („Song blockieren“) und Interpreten („Künstler „[Name]“ blockieren“).
  - **Zweifach-Aktion**:
    - Sendet bei eingeloggtem Account automatisch Dislike an YouTube Music.
    - Speichert Track-ID bzw. Artist-ID in lokaler persistenter Sperrliste (`blocked_tracks`, `blocked_artists`).
  - **Aktives Überspringen in Echtzeit**:
    - Sobald ein blockierter Song im Automix, Radio, Shuffle oder in Playlists aufgerufen wird, überspringt `play_current_at` ihn sofort und nahtlos.
    - Wird der aktuell spielende Song blockiert, schaltet BitChord sofort zum nächsten Song weiter.
  - **Verwaltung in den Einstellungen**:
    - In **Settings > Playback > Blockierte Inhalte** werden alle blockierten Titel und Künstler übersichtlich mit rotem `⊖`-Badge aufgelistet und können per 1-Klick-Button „Entblocken“ wieder freigegeben werden.

---

### Step 36: AI Music & Slop Blocker (ZoundHub & Heuristik-Engine)

- **Hintergrund (Spicetify AI Band Blocker & ZoundHub)**:
  - Musik-Streamingdienste (Spotify, YouTube Music) werden zunehmend von automatisierten Bot-Accounts mit Tausenden generierten KI-Songs (Suno, Udio) überflutet.
  - Community-Projekte wie *AI Band Blocker* und *ZoundHub* setzen auf ein mehrstufiges Filtersystem, um menschliche Musik von KI-Massenware zu trennen.
- **Geplante Heuristik-Filter in BitChord**:
  - **`skip 2024 onwards`**: Greift bei verdächtigen Neuveröffentlichungen ab dem KI-Boom (2024+).
  - **`skip AI visuals`**: Heuristische Analyse von generischen KI-Album-Covern (glänzende Midjourney-Ästhetik, typische Artefakte).
  - **`skip anonymous`**: Filtert Künstler gänzlich ohne Biografie, ohne verlinkte Socials, ohne verifizierten Publisher/Distributor.
  - **`skip high output`**: Erkennt Bot-Muster (z. B. Künstler mit 20 Alben innerhalb von 3 Monaten).
  - **`skip inconsistent style`**: Schlägt an, wenn ein Künstlerprofil ohne roten Faden querbeet extreme Genres wechselt.
  - **`ZoundHub Cloud Confidence`**:
    - Integrierter Schwellenwert (z. B. Standard `80% Confidence`).
    - Liegt die berechnete oder aus ZoundHub-Datenbanken bezogene KI-Wahrscheinlichkeit bei $\ge 80\%$, wird der Song automatisch übersprungen.
  - **Opt-in in den Einstellungen**:
    - Der Nutzer kann den AI-Blocker in *Settings > Playback* aktivieren und die Filter-Tags (`skip anonymous`, `confidence threshold`) nach eigenem Geschmack einstellen.

---

### Step 37: Spotify Friend Activity Feed ("Freundesaktivität" / Live-Aktivität von Freunden)

- **Hintergrund**:
  - In Spotify Desktop befindet sich oben rechts der Button für die *„Freundesaktivität“* (Friend Activity Panel).
  - Zeigt in Echtzeit, welche Musik Freunde gerade hören, inklusive Profilbild, Name, Songtitel, Interpret, Album/Playlist-Kontext und Live-Equalizer-Balken.
- **Möglichkeit über den verknüpften Spotify-Account**:
  - Durch den bestehenden Spotify-Login (`web-api`) kann BitChord über Spotifys Buddy-API (`https://spclient.wg.spotify.com/presence-view/v1/buddylist`) die Live-Aktivität deiner Spotify-Freunde abrufen.
- **Geplante Umsetzung in BitChord PC**:
  - **Button in der Top-Bar / Rechtes Seitenpanel**:
    - Ein dezentes Freunde-Icon (`icon::USERS`) in der oberen Leiste klappt das Friend-Activity-Panel auf oder wechselt den Tab im rechten Seitenpanel.
  - **Live-Feed-Einträge**:
    - Avatar des Freundes, Name, aktuell gehörter Track mit Live-Pulsieren, wenn der Freund gerade hört.
    - Klick auf den Freund: Öffnet dessen Profil und öffentliche Playlists.
    - Klick auf den Song: Spielt den Track direkt in BitChord ab!
  - **Status**: Für spätere Umsetzung vorgemerkt, vorerst nur dokumentiert.

---

### Step 38: Mausrad-Lautstärke-Schrittweite & Feineinstellung (Spotify-Präzision)

## Step 38: Mausrad-Lautstärke-Schrittweite & Feineinstellung — UMSETZUNG IN v2.1.0

- **Umsetzung**:
  - Im Player-Balken (`src/ui/shell.rs`) sowie im Mini-Player (`src/ui/mini.rs`) den Mausrad-Delta-Divisor von bisher `500.0` auf `1200.0` kalibriert.
  - **Ergebnis**: Pro Mausrad-Tick ändert sich die Lautstärke um exakt **~5 %** (identisch zum Spotify Desktop-Verhalten), statt vorheriger zu grober 12 %-Sprünge.

---

### Step 39: Cloud-Relay Spotify Connect (Unterwegs über Mobile Daten $\leftrightarrow$ PC zuhause)

- **Idee & Architektur**:
  - Wie vom Nutzer vorgeschlagen: Nutzung der bestehenden **Listen Together / Jam-Relay-Infrastruktur** (`wss://ytm-rs-jam.grapedjuice.workers.dev`) als private Geräte-Brücke.
  - **Vorteil**: Funktioniert auch, wenn das Smartphone unterwegs im 5G-/Mobilfunknetz ist und der PC zuhause im WLAN/LAN steht!
  - **Geräte-Auswahl (Audio Output)**:
    - Auf dem Smartphone kann im `AudioOutputSheet` neben den Handy-Lautsprechern auch *„Leons PC“* ausgewählt werden.
    - Am PC kann im `devices_dialog` *„Leons Phone“* ausgewählt werden.
    - Audio spielt auf dem jeweils aktiven Ausgabegerät; Steuerbefehle (Pause, Skip, Lautstärke) synchronisieren sich live über den sicheren WebSocket-Relay.
  - **Status**: Vorerst nur dokumentiert und recherchiert, wird noch nicht umgesetzt.

---

## Step 40: Rechtes Details-Panel: Kontext-Header (Playlist-Name) & Album-Navigation — UMSETZUNG IN v2.1.0

- **Kontext-Header oben im Seitenpanel**:
  - `App::playing_context()` ermittelt dynamisch den Namen der spielenden Playlist (z. B. *"Liked Music"*, *"Dein Mixtape 2"*, etc.) bzw. des Albums.
  - Ist kein Kontext vorhanden, steht dort standardmäßig *"Now Playing"*.
  - Ist der Header ein Kontext-Link (Playlist/Album), blendet sich bei Hover ein Unterstrich ein und ein Klick navigiert direkt zur Playlist oder zum Album (`Action::Open(target)`).
  - Lange Titel laufen flüssig als Marquee, ohne den Close-Button zu bedrängen.
- **Album-Navigation per Klick auf Cover & Songtitel**:
  - **Klick auf das Album-Cover**: Linksklick auf das große Cover öffnet direkt die Album-Ansicht (`Action::Open(Target::Album(id))`). Cursor wechselt auf `PointingHand`.
  - **Klick auf den Songtitel**: Linksklick auf den Songtitel öffnet ebenfalls direkt das Album. Bei Hover zeigt der Titel den Spotify-typischen dezenten Unterstrich.

---

## Step 41: Horizontale Shelves (Playlists/Alben): Direktes Drag- & Swipe-Scrolling — UMSETZUNG IN v2.0.0

- **Freies Drag & Swipe**: Echte egui Trägheitsphysik (`DragScroll::Always`) mit flüssigem Swipen auf Playlists, Shelves und Alben.

---

## Step 42: Titelbreite & Spacing-Harmonisierung — UMSETZUNG IN v2.0.0

- **Quick Picks & Home**: +65 px mehr Platz für Songtitel, Hover-Fade des Drei-Punkte-Menüs.
- **Sidebar**: 20 px / 36 px Sicherheitsabstand zur Scrollbar.

---

## Step 43: Scrollbar-Springen auf der Startseite & Seitenwechsel-Bounce — UMSETZUNG IN v2.1.0

- **1. Scrollbar-Springen eliminiert**:
  - ScrollArea auf `ScrollBarVisibility::AlwaysVisible` gesetzt. Die Scrollbar bleibt absolut stabil an derselben Koordinate verankert und ruckelt/springt beim Nachladen von Shelves nicht mehr nach links oder rechts.
- **2. Vertikaler Seitenwechsel-Bounce entfernt**:
  - `ui.add_space(18.0 * (1.0 - t));` komplett entfernt.
  - Seitenwechsel blenden nun mit einem sauberen, ruhigen Opacity-Fade (`0.15 + 0.85 * t`) weich ein – absolut flatter- und sprungfrei wie bei Spotify/Apple Music Desktop.

---

*Erstellt und aktualisiert am 09.11.2026 – BitChord PC Master Roadmap.*




