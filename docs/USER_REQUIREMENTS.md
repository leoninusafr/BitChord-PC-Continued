# BitChord PC Overhaul — Nutzeranforderungen & Feature-Spezifikation

Dieses Dokument sichert den vollständigen Prompt und alle Anforderungen des Nutzers zur Überarbeitung der BitChord Windows/PC-Version im Vergleich zur Mobile-Version und Premium-Features bekannter Musikplayer (Spotify, Apple Music, Tidal, Spottube, Spicetify).

---

## 1. Original-Anforderung & Kernwünsche (Zusammenfassung)

> "Moin, das hier soll der Musik-Ordner sein und zwar würde ich dich bitten, einmal die Bit-Cord Windows-Version zu nehmen und sie zu überarbeiten, weil die kann aktuell nicht so ganz mithalten mit der Mobile-Version. Das heißt, du kannst dir einfach da das allerneueste Update der Mobile-Version anschauen und versuchst das dann halt sozusagen clean in die Windows-Version zu übertragen..."

### A. Remote-Steuerung & "Spotify Connect" Feeling
- **Nahtlose Steuerung ohne Web-Hürden**: Musik auf dem PC abspielen und gleichzeitig am Smartphone (oder Zweitgerät) sehen, was gerade läuft, pausieren, weiterspringen, Lautstärke anpassen.
- **Geräte-Auswahl / Speaker Switch**: Wie bei Spotify Connect ein Dialog („Über welches Gerät abspielen?“), um aktiv zwischen PC, Mobilgerät oder verbundenen Boxen umzuschalten.
- **Echte QR-Codes & Direktlinks**: Schnelle Kopplung per PNG-QR-Code (keinesfalls Text/ASCII-Art) und Klick-Link.

### B. "Listen Together" / Spotify Jam (Discord & WAN)
- **Synchronisiertes Hören mit Freunden**: Wie Spotify Jam oder Discord Listen Along.
- **Global / WAN**: Nicht nur im selben lokalen WLAN, sondern auch über das Internet / Discord.
- **Gemeinsame Warteschlange**: Beide können Titel zur Queue hinzufügen, pausieren und skippen.
- **Unabhängige Lautstärke**: Jeder Teilnehmer behält die volle, individuelle Kontrolle über seine eigene Lautstärke.

### C. Player-Leiste (Bottom Bar) – Volle Breite statt Enge
- **Keine gequetschte zentrierte Pill**: Auf dem PC war die Leiste eng zentriert und wirkte wie minderwertiges Fake-Liquid-Glass.
- **Vollbreite (Edge-to-Edge wie Spotify / BitChord Mobile)**:
  - **Links**: Track-Cover, Titel, Artist, Herz-Favoriten-Button.
  - **Mitte**: Shuffle-Toggle, Zurück, Play/Pause (mit taktilem Button-Press), Weiter, Repeat-Modus sowie eine breite Scrubber-Bar mit verstrichener / verbleibender Zeit.
  - **Rechts**: Immer sichtbare Lautstärkeleiste, Songtexte (Lyrics), Warteschlange (Queue), Titeldetails (Details Panel) und Geräte-/Connect-Button.

### D. Apple-Style Lautstärkeregler & Mausrad-Scrubbing
- **Immer sichtbar**: Kein lästiges Hover-Popover mehr, das erst aufploppt und dann den Hintergrund scrollt.
- **Dynamische Expansion (Apple Control Center / Dynamic Capsule)**:
  - Ruhezustand: Elegante, schlanke Kapsel (~4-6px Höhe).
  - Bei Hover oder beim Ziehen (Active Drag): Weitet sich weich auf (~10-12px) für intuitive und taktile Treffsicherheit.
- **Mausrad-Steuerung**: Drehen am Mausrad direkt über der Lautstärke regelt die Lautstärke stufenlos und präzise, **ohne** dabei die Seite im Hintergrund zu scrollen!
- **Klick-to-Seek & weiches Ziehen**.

### E. Spotify-Sync (Spottube-Prinzip)
- **Nahtlose Integration der Spotify-Bibliothek**: Liked Songs und Playlists direkt in BitChord anzeigen und abspielen.
- **Audio-Streaming via YouTube Music**: Nutzt hochqualitative Streams von YTM im Hintergrund für freies, werbefreies Hören.
- **Strikter Musik-Fokus**: Podcasts, Hörspiele und Audiobooks werden rigoros herausgefiltert.

### F. Navigation & Layout-Bereinigung
- **Redundanten Settings-Button entfernen**: Ein Klick auf das Profil-Avatar in der Top-Bar führt bereits in die Einstellungen. Ein zweiter Button in der Sidebar ist überflüssig.
- **Spotify/Spicetify Top-Bar**:
  - Vor-/Zurück-Navigation.
  - **Würfel-Symbol ("Play Random Song")**: Spielt per Zufall einen Song aus der gesamten Musik-Welt ab (Spicetify-Feature).
  - Zentrierte Suchleiste, flankiert von Home und Discover.
  - Geräte-Modal und Profil-Avatar.
- **Rechtes Info-Panel ("Aktueller Titel")**:
  - Großes Album-Cover, Künstler-Informationen, direkter Künstler-Besuch, Vorschau auf nächste Titel und synchronisierte Lyrics.

### G. Taktile Animationen & Micro-Interactions
- **Mobile-App-Gefühl**: Schnelle, responsive Animationen (Spring-Physics, `scale(0.96)` Button-Press, weiche Übergänge).
- **Keine Zähigkeit**: Weder zu langsam noch abgehackt – flüssiges, modernes 60+ FPS Feeling.
- **Herz-Button statt Stern**: Favorit mit Pop-Skalierung beim Liken.
- **Shuffle-Logik wie Spotify**: Fester Ein-/Aus-Zustand mit Indikator; beim Ausschalten wird die ursprüngliche Song-Reihenfolge exakt wiederhergestellt.
