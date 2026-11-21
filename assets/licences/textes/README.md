# Textes types des licences

Pour un paquet livré qui ne joint aucun fichier de licence (la famille egui, accesskit, objc2, `clipboard-win`…), le relevé reprend le texte type de sa licence, précédé de « Copyright (c) » et des auteurs que le paquet déclare. MIT est préféré quand le paquet laisse le choix.

| Fichier | Licence | Provenance |
|---|---|---|
| `MIT.txt` | MIT | `LICENSE-MIT` du paquet `serde` 1.0.229, tel quel |
| `Apache-2.0.txt` | Apache-2.0 | `LICENSE-APACHE` du paquet `serde` 1.0.229, tel quel |
| `BSL-1.0.txt` | Boost Software License 1.0 | `LICENSE` du paquet `error-code` 3.4.0, tel quel (même auteur que `clipboard-win`) |
| `MPL-2.0.txt` | Mozilla Public License 2.0 | `LICENSE.txt` du paquet `option-ext` 0.2.0, tel quel |

`MPL-2.0.txt` sert aussi à un paquet qui joint seulement l'avis court de la MPL sans son texte : `serialport` (« This Source Code Form is subject to the terms of the Mozilla Public License… »). Le relevé ajoute alors le texte entier.

Le relevé lui-même est dans le dossier au-dessus, fait par `tests/licences.rs` (commande dans le README de l'outil).
