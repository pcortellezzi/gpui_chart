# Plan de Développement : gpui_chart (High-Performance Plotting)

L'objectif est de transformer `gpui_chart` en une librairie de graphiques de niveau industriel (type `egui_plot` ou `TradingView`), optimisée pour GPUI et prête à être intégrée dans `adabraka-ui`.

## Objectifs Clés
1.  **Performance** : Rendu fluide avec des centaines de milliers de points (Culling, LOD, Stable Binning).
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
- [x] **Nettoyage ("De-coupling")**
- [x] **Abstraction des Échelles (`Scales`)**
- [x] **Système de Transformation (Transform Helper)**
- [x] **Auto-Range Dynamique ("Auto-Scale Y")**

## Phase 2 : Interactivité & UX (Le "Look & Feel")
- [x] **Navigation Avancée (Clavier)**
- [x] **Navigation Tactile & Trackpad**
- [x] **Inertial Scrolling (Physique)**
- [x] **Curseur & Inspection (Crosshair & Tooltip)**
- [x] **Zoom par Zone (Box Zoom)**
- [x] **Légende Interactive**

## Phase 2.5 : Composition & Synchronisation (Dashboarding)
- [x] **Mini-Map / Navigator**
- [x] **Synchronisation des Axes (Linked Axes)**
- [x] **Crosshair Globalisé & Synchronisé (Multi-View)**
- [x] **Gestion de la Visibilité des Axes**
- [x] **Exposition du Transform (Pour Vues Externes)**
- [x] **Gestion des Marges (Layout Alignment)**

## Phase 2.6 : Layout Dynamique & Multi-Pane
- [x] **Système de Panneaux (Panes)**
- [x] **Superposition (Overlays)**
- [x] **Séparateurs Redimensionnables (Splitters)**

## Phase 2.7 : Refonte TradingView (Architecture Découplée)
- [x] **Découplage Pane/Container**
- [x] **Gestion des Gouttières Globale (Gutters)**
- [x] **Axes X Multiples & Synchronisés**
- [x] **Axes Y Stackés**
- [x] **API Fluide (Builder Pattern)**

## Phase 3 : Richesse Visuelle & Primitives
- [x] **Candlestick (Bougies Japonaises)**
- [x] **Nouveaux Types de Tracés** (Area, Heatmap, Bar, StepLine)
- [x] **Annotations & Primitives Géométriques** (VLine, HLine, Rect, Text)
- [x] **Système Multi-Axes & Layout Flexible**

## Phase 4 : Optimisation (Performance & Intégrité)
- [x] **Streaming Optimisé (Ring Buffer)**
- [x] **LOD (Level of Detail) CPU**
- [x] **LOD Pyramidal**
- [x] **Occlusion Culling**
- [x] **Stable Binning (Anti-Jitter)**
- [x] **Intégrité des Données (Peak Preservation & Numerical Stability)**

## Phase Finalisation v1.0 (Architecture Déclarative)
- [x] **Refonte du Modèle de Données**
- [x] **Synchronisation de Structure**
- [x] **Centralisation du Rendu & Événements**
- [x] **Système de Thème (Theming v1.0)**
- [x] **Robustesse & Packaging**

## Phase 5 : Intégration & Export
- [ ] **Export & Capture** : API `save_to_image()` ou `copy_to_clipboard()` (PNG/SVG).
- [ ] **Headless Rendering** : Génération d'images sans fenêtre (CLI/Backend).
- [ ] **Style System** : Intégration avec les tokens de couleur `adabraka-ui`.
- [ ] **Composant Réutilisable** : Packaging final de `ChartContainer`.

## Phase 6 : Extension vers la parité ScottPlot (Visualisation Scientifique)
- [ ] **Scatter Plot** : Points non connectés avec marqueurs (Indispensable pour données scientifiques).
- [ ] **Error Bars** : Affichage de la variabilité (écart-type).
- [ ] **Bubble Plot** : Dimension Z via la taille des marqueurs.
- [ ] **Statistiques** : Box & Whisker, Violin Plots, Histogrammes (calcul de distribution).
- [ ] **Échelles Logarithmiques** : Support complet (Log10, Log2, Ln).
- [ ] **Interpolation Avancée** : Heatmaps avec interpolation bicubique/bilinéaire et ColorMaps (Viridis, Magma).
- [ ] **Décorateurs Avancés** : Flèches, Images de fond, Spans (bandes horizontales/verticales).

---

## Backlog & Améliorations Futures
- [ ] **Highlight on Hover** : Mise en surbrillance de l'élément sous la souris.
- [ ] **Rich Tooltips** : Infobulles multi-séries complexes.
- [ ] **Drag & Drop de Séries** : Déplacement de séries entre Panes à la souris.
- [ ] **ContextMenu des Séries** : Clic droit pour configurer couleur, type de tracé, etc.
- [ ] **Pinning & Mesure** : Outil de mesure de distance (prix/temps).
- [ ] **Paramétrage des touches** : Custom KeyBindings.
- [ ] **Signal Plot** : Renderer ultra-rapide pour taux d'échantillonnage fixe.
- [ ] **Inverted Axis** : API pour inverser un axe (ex: Graphique de profondeur).
- [ ] **WebGL / WGPU backend** : Pour des millions de points.

---

## 🎯 Priorités Courtes (Next Steps)
1. **Scatter Plot** : Combler le vide pour les données scientifiques non ordonnées.
2. **Échelles Logarithmiques** : Crucial pour les analyses techniques et scientifiques.
3. **Rich Tooltips** : Améliorer la lecture des données multi-séries.
4. **Export Image** : Fonctionnalité basique d'export PNG/SVG.