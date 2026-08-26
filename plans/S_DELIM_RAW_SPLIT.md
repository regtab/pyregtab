# План: сырое разбиение S_delim (паритет с jRegTab 0.5.0)

**Статус:** РЕАЛИЗОВАН (2026-08-26; результаты и отклонения от плана — в §9)
**Дата:** 2026-08-26
**Upstream:** `d:\YandexDisk\code2\jregtab` @ v0.5.0 (`035ff1a`), коммит поведения
`adab03f` «Pass delimited tokens through verbatim (S_delim)»
**Характер:** ломающее изменение семантики исполнения; синтаксис, грамматика
и сериализатор не затрагиваются

---

## 1. Контекст

`S_delim = (δ, S_atom)` по формальной модели (`def:delimited-content-spec`)
декомпозирует текст ячейки на подстроки `sₖ ∈ Σ*` и применяет `S_atom` к каждой
**как есть**. pyRegTab нарушал это в одном месте — `split_with_spans` в
[src/matcher.rs](../src/matcher.rs): каждый токен прогонялся через `java_trim`,
а пустые молча выбрасывались. Делимитированная спецификация оказывалась
единственной, где текст не доходил до атома сырым — атомарные и compound-сегменты
уже передавали его дословно.

Практический мотив: в `regtab-eval-on-atbench` (ATBench из Auto-Tables, VLDB 2023)
эталоны страты *explode* порождены `pandas str.split(',')`, который сохраняет
пробелы токенов (`'a, b'` → `'a'`, `' b'`) и пустые строки. Принудительный trim делал
~41 из 48 кейсов страты невыразимыми.

Upstream закрыл это в jRegTab 0.5.0: обрезка стала opt-in через экстрактор атома
(`=TRIM` / `=NORM`), а поведение зафиксировано новой нормативной секцией
`conformance/semantic/` и пунктом 5 контракта корпуса.

## 2. Установленные факты (разведка перед реализацией)

- Обрезка жила ровно в одном хелпере; оба вызова — `process_delimited`
  (самостоятельная делимитированная ячейка) и `process_compound`, ветка
  `ContentSpec::Delimited` — шли через него. В jRegTab она была продублирована
  в двух местах.
- `java_trim` ([src/util.rs](../src/util.rs)) нужен для `Extractor::Trimmed`
  ([src/spec.rs](../src/spec.rs)) — остаётся; убирался только его вызов из
  `split_with_spans` и импорт в `matcher.rs`.
- Экстрактор применяется **после** split, в `process_atomic`. То есть `=TRIM` уже
  тогда «вставал» на каждый токен отдельно, а на делимитированном пути был
  no-op. Следствие: добавление `=TRIM` в паттерны 045/055 не меняет поведение
  ни до, ни после правки ядра — это безопасно делать первым шагом.
- `process_atomic` не отбрасывает пустую строку (единственный ранний выход —
  `Idd::Skip`), поэтому пустой токен корректно даёт item с `s == ""`.
- `CellItem.index` семантически наблюдаем: `FilterTerm::PosExact/PosOffset/PosRange`
  сравнивают его. До правки `process_delimited` отдавал «сырую» позицию из
  `enumerate` (с дырами на выброшенных токенах), а `process_compound` — плотный
  счётчик. После правки оба дают непрерывные `0..n-1`, расхождение между путями
  исчезает само.
- **Нюанс, которого нет в jRegTab:** `split_with_spans` возвращает байтовые спаны
  (provenance токена в исходном тексте ячейки). До правки спан указывал на
  обрезанный токен — отсюда вычисление `lead`. При переходе на сырые токены спан
  становится `(base + start, base + start + part.len())`, а `lead` уходит совсем.
  В Java спанов нет вообще — там правка сводилась к удалению двух строк.
- `CellItem.span` в Rust больше нигде не читается — только копируется в Python
  (`PyCellDerivedItem`). Его док-комментарий уже описывал пост-фиксовую семантику.
- `.gitattributes` уже содержал `conformance/** -text` — побайтовый синк безопасен.
- CI (`.github/workflows/ci.yml`) гоняет `cargo test --no-default-features` и
  `pytest tests -q` — новые тесты подхватываются сами, workflow править не нужно.

## 3. Ядро

Единственная правка поведения — `split_with_spans` в `src/matcher.rs`:

```rust
/// Parts of a literal split, verbatim, as (part position, part text, byte span
/// in the original cell text); `base` is the offset of `text` within that cell
/// text. Per `def:delimited-content-spec` parts are passed on untrimmed and empty
/// parts are kept, so `n` parts always yield indices `0..n-1`; whitespace removal
/// is opt-in via the atom's extractor (`=TRIM` / `=NORM`).
fn split_with_spans(delim: &str, text: &str, base: usize) -> Vec<(usize, String, (usize, usize))> {
    let mut out = Vec::new();
    let mut start = 0usize;
    for (i, part) in split_literal(delim, text).into_iter().enumerate() {
        let from = base + start;
        let to = from + part.len();
        start += part.len() + delim.len();
        out.push((i, part, (from, to)));
    }
    out
}
```

Плюс удаление `java_trim` из импорта `matcher.rs`. Тела `process_delimited`
и `process_compound` не менялись: первый уже использовал `i` (теперь непрерывный),
второй — свой плотный `item_index` и передаёт `base = pos`, поэтому спан
вложенного в compound токена остаётся в координатах всего текста ячейки.

## 4. Юнит-тесты ядра

`split_with_spans` и `process_delimited` приватны, поэтому тесты — инлайн-модулем
`#[cfg(test)] mod tests` в конце `src/matcher.rs` (первый такой в `src/`; файловый
`src/tests.rs` остаётся под корпус и e2e). Стиль как в `tests.rs`: `snake_case`
без префикса `test_`, голые `assert_eq!`, вспомогательные `part(...)` / `item(...)`
для читаемых ожиданий.

Прямые тесты `split_with_spans`: сохранение ведущего/хвостового пробела токена;
пустой токен внутри (`"a,,b"`) с нулевой шириной спана; краевые пустые токены
(`"a,b,"` и `",a"`); непрерывность индексов `0..n-1`; сдвиг спанов по ненулевому
`base` (случай вложения в compound) с обратной проверкой `cell[from..to] == s`;
байтовая точность на многобайтовом тексте.

Сквозные тесты через `compile` + `match_atp` на таблице 1×1, с проверкой
`(s, span, index)`: сырые items у делимитированной ячейки; item на каждый пустой
токен; `=TRIM` как opt-in обратно в обрезку (при этом спаны остаются сырыми — они
до-экстракторные, и пустые токены не воскрешаются); делимитированный сегмент
внутри compound.

**Проверка на дискриминирующесть.** После того как тесты стали зелёными, старое
поведение (`java_trim` + отбрасывание пустых) было временно возвращено —
упали **все 10** новых тестов, 4 прежних остались зелёными. Затем правка
восстановлена. Зелёный тест, зелёный при любом поведении, бесполезен.

## 5. Синк conformance-корпуса

`conformance/UPSTREAM` перепинен на `035ff1a139e885e4cea85aa66a33e89a6b30f8c9` /
`v0.5.0`; `conformance/` синкнут из jregtab **побайтово** (`git show <tag>:<path>`
в бинарном режиме, без какой-либо EOL-конверсии). `UPSTREAM` — файл, локальный
для pyRegTab, его нужно пересоздавать после синка.

Пришло из upstream (диффстат v0.4.1 → v0.5.0 по `conformance/` — ровно 20 файлов):

| Файл | Изменение |
|---|---|
| `README.md` | секция «Semantics of S_delim», пункт 5 контракта, «Semantic cases», layout |
| `VERSION` | `generated: 2026-08-26` |
| `positive/task_045.rtl` + `.expected.rtl` | `VAL` → `VAL=TRIM` в делимитированном атоме |
| `positive/task_055.rtl` + `.expected.rtl` | то же |
| `positive/delim_raw.rtl` + `.expected.rtl` | новый кейс: обе формы рядом |
| `semantic/{delim_raw_tokens,delim_empty_tokens,delim_trim,compound_delim_raw}/` | 12 новых файлов |

После синка: `positive/` = 304 файла (152 пары), `negative/` = 15, `semantic/` = 4 кейса.

**Контроль целостности:** все 333 файла посверены с upstream побайтово — 0 расхождений;
сырой CRLF в `task_099` (compound-делимитер `'\r\n'`) и табы в `task_101` целы;
среди остальных 300 файлов ни одного «whitespace-only» диффа.

## 6. Раннер пункта 5 контракта

`tests/test_semantic_conformance.py` — порт
`ru.icc.regtab.conformance.RtlSemanticConformanceTest`. Параметризация glob-ом по
каталогам `conformance/semantic/*`, два теста на кейс: «кейс полный» (есть
`pattern.rtl`, `input.csv`, `expected.csv`) и «исполнение даёт `expected.csv`».

Переиспользует из `tests/task_runner.py` готовые `load_table`, `load_recordset`,
`assert_matches`, константы `STRICT`/`FLEXIBLE`/`CONFORMANCE` — они уже портированы
с `CsvTableLoader`/`CsvRecordsetLoader`/`RecordsetAssert`. Конвейер кейса тот же,
что в `run_task_variant`: `AtpMatcher.match` → `TableInterpreter` со стратегией
`RECORD_FIRST` → `pattern.transform(...)`.

**Ловушка:** дефолты семантических кейсов НЕ совпадают с задачными.
Java-раннер задаёт `(STRICT, STRICT, expectedHasHeader=false)`, тогда как
`task_runner.load_match_options` даёт `expectedHasHeader=True`. Переиспользовать
его нельзя — в новом файле свой мини-лоадер `options.json` (три ключа, `None`/пустая
строка не переопределяют, порядковые ключи в upper-case, неизвестные ключи
игнорируются, неизвестное значение порядка → ошибка). При `expectedHasHeader=false`
эталон сопоставляется **позиционно** со схемой, которую построил паттерн, — так
автогенерируемые имена атрибутов реализации не попадают в контракт.

Ловушка подтвердилась на практике: до пересборки нативного ядра 3 из 4 кейсов
падали именно на сырых токенах — раннер дискриминирующий.

## 7. Паттерны, зависевшие от старого поведения

Делимитированную спецификацию используют только задачи 045, 055, 101; ведущие
пробелы есть в 045 и 055, задача 101 на табах не затронута.

- `tests/atp_patterns.py` — файл автогенерируемый (`tools/translate_atp.py`).
  Вместо ручной правки прогнан сам генератор против jregtab@v0.5.0: он дал ровно
  две изменённые строки (`pattern_045`, `pattern_055`, форма
  `.extract(StringExtractor.trimmed())`) и никакого другого дрейфа. Применён его
  вывод дословно. Этот прогон заодно служит проверкой, что корпус и upstream
  консистентны.
- `tests/test_dsl.py` — зеркальный DSL-кейс 045 синхронизирован по образцу
  upstream `DslSpikeTest.task045`: RTL-строка получает `VAL=TRIM`, DSL —
  `.extract(TRIM)` перед `.split_by(",")`.

`tests/test_rtl_tasks.py` править не потребовалось — он берёт RTL прямо из корпуса
(`task_runner.task_rtl`), поэтому починился синком. Эталоны
`tests/fixtures/tasks/**/expected_*.csv` НЕ трогались.

## 8. Документация

Зеркалит upstream-коммит `adab03f`:

- `docs/model/atp.md`, «Delimited content specification» — абзац о том, что
  `sₖ ∈ Σ*` передаётся в `S_atom` дословно, пустая подстрока даёт item с пустым
  значением, `n` подстрок → ровно `n` items, обрезка — работа экстрактора `ξ`;
  плюс уточнение формулировки фазы 1 разрешения содержимого.
- `docs/rtl-reference.md`, «### Delimited» — блок «Splitting is verbatim»
  (пробелы и пустые токены сохраняются, соответствие `pandas.Series.str.split`),
  рецепт `(VAL=TRIM){','}` / `(VAL=NORM){','}`, warning «Changed in 0.5.0»
  с миграционной инструкцией; пример Task 45 обновлён под новую форму паттерна.

`CHANGELOG.md` в проекте нет — не заводился.

## 9. Результат

```
cargo test --no-default-features    14 passed   (было 4)
pytest tests -q                   1918 passed   (было 1908; +8 semantic, +2 delim_raw)
tests/fixtures/                     не тронуты — 0 изменённых expected_*.csv
```

В диффе нет других поведенческих изменений, кроме `split_with_spans`.

**Отклонения от плана (оба несущественные):**

1. В ожидании одного юнит-теста (`=TRIM` на `"a, ,b"`) спаны были посчитаны
   неверно при написании плана — строка длиной 5, спаны `(2,3)` и `(4,5)`.
   Исправлено по факту прогона; поведение ядра корректно.
2. `atp_patterns.py` планировалось править вручную в стиле локального алиаса
   `TRIM = StringExtractor.trimmed()`; фактически применён дословный вывод
   генератора, который использует fluent-форму `.extract(StringExtractor.trimmed())`.

**Окружение:** в `.venv` отсутствовал `maturin` — без него pytest гонял старое
скомпилированное ядро. Установлен (`maturin 1.15.0`), пересборка
`python -m maturin develop --release`. CI ставит его сам (`pip install maturin pytest`).

## 10. Версия

Ломающее изменение поведения → следующий релиз минимум **0.5.0**. Бампу подлежат
`Cargo.toml`, `Cargo.lock`, `pyproject.toml` и `python/pyregtab/__init__.py`
(`__version__`). Учесть, что `docs/rtl-reference.md` уже содержит
`!!! warning "Changed in 0.5.0"`, поэтому бамп и релиз делаются вместе.

## 11. Что осознанно не трогалось

Грамматика `grammar/RTL.g4`, парсер, ATP→RTL сериализатор, `java_trim`,
`split_literal`, `Extractor`, эталоны `tests/fixtures/tasks/**/expected_*.csv`,
CI-workflow. Канонические формы изменились только у `task_045`/`task_055` —
из-за добавленного `=TRIM` в самих паттернах, и пришли готовыми из корпуса.
