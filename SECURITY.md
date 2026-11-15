# Security

[English](#english) · [Deutsch](#deutsch)

## English

UrlBridge starts local programs from links, so security problems are taken seriously.

### Reporting a vulnerability
Please do not open a public issue for a security problem. Use [Report a vulnerability](../../security/advisories/new) on the Security tab of this repository instead; the report stays private until a fix is released.

Helpful details:
- the version (`UrlBridge.exe --version`) and the Windows version,
- the configuration of the affected protocol, without internal names,
- the link or the steps that trigger the problem, and what happened instead of what you expected.

### Scope
The threat model in the [README](README.md#threat-model) describes what UrlBridge protects against. Reports are especially welcome for:
- a link that starts a program with arguments the configuration does not allow,
- a way for a user without administrator rights to change what UrlBridge starts,
- a start without the confirmation dialog.

Not in scope are actions that need administrator rights, a protocol a user registers for their own account only, and the security of the programs UrlBridge starts.

### Supported versions
Only the latest release receives fixes.

## Deutsch

UrlBridge startet lokale Programme aus Links, Sicherheitsprobleme werden deshalb ernst genommen.

### Eine Sicherheitslücke melden
Für ein Sicherheitsproblem bitte kein öffentliches Issue anlegen, sondern [Report a vulnerability](../../security/advisories/new) auf dem Reiter „Security“ dieses Repos verwenden; die Meldung bleibt privat, bis eine Korrektur veröffentlicht ist.

Hilfreiche Angaben:
- die Version (`UrlBridge.exe --version`) und die Windows-Version,
- die Konfiguration des betroffenen Protokolls, ohne interne Namen,
- der Link oder die Schritte, die das Problem auslösen, und was statt des Erwarteten passiert ist.

### Umfang
Das Bedrohungsmodell in der [README](README.md#bedrohungsmodell) beschreibt, wogegen UrlBridge schützt. Besonders willkommen sind Meldungen zu:
- einem Link, der ein Programm mit Argumenten startet, die die Konfiguration nicht erlaubt,
- einem Weg, auf dem ein Benutzer ohne Administratorrechte ändern kann, was UrlBridge startet,
- einem Start ohne Bestätigungsdialog.

Nicht dazu gehören Handlungen, die Administratorrechte brauchen, ein Protokoll, das ein Benutzer nur für sein eigenes Konto registriert, und die Sicherheit der Programme, die UrlBridge startet.

### Unterstützte Versionen
Nur das neueste Release erhält Korrekturen.
