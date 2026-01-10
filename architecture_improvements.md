# Améliorations Architecturales : gpui_chart

Ce document suit les optimisations identifiées pour passer la librairie à un niveau industriel.

## 1. Performance & Rendu (Priorité Haute)
- [x] **Implémenter le Culling (Rendu Sélectif) :**
    - [x] Optimiser `LinePlot` pour ne dessiner que les points dans le domaine visible (Recherche binaire sur X).
    - [x] Appliquer le culling aux autres types (`Area`, `Bar`, `Candlestick`, `StepLine`).
- [x] **Accélération des calculs de domaine :**
    - [x] Cache des min/max par segments (Chunks) implémenté dans `VecDataSource` pour l'auto-scale instantané.
- [x] **Réduire les allocations :**
    - [x] Rendu direct via itérateurs (suppression de `.collect()` dans `AreaPlot`, `StepLinePlot`).
    - [x] Implémentation d'une décimation basique (LOD) dans `LinePlot` pour alléger les tracés.
    - [x] Fusion des itérations de remplissage et de contour dans `AreaPlot`.

## 2. État & Concurrence
- [x] **Résoudre le conflit de Threading :**
    - [x] Remplacer `Rc<RefCell<dyn PlotRenderer + Send + Sync>>` par une structure compatible (`Arc<parking_lot::RwLock>`).
- [x] **Assurer que le stockage des données est efficace :**
    - [x] Implémentation de `StreamingDataSource` (Ring Buffer via `VecDeque`) pour les mises à jour temps réel sans réallocation.
    - [x] Maintenance incrémentale du cache de bornes pour le streaming.

## 3. Découplage & Architecture des Fichiers
- [x] **Refactoriser `ChartContainer` :**
    - [x] Extraire `GutterManager` pour la gestion des marges.
    - [x] Extraire `AxisRenderer` pour le dessin des ticks et labels.
- [x] **Système de Thème :**
    - [x] Créer une structure `ChartTheme` centralisée pour les couleurs, polices et épaisseurs.

## 4. Richesse & Robustesse
- [x] **Gestion des erreurs :**
    - [x] Code exempt de `.unwrap()`, `.expect()` et `panic!` en production (vérifié via grep).
- [x] **Tests :**
    - [x] Benchmarks de performance validés : `get_y_range` en ~8µs et `iter_range` en ~30µs pour 100k points.
    - [x] Culling et Cache de segments opérationnels.
