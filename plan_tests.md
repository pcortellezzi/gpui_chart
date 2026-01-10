# Plan de Test : gpui_chart (Objectif 100% Couverture)

Ce document liste l'intégralité des tests à implémenter pour garantir la robustesse et la non-régression de la bibliothèque.

---

## 1. Fondations Mathématiques & Transformations (`scales.rs`, `transform.rs`)
Objectif : Précision absolue des projections de coordonnées.

- [x] **Échelles Linéaires (`ChartScale::Linear`)**
    - [x] Mapping direct (donnée -> pixel).
    - [x] Inversion (pixel -> donnée).
    - [x] Formatage des ticks (Petites valeurs, grands nombres, timestamps).
- [ ] **Échelles Logarithmiques (`ChartScale::Log`)**
    - [ ] Mapping logarithmique de base.
    - [ ] Gestion des cas d'erreur (valeurs ≤ 0) : s'assurer qu'elles ne font pas paniquer.
    - [ ] Génération de ticks sur échelle log.
- [x] **Transformations de Tracé (`PlotTransform`)**
    - [x] Projection Data Point -> Screen Point (avec marges de fenêtre).
    - [x] Transformation inverse Screen -> Data.
    - [ ] Projection sur axe unique (X ou Y indépendamment).

---

## 2. Gestion des Données (`data_types.rs`)
Objectif : Fiabilité du streaming et efficacité du culling.

- [x] **AxisRange (Logique de fenêtre)**
    - [x] Panoramique (Pan).
    - [x] Zoom centré sur un pivot (Conservation du pivot virtuel).
    - [x] Clamping intelligent (Support des coordonnées virtuelles hors-limites).
- [ ] **Itérateurs de Source (`PlotDataSource`)**
    - [ ] **Culling Précis** : Vérifier que `iter_range` renvoie exactement les points visibles + 1 point de débordement (pour la continuité des lignes).
    - [ ] **Streaming (VecDataSource)** : Validation du cache de bornes après ajout massif.
    - [x] **Streaming (StreamingDataSource)** :
        - [x] Éviction correcte des données au-delà de la capacité.
        - [x] Recalcul automatique des bornes (x_min/x_max) après éviction.
- [ ] **Métriques de Tracé**
    - [ ] Calcul de l'espacement suggéré (`suggested_spacing`) : Vérifier la convergence de la moyenne mobile exponentielle.

---

## 3. Orchestration & Layout (`gutter_manager.rs`, `layout.rs`)
Objectif : Alignement parfait des graphiques et des axes.

- [ ] **GutterManager (Calcul des marges)**
    - [ ] Calcul avec 0 axe (marges nulles).
    - [ ] Somme des largeurs pour N axes stackés à droite.
    - [ ] Calcul des marges opposées (Gauche vs Droite).
    - [ ] Alignement vertical entre Panes (vérifier que les zones de dessin sont parfaitement alignées).
- [ ] **Splitters & Redimensionnement**
    - [ ] Validation de `resize_panes` : somme des poids (weights) constante après redimensionnement.
    - [ ] Protection contre les tailles négatives ou nulles.

---

## 4. Tests d'Interactions via ViewController (Découplage UI)
Objectif : Simuler l'expérience utilisateur sans dépendre de la boucle de rendu GPUI.

- [ ] **Interaction Mouse / Drag (Panning)**
    - [ ] Simuler un `delta` de mouvement souris et vérifier le décalage proportionnel de `AxisRange`.
    - [ ] Vérifier que le panoramique s'arrête aux limites virtuelles (clamping).
- [ ] **Interaction Scroll / Ctrl (Zoom)**
    - [ ] Calcul du `zoom_factor` à partir d'un `ScrollDelta`.
    - [ ] Validation de la conservation du pivot sous le curseur simulé.
- [ ] **Interaction Box Zoom (Selection)**
    - [ ] Projection d'un rectangle `Bounds<Pixels>` vers un nouveau `AxisRange` de données.
- [ ] **Gestion des États Partagés**
    - [ ] Mise à jour du `hover_x` dans `SharedPlotState` lors d'un déplacement simulé.
    - [ ] Synchronisation : vérifier qu'un changement sur l'axe X impacte toutes les Panes abonnées.

## 5. Composants & Interactions (`chart_pane.rs`, `chart_container.rs`)
Objectif : Comportement UI prévisible.

- [ ] **Auto-Fit Intelligent**
    - [ ] **Auto-fit X** : Couvrir l'intégralité des séries de toutes les panes.
    - [ ] **Auto-fit Y Dynamique** : Vérifier que l'axe Y s'ajuste uniquement en fonction des points **visibles** dans la fenêtre X actuelle.
- [ ] **Gestion des Panes**
    - [x] Création et insertion de pane à un index spécifique.
    - [x] Suppression de pane.
    - [x] Réordonnancement (Swap) de panes.
- [ ] **Gestion des Séries**
    - [x] Migration d'une série entre deux panes (changement de parent).
    - [x] Toggle de visibilité (impact sur l'auto-fit).
    - [ ] **Isolation** : Création automatique d'un nouvel axe Y lors de l'isolation.
    - [ ] **Réintégration** : Suppression de l'axe Y orphelin après réintégration sur l'axe principal.
- [ ] **Physique & UX**
    - [ ] **Inertie** : Vérifier la décroissance de la vélocité frame par frame.
    - [ ] **Box Zoom** : Calcul des nouvelles bornes à partir des coordonnées d'un rectangle de sélection.

## 6. Types de Tracés Spécifiques (`plot_types/*.rs`)
Objectif : Rendu correct des primitives de données.

- [ ] **CandlestickPlot**
    - [ ] Logique de couleur (Close > Open = Green, etc.).
    - [ ] Calcul de la largeur des bougies par rapport à l'espacement des données.
- [ ] **StepLinePlot**
    - [ ] Validation du mode `Pre` (marche avant le point).
    - [ ] Validation du mode `Mid` (marche centrée).
    - [ ] Validation du mode `Post` (marche après le point).
- [ ] **AreaPlot**
    - [ ] Calcul de la base (Baseline) : Remplissage vers le haut ou vers le bas.
- [ ] **HeatmapPlot**
    - [ ] Alignement des cellules sur la grille de données.

---

## 7. Navigation & Visualisation (`navigator_view.rs`, `rendering.rs`)
Objectif : Navigation fluide dans l'historique.

- [ ] **NavigatorView**
    - [ ] Calcul de la "fenêtre éclairée" (highlight bounds).
    - [ ] Drag de la fenêtre : synchronisation avec l'axe X principal.
    - [ ] Handles de redimensionnement : Zoom X synchrone.
- [ ] **Grilles & Tags**
    - [ ] Densité des lignes de grille (éviter l'effet "moiré" ou la saturation).
    - [ ] Formatage des tags d'axes en fonction de la précision des données.

---

## Guide d'implémentation pour l'Agent

1. **Priorité 1 (Logique Pure)** : Utiliser des `#[test]` standards. Ne nécessite pas `gpui::test`.
2. **Priorité 2 (Modèles GPUI)** : Utiliser `#[gpui::test]` avec `TestAppContext`. Se limiter aux modifications d'état (`cx.update`).
3. **Éviter** : Les tests de layout réels (ouverture de fenêtres avec des vues complexes) dans des fichiers externes, pour prévenir le crash du compilateur (`SIGSEGV`).
