# Documentation Mortar

## Sommaire

| Document | Description |
|----------|-------------|
| [Guide d'utilisation](guide.md) | Tutoriel complet pour utiliser l'application |
| [Reference API](api.md) | Documentation de l'API REST |
| [Architecture](architecture.md) | Architecture technique du projet |

## Documentation Rust (rustdoc)

Pour generer la documentation du code source :

```bash
# Generer et ouvrir dans le navigateur
cargo doc --open

# Generer avec les dependances privees
cargo doc --document-private-items

# Generer sans les dependances
cargo doc --no-deps
```

La documentation est generee dans `target/doc/mortar/index.html`.

## Structure de la documentation

```
doc/
├── README.md        # Ce fichier (index)
├── guide.md         # Guide utilisateur
├── api.md           # Reference API REST
└── architecture.md  # Architecture technique
```

## Liens utiles

- [README principal](../README.md) - Vue d'ensemble du projet
- [Cargo.toml](../Cargo.toml) - Configuration et dependances
- [lib.rs](../src/lib.rs) - Code source documente (rustdoc)
