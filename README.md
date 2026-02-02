# Mortar - Calculateur Balistique pour Mortier 60mm

Système de calcul de solutions de tir pour mortier 60mm avec interface CLI et web.

## Fonctionnalites

- **Calcul de solution de tir** : Distance, azimut, elevation
- **4 types de munitions** : Practice (M879), HE (M821), Smoke (M819), Flare (M853A1)
- **Systeme d'anneaux** : 5 niveaux de precision (0R-4R)
- **3 types de cibles** : Infanterie, Vehicule, Soutien
- **Dispersion ajustee** : Prise en compte du denivele (+5%/m si mortier plus haut, -1%/m si plus bas)
- **Correction de tir** : Ajustement base sur les deviations observees
- **Interface double** : CLI interactive + Web UI responsive

## Installation

### Depuis les sources

```bash
# Cloner le repository
git clone <repository-url>
cd mortar

# Compiler
cargo build --release

# Lancer le serveur (CLI + Web)
cargo run --release --bin server
```

### Avec Docker

```bash
# Build et run
docker-compose up --build

# Ou manuellement
docker build -t mortar-calculator .
docker run -p 3000:3000 mortar-calculator
```

## Utilisation

### Interface Web

Acceder a `http://localhost:3000` apres le lancement du serveur.

1. Ajouter un mortier avec ses coordonnees et type de munition
2. Ajouter une cible avec ses coordonnees et type
3. Selectionner mortier et cible
4. Cliquer sur "Calculer Solution"

### Interface CLI

Le serveur expose egalement une interface CLI interactive :

```
=== MORTAR CALCULATOR CLI ===

Commands:
  help, h                              Show this help
  list, ls                             List all mortars and targets
  add_mortar, am <n> <e> <x> <y> [ammo]  Add mortar
  add_target, at <n> <e> <x> <y> [type]  Add target
  rm_mortar, rmm <name>                Remove mortar
  rm_target, rmt <name>                Remove target
  set_ammo, sa <mortar> <ammo>         Set mortar ammo type
  set_type, st <target> <type>         Set target type
  calc, c <mortar> <target>            Calculate firing solution
  correct, cor <target> <V> <H>        Correct target position
  clear                                Clear screen
```

#### Exemple de session

```bash
> am M1 100 0 0 HE
Mortar 'M1' added with HE ammo

> at T1 50 500 300 INF
Target 'T1' added as INFANTERIE

> calc M1 T1
=== SOLUTION DE TIR: M1 -> T1 ===

  Distance:       583.1 m
  Azimut:         59.0 deg
  Diff Elevation: 50.0 m (signe: +50.0 m)

  Ogive mortier:  HE
  Type cible:     INFANTERIE
  Ogive suggeree: HE

  >>> ELEVATION HE <<<
  Elev: 0R:1150.2 1R:1128.5 2R:1106.8 3R:1085.1 4R:1063.4
  Disp: 0R:35.0m 1R:80.5m 2R:136.5m 3R:189.0m 4R:241.5m
```

## API REST

| Endpoint | Methode | Description |
|----------|---------|-------------|
| `/api/health` | GET | Health check |
| `/api/types` | GET | Liste des types disponibles |
| `/api/mortars` | GET/POST/DELETE | CRUD mortiers |
| `/api/mortars/ammo` | POST | Changer type de munition |
| `/api/targets` | GET/POST/DELETE | CRUD cibles |
| `/api/targets/type` | POST | Changer type de cible |
| `/api/targets/correct` | POST | Appliquer correction |
| `/api/calculate` | POST | Calculer solution de tir |

### Exemple avec curl

```bash
# Ajouter un mortier
curl -X POST http://localhost:3000/api/mortars \
  -H "Content-Type: application/json" \
  -d '{"name": "M1", "elevation": 100, "x": 0, "y": 0, "ammo_type": "HE"}'

# Ajouter une cible
curl -X POST http://localhost:3000/api/targets \
  -H "Content-Type: application/json" \
  -d '{"name": "T1", "elevation": 50, "x": 500, "y": 300, "target_type": "INFANTERIE"}'

# Calculer la solution
curl -X POST http://localhost:3000/api/calculate \
  -H "Content-Type: application/json" \
  -d '{"mortar_name": "M1", "target_name": "T1"}'
```

## Structure du projet

```
mortar/
├── Cargo.toml              # Manifest Rust
├── Dockerfile              # Image Docker multi-stage
├── docker-compose.yml      # Configuration Docker Compose
├── src/
│   ├── lib.rs              # Bibliotheque core (balistique, dispersion)
│   ├── main.rs             # CLI standalone
│   ├── bin/
│   │   ├── server.rs       # Serveur web Axum + CLI
│   │   ├── smooth_csv.rs   # Utilitaire lissage PCHIP
│   │   └── test_smooth.rs  # Tests visualisation
│   └── web/
│       ├── index.html      # Interface web
│       ├── style.css       # Styles (theme militaire)
│       └── app.js          # Logique frontend
├── data/
│   ├── metrics.json        # Donnees de dispersion
│   ├── PRACTICE/           # Tables M879 (0R-4R)
│   ├── HE/                 # Tables M821 (0R-4R)
│   ├── SMOKE/              # Tables M819 (1R-4R)
│   └── FLARE/              # Tables M853A1 (1R-4R)
└── doc/                    # Documentation
```

## Tables balistiques

Les tables sont au format CSV avec les colonnes :
- `range_m` : Portee en metres
- `elev_mil` : Elevation en milliemes

### Types de munitions

| Type | Designation | Anneaux | Usage |
|------|-------------|---------|-------|
| PRACTICE | M879 | 0R-4R | Entrainement |
| HE | M821 | 0R-4R | Explosif (fragmentation) |
| SMOKE | M819 | 1R-4R | Fumigene (ecran) |
| FLARE | M853A1 | 1R-4R | Eclairant |

## Calcul de dispersion

La dispersion est ajustee selon le denivele entre le mortier et la cible :

```
delta = elevation_mortier - elevation_cible

si delta >= 0 (mortier plus haut):
    dispersion = base * (1 + delta * 0.05)  // +5% par metre

si delta < 0 (mortier plus bas):
    dispersion = base * (1 + delta * 0.01)  // -1% par metre
```

**Exemple** : Mortier a 105m, cible a 100m, dispersion base 39m (HE 2R)
```
delta = 105 - 100 = 5m
dispersion = 39 * (1 + 5 * 0.05) = 39 * 1.25 = 48.75m
```

## Documentation

### Generation de la doc Rust

```bash
cargo doc --open
```

### Documentation additionnelle

Voir le dossier `doc/` pour :
- [Architecture](doc/architecture.md)
- [API Reference](doc/api.md)
- [Guide d'utilisation](doc/guide.md)

## Dependances

- **axum** : Framework web async
- **tokio** : Runtime async
- **serde** : Serialisation JSON
- **csv** : Parsing des tables balistiques
- **rustyline** : CLI interactive avec historique

## Licence

MIT
