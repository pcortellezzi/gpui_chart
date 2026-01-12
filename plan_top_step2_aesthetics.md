# Étape 2 : Algorithme LTTB pour l'Esthétique

## Objectifs
- Implémenter l'algorithme LTTB (Largest Triangle Three Buckets) pour préserver la forme visuelle des courbes.
- Offrir une option de rendu "Haute Fidélité" dans la librairie.

## Checklist
- [ ] **LTTB Engine** :
    - [ ] Implémenter l'algorithme LTTB dans `src/aggregation.rs` (fonction `decimate_lttb`).
    - [ ] Optimiser l'implémentation pour minimiser les calculs de triangles (calcul d'aire simplifié).
- [ ] **API Update** :
    - [ ] Étendre l'énumération existante `AggregationMode` dans `src/data_types.rs` pour inclure `LTTB`.
    - [ ] Mettre à jour `PolarsDataSource` pour gérer ce nouveau mode (Attention : LTTB nécessite un accès séquentiel aux buckets, difficile en pur Lazy Polars, envisager une implémentation hybride ou UDF).
- [ ] **Tests Visuels** :
    - [ ] Créer un test comparatif produisant des sets de données où M4 "scintille" (bruit) mais LTTB reste fluide (forme).

## Prompt pour l'Agent de Développement
```text
Tu es un expert en algorithmique géométrique. Tu dois implémenter l'algorithme LTTB pour améliorer l'esthétique des graphiques.

CONTEXTE :
Nous avons déjà une enum `AggregationMode` avec `MinMax` et `M4`. Nous voulons ajouter `LTTB`.

TACHES :
1. Dans `src/aggregation.rs`, implémente la fonction `decimate_lttb(data: &[PlotData], max_points: usize)`.
2. L'algorithme doit diviser les données en buckets et, pour chaque bucket, choisir le point qui forme le triangle de plus grande aire avec le point sélectionné dans le bucket précédent et la moyenne du bucket suivant.
3. Étends `AggregationMode` dans `src/data_types.rs` avec le variant `LTTB`.
4. Mets à jour `PolarsDataSource::iter_aggregated` pour supporter ce mode. Note : Si l'implémentation pure expression est impossible/trop complexe, utilise une approche optimisée sur des chunks ou via `map_partitions`.

RÉFÉRENCE :
L'algorithme LTTB est décrit par Sveinn Steinarsson. Il est crucial pour la réduction de données temporelles sans perte de perception de forme.
```
