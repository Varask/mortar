# Architecture du projet Mortar

## Vue d'ensemble

Mortar est structure en plusieurs couches :

```
┌─────────────────────────────────────────────────────────┐
│                    Interfaces                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │
│  │   Web UI    │  │  REST API   │  │      CLI        │  │
│  │  (app.js)   │  │   (Axum)    │  │  (rustyline)    │  │
│  └──────┬──────┘  └──────┬──────┘  └────────┬────────┘  │
└─────────┼────────────────┼──────────────────┼───────────┘
          │                │                  │
          └────────────────┼──────────────────┘
                           │
┌──────────────────────────┼──────────────────────────────┐
│                    server.rs                            │
│              (AppState, Handlers)                       │
└──────────────────────────┼──────────────────────────────┘
                           │
┌──────────────────────────┼──────────────────────────────┐
│                      lib.rs                             │
│           (Core: Ballistics, Dispersion)                │
└──────────────────────────┼──────────────────────────────┘
                           │
┌──────────────────────────┼──────────────────────────────┐
│                    data/                                │
│      (CSV Tables, metrics.json)                         │
└─────────────────────────────────────────────────────────┘
```

## Composants

### 1. Bibliotheque Core (`lib.rs`)

La bibliotheque core est independante et peut etre utilisee sans le serveur.

#### Types de donnees

```rust
// Types de munitions
enum AmmoKind { Practice, He, Smoke, Flare }

// Types de cibles
enum TargetType { Infanterie, Vehicule, Soutien }

// Positions
struct Position { name, elevation, x, y }
struct MortarPosition { ..., ammo_type }
struct TargetPosition { ..., target_type }

// Balistique
struct BallisticPoint { range_m, elev_mil }
struct BallisticTable { points: Vec<BallisticPoint> }

// Solution
struct FiringSolution { distance_m, azimuth_deg, ... }
```

#### Fonctions principales

| Fonction | Description |
|----------|-------------|
| `load_ballistics()` | Charge les tables CSV |
| `load_dispersion()` | Charge metrics.json |
| `calculate_solution_with_dispersion()` | Calcule la solution complete |
| `calculate_dispersion()` | Ajuste la dispersion au denivele |
| `apply_correction()` | Corrige une position de cible |

### 2. Serveur (`server.rs`)

Le serveur combine une API REST (Axum) et une CLI interactive.

#### Etat de l'application

```rust
struct AppState {
    ballistics: BTreeMap<(AmmoKind, Ring), BallisticTable>,
    dispersions: DispersionTable,
    mortars: RwLock<Vec<MortarPosition>>,
    targets: RwLock<Vec<TargetPosition>>,
}
```

L'etat est partage entre les threads via `Arc<AppState>` et les donnees mutables sont protegees par `RwLock`.

#### Routes API

```
GET  /api/health          -> HealthResponse
GET  /api/types           -> TypesResponse
GET  /api/mortars         -> MortarListResponse
POST /api/mortars         -> SuccessResponse
DELETE /api/mortars       -> SuccessResponse
POST /api/mortars/ammo    -> SuccessResponse
GET  /api/targets         -> TargetListResponse
POST /api/targets         -> SuccessResponse
DELETE /api/targets       -> SuccessResponse
POST /api/targets/type    -> SuccessResponse
POST /api/targets/correct -> CorrectionResponse
POST /api/calculate       -> FiringSolution
```

### 3. Interface Web (`src/web/`)

Application single-page en JavaScript vanilla.

#### Fichiers

| Fichier | Role |
|---------|------|
| `index.html` | Structure HTML |
| `style.css` | Styles (theme militaire olive/khaki) |
| `app.js` | Logique frontend (fetch API) |

#### Flux de donnees

```
User Action -> app.js -> fetch(/api/...) -> server.rs -> lib.rs
                                                 |
UI Update <- app.js <- JSON Response <-----------┘
```

### 4. Donnees (`data/`)

#### Structure des fichiers

```
data/
├── metrics.json           # Dispersions par munition/anneau
├── PRACTICE/
│   ├── M879_PRACTICE_0R.csv
│   ├── M879_PRACTICE_1R.csv
│   ├── M879_PRACTICE_2R.csv
│   ├── M879_PRACTICE_3R.csv
│   └── M879_PRACTICE_4R.csv
├── HE/
│   └── M821_HE_*.csv
├── SMOKE/
│   └── M819_SMOKE_*.csv   # 1R-4R seulement
└── FLARE/
    └── M853A1_FLARE_*.csv # 1R-4R seulement
```

#### Format CSV

```csv
range_m,elev_mil,time_flight_s,delta_elev_per_100m_mil,time_flight_per_100m_s
50,1540,13.2,61,
100,1479,13.2,63,0.2
150,1416,13.4,62,0.2
```

Seules les colonnes `range_m` et `elev_mil` sont utilisees.

#### Format metrics.json

```json
{
    "dispersion": {
        "HE": {
            "0R": 10,
            "1R": 23,
            "2R": 39,
            "3R": 54,
            "4R": 69
        },
        "PRACTICE": { ... },
        "SMOKE": { ... },
        "FLARE": { ... }
    }
}
```

## Flux de calcul

### Calcul de solution

```
1. Recevoir mortar_name, target_name
                │
2. Recuperer positions depuis AppState
                │
3. Calculer geometrie:
   - distance = sqrt((x2-x1)² + (y2-y1)²)
   - azimuth = atan2(dx, dy)
   - elevation_diff = |e1 - e2|
                │
4. Pour chaque (AmmoKind, Ring):
   - Interpoler elevation depuis BallisticTable
   - Calculer dispersion ajustee
                │
5. Construire FiringSolution
                │
6. Retourner JSON
```

### Calcul de dispersion

```
delta = mortar_elevation - target_elevation

if delta >= 0:
    factor = 1 + delta * 0.05    # +5% par metre
else:
    factor = 1 + delta * 0.01    # -1% par metre

adjusted_dispersion = base_dispersion * factor
```

### Correction de tir

```
Deviation observee: (vertical_m, horizontal_m)
- vertical: Nord(-) / Sud(+)
- horizontal: Ouest(-) / Est(+)

Correction appliquee:
- new_x = target_x - horizontal_m
- new_y = target_y - vertical_m
```

## Concurrence

- Le serveur utilise Tokio pour l'async
- `AppState` est partage via `Arc`
- `mortars` et `targets` sont proteges par `RwLock`
- Les tables balistiques et dispersions sont immutables apres chargement

## Extension

### Ajouter un nouveau type de munition

1. Ajouter la variante dans `AmmoKind`
2. Mettre a jour `AmmoKind::from_str()` et `as_str()`
3. Ajouter les fichiers CSV dans `data/`
4. Mettre a jour `load_ballistics_from()`
5. Ajouter les dispersions dans `metrics.json`

### Ajouter un nouveau type de cible

1. Ajouter la variante dans `TargetType`
2. Mettre a jour `TargetType::from_str()` et `as_str()`
3. Definir la munition suggeree dans `suggested_ammo()`
