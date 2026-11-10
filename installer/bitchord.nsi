; BitChord Windows Installer Script (NSIS)
; Generates a clean, modern installer with 1-click complete residue-free uninstall

!define PRODUCT_NAME "BitChord"
!define PRODUCT_VERSION "2.1.0"
!define PRODUCT_PUBLISHER "jackychan4891"
!define PRODUCT_WEB_SITE "https://github.com/jackychan4891/BitChord-PC-Continued"
!define PRODUCT_EXE "bitchord.exe"
!define PRODUCT_UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}"

!ifndef BIN_DIR
  !define BIN_DIR "..\target\x86_64-pc-windows-gnu\release"
!endif

; Target amd64-unicode
Target amd64-unicode

; Compression
SetCompressor /SOLID lzma

; Request execution level: user (per-user install into LocalAppData\Programs\BitChord, no admin required, like Spotify/VSCode)
RequestExecutionLevel user

; Name & Output file
Name "${PRODUCT_NAME}"
OutFile "Output\bitchord-setup-${PRODUCT_VERSION}.exe"

; Modern UI 2
!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"

; Interface Settings
!define MUI_ABORTWARNING
!define MUI_ICON "..\assets\icon\icon.ico"
!define MUI_UNICON "..\assets\icon\icon.ico"

; Default Installation Directory: $LOCALAPPDATA\Programs\BitChord
InstallDir "$LOCALAPPDATA\Programs\${PRODUCT_NAME}"
InstallDirRegKey HKCU "Software\${PRODUCT_NAME}" "InstallDir"

; Installer Pages
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES

; Finish Page: Option to launch BitChord
!define MUI_FINISHPAGE_RUN "$INSTDIR\${PRODUCT_EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "BitChord jetzt starten"
!insertmacro MUI_PAGE_FINISH

; Uninstaller Pages
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

; Languages (German first, English second)
!insertmacro MUI_LANGUAGE "German"
!insertmacro MUI_LANGUAGE "English"

; Ensure any running instances are closed before install/uninstall
Function .onInit
  nsExec::Exec 'taskkill /F /IM bitchord.exe'
FunctionEnd

Function un.onInit
  nsExec::Exec 'taskkill /F /IM bitchord.exe'
FunctionEnd

; Installer Section
Section "Hauptprogramm" SecMain
  SetOutPath "$INSTDIR"
  SetOverwrite on

  ; Install binaries & docs
  File "${BIN_DIR}\${PRODUCT_EXE}"
  File "..\README.md"
  File "..\LICENSE"

  ; Create uninstaller
  WriteUninstaller "$INSTDIR\uninstall.exe"

  ; Save InstallDir to registry
  WriteRegStr HKCU "Software\${PRODUCT_NAME}" "InstallDir" "$INSTDIR"

  ; Start Menu Shortcuts
  CreateDirectory "$SMPROGRAMS\${PRODUCT_NAME}"
  CreateShortcut "$SMPROGRAMS\${PRODUCT_NAME}\${PRODUCT_NAME}.lnk" "$INSTDIR\${PRODUCT_EXE}" "" "$INSTDIR\${PRODUCT_EXE}" 0
  CreateShortcut "$SMPROGRAMS\${PRODUCT_NAME}\Deinstallieren.lnk" "$INSTDIR\uninstall.exe" "" "$INSTDIR\uninstall.exe" 0

  ; Desktop Shortcut
  CreateShortcut "$DESKTOP\${PRODUCT_NAME}.lnk" "$INSTDIR\${PRODUCT_EXE}" "" "$INSTDIR\${PRODUCT_EXE}" 0

  ; Register ytm-rs:// protocol (Spotify Jam / BitChord invite links)
  WriteRegStr HKCU "Software\Classes\ytm-rs" "" "URL:ytm-rs Protocol"
  WriteRegStr HKCU "Software\Classes\ytm-rs" "URL Protocol" ""
  WriteRegStr HKCU "Software\Classes\ytm-rs\DefaultIcon" "" "$INSTDIR\${PRODUCT_EXE},0"
  WriteRegStr HKCU "Software\Classes\ytm-rs\shell\open\command" "" '"$INSTDIR\${PRODUCT_EXE}" "%1"'

  ; Register in Windows "Installierte Apps" / Apps & Features (Windows Settings Uninstall)
  WriteRegStr HKCU "${PRODUCT_UNINST_KEY}" "DisplayName" "${PRODUCT_NAME}"
  WriteRegStr HKCU "${PRODUCT_UNINST_KEY}" "DisplayVersion" "${PRODUCT_VERSION}"
  WriteRegStr HKCU "${PRODUCT_UNINST_KEY}" "Publisher" "${PRODUCT_PUBLISHER}"
  WriteRegStr HKCU "${PRODUCT_UNINST_KEY}" "URLInfoAbout" "${PRODUCT_WEB_SITE}"
  WriteRegStr HKCU "${PRODUCT_UNINST_KEY}" "DisplayIcon" "$INSTDIR\${PRODUCT_EXE},0"
  WriteRegStr HKCU "${PRODUCT_UNINST_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "${PRODUCT_UNINST_KEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegStr HKCU "${PRODUCT_UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegDWORD HKCU "${PRODUCT_UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${PRODUCT_UNINST_KEY}" "NoRepair" 1

  ; Calculate estimated size
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKCU "${PRODUCT_UNINST_KEY}" "EstimatedSize" "$0"
SectionEnd

; Uninstaller Section (100% RESTLOSE ENTFERNUNG)
Section "Uninstall"
  ; 1. Delete installed files
  Delete "$INSTDIR\${PRODUCT_EXE}"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\uninstall.exe"
  RMDir /r "$INSTDIR"

  ; 2. Delete Shortcuts
  Delete "$SMPROGRAMS\${PRODUCT_NAME}\${PRODUCT_NAME}.lnk"
  Delete "$SMPROGRAMS\${PRODUCT_NAME}\Deinstallieren.lnk"
  RMDir "$SMPROGRAMS\${PRODUCT_NAME}"
  Delete "$DESKTOP\${PRODUCT_NAME}.lnk"

  ; 3. Delete Registry entries
  DeleteRegKey HKCU "${PRODUCT_UNINST_KEY}"
  DeleteRegKey HKCU "Software\Classes\ytm-rs"
  DeleteRegKey HKCU "Software\${PRODUCT_NAME}"

  ; 4. RESTLOSE ENTFERNUNG: Alle App-Daten, Caches, Tokens & Einstellungen bereinigen
  RMDir /r "$APPDATA\ytm-rs"
  RMDir /r "$LOCALAPPDATA\ytm-rs"
  RMDir /r "$APPDATA\${PRODUCT_NAME}"
  RMDir /r "$LOCALAPPDATA\${PRODUCT_NAME}"
SectionEnd
