# Aim Calibration

Application Windows qui vous aide à trouver **la sensibilité de souris idéale** pour vos jeux FPS (Valorant, Counter-Strike 2, Apex Legends, Overwatch 2, Call of Duty…), sous forme de mini-jeu de visée en plein écran.

Aucune connaissance préalable n'est nécessaire : l'application explique chaque réglage, détecte votre souris et peut même mesurer son DPI avec une simple carte bancaire.

## Télécharger

Rendez-vous dans l'onglet [**Releases**](https://github.com/MasterBougli/AimCalibration/releases), téléchargez `AimCalibration.exe` et lancez-le. Aucune installation n'est requise.

> Au premier lancement, Windows SmartScreen peut afficher un avertissement, car l'exécutable n'est pas signé : cliquez sur **Informations complémentaires**, puis **Exécuter quand même**.

## Comment ça marche

1. **Configuration** : indiquez le DPI de votre souris, le jeu dont vous connaissez la sensibilité et sa valeur actuelle. Vous ne savez pas ? Choisissez « Je ne sais pas » et appuyez sur **M** pour détecter et mesurer votre DPI.
2. **6 manches de test** : chaque manche compare deux sensibilités, **A** et **B**, à l'aveugle. Pour chacune, vous éliminez 12 cibles (flick), puis vous suivez une cible mobile pendant 8 secondes (tracking).
3. **Votre choix** : après chaque manche, l'application affiche vos statistiques (précision, temps par cible, tracking) et suggère la meilleure. Vous gardez celle qui vous semble la plus naturelle, et l'écart entre les deux sensibilités est divisé par deux.
4. **Résultat** : votre sensibilité idéale en **cm/360°**, convertie pour chaque jeu supporté, avec l'eDPI. Appuyez sur **S** pour la sauvegarder dans `resultats_sensibilite.txt`.

La mesure lit les déplacements bruts du capteur de la souris : l'accélération et la vitesse du pointeur Windows n'ont aucun effet, comme dans les vrais FPS.

## Commandes

| Touche | Action |
| --- | --- |
| ↑ ↓ / Tab | Changer de champ |
| ← → | Changer de jeu |
| Entrée | Commencer |
| M | Détecter la souris et mesurer le DPI |
| H / F1 | Aide sur les termes (DPI, cm/360°, eDPI, FOV) |
| Échap | Pause pendant un test, retour ou quitter |
| F11 | Basculer le plein écran |

## Jeux supportés

Valorant, Counter-Strike 2, Apex Legends, Overwatch 2, Call of Duty (MW / Warzone), Team Fortress 2 / Quake.

**League of Legends** n'est pas un FPS : seule la vitesse du curseur compte, et une valeur en cm/360° n'y a pas de sens. L'application donne quand même des conseils de réglage adaptés.

## Compiler depuis les sources

Nécessite [Rust](https://www.rust-lang.org/tools/install) (édition 2024).

```powershell
cargo build --release
```

L'exécutable est généré dans `target\release\aim_calibration.exe`.

## Licence

**Logiciel propriétaire — tous droits réservés.** Le code source est public uniquement pour consultation.

Vous pouvez télécharger et utiliser gratuitement l'application pour un usage personnel. Il est **interdit** de la vendre, de la modifier, de la redistribuer ou de réutiliser son code sans l'autorisation écrite de l'auteur. Voir [`LICENSE`](LICENSE) pour les conditions complètes.

Les bibliothèques tierces utilisées sont listées dans [`THIRD_PARTY_LICENSES.md`](THIRD_PARTY_LICENSES.md).
