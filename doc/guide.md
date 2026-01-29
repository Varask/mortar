# Guide d'utilisation

## Demarrage rapide

### 1. Lancer le serveur

```bash
# Depuis les sources
cargo run --release --bin server

# Ou avec Docker
docker-compose up
```

Le serveur demarre sur le port 3000 et affiche :
```
Loading ballistics from: data
Loaded 18 ballistic tables
Loaded 20 dispersion entries
Serving static files from: src/web
Server starting on http://0.0.0.0:3000

Type 'help' for CLI commands
>
```

### 2. Acceder aux interfaces

- **Web UI** : http://localhost:3000
- **CLI** : Directement dans le terminal du serveur

---

## Interface Web

### Panneau Mortiers

1. Remplir les champs :
   - **Nom** : Identifiant unique (ex: M1, Alpha)
   - **Elev** : Altitude en metres
   - **X** : Coordonnee Est-Ouest
   - **Y** : Coordonnee Nord-Sud
   - **Ogive** : Type de munition (HE, PRACTICE, SMOKE, FLARE)

2. Cliquer sur **Ajouter**

Le mortier apparait dans la liste. Vous pouvez :
- Cliquer dessus pour le selectionner
- Changer le type de munition via le menu deroulant
- Le supprimer avec le bouton X

### Panneau Cibles

Meme principe que les mortiers, avec le type de cible :
- **INFANTERIE** (INF) : Personnel ennemi
- **VEHICULE** (VEH) : Vehicules legers
- **SOUTIEN** (SOU) : Marquage, ecran fumigene

### Calcul de solution

1. Selectionner un mortier dans la liste ou le menu deroulant
2. Selectionner une cible
3. Cliquer sur **CALCULER SOLUTION**

### Lecture des resultats

#### Metriques
- **Distance** : Distance horizontale mortier-cible
- **Azimut** : Direction du tir (0° = Nord, 90° = Est)
- **Diff Elev** : Difference d'altitude absolue

#### Badges
- **Ogive mortier** : Type de munition charge
- **Type cible** : Classification tactique
- **Ogive suggeree** : Recommandation basee sur le type de cible

#### Cartes d'elevation

Affiche pour chaque anneau (0R a 4R) :
- **Elevation en mils** (valeur principale)
- **Dispersion ajustee** (±Xm)

Les valeurs sont pour le type de munition du mortier.

#### Tableau complet

Montre les elevations et dispersions pour TOUS les types de munitions.
La ligne correspondant a la munition du mortier est surlignee.

### Correction de tir

Apres un tir, si l'impact devie de la cible :

1. Observer la deviation :
   - **Verticale** : Nord (valeur negative) ou Sud (valeur positive)
   - **Horizontale** : Ouest (negative) ou Est (positive)

2. Entrer les valeurs dans la section "Correction de tir"

3. Cliquer sur **Appliquer correction**

Une nouvelle cible `NomOriginal_C` est creee avec les coordonnees corrigees.

**Exemple** : L'obus tombe 30m au Nord et 20m a l'Est
- Vertical : `-30`
- Horizontal : `20`

---

## Interface CLI

### Commandes disponibles

| Commande | Alias | Description |
|----------|-------|-------------|
| `help` | `h` | Afficher l'aide |
| `list` | `ls` | Lister mortiers et cibles |
| `add_mortar <n> <e> <x> <y> [ammo]` | `am` | Ajouter un mortier |
| `add_target <n> <e> <x> <y> [type]` | `at` | Ajouter une cible |
| `rm_mortar <name>` | `rmm` | Supprimer un mortier |
| `rm_target <name>` | `rmt` | Supprimer une cible |
| `set_ammo <mortar> <ammo>` | `sa` | Changer la munition |
| `set_type <target> <type>` | `st` | Changer le type de cible |
| `calc <mortar> <target>` | `c` | Calculer solution |
| `correct <target> <V> <H>` | `cor` | Corriger une cible |
| `clear` | - | Effacer l'ecran |
| `exit` | `q` | Quitter |

### Exemples

#### Session complete

```bash
# Ajouter un mortier
> am M1 100 0 0 HE
Mortar 'M1' added with HE ammo

# Ajouter une cible
> at T1 50 500 300 INF
Target 'T1' added as INFANTERIE

# Lister les positions
> ls

--- MORTIERS (1) ---
  M1 : X=0 Y=0 E=100m [HE]

--- CIBLES (1) ---
  T1 : X=500 Y=300 E=50m [INFANTERIE]

# Calculer la solution
> c M1 T1

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

# Corriger apres observation (30m Nord, 20m Est)
> cor T1 -30 20
Nouvelle cible corrigee: T1_C

  Original:  T1 -> X=500 Y=300
  Deviation: V=-30m (N-/S+) H=+20m (O-/E+)
  Corrige:   T1_C -> X=480 Y=330

# Recalculer avec la cible corrigee
> c M1 T1_C
```

#### Changer de munition

```bash
> sa M1 SMOKE
Mortar 'M1' ammo set to SMOKE

> c M1 T1
# La solution utilise maintenant les tables SMOKE
```

---

## Concepts cles

### Systeme de coordonnees

- **X** : Axe Est-Ouest (positif vers l'Est)
- **Y** : Axe Nord-Sud (positif vers le Nord)
- **Elevation** : Altitude au-dessus du niveau de reference

### Azimut

L'azimut est mesure en degres depuis le Nord, dans le sens horaire :
- 0° / 360° = Nord
- 90° = Est
- 180° = Sud
- 270° = Ouest

### Anneaux de precision

Les anneaux (0R a 4R) representent differents niveaux de puissance/portee.
- **0R** : Charge minimale, portee reduite, plus precis
- **4R** : Charge maximale, portee maximale, moins precis

Choisir l'anneau en fonction de :
1. La distance a la cible
2. La precision requise
3. Les conditions (vent, etc.)

### Types de munitions

| Type | Usage principal |
|------|-----------------|
| **HE** | Destruction, neutralisation |
| **PRACTICE** | Entrainement (inerte) |
| **SMOKE** | Ecran fumigene, marquage |
| **FLARE** | Eclairage nocturne |

### Dispersion

La dispersion indique le rayon probable d'impact (CEP).
Elle augmente avec :
- L'anneau (plus de puissance = moins precis)
- Le denivele positif (mortier plus haut que cible)

Formule d'ajustement :
```
Si mortier plus haut : dispersion × (1 + denivele × 5%)
Si mortier plus bas  : dispersion × (1 - denivele × 1%)
```

---

## Bonnes pratiques

1. **Verifier les coordonnees** avant de calculer
2. **Utiliser la correction** apres chaque tir observe
3. **Choisir l'anneau** en fonction de la distance ET de la precision requise
4. **Privilegier HE** pour les cibles dures, **SMOKE** pour l'ecran
5. **Sauvegarder les positions** importantes (elles sont perdues a l'arret du serveur)
