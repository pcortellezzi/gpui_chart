# Plan de Développement : gpui_chart (High-Performance Plotting)

L'objectif est de transformer `gpui_chart` en une librairie de graphiques de niveau industriel (type `egui_plot` ou `ScottPlot`), optimisée pour GPUI et prête à être intégrée dans `adabraka-ui`.

## Objectifs Clés
1.  **Performance** : Rendu fluide avec des centaines de milliers de points (Culling, LOD).
2.  **Interactivité** : Tooltips, Crosshairs, Zoom par zone, Sélection.
3.  **Richesse** : Support complet de `gpui-d3rs` (Scales, Shapes), Axes multiples, Légendes.
4.  **Intégration** : API idiomatique pour `adabraka-ui`.

---

## Protocoles Impératifs de Développement
**À respecter rigoureusement pour chaque tâche :**

1.  **Tests Systématiques :** Chaque nouvelle fonction doit avoir son test unitaire associé.
2.  **Non-Régression :** Exécuter `cargo test` après chaque développement pour valider la nouvelle fonctionnalité ET l'absence de régression sur l'existant.
3.  **Compilation Garantie :** Le projet doit toujours compiler (`cargo build`) à la fin d'une étape.
4.  **Validation Visuelle :** Exécuter l'application pour confirmer visuellement le résultat (rendu graphique).
5.  **Documentation Continue :** Mettre à jour la documentation (commentaires de code, README) immédiatement.
6.  **Vérification des Écritures :** Après chaque modification de fichier (`write` ou `replace`), relire systématiquement le fichier (`read_file`) pour garantir son intégrité.
7.  **Analyse Préalable (Context Awareness) :** Ne jamais modifier un fichier sans avoir lu son contenu actuel et ses imports au préalable (pour respecter le style et éviter les hallucinations).
8.  **Robustesse (Zero Panic) :** Interdiction stricte des `.unwrap()` ou `.expect()` dans le code de production. Utiliser une gestion d'erreurs propre (`Result`, logs).
9.  **Qualité Statique :** Le code doit viser à satisfaire `cargo clippy` (Rust idiomatique).
10. **Auto-Correction Raisonnée :** En cas d'erreur (compilation/test), analyser l'erreur explicitement avant de proposer une correction. Pas de "fix" à l'aveugle.
11. **Propreté des Commentaires :** Pas de commentaires "réflexifs" (pensées de l'IA). Les commentaires doivent être techniques. Nettoyer tout commentaire temporaire avant la compilation finale.

## Phase 1 : Refonte Architecturale & Coordonnées (Fondations)
Avant d'ajouter des fonctionnalités, il faut solidifier la gestion de l'espace et **nettoyer la logique métier**.

- [x] **Nettoyage ("De-coupling")**
    - Supprimé les types spécifiques au métier (`TimeMarker`, `Goldbach`).
- [x] **Abstraction des Échelles (`Scales`)**
    - Logique de ticks et formatage déléguée à `ChartScale` (via `d3rs`).
- [x] **Système de Transformation (Transform Helper)**
    - Créé `PlotTransform` avec `data_to_screen` et `screen_to_data`.
- [x] **Auto-Range Dynamique ("Auto-Scale Y")**
    - Implémenté `auto_fit_axes` dans `ChartView`.

## Phase 2 : Interactivité & UX (Le "Look & Feel")
C'est ici que l'écart avec `egui_plot` se réduit.

- [x] **Navigation Avancée (Clavier)**
    - Support du clavier (Flèches pour pan, `+/-` pour zoom, `0` reset).
- [ ] **Navigation Tactile & Trackpad**
    - Support des gestuelles Trackpad (Pinch-to-zoom fluide, Pan deux doigts).
- [ ] **Inertial Scrolling (Physique)**
    - Ajouter une inertie ("momentum") lors du glissement pour un rendu natif et fluide (type iOS/macOS).
- [x] **Curseur & Inspection (Crosshair & Tooltip)**
    - Capturer la position de la souris (`Hover`).
    - **Mode "Magnétique" :** Le curseur "colle" intelligemment aux points d'intérêt (High/Low/Close) lors de l'inspection ou du dessin.
    - Projeter la position souris -> données (`screen_to_data`).
    - "Snapping" : Trouver le point de donnée le plus proche.
    - Afficher une infobulle (Tooltip) flottante via `overlay` GPUI.
- [ ] **Zoom par Zone (Box Zoom)**
    - Gestionnaire d'état pour un outil de sélection (Drag rectangulaire).
    - Mettre à jour le domaine (`x_min`, `x_max`, etc.) basé sur le rectangle relâché.
- [ ] **Légende Interactive**
    - Overlay affichant la liste des séries.
    - Toggle de visibilité par série (clic sur la légende).

## Backlog & Améliorations Futures
- [ ] **Paramétrage des touches** : Permettre à l'utilisateur de définir ses propres KeyBindings pour les actions de navigation.
- [ ] **Interactivité des Axes** : Permettre de redimensionner (stretch) un seul axe en le glissant directement.
- [ ] **Échelles Logarithmiques** : Support complet des échelles log dans `ChartScale`.
- [ ] **Thèmes avancés** : Export des styles vers un fichier de config externe.
- [ ] **WebGL / WGPU backend** : Pour des performances encore plus extrêmes sur des millions de points.

## Phase 2.5 : Composition & Synchronisation (Dashboarding)
Pour permettre des dispositions complexes (Indicateurs en bas, DOM à droite).

- [ ] **Mini-Map / Navigator**
    - Un petit graphique simplifié en bas montrant l'historique complet avec une fenêtre glissante pour naviguer rapidement.
- [ ] **Synchronisation des Axes (Linked Axes)**
    - Permettre à plusieurs instances de `ChartView` de partager le même objet d'état `AxisDomain` (via `Rc<RefCell>` ou `Model<T>`).
    - *Cas d'usage :* Zoomer sur le graphique des Prix zoome instantanément le RSI situé en dessous.
- [ ] **Exposition du Transform (Pour Vues Externes)**
    - Rendre le système de conversion `Scale` public et observable.
    - *Cas d'usage (DOM) :* Une vue "Tableau" (qui n'est pas un Chart) peut observer l'échelle Y du graphique principal pour ajuster la hauteur de ses lignes ou filtrer les prix visibles.
- [ ] **Gestion des Marges (Layout Alignment)**
    - Système pour fixer/synchroniser la largeur des marges entre plusieurs graphiques superposés.

## Phase 3 : Richesse Visuelle & Primitives
Exploiter `gpui-d3rs` pour le dessin.

- [ ] **Candlestick (Bougies Japonaises)**
    - Porter et adapter l'implémentation existante située dans `../src/ui/`.
    - S'assurer de la gestion correcte des couleurs (Hausse/Baisse) et de la largeur dynamique des bougies selon le zoom.
- [ ] **Nouveaux Types de Tracés**
    - **Area Chart** : Remplissage sous courbe.
    - **Heatmap / Grid** : Grille de rectangles colorés avec support de texte (valeurs numériques). Idéal pour afficher des carnets d'ordres (DOM) historiques ou de la densité de volume. Doit supporter l'agrégation (LOD) spatiale.
    - **Bar Chart** : Histogrammes.
    - **Step Line** : Lignes en escalier.
- [ ] **Annotations & Primitives Géométriques**
    - Remplacent `TimeMarker` et `Goldbach`.
    - Primitives : `VLine` (Ligne Verticale infinie), `HLine` (Ligne Horizontale infinie), `Rect` (Zones), `Text`.
    - Permet à l'utilisateur de composer ses propres indicateurs métier.
- [ ] **Système Multi-Axes & Layers (Flexible)**
    - Architecture : `Chart` possède une collection d'`Axes` (X1, X2... Y1, Y2...) et de `Series`.
    - Chaque `Series` est liée à une paire d'ID d'axes (ex: `xaxis: "x1", yaxis: "y2"`).

## Phase 4 : Optimisation (Performance)
Pour gérer le "Big Data".

- [ ] **Streaming Optimisé (Ring Buffer)**
    - Utiliser des structures de données circulaires ou par blocs (Chunks) pour éviter les réallocations coûteuses (`Vec::push`) lors de l'arrivée de données temps réel haute fréquence.
- [ ] **LOD (Level of Detail) / Décimation**
    - **Source de vérité :** S'inspirer directement de la logique implémentée dans `../src/` du projet parent.
    - **Stratégie Configurable :** Permettre de définir des seuils d'agrégation manuels (ex: "passer en vue 1h si plage > 1 jour") OU automatiques (basés sur la densité de pixels visuelle).
    - **Agrégation X & Y :** Supporter non seulement le temps (X) mais aussi le prix (Y) pour les Heatmaps/DOM.
    - Implémenter l'agrégation dynamique (fusion de bougies, simplification de lignes).
    - Évite de saturer le GPU et le CPU pour des détails invisibles.
- [ ] **Occlusion Culling**
    - Ne pas envoyer de commandes de dessin pour les points hors du `Bounds` visible.

## Phase 5 : Intégration Adabraka-UI
- [ ] **Export & Capture**
    - API `save_to_image()` ou `copy_to_clipboard()` pour partager les analyses instantanément.
- [ ] **Style System**
    - Utiliser les tokens de couleur de `adabraka-ui` (thèmes).
- [ ] **Composant Reutilisable**
    - Packager `ChartView` pour qu'il soit instantiable facilement avec une API fluide type Builder pattern.

---

## État actuel vs Cible
| Fonctionnalité | Actuel (`gpui_chart`) | Cible (`egui_plot` like) |
|---|---|---|
| **Coordonnées** | Calcul manuel linéaire | `gpui-d3rs` Scales (Log, Time) |
| **Ticks/Grille** | `LinearScale` basique | `TimeScale` intelligent |
| **Inspection** | Crosshair, Tags, Tooltip | Tooltip, Crosshair, Snap |
| **Zoom** | Molette & Keyboard | Box Zoom, Auto-Y on Zoom |
| **Performance** | Dessine tout | Culling & Downsampling |