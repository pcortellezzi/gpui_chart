# Plan d'Optimisation GPU : gpui_chart (via Blade)

L'objectif est de déléguer la géométrie et la projection au GPU pour supporter des millions de points à 144Hz, en utilisant **Blade**, l'API graphique de GPUI.

## 1. Architecture des Données (VRAM)
Passer d'un modèle "Push" (le CPU envoie des commandes de dessin à chaque trame) à un modèle "Resident" (les données stagnent sur le GPU).

- [ ] **Vertex Buffers Statiques/Dynamiques** : Concevoir des structures de données alignées (POD) pour Blade.
    - `PricePoint` : `{ f32 x, f32 y }`
    - `CandleData` : `{ f32 time, f32 open, f32 high, f32 low, f32 close }`
- [ ] **Ring Buffers GPU** : Pour le streaming, utiliser des buffers circulaires en mémoire vidéo pour éviter les transferts CPU->GPU massifs. Seuls les nouveaux points sont envoyés.

## 2. Shaders de Projection (Vertex Shaders)
Éliminer le coût du `PlotTransform` sur le CPU.

- [ ] **Uniforms de Transformation** : Envoyer le domaine visible (`x_min`, `x_max`, `y_min`, `y_max`) et les `Bounds` de l'écran sous forme de matrice ou de vecteurs.
- [ ] **Projection On-the-fly** : Le Vertex Shader calcule la position finale en pixels :
  `pos_pixels = (data_val - domain_min) / (domain_max - domain_min) * screen_size`

## 3. Rendu par Instanciation (Candlesticks & Bars)
Le type de rendu le plus rapide pour les graphiques financiers.

- [ ] **Modèle Unique** : Définir une "bougie type" (un rectangle et une ligne verticale) une seule fois.
- [ ] **Instanced Drawing** : Utiliser Blade pour dessiner N instances en un seul appel système. Le GPU ajuste la couleur (vert/rouge) et les hauteurs (OHLC) en lisant le `CandleData` buffer.

## 4. Rendu de Lignes Haute Performance (SDF / Polylines)
- [ ] **Shaders de Lignes** : Implémenter des techniques de *Signed Distance Fields* (SDF) pour dessiner des lignes d'épaisseur variable avec un anti-aliasing parfait, sans passer par `PathBuilder` qui triangule sur le CPU.
- [ ] **Strip Rendering** : Utiliser des `TriangleStrips` pour les lignes continues et les zones (Area).

## 5. Compute Shaders (Culling & LOD)
La puissance brute pour les datasets massifs (> 1M points).

- [ ] **GPU Culling** : Un *Compute Shader* analyse le buffer de données et génère un `IndexBuffer` contenant uniquement les points visibles.
- [ ] **LOD Dynamique** : Agréger les données (ex: fusionner 100 points en 1 pixel) directement sur le GPU en fonction du niveau de zoom.

## 6. Intégration GPUI / Blade
- [ ] **Custom Primitives** : Créer une primitive personnalisée qui s'insère dans le pipeline de rendu de GPUI.
- [ ] **Synchronization** : Gérer les barrières mémoire pour s'assurer que le streaming de données est terminé avant que Blade ne commence le rendu de la trame.

---

## Étapes de Migration suggérées
1. **POC** : Rendre un simple `BarPlot` via Blade en instanciation.
2. **Hybrid** : Garder les axes en `div` (GPUI) mais le contenu du graphique en Blade.
3. **Full GPU** : Passer les lignes et les zones sur shaders.
4. **Compute** : Ajouter le culling GPU pour la scalabilité ultime.
