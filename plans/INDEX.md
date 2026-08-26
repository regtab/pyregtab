# Планы

Указатель по каталогу `plans/`: один план — одна крупная работа, самый свежий сверху.
Статус и результат каждой работы — в шапке и в разделе «Результат» самого плана.

- [ANCH_MOVES_ATTRIBUTE.md](ANCH_MOVES_ATTRIBUTE.md) — `ANCH(n)`/`REC(n)` переставляет
  сам атрибут (имя вместе со значениями), а не только значения; синк двух семантических
  conformance-кейсов и паритет с jRegTab 0.5.1. **РЕАЛИЗОВАН** (2026-08-26).
- [S_DELIM_RAW_SPLIT.md](S_DELIM_RAW_SPLIT.md) — сырое разбиение делимитированной
  спецификации: токены передаются в атом дословно, обрезка стала opt-in через `=TRIM`/`=NORM`;
  паритет с jRegTab 0.5.0. **РЕАЛИЗОВАН** (2026-08-26).
- [EMBEDDED_RTL_DSL.md](EMBEDDED_RTL_DSL.md) — встроенный DSL `pyregtab.dsl`: fluent-фабрики
  паттернов вместо многословного ATP API, порт `ru.icc.regtab.dsl.Rtl`.
- [PYREGTAB_MIGRATION_PLAN.md](PYREGTAB_MIGRATION_PLAN.md) — исходная миграция
  jRegTab → pyRegTab (вариант A: нативное ядро на Rust + Python-обёртка).
  **РЕАЛИЗОВАН** (сверка 2026-07-10).
