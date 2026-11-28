# ATSI v0

Adaptador de protocolo para `taikyoku-shogi.github.io/play`. Es una capa delgada
sobre el mismo `Board` y `Searcher` que usa USI: no cambia movegen, búsqueda ni
evaluación. Implementación: `src/atsi.rs`, binario `taikyokushogi-atsi`.

## Uso

```sh
cargo run --release --bin taikyokushogi-atsi
# o, para la web:
websocketd --port=8080 ./target/release/taikyokushogi-atsi
```

`TAIKYOKU_ATSI_DEPTH` fija la profundidad (por defecto 3).

## Comandos

| Comando | Respuesta |
|---|---|
| `atsiinit v0` | `atsiok` |
| `identify` | `info "nombre" "descripción" "autor" "versión"` |
| `time <segundos> [incremento]` | — (presupuesto: `incremento + segundos/900`) |
| `player <0\|1>` | — |
| `startgame initial\|<tsfen>` | `move ...` si el motor juega primero |
| `opmove <origen> <destino> [tiempo] [optiempo] [paso]` | aplica jugada rival y responde `move ...` |
| `win` / `loss` / `draw` | `log Game over: ...` |
| `tsfen` | `log TSFEN: <tsfen>` |
| `setparam ...` | sin efecto (reservado) |
| `quit` | termina la sesión |

Tras cada `move`, el motor emite `eval <cp>` (material desde la perspectiva
blanca). El movimiento de león se expresa con una tercera posición.

## Notación de casillas

Igual que el motor original: archivo `36 - columna` (1..36), rango `a`..`z` y
después letras dobladas `aa`..`jj`. Ejemplo: `19aa`.

## Features añadidos desde tomatene-v1

- `src/eval/positional.rs`: bonos de centralidad y proximidad a las familias
  reales, plegados en la tabla PSQT (coste incremental cero). **Desactivados
  por defecto**: `TAIKYOKU_POSITIONAL=1` los activa. Así la evalución y los
  tests de fuerza existentes no se alteran.
- `src/reverse_attack.rs`: índice inverso de ataques (auxiliar, no toca movegen).
- `src/piece_features.rs`: features derivados de los descriptores de movimiento
  (`taikyokushogi-cli piece-features` exporta JSON).
