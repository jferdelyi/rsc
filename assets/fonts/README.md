# Polices

| Fichier | Rôle | Provenance |
|---|---|---|
| `lucide.ttf` | les icônes de la fenêtre (boutons, navigation des réglages) | paquet npm `lucide-static` 1.74.0, fichier `font/lucide.ttf`, tel quel |
| `LICENSE-lucide.txt` | sa licence : ISC, et MIT pour les icônes reprises de Feather | le fichier `LICENSE` du même paquet, tel quel |

Les icônes s'appellent par leur nom dans `src/ui/icons.rs`, avec le point de code que donne `font/codepoints.json` du même paquet. Pour changer de version : reprendre ces deux fichiers d'un même paquet, puis relancer les tests, qui vérifient que chaque icône utilisée est dans la police.
