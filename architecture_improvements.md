# Améliorations Architecturales : gpui_chart

Ce document suit les optimisations identifiées pour passer la librairie à un niveau industriel.

## 1. Performance & Rendu (Priorité Haute)
- [x] **Implémenter le Culling (Rendu Sélectif) :**
    - [x] Optimiser `LinePlot` pour ne dessiner que les points dans le domaine visible (Recherche binaire sur X).
    - [x] Appliquer le culling aux autres types (`Area`, `Bar`, `Candlestick`, `StepLine`).
- [ ] **Accélération des calculs de domaine :**
    - [ ] Cache des min/max par segments (Segment Tree ou similaire) pour l'auto-scale instantané sur >100k points.
- [ ] **Réduire les allocations :**
    - [ ] Utiliser des `PrimitiveBatch` ou recycler les `PathBuilder` si possible.

## 2. État & Concurrence
- [x] **Résoudre le conflit de Threading :**
    - [x] Remplacer `Rc<RefCell<dyn PlotRenderer + Send + Sync>>` par une structure compatible (`Arc<parking_lot::RwLock>`).
- [ ] **Assurer que le stockage des données est efficace :**
    - [ ] Implémenter des structures de données pour les mises à jour temps réel (Ring Buffer).

## 3. Découplage & Architecture des Fichiers
- [x] **Refactoriser `ChartContainer` :**
    - [x] Extraire `GutterManager` pour la gestion des marges.
    - [x] Extraire `AxisRenderer` pour le dessin des ticks et labels.
- [x] **Système de Thème :**
    - [x] Créer une structure `ChartTheme` centralisée pour les couleurs, polices et épaisseurs.

## 4. Richesse & Robustesse
- [ ] **Gestion des erreurs :**
    - [ ] Remplacer les rares `.unwrap()` par une gestion propre.
- [ ] **Tests :**
    - [ ] Ajouter des tests de performance pour mesurer l'impact du culling.
