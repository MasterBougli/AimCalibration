# Aim Calibration

Application Windows qui vous aide à trouver **la sensibilité de souris idéale** pour vos jeux FPS (Valorant, Counter-Strike 2, Apex Legends, Overwatch 2, Call of Duty…), sous forme de mini-jeu de visée en plein écran.

Aucune connaissance préalable n'est nécessaire : l'application explique chaque réglage, détecte votre souris et peut même mesurer son DPI avec une simple carte bancaire.

## Télécharger

Rendez-vous dans l'onglet [**Releases**](https://github.com/MasterBougli/AimCalibration/releases), téléchargez `AimCalibration.exe` et lancez-le. Aucune installation n'est requise.

> Au premier lancement, Windows SmartScreen peut afficher un avertissement, car l'exécutable n'est pas signé : cliquez sur **Informations complémentaires**, puis **Exécuter quand même**.

## Menu principal

- **Trouver ma sensibilité** : le calibrage décrit ci-dessous.
- **Entraînement** : Flick, Tracking, Micro-ajustements, Switching, Réflexes, et une routine guidée de 10 minutes (5 × 2 min).
- **Historique** : courbe de progression par exercice.
- **Viseur** : couleur, écart, longueur, épaisseur et point central.
- **Aide** : DPI, cm/360°, eDPI et FOV expliqués simplement.

Les réglages (DPI, sensibilité, viseur, difficulté) sont mémorisés dans `reglages.txt`. Les scores s'ajoutent à `historique.txt`.

Les exercices proposent quatre difficultés (Facile, Normal, Difficile, Expert), ou un mode **personnalisé**. Les difficultés Difficile et Expert utilisent des cibles humanoïdes : un tir à la tête compte comme un headshot. Chaque réglage s'ajuste aussi séparément :

| Réglage | Flick | Tracking | Valeurs |
| --- | --- | --- | --- |
| Durée | ✓ | ✓ | 30 s, 60 s, 90 s, 2 min |
| Taille des cibles | ✓ | ✓ | Très grande → Minuscule |
| Cibles mobiles | ✓ | | Immobiles, Lentes, Rapides |
| Durée de vie des cibles | ✓ | | Illimitée, 2 s, 1,2 s, 0,8 s |
| Vitesse de la cible | | ✓ | Lente → Très rapide |
| Mouvements verticaux | | ✓ | Oui, Non |
| Zone d'apparition | ✓ | ✓ | Étroite, Normale, Large |

Un record est gardé par mode, difficulté prédéfinie et durée, dans `records_entrainement.txt`. Après un calibrage, votre sensibilité idéale est pré-remplie pour vous entraîner directement avec.

## Trouver sa sensibilité : comment ça marche

1. **Configuration** : indiquez le DPI de votre souris, le jeu dont vous connaissez la sensibilité et sa valeur actuelle. Vous ne savez pas ? Choisissez « Je ne sais pas » et appuyez sur **M** pour détecter et mesurer votre DPI.
2. **6 manches de test** : chaque manche compare deux sensibilités, **A** et **B**, à l'aveugle. Pour chacune, vous éliminez 12 cibles (flick), puis vous suivez une cible mobile pendant 8 secondes (tracking).
3. **Votre choix** : après chaque manche, l'application affiche vos statistiques (précision, temps par cible, tracking) et suggère la meilleure. Vous gardez celle qui vous semble la plus naturelle, et l'écart entre les deux sensibilités est divisé par deux.
4. **Résultat** : votre sensibilité idéale en **cm/360°**, convertie pour chaque jeu supporté, avec l'eDPI. Appuyez sur **S** pour la sauvegarder dans `resultats_sensibilite.txt`.

La mesure lit les déplacements bruts du capteur de la souris : l'accélération et la vitesse du pointeur Windows n'ont aucun effet, comme dans les vrais FPS.

## Commandes

| Touche | Action |
| --- | --- |
| ↑ ↓ / Tab / souris | Naviguer dans le menu et les champs |
| ← → | Changer de jeu ou de difficulté |
| Entrée / clic | Valider, commencer |
| M | Détecter la souris et mesurer le DPI |
| H / F1 | Aide sur les termes (DPI, cm/360°, eDPI, FOV) |
| Échap | Pause pendant un test, retour au menu principal, quitter depuis le menu |
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
