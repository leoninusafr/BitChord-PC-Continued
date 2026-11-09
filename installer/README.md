# BitChord Windows Installer & 100% Residue-Free Uninstallation

Dieses Verzeichnis enthält die Konfigurationen zur Erstellung des Windows-Installers (`bitchord-setup-2.0.0.exe`).

## Features des Installers

1. **Ein-Klick-Installation**:
   - Modernes UI im Windows 11 / 10 Fluent-Stil.
   - Benutzer-Installation (keine lästigen UAC-/Admin-Berechtigungsabfragen nötig).
   - Standard-Installationspfad: `%LOCALAPPDATA%\Programs\BitChord`.
   - Startmenü- & Desktop-Verknüpfungen.
   - Automatische Protokoll-Registrierung für `ytm-rs://` (für nahtlose Spotify Jam / Listen-Together-Einladungslinks).

2. **100% Restlose Deinstallation ("Clean Wipe")**:
   - Kann direkt über die Windows-Einstellungen deinstalliert werden (**Einstellungen** > **Apps** > **Installierte Apps** > **BitChord** > **Deinstallieren**).
   - Entfernt das Programmverzeichnis vollständig.
   - Entfernt alle Verknüpfungen (Startmenü und Desktop).
   - Bereinigt sämtliche Registry-Einträge (`HKCU\Software\Classes\ytm-rs`, `HKCU\Software\BitChord`, `Uninstall`).
   - **Wichtig**: Löscht restlos alle temporären Daten, Caches, Tokens und Einstellungen aus:
     - `%APPDATA%\ytm-rs`
     - `%LOCALAPPDATA%\ytm-rs`
     - `%APPDATA%\BitChord`
     - `%LOCALAPPDATA%\BitChord`
   - Nach der Deinstallation verbleiben keinerlei Reste auf dem System.

## Erstellung des Installers

- **NSIS (Native / Linux & Windows Cross-Compilation)**:
  ```bash
  makensis installer/bitchord.nsi
  ```
  Erzeugt die Datei `installer/Output/bitchord-setup-2.0.0.exe`.

- **Inno Setup (Windows / CI)**:
  ```bash
  iscc installer/bitchord.iss
  ```
  Erzeugt ebenfalls `installer/Output/bitchord-setup-2.0.0.exe`.
