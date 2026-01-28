# Reference API REST

Base URL: `http://localhost:3000`

## Endpoints

### Health Check

```
GET /api/health
```

**Response**
```json
{
    "status": "ok",
    "version": "0.1.0"
}
```

---

### Types disponibles

```
GET /api/types
```

**Response**
```json
{
    "ammo_types": ["PRACTICE", "HE", "SMOKE", "FLARE"],
    "target_types": ["INFANTERIE", "VEHICULE", "SOUTIEN"]
}
```

---

## Mortiers

### Lister les mortiers

```
GET /api/mortars
```

**Response**
```json
{
    "positions": [
        {
            "name": "M1",
            "elevation": 100.0,
            "x": 0.0,
            "y": 0.0,
            "ammo_type": "He"
        }
    ]
}
```

### Ajouter un mortier

```
POST /api/mortars
Content-Type: application/json
```

**Request Body**
```json
{
    "name": "M1",
    "elevation": 100.0,
    "x": 0.0,
    "y": 0.0,
    "ammo_type": "HE"    // optionnel, defaut: "HE"
}
```

**Response (201)**
```json
{
    "success": true,
    "message": "Mortar 'M1' added with HE"
}
```

**Errors**
- `400` - Name cannot be empty
- `409` - Mortar already exists

### Supprimer un mortier

```
DELETE /api/mortars
Content-Type: application/json
```

**Request Body**
```json
{
    "name": "M1"
}
```

**Response**
```json
{
    "success": true,
    "message": "Mortar 'M1' deleted"
}
```

**Errors**
- `404` - Mortar not found

### Changer le type de munition

```
POST /api/mortars/ammo
Content-Type: application/json
```

**Request Body**
```json
{
    "name": "M1",
    "ammo_type": "SMOKE"
}
```

**Response**
```json
{
    "success": true,
    "message": "Mortar 'M1' ammo set to SMOKE"
}
```

**Errors**
- `400` - Invalid ammo type
- `404` - Mortar not found

---

## Cibles

### Lister les cibles

```
GET /api/targets
```

**Response**
```json
{
    "positions": [
        {
            "name": "T1",
            "elevation": 50.0,
            "x": 500.0,
            "y": 300.0,
            "target_type": "Infanterie"
        }
    ]
}
```

### Ajouter une cible

```
POST /api/targets
Content-Type: application/json
```

**Request Body**
```json
{
    "name": "T1",
    "elevation": 50.0,
    "x": 500.0,
    "y": 300.0,
    "target_type": "INFANTERIE"    // optionnel, defaut: "INFANTERIE"
}
```

**Types de cible valides**: `INFANTERIE`, `INF`, `VEHICULE`, `VEH`, `SOUTIEN`, `SOU`

**Response (201)**
```json
{
    "success": true,
    "message": "Target 'T1' added as INFANTERIE"
}
```

### Supprimer une cible

```
DELETE /api/targets
Content-Type: application/json
```

**Request Body**
```json
{
    "name": "T1"
}
```

### Changer le type de cible

```
POST /api/targets/type
Content-Type: application/json
```

**Request Body**
```json
{
    "name": "T1",
    "target_type": "VEHICULE"
}
```

---

## Calcul

### Calculer une solution de tir

```
POST /api/calculate
Content-Type: application/json
```

**Request Body**
```json
{
    "mortar_name": "M1",
    "target_name": "T1"
}
```

**Response**
```json
{
    "distance_m": 583.095,
    "azimuth_deg": 59.036,
    "elevation_diff_m": 50.0,
    "signed_elevation_diff_m": 50.0,
    "mortar_ammo": "HE",
    "target_type": "INFANTERIE",
    "recommended_ammo": "HE",
    "solutions": {
        "PRACTICE": {
            "0R": 1152.3,
            "1R": 1130.5,
            "2R": 1108.7,
            "3R": 1086.9,
            "4R": 1065.1
        },
        "HE": {
            "0R": 1150.2,
            "1R": 1128.5,
            "2R": 1106.8,
            "3R": null,      // hors portee
            "4R": null
        },
        "SMOKE": { ... },
        "FLARE": { ... }
    },
    "dispersions": {
        "PRACTICE": {
            "0R": 35.0,
            "1R": 80.5,
            "2R": 136.5,
            "3R": 189.0,
            "4R": 241.5
        },
        "HE": { ... },
        "SMOKE": { ... },
        "FLARE": { ... }
    },
    "selected_solution": {
        "ammo_type": "HE",
        "elevations": {
            "0R": 1150.2,
            "1R": 1128.5,
            "2R": 1106.8,
            "3R": null,
            "4R": null
        },
        "dispersions": {
            "0R": 35.0,
            "1R": 80.5,
            "2R": 136.5,
            "3R": 189.0,
            "4R": 241.5
        }
    }
}
```

**Errors**
- `404` - Mortar or target not found

---

## Correction

### Appliquer une correction de tir

```
POST /api/targets/correct
Content-Type: application/json
```

**Request Body**
```json
{
    "target_name": "T1",
    "vertical_m": -50.0,    // Nord(-) / Sud(+)
    "horizontal_m": 30.0    // Ouest(-) / Est(+)
}
```

Cette requete cree une nouvelle cible `T1_C` avec les coordonnees corrigees.

**Response**
```json
{
    "success": true,
    "original": "T1",
    "corrected": "T1_C",
    "correction_applied": {
        "vertical_m": -50.0,
        "horizontal_m": 30.0,
        "new_x": 470.0,
        "new_y": 350.0
    }
}
```

**Explication de la correction**

L'obus est tombe a `(vertical_m, horizontal_m)` de la cible.
- `vertical_m = -50` : L'obus est tombe 50m au **Nord** de la cible
- `horizontal_m = 30` : L'obus est tombe 30m a l'**Est** de la cible

La correction inverse la deviation :
- `new_x = old_x - horizontal_m = 500 - 30 = 470` (decale vers l'Ouest)
- `new_y = old_y - vertical_m = 300 - (-50) = 350` (decale vers le Sud)

---

## Codes d'erreur

| Code | Description |
|------|-------------|
| 200 | Succes |
| 201 | Cree |
| 400 | Requete invalide |
| 404 | Ressource non trouvee |
| 409 | Conflit (doublon) |
| 500 | Erreur serveur |

## Format des erreurs

```json
{
    "error": "Message d'erreur"
}
```
