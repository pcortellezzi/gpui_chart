# Validation Qualité : Top Niveau Aggregation

## Objectifs
- Vérifier la robustesse aux cas limites (données vides, 1 point, NaN, Inf).
- Valider la non-régression sur les sources existantes (Vec, Streaming).
- Certifier les performances.

## Checklist QA
- [ ] **Tests de Robustesse** :
    - [ ] Tester M4 et LTTB avec 0, 1, 2 points.
    - [ ] Tester avec des valeurs `f64::NAN` et `f64::INFINITY` dans Polars.
- [ ] **Exactitude Visuelle** :
    - [ ] Vérifier que M4 retourne bien les bornes exactes du dataset original.
    - [ ] Vérifier que LTTB produit bien `max_points` points exactement.
- [ ] **Performance Profile** :
    - [ ] Lancer un benchmark `cargo bench` (si disponible) ou un test release.
    - [ ] Vérifier la consommation mémoire sous Polars (pas de fuite lors du group_by).

## Prompt pour l'Agent de Qualité
```text
Tu es un agent QA spécialisé dans les systèmes critiques et la performance. Ton rôle est de valider les nouvelles fonctionnalités d'agrégation "Top Niveau".

TACHES :
1. Analyse le code de `src/aggregation.rs` et `src/polars_source.rs`.
2. Identifie les risques potentiels : division par zéro dans le calcul de bin_size, gestion des types Option/Result dans Polars.
3. Écris un fichier de test `tests/qa_aggregation.rs` qui pousse les algorithmes M4 et LTTB (si implémenté) dans leurs retranchements :
   - Injections de NaN.
   - Séries temporelles avec des trous (gaps).
   - Datasets massifs (> 5M de lignes).
4. Valide que le changement de mode via `with_aggregation_mode` fonctionne correctement.
5. Rapporte tout temps de calcul supérieur à 20ms pour 1M de points sur Polars (Benchmark actuel ~15ms).
6. Vérifie que la feature flag `polars` est correctement isolée et que la librairie compile sans elle.
```
