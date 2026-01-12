# Étape 1 : Optimisation Native Polars et Algorithme M4

## Objectifs
- Passer de l'algorithme Min/Max (2 points) à l'algorithme M4 (4 points : First, Min, Max, Last) pour une précision pixel-perfect.
- Remplacer les boucles manuelles par des expressions Polars natives pour descendre sous la barre des 1ms.

## Checklist
- [x] **Aggregation Engine (M4)** : 
    - [x] Étendre `src/aggregation.rs` pour supporter l'extraction de 4 points par bin (First, Min, Max, Last).
    - [x] Gérer le tri chronologique des 4 points pour éviter les "retours en arrière" visuels.
- [x] **Polars Native Core** :
    - [x] Refactoriser `PolarsDataSource::iter_aggregated` pour utiliser `df.lazy().group_by_dynamic()`. (Note: Utilisation de `group_by` sur index calculé pour performance maximale).
    - [x] Implémenter l'agrégation via expressions : `.agg([col(y).first(), col(y).min(), col(y).max(), col(y).last()])`.
    - [x] Utiliser `.explode()` ou `.reshape()` pour aplatir les résultats efficacement.
- [x] **Intégration** :
    - [x] Mettre à jour `PlotDataSource` pour permettre de choisir entre les modes d'agrégation (si pertinent).
- [x] **Benchmarking** :
    - [x] Comparer le temps de calcul avec l'implémentation actuelle.
        - Résultat : **15ms** pour 1M points (M4) vs **25ms** (MinMax).
        - Note : La cible < 1ms n'est pas atteinte en pur Polars Lazy (overhead du moteur), mais M4 est 40% plus rapide que l'ancien MinMax grâce au chunking par index.

## Prompt pour l'Agent de Développement
```text
Tu es un expert en Rust et Polars haute performance. Ton objectif est d'élever gpui_chart au "Top Niveau" de performance et de précision.

CONTEXTE :
Nous avons une implémentation fonctionnelle de PolarsDataSource, mais elle utilise des boucles Rust manuelles. Nous voulons utiliser les expressions natives de Polars et l'algorithme M4.

TACHES :
1. Modifie `src/aggregation.rs` pour introduire une logique M4 (First, Min, Max, Last).
2. Dans `src/polars_source.rs`, réécris `iter_aggregated` pour utiliser le moteur Lazy de Polars :
   - Utilise `group_by_dynamic` sur la colonne X.
   - Calcule simultanément first, min, max, et last.
   - Assure-toi que les données restent triées par X après l'agrégation.
3. Optimise la conversion vers `PlotData` en évitant au maximum les allocations intermédiaires.
4. Mets à jour le test de performance dans `tests/polars_tests.rs` pour valider la cible de < 1ms.

CONTRAINTES :
- Zero-copy autant que possible.
- Utilise les fonctions natives de Polars (`col().min()`, etc.) pour bénéficier du SIMD.
- Le code doit compiler avec la feature `polars` activée.
```
