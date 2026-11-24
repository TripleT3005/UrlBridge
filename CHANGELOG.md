# Changelog

All notable changes to UrlBridge, grouped by version in the style of [Keep a Changelog](https://keepachangelog.com/). The section of each version is used as its release notes. Changes on `main` that are not released yet are collected under [Unreleased] and move into the section of the next version.

Alle wichtigen Änderungen an UrlBridge, gegliedert nach Version im Stil von [Keep a Changelog](https://keepachangelog.com/de/). Der Abschnitt einer Version wird als ihre Release-Notizen verwendet. Änderungen auf `main`, die noch nicht veröffentlicht sind, stehen unter [Unreleased] und wandern mit der nächsten Version in deren Abschnitt.

## [Unreleased]

## [1.0.0] – 2026-11-24

### English

#### Added
- 36 more languages, so UrlBridge now speaks 38 languages, including Arabic and Hebrew with right-to-left dialogs. [LANGUAGES.md](LANGUAGES.md) lists them all; the new translations are AI-generated and not yet checked by native speakers.

#### Changed
- First stable version: the configuration format, the command line and the exit codes are now considered stable.

### Deutsch

#### Neu
- 36 weitere Sprachen, UrlBridge gibt es damit in 38 Sprachen, darunter Arabisch und Hebräisch mit Dialogen von rechts nach links. [LANGUAGES.md](LANGUAGES.md) listet alle auf; die neuen Übersetzungen sind von einer KI erzeugt und noch nicht von Muttersprachlern geprüft.

#### Geändert
- Erste stabile Version: Konfigurationsformat, Kommandozeile und Exit-Codes gelten jetzt als stabil.

## [0.2.1] – 2026-11-16

### English

#### Security
- Reading the permissions of a file now checks the data returned by Windows directly at every access. This addresses three findings of the code analysis; no way to exploit them is known, and the behavior is unchanged.

### Deutsch

#### Sicherheit
- Beim Lesen der Rechte einer Datei werden die von Windows gelieferten Daten jetzt direkt bei jedem Zugriff geprüft. Das behebt drei Meldungen der Codeanalyse; ein Weg, sie auszunutzen, ist nicht bekannt, und das Verhalten ist unverändert.

## [0.2.0] – 2026-11-15

### English

#### Added
- German as a second language: dialogs, messages and console output follow the Windows display language and fall back to English.
- Language files in the `lang` folder next to the program, so further languages can be added as plain text files; [LANGUAGES.md](LANGUAGES.md) lists them.
- The optional setting `language` in the configuration uses one language for all users.

#### Changed
- `Install.ps1` also installs the `lang` folder.
- `--validate` shows the language in use and warns about language files that standard users can modify; such files are ignored.

### Deutsch

#### Neu
- Deutsch als zweite Sprache: Dialoge, Meldungen und Konsolenausgabe folgen der Anzeigesprache von Windows und fallen auf Englisch zurück.
- Sprachdateien im Ordner `lang` neben dem Programm, damit sich weitere Sprachen als einfache Textdateien ergänzen lassen; [LANGUAGES.md](LANGUAGES.md) listet sie auf.
- Die optionale Einstellung `language` in der Konfiguration legt eine Sprache für alle Benutzer fest.

#### Geändert
- `Install.ps1` installiert auch den Ordner `lang`.
- `--validate` zeigt die verwendete Sprache und warnt vor Sprachdateien, die Standardbenutzer verändern können; solche Dateien werden ignoriert.

## [0.1.0] – 2026-11-15

### English

#### Added
- Custom URL protocols that start a configured program, for example `bridge-rdp://pc0042` for Remote Desktop.
- One configuration file for all protocols, changeable only by administrators.
- Strict value types for everything that comes from a link: `host`, `hostname`, `ipv4`, `integer`, `token` and `choice`.
- A confirmation dialog before every start that shows the program and its arguments.
- Checks that the configuration, UrlBridge and local target programs cannot be modified by standard users.
- Entries in the Windows event log for every start, cancellation and rejected link.
- `--test` to see what a link would start and `--validate` to check the configuration.
- `Install.ps1` and `Uninstall.ps1` for installation by hand or by software distribution, with an entry under *Settings → Apps*.

### Deutsch

#### Neu
- Eigene URL-Protokolle, die ein festgelegtes Programm starten, zum Beispiel `bridge-rdp://pc0042` für Remotedesktop.
- Eine Konfigurationsdatei für alle Protokolle, die nur Administratoren ändern können.
- Strenge Wertetypen für alles, was aus einem Link kommt: `host`, `hostname`, `ipv4`, `integer`, `token` und `choice`.
- Ein Bestätigungsdialog vor jedem Start, der das Programm und seine Argumente zeigt.
- Prüfungen, dass Standardbenutzer weder die Konfiguration noch UrlBridge noch lokale Zielprogramme verändern können.
- Einträge im Windows-Ereignisprotokoll für jeden Start, jeden Abbruch und jeden abgelehnten Link.
- `--test` zeigt, was ein Link starten würde, und `--validate` prüft die Konfiguration.
- `Install.ps1` und `Uninstall.ps1` für die Installation von Hand oder per Softwareverteilung, mit Eintrag unter *Einstellungen → Apps*.
