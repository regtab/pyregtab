# План: паритет с jRegTab 0.7.1 и замена jRegTab в regtab-eval-on-atbench

**Статус:** РЕАЛИЗОВАН (2026-09-08; результаты и отклонения от плана — в §7)
**Дата:** 2026-09-08
**Upstream:** `d:\YandexDisk\code2\jregtab` @ v0.7.1 (`f009333`); pyRegTab до этого плана —
паритет с jRegTab 0.5.1.
**Характер:** ломающее изменение семантики (`JOIN` → произведение записей, прежняя
свёртка — `CONCAT`), новый синтаксис (`CONCAT`, именованный ключ, `{0}`/`{1}`),
диагностика интерпретатора, оптимизация провайдеров без изменения результата,
исправление матчера. Версия 0.7.1 (номер синхронизирован с jRegTab).

---

## 1. Контекст

jRegTab с 0.5.1 до 0.7.1 получил шесть работ (`jregtab/plans/INDEX.md`), pyRegTab отстал по
всем. Потребитель — `regtab-eval-on-atbench`: 244 RTL-решения на ATBench, раннер
`tools/regtab-runner/RtlRunner.java` на `ru.icc.regtab:regtab:0.7.1`; три решения используют
новый `JOIN` (произведение), страта subtitle зависит от сабровов нулевой ширины, крупные
таблицы (до 1,19 млн ячеек) — от индекса провайдеров 0.5.2. Цель: pyRegTab исполняет все 244
решения с побайтно тем же `output.csv`, что и Java-раннер, и проходит новый conformance-корпус.

| jRegTab | план апстрима | что переносится |
|---|---|---|
| 0.5.2 | `PERF_LARGE_TABLES.md` | пространственный индекс над items, область кандидатов из спецификации фильтра, ранний выход по k |
| 0.5.3 | `PERF_MEMORY_LAYOUT.md` | Java-специфично (лямбды `IntRange`, `IdentityHashMap`); в Rust-ядре уже компактно — не переносится |
| 0.6.0 | `ITM_JOIN_PRODUCT.md` | `CONCAT(K)` = прежний `JOIN(K)` без dedup и с предусловиями; `JOIN(K)` = произведение записей; `rec` многозначная, множество `J`; `Diagnostic`, строгий режим |
| 0.7.0 | `RTL_NAMED_RECORD_KEY.md` | `RecordKey` (позиции и/или имена), `CONCAT(0,'A')`, канонический вид `CONCAT(0, 1, 'A', 'B')` |
| 0.7.0 | `DIAG_ANCHOR_WITHOUT_RECORD.md` | «anchor has no record — REC missing?», «none of the provided items has a record…», флаг `inherited` у действия, множество `C` |
| 0.7.1 | `ZERO_WIDTH_SUBROW.md` | сабров/субтаблица нулевой ширины — пустое совпадение; `{0}`/`{1}`; ошибка `{n}` как диагностика компиляции |
| 0.5.0 | `CONFORMANCE_SEMANTIC_SECTION.md` | раннер уже есть; синк 11 новых семантических кейсов, 2 negative, 2 curated positive |

## 2. Установленные факты

- Семантика провайдера в [src/semantics.rs](../src/semantics.rs) — полный перебор
  `sem.cell_items` + сортировка на каждый вызов: квадратично по числу ячеек, как в jRegTab 0.5.1.
  `SemanticsCore` неизменяем после построения, поэтому индекс строится один раз в `interpret`
  и передаётся в `provide`; области кандидатов (`CandidateScope`) выводятся из `FilterCond`
  один раз при инстанцировании провайдера (`matcher::to_provider_inst`).
- Порядок линеаризации: `(row, col)` по τ, внутри ячейки — `index` по возрастанию (в том
  числе для обратных обходов) — как `ItemLinearization.comparator`. Стабильная сортировка
  сохраняет порядок создания при равенстве.
- `WorkingState.rec: IndexMap<usize, Vec<ItemId>>` — одна запись на якорь; `apply_join`
  делает свёртку с dedup (0.5.x).
- Матчер: `while (stack.len() < max) && i < n` и `cells[next - 1]` — те же два дефекта, что
  в Java 0.7.0 (`ZERO_WIDTH_SUBROW.md`). `Quantifier::exactly` требует `n >= 2`.
- `tests/atp_patterns.py` генерируется `tools/translate_atp.py` из Java-тестов апстрима;
  12 задач с `JOIN` переехали на `CONCAT`, `task_098` — на ключ `{0,1,2,3}`.
- Conformance: `diff -r` показывает 12 изменённых пар positive (`JOIN`→`CONCAT`), 2 новые
  пары, 2 negative, 11 semantic-кейсов, README, VERSION. Фикстуры задач апстрима не менялись.
- Java есть локально (`JAVA_HOME=C:\Java\jdk-25.0.2`), jar-раннер atbench собран — можно
  сравнить выходы напрямую на всех 244 решениях.

## 3. Ядро (Rust)

1. `spec.rs`: `OperationType::Concat`; `RecordKey { positions, names }` с валидацией
   (позиции ≥ 0, имена непустые); `ActionSpec.key: RecordKey` вместо `key_positions`;
   проверки `ActionSpec::new` для `Concat` как для `Join`; `Quantifier::exactly(n)` при `n ≥ 0`.
2. `rtl/`: ключевое слово `CONCAT`; `keyRef : INT | STRING` для `CONCAT(...)`/`JOIN(...)`;
   пустое имя — ошибка компиляции с позицией; `{n}` вне диапазона `int` — ошибка с позицией;
   сериализатор — позиции по возрастанию, затем имена лексикографически в одинарных кавычках.
3. `semantics.rs`: `rec: IndexMap<usize, Vec<Vec<ItemId>>>`, множества `J` и `C`,
   `Diagnostic`, `report` (строгий режим — `Err`), `apply_concat` (предусловия (i)–(iii),
   no effect + диагностика), `apply_join` (произведение, `compat_K`, `agree`, `dedup`,
   отложенное потребление), `live_anchors()`; проверки согласованности по живым якорям и
   каждой записи. `ItemIndex` + `CandidateScope` — порт `CellDerivedItemIndex`,
   `CandidateScope`, `CandidateScopes`.
4. `interp.rs`: пять корзин `str → avp → rec → concat → join`; `check_records` для явных
   действий; `InterpreterCfg.strict_preconditions`; `interpret` возвращает и диагностику;
   схема и записи — по тройкам (якорь, запись, позиция) над живыми якорями.
5. `matcher.rs`: снятие `i < n`, пустая итерация (`next == i`) → `min = 0`, break; пустые
   сабровы/субтаблицы не регистрируются в ITM; `ActionInst.inherited`; `scope` в
   `ProviderInst::Cell`.

## 4. Python-слой

- `RecordKey`, `Diagnostic`, `OperationType.CONCAT`; `ActionSpec.concat(...)`,
  `ActionSpec.join(...)` с `key=` (int | str | RecordKey | iterable), `key_positions=` сохранён;
  `ActionSpec.key` / `.key_positions`; `TableInterpreter.with_strict_preconditions(bool)`,
  `.diagnostics()`; `Quantifier.exactly(0|1)`.
- `pyregtab.dsl`: `concat(...)`, `join(...)` с ведущим ключом (int / str / set / RecordKey).
- CLI-раннер `python -m pyregtab.runner <pattern.rtl>` — контракт `RtlRunner.java` из
  regtab-eval-on-atbench: `./input.csv` → `./output.csv`, заголовок всегда, RFC 4180 с
  многострочными ячейками, неровные строки дополняются, `None` → `""`, коды 0/1/2;
  диагностики интерпретатора — в stderr.

## 5. Тесты

- Rust: порт `WorkingStateConcatTest`/`WorkingStateJoinTest`, рандомизированная
  эквивалентность нового `provide` полному перебору (аналог
  `CellDerivedItemProviderEquivalenceTest`), сабровы нулевой ширины.
- Python: порт `TableInterpreterMultiRecordTest` (`tests/test_join_concat.py`), кванторы
  `{0}`/`{1}`, DSL `concat`; `tests/atp_patterns.py` регенерируется переводчиком;
  `test_dsl.py` — `JOIN(0)` → `CONCAT(0)`; task-раннер и раннер semantic-секции требуют
  пустых `diagnostics()` (как в апстриме).
- Внешняя проверка: все 244 решения regtab-eval-on-atbench через Java-раннер и через
  `python -m pyregtab.runner`, побайтное сравнение `output.csv`; время на крупных входах.

## 6. Документация и версия

`docs/rtl-reference.md`, `docs/model/itm.md`, `docs/model/atp.md`, `docs/api.md`,
`docs/architecture.md`, `docs/embedded-rtl.md`, `docs/index.md`, `README.md` — порт правок
апстрима в Python-терминах; `grammar/RTL.g4` + `grammar/UPSTREAM`, `conformance/` +
`conformance/UPSTREAM` — v0.7.1; версия 0.7.1 в `Cargo.toml`, `pyproject.toml`,
`python/pyregtab/__init__.py`; CI: differential-job на v0.7.1.

## 7. Результат

```
cargo test --no-default-features     45 passed  (было 21; +10 WorkingState concat, +8 join,
                                                +1 рандомизированная эквивалентность
                                                индексированного provide полному перебору
                                                (> 10 000 вызовов на 25 случайных таблицах),
                                                +5 сабровы нулевой ширины)
cargo test (с фичей python)          45 passed
cargo clippy --all-targets -D warnings — чисто в обеих конфигурациях
pytest tests -q                    1981 passed  (было 1925: +11 semantic-кейсов ×2,
                                                +2 negative, +2 positive ×2, +28 test_join_concat.py)
mkdocs build --strict                 чисто
tools/check_grammar_sync.py           OK (пин v0.7.1, sha256 81387334…)
diff -r -x UPSTREAM ../jregtab/conformance conformance — расхождений нет
```

**Внешняя проверка (regtab-eval-on-atbench, 244 решения с данными):** Java-раннер на
`ru.icc.regtab:regtab:0.7.1` и `python -m pyregtab.runner` дают **побайтно одинаковый
`output.csv` во всех 244 кейсах** (скрипт `atbench_diff.py` в scratchpad сессии; итог
`Counter({'same': 244})`). Суммарное время 244 прогонов: Java 71 с (включая старт JVM),
pyRegTab 46 с. Самый крупный вход `stack_test11` (84 888 × 14 = 1,19 млн ячеек, identity-паттерн):
Java 7,0 с, pyRegTab 7,5 с (парсинг CSV 1,0 с, матчинг 0,7 с, интерпретация 5,2 с, запись 0,6 с);
до индекса провайдеров этот кейс в pyRegTab был квадратичным (как в jRegTab 0.5.1).

**Отклонения от плана и решения по ходу:**

1. `RecordKey` в Python: фабрики `positions(…)`/`names(…)` из Java конфликтуют с одноимёнными
   свойствами, поэтому оставлены свойства `positions`/`names`, конструктор
   `RecordKey(positions=(), names=())`, `RecordKey.of(positions=None, names=None)` и `empty()`.
   `ActionSpec.concat/join(*providers, key=…)` принимает int, str, `RecordKey` или итерируемое
   из позиций и имён; `key_positions=` и одноимённый getter сохранены для кода под 0.5.x.
2. `ActionSpec.key_positions` (Rust) переименован в `key: RecordKey`; `ActionSpec::key_positions()`
   оставлен как аксессор.
3. Пустые сабровы/субтаблицы в `SyntaxMatch` не регистрируются вовсе (в Java они остаются в
   `MatchResult` как пустые интервалы, но в ITM не попадают); наблюдаемое поведение то же.
4. Целочисленный литерал вне диапазона `int` отклоняется лексером с позицией
   («Invalid integer literal 99999999999: out of int range»), а не только в кванторе.
5. Строгий режим — `RuntimeError` (Java: `IllegalStateException`) с текстом
   `<OP> skipped at CellDerivedItem[…]: <message>`.
6. Дополнительно к плану: `Recordset.to_csv(..., quote_all=False, newline="
")` — формат
   раннера пишется ядром (запись 1,19 млн записей 4,5 с → 0,6 с); `WorkingState` на `FxHash`
   (`rustc-hash`), проверки согласованности и генерация записей без аллокаций на запись
   (интерпретация stack_test11 7,0 с → 5,2 с).
7. `PERF_MEMORY_LAYOUT` (0.5.3) не переносился: Rust-ядро и так хранит ячейки/items плоскими
   структурами.
8. Regtab-eval-on-atbench не менялся: замена `java -jar regtab-runner.jar <rtl>` на
   `python -m pyregtab.runner <rtl>` в `harness/run.py` — решение того проекта.
