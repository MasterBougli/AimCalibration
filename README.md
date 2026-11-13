# Aim Calibration

Application Windows qui vous aide à trouver **la sensibilité de souris idéale** pour vos jeux FPS (Valorant, Counter-Strike 2, Apex Legends, Overwatch 2, Call of Duty…), sous forme de mini-jeu de visée en plein écran.

Aucune connaissance préalable n'est nécessaire : l'application explique chaque réglage, détecte votre souris et peut même mesurer son DPI avec une simple carte bancaire.

## Télécharger

Rendez-vous dans l'onglet [**Releases**](https://github.com/MasterBougli/AimCalibration/releases), téléchargez `AimCalibration.exe` et lancez-le. Aucune installation n'est requise.

> Au premier lancement, Windows SmartScreen peut afficher un avertissement, car l'exécutable n'est pas signé : cliquez sur **Informations complémentaires**, puis **Exécuter quand même**.

## Menu principal

- **Trouver ma sensibilité** : le calibrage décrit ci-dessous.
- **Entraînement** : Flick, Tracking, Micro-ajustements, Switching, Réflexes, Peeks, Échauffement, et une routine guidée de 10 minutes (5 × 2 min).

L'échauffement s'arrête dès que le rythme retrouve la moyenne des dernières sessions de flick. Après un exercice de clic, le résultat indique le dépassement de la cible, le tremblement en fin de geste et le temps d'ajustement avant le clic. La sensibilité verticale et la sensibilité en visée (zoom ×1,5 en maintenant le clic droit) se règlent avec le DPI et le FOV. Après un tir raté, le trajet du viseur s'affiche une seconde, avec la cible marquée en jaune. Le résultat indique aussi la régularité : l'écart entre les flicks rapides et les flicks lents. Le menu principal montre l'objectif de la semaine. Le menu Options règle le volume et permet d'inverser l'axe vertical. L'écran de résultat garde le tracé du pire tir raté de la session.
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

Un record est gardé par mode, difficulté prédéfinie et durée, dans `records_entrainement.txt`. Après un calibrage, votre sensibilité idéale est pré-remplie pour vous entraîner directement avec. Chaque jeu garde sa propre sensibilité, son FOV et ses multiplicateurs.

La routine de 10 minutes commence par le flick, puis enchaîne les deux exercices où vos dernières sessions sont le plus loin de votre record. Sans historique, elle reprend l'ordre Flick, Tracking, Micro, Switching, Réflexes.

Si « Améliorer la précision du pointeur » est activé dans Windows, un message l'indique au menu : cette accélération fausse le cm/360°.

Sur l'écran de résultat, **C** copie la ligne du jeu sélectionné (`Valorant · 800 DPI · 0.352`). Le bilan d'un exercice sépare aussi les flicks courts et les flicks longs, et signale un biais répété (trop à droite, trop haut) : ce biais ne se corrige pas en changeant la sensibilité.

## Trouver sa sensibilité : comment ça marche

1. **Configuration** : indiquez le DPI de votre souris, le jeu dont vous connaissez la sensibilité et sa valeur actuelle. Vous ne savez pas ? Choisissez « Je ne sais pas » et appuyez sur **M** pour détecter et mesurer votre DPI.
2. **Réglage en direct** (Entrée) : 40 secondes de flick, puis 24 secondes de tracking en strafe. La sensibilité change pendant que vous visez. Un dépassement la baisse, un viseur qui reste avant la cible la monte. Quand les corrections deviennent minuscules, le réglage est stabilisé. Le chiffre n'apparaît qu'à la fin. Une courbe montre par où il est passé.
3. **Confirmation à l'aveugle** : 10 cibles sur la valeur trouvée, 10 sur une valeur 12 % à côté, sans afficher laquelle est laquelle. Si vous préférez la variante, elle remplace le résultat. Échap garde la valeur trouvée.
4. **Comparaison en manches** (touche C) : 6 manches. Chaque manche compare deux sensibilités, **A** et **B**, à l'aveugle. Pour chacune, vous éliminez 12 cibles (flick), puis vous suivez une cible mobile pendant 8 secondes (tracking). Vous gardez celle qui vous semble la plus naturelle, et l'écart est divisé par deux.
5. **Résultat** : votre sensibilité idéale en **cm/360°**, convertie pour chaque jeu supporté, avec l'eDPI. Chaque calibrage est gardé dans l'historique (vue « Sensibilité »). Appuyez sur **S** pour la sauvegarder dans `resultats_sensibilite.txt`.

Le tracking des exercices, le réglage en direct et les manches A/B suivent le même strafe : la cible inverse sa direction d'un coup.

**Flick en mouvement** : le viseur dérive, puis une cible apparaît dans l'axe du mouvement. **Contre-strafe** : le tir ne compte que quand la cible s'arrête et devient verte.

**Défi du jour** : les cibles sont toujours les mêmes. Le bilan compare le score à celui de la veille.

Sur le résultat, **T** ouvre la liste de transfert vers le jeu. La vitesse du pointeur Windows (1 à 20) y est affichée pour League of Legends.

La routine laisse 15 secondes de repos entre deux exercices. Le clic passe le repos.

La mesure lit les déplacements bruts du capteur de la souris : l'accélération et la vitesse du pointeur Windows n'ont aucun effet, comme dans les vrais FPS.

## Commandes

| Touche | Action |
| --- | --- |
| ↑ ↓ / Tab / souris | Naviguer dans le menu et les champs |
| ← → | Changer de jeu ou de difficulté |
| Entrée / clic | Valider, lancer le réglage en direct |
| C | Copier le réglage du jeu sélectionné (écran de résultat). Depuis l'écran de sensibilité : comparaison en 6 manches |
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
