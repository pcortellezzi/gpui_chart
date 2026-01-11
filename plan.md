# Plan de Développement : gpui_chart (High-Performance Plotting)

L'objectif est de transformer `gpui_chart` en une librairie de graphiques de niveau industriel (type `egui_plot` ou `TradingView`), optimisée pour GPUI et prête à être intégrée dans `adabraka-ui`.

## Objectifs Clés
1.  **Performance** : Rendu fluide avec des centaines de milliers de points (Culling, LOD).
2.  **Interactivité** : Tooltips, Crosshairs, Zoom par zone, Sélection, Drag & Drop.
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
12. **Interdiction des Placeholders :** Ne jamais utiliser `// ...` dans les outils. Toujours fournir le code complet.

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
- [x] **Navigation Tactile & Trackpad**
    - Support des gestuelles Trackpad (Pan deux doigts, Pinch-to-zoom via modifiers).
- [x] **Inertial Scrolling (Physique)**
    - Ajouter une inertie ("momentum") lors du glissement pour un rendu natif et fluide.
- [x] **Curseur & Inspection (Crosshair & Tooltip)**
    - Capturer la position de la souris (`Hover`).
    - **Mode "Magnétique" :** Le curseur "colle" intelligemment aux points d'intérêt (High/Low/Close) lors de l'inspection ou du dessin.
    - Projeter la position souris -> données (`screen_to_data`).
    - "Snapping" : Trouver le point de donnée le plus proche.
    - Afficher une infobulle (Tooltip) flottante via `overlay` GPUI.
- [x] **Zoom par Zone (Box Zoom)**
    - Gestionnaire d'état pour un outil de sélection (Drag rectangulaire).
    - Mettre à jour le domaine (`x_min`, `x_max`, etc.) basé sur le rectangle relâché.
- [x] **Légende Interactive**
    - Overlay affichant la liste des séries.
    - Toggle de visibilité par série (clic sur la légende).
    - Auto-fit intelligent ignorant les séries masquées.

## Backlog & Améliorations Futures
- [ ] **Highlight on Hover** : Utiliser `SharedPlotState.hover_x` dans les renderers pour mettre en surbrillance l'élément sous la souris (bougie, barre, point sur ligne).
- [ ] **Drag & Drop de Séries** : Permettre de glisser une série directement d'une Pane à une autre à la souris (alternative au bouton ▲/▼).
- [ ] **ContextMenu des Séries** : Clic droit sur une série ou son étiquette pour changer la couleur, le type de tracé (ex: Ligne -> Step) ou les paramètres LOD.
- [ ] **Pinning & Mesure** : Clic simple pour épingler une valeur ou mesurer la distance (prix/temps) entre deux points.
- [ ] **Paramétrage des touches** : Permettre à l'utilisateur de définir ses propres KeyBindings pour les actions de navigation.
- [x] **Robustesse & Tests d'Interactions** :
    *   Extraire la logique de manipulation des axes dans un `ViewController` découplé de l'UI pour permettre des tests 100% automatisés sans macros GPUI complexes.
    *   Couvrir les cas limites de redimensionnement de panneaux (splitters).
- [ ] **Échelles Logarithmiques** : Support complet des échelles log dans `ChartScale`.
- [ ] **Thèmes avancés** : Export des styles vers un fichier de config externe.
- [ ] **WebGL / WGPU backend** : Pour des performances encore plus extrêmes sur des millions de points.

## Phase 2.5 : Composition & Synchronisation (Dashboarding)
Pour permettre des dispositions complexes (Indicateurs en bas, DOM à droite).

- [x] **Mini-Map / Navigator**
    - Un petit graphique simplifié en bas montrant l'historique complet avec une fenêtre glissante pour naviguer rapidement.
    - Support de la navigation 2D (X/Y) et verrouillage d'axe.
    - Clamping aux limites des données ou limites paramétrables (ex: Y min = 0).
- [x] **Synchronisation des Axes (Linked Axes)**
    - **FIXÉ :** Synchronisation bidirectionnelle stable via `zoom_at` (pivot préservé).
    - Partage d'état `AxisRange` via `Entity<T>`.
    - **Calcul vs Rendu :** Séparation des bornes "idéales" pour le calcul et "clampées" pour le rendu visuel.
- [x] **Crosshair Globalisé & Synchronisé (Multi-View)**
    - **FIXÉ :** Crosshair vertical partagé sur tous les panneaux synchronisés.
    - **FIXÉ :** Crosshair horizontal et Tag Y locaux au graphique survolé (évite les doublons).
    - **FIXÉ :** Style des étiquettes harmonisé (Fond Blanc / Texte Noir, 12px) et coordonnées locales corrigées.
- [x] **Gestion de la Visibilité des Axes**
    - Système pour masquer sélectivement l'axe X ou Y (Tags X affichés uniquement sur l'axe visible).
- [x] **Exposition du Transform (Pour Vues Externes)**
    - Système de conversion `Scale` public et observable.
- [x] **Gestion des Marges (Layout Alignment)**
    - Système pour fixer/synchroniser la largeur des marges entre plusieurs graphiques superposés via `margin_left`.

## Phase 2.6 : Layout Dynamique & Multi-Pane
- [x] **Système de Panneaux (Panes)**
    - Gérer l'agencement automatique : Top, Bottom, Left, Right.
    - Ajouter des boutons d'action (UI) pour déplacer une vue (Monter/Descendre/Déplacer).
- [x] **Superposition (Overlays)**
    - Permettre de fusionner deux vues (ex: Volume par-dessus le Prix) avec des axes Y indépendants (Y1 à gauche, Y2 à droite).
- [x] **Séparateurs Redimensionnables (Splitters)**
    - Implémenter des zones de saisie ("grip") entre les graphiques pour ajuster leur hauteur relative par drag.

## Phase 2.7 : Refonte TradingView (Architecture Découplée)
Objectif : Séparer strictement le dessin (Panes) du layout et des axes (Container).

- [x] **Découplage Pane/Container**
    - `ChartPane` : Zone de dessin passive (Canvas) sans gestion de marges d'axes.
    - `ChartContainer` : Orchestrateur gérant les gouttières globales et le layout des Panes.
- [x] **Gestion des Gouttières Globale (Gutters)**
    - Calculer les marges à Gauche/Droite/Haut/Bas basées sur la somme des axes Y stackés de toutes les Panes.
    - Dessiner tous les axes (X et Y) dans ces gouttières, en dehors des zones de dessin des Panes.
- [x] **Axes X Multiples & Synchronisés**
    *   Permettre d'empiler plusieurs axes X dans les gouttières (Haut/Bas).
    *   Idéal pour afficher plusieurs fuseaux horaires simultanément (ex: UTC, New York, Heure Locale).
    *   Tous les axes X d'un conteneur partagent le même domaine temporel mais utilisent des transformations de labels différentes.
- [x] **Axes Y Stackés (Option B)**
    - Permettre d'empiler plusieurs axes Y dans la même gouttière (ex: Axe Prix | Axe RSI).
    - Rendu de l'axe Y restreint verticalement à la hauteur de sa Pane parente.
- [x] **API Fluide (Builder Pattern)**
    - Syntaxe type `Chart::new().with_pane(Pane::new().series(...))` inspirée de GPUI.

## Phase 3 : Richesse Visuelle & Primitives
Exploiter `gpui-d3rs` pour le dessin.

- [x] **Candlestick (Bougies Japonaises)**
    - Porter et adapter l'implémentation existante.
    - Gestion correcte des couleurs (Hausse/Baisse) et largeur dynamique des bougies.
- [X] **Nouveaux Types de Tracés**
    - **Area Chart** : Remplissage sous courbe. (Implémenté localement dans `AreaPlot`)
    - **Heatmap / Grid** : Grille de rectangles colorés avec support de texte.
    - **Bar Chart** : Histogrammes. (Implémenté localement dans `BarPlot`)
    - **Step Line** : Lignes en escalier. (Implémenté localement dans `StepLinePlot`)
- [x] **Annotations & Primitives Géométriques**
    - Remplacent `TimeMarker` et `Goldbach`.
    - Primitives : `VLine`, `HLine`, `Rect`, `Text`.
    - Permet à l'utilisateur de composer ses propres indicateurs métier.
- [x] **Système Multi-Axes & Layout Flexible (Refonte)**
    - **Architecture :** Abandonné la logique implicite au profit du `ChartContainer`.
    - **Structure :** Chaque axe possède une config explicite : `Edge` (Top/Bottom/Left/Right), `Width/Height`.
    - **Interaction :** Drag/Zoom 1:1 sur les axes et double-clic pour reset.

## Phase 4 : Optimisation (Performance)
Pour gérer le "Big Data".

- [x] **Streaming Optimisé (Ring Buffer)**
    - Utiliser des structures de données circulaires ou par blocs (Chunks) pour éviter les réallocations coûteuses (`Vec::push`) lors de l'arrivée de données temps réel haute fréquence. (Implémenté via `StreamingDataSource` chunked).
- [x] **LOD (Level of Detail) CPU**
    - **API Standardisée :** Ajout de `iter_aggregated` au trait `PlotDataSource`.
    - **Renderers Intelligents :** `BarPlot` et `CandlestickPlot` utilisent désormais la densité de pixels pour demander des données agrégées.
    - **Binning Dynamique :** Implémenté pour `VecDataSource` (moyenne/OHLC à la volée).
- [x] **LOD Pyramidal**
    - Pré-calculer les agrégats à différentes échelles (Mipmaps) pour accélérer le binning sur les datasets massifs (O(1) vs O(N)). Implémenté pour `VecDataSource`.
- [x] **Occlusion Culling**
    - Ne pas envoyer de commandes de dessin pour les points hors du `Bounds` visible. (Implémenté via `iter_range` et `partition_point` binaire).

## Phase Finalisation v1.0 (Architecture Déclarative)
L'objectif est d'offrir une API "GPUI-native" où la structure du graphique est décrite dans le `render()` tout en conservant l'état (zoom, splitters) dans une entité unique.

- [x] **Refonte du Modèle de Données (Découplage État/Vue)**
    - [x] Créer `ChartElement`, `PaneElement` et `AxisElementConfig` (Builders légers).
    - [x] Implémenter les fonctions globales `chart()`, `pane()`, `axis()`.
    - [x] Faire de `ChartContainer` l'unique `Entity` (View) persistante.
    - [x] Supprimer l'entité `ChartPane` (devient une structure de données passive `PaneElement`).

- [x] **Synchronisation de Structure (`sync_from_element`)**
    - [x] Implémenter la logique de réconciliation dans `ChartContainer`.
    - [x] Préserver les poids (`weights`) des panneaux lors du redimensionnement par splitter, même si la structure est redéfinie.
    - [x] Gérer l'ajout/suppression dynamique d'axes ou de panneaux sans perdre le zoom des axes restants.

- [x] **Centralisation du Rendu & Événements**
    - [x] Déplacer la logique d'interaction (Pan, Zoom, Inertie) de `ChartPane` vers `ChartContainer`.
    - [x] Unifier le calcul des gouttières (`GutterManager`) pour tout le conteneur.
    - [x] Centraliser le dessin des légendes et des overlays (Tooltips, Crosshairs).

- [x] **Système de Thème (Theming v1.0)**
    - [x] Intégration complète de `ChartTheme` dans tous les renderers (via `SharedPlotState` pour éviter le prop-drilling).
    - [x] Support natif du basculement automatique Light/Dark (`set_theme`).
    - [x] Permettre la surcharge du thème au niveau du builder `.theme(my_theme)`.

- [x] **Robustesse & Packaging (Finalisation v1.0)**
    - [x] **Robustesse (Zero Crash)** :
        - [x] Gestion de l'état "No Data" (vérifié par tests unitaires et analyse statique).
        - [x] Gestion des Domaines Plats (`min == max`) (géré nativement par `ChartScale`).
        - [x] Audit "Zero Panic" (audit statique effectué).
    - [x] **Packaging API** :
        - [x] Masquer les modules internes (`GutterManager`, etc.) et n'exporter que la surface publique via `lib.rs` (Façade).
        - [x] Vérifier et nettoyer `Cargo.toml`.
    - [x] **Documentation** :
        - [x] Créer un README.md avec un exemple "Hello World" minimaliste.
        - [ ] Ajouter la Rustdoc sur les structs principales (`Chart`, `Series`) (Partiellement fait via commentaires existants, suffisant pour v1).

## Phase 5 : Intégration Adabraka-UI
- [ ] **Export & Capture**
    - API `save_to_image()` ou `copy_to_clipboard()` pour partager les analyses instantanément.
- [ ] **Style System**
    - Utiliser les tokens de couleur de `adabraka-ui` (thèmes).
- [ ] **Composant Reutilisable**
    - Packager `ChartContainer` pour qu'il soit instantiable facilement avec une API fluide type Builder pattern: .chart(...).panes(...).axe_x(...).axe_y(...).axe_x(...).minimap(...)

---

## État actuel vs Cible
| Fonctionnalité | Actuel (`gpui_chart`) | Cible (`TradingView` like) |
|---|---|---|
| **Architecture** | Conteneur Global + Panes passives | Environnement de travail complet |
| **Coordonnées** | Scales (Log, Time) via `d3rs` | LOD & Shaders pour haute performance |
| **Ticks/Grille** | `TimeScale` intelligent | Formateurs multi-fuseaux horaires |
| **Inspection** | Tags, Crosshair, Tooltip | Snap magnétique avancé |
| **Zoom** | Molette, Keyboard, Box, Axes | Zoom 1:1 ultra-précis |
| **Axes Y** | N axes stackés par Pane | Indépendance totale et interaction 1:1 |
| **Axe X** | Axes X synchronisés (multi-TZ) | Synchro native via domaine partagé |
| **Performance** | Dessine tout | Culling, LOD & WGPU backend |
