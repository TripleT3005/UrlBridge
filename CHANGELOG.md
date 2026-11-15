# Changelog

All notable changes to UrlBridge, grouped by version in the style of [Keep a Changelog](https://keepachangelog.com/). The section of each version is used as its release notes. Changes on `main` that are not released yet are collected under [Unreleased] and move into the section of the next version.

Alle wichtigen Änderungen an UrlBridge, gegliedert nach Version im Stil von [Keep a Changelog](https://keepachangelog.com/de/). Der Abschnitt einer Version wird als ihre Release-Notizen verwendet. Änderungen auf `main`, die noch nicht veröffentlicht sind, stehen unter [Unreleased] und wandern mit der nächsten Version in deren Abschnitt.

## [Unreleased]

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
