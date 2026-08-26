# План: ANCH(n)/REC(n) переставляет атрибут, а не только значения (паритет с jRegTab 0.5.1)

**Статус:** РЕАЛИЗОВАН (2026-08-26; результаты и отклонения от плана — в §9)
**Дата:** 2026-08-26
**Upstream:** `d:\YandexDisk\code2\jregtab` @ v0.5.1 (`c126337`), коммит поведения
`a092102` (merge `43c1fa9`) «Fix ANCH(n)/REC(n): move the anchor attribute, not just
its values»
**Характер:** исправление семантики пост-трансформации; грамматика, парсер, matcher
и сериализатор не затрагиваются

---

## 1. Контекст

`apply_anchor_at_position` в [src/spec.rs](../src/spec.rs) строит вектор перестановки
`reordered`, применяет его к значениям каждой записи и возвращает **исходную схему**:
`Ok(RecordsetCore { schema: rs.schema, records })`. Для анонимных атрибутов (`$a_i`,
имя = позиция) это незаметно, но для именованных (полученных через `AVP`) связка
имя ↔ значение разъезжается: столбец с именем якоря получает чужие значения. Паттерн

    <ANCH(4)>
    [[ATTR]+]
    [COL->AVP [VAL]{4}[(VAL: ROW*->REC){','}][VAL]]+

на шапке `Dato,Tid,Eksamen,Fagkode,Lokaler,Klasse` даёт схему «Lokaler,Dato,Tid,…»
со сдвинутыми значениями вместо «Dato,Tid,…,Lokaler,Klasse».

Целевая семантика (принята в апстриме): `ANCH(n)` перемещает **сам атрибут** — имя
вместе со значениями — на 0-based позицию `n`; связка имя ↔ значение в каждой записи
неизменна, меняется только порядок схемы. Правило одно для именованных и анонимных:
анонимные имена **не** перенумеровываются, они переезжают вместе со своим атрибутом
(`$a_1..$a_4` при `ANCH(2)` → `$a_2, $a_3, $a_1, $a_4`). Последовательность значений
по позициям при этом не меняется — все header-less эталоны задач остаются зелёными.

Эталон: `src/main/java/ru/icc/regtab/interpret/AnchorAttributeAtPosition.java`,
тесты `AnchorAttributeAtPositionTest` и `RtlAnchorPositionFormsTest` в jregtab.

## 2. Установленные факты (разведка перед реализацией)

- Образец стиля в проекте — `apply_schema_reordering` ([src/spec.rs](../src/spec.rs)):
  собирает `new_attrs`, `Schema::new(new_attrs)?`, переставляет значения по индексам.
  Исправленная `apply_anchor_at_position` — его частный случай.
- `Schema::new` ([src/recordset.rs](../src/recordset.rs)) отвергает дубликаты;
  перестановка уникальной схемы остаётся уникальной, но `?` оставляем для единообразия.
- Python-обёртка `PyAnchorAttributeAtPosition` ([src/py.rs](../src/py.rs)) уже отвергает
  отрицательную позицию и вызывает то же ядро — правок не требует.
- Все три формы уже сходятся к `Transformation::AnchorAttributeAtPosition`:
  - префикс `<ANCH(n)>` → [src/rtl/build.rs](../src/rtl/build.rs) и слияние
    в [src/rtl/mod.rs](../src/rtl/mod.rs) (там же проверка конфликта `ANCH`/`REC`);
  - inline `REC(n)`, RTL-путь → `ast::collect_rec_params`
    ([src/rtl/ast.rs](../src/rtl/ast.rs)), который явно спускается в delimited-спецификации
    (`CompSegAst::Delim`, `XSpecAst::Delim`, `ContAst::Delim` → `walk_atom(&d.atom, …)`);
  - inline `REC(n)`, ATP/Python-путь `TablePattern::of` → `extract_inline_transformations`
    → `actions_of`, где `ContentSpec::Delimited(d) => d.atom.actions`.

  Вывод: `[(VAL: ROW*->REC(n)){','}]` виден компилятору по обоим путям, правок кода
  здесь не нужно — поведение фиксируется тестом.
- В `src/spec.rs` тестового модуля нет. Прецедент для приватной функции — инлайновый
  `#[cfg(test)] mod tests` в конце файла (как в `src/matcher.rs`), `snake_case` без
  префикса `test_`, голые `assert_eq!`.
- `conformance/` — байт-в-байт зеркало апстрима, помечено `-text` в `.gitattributes`.
  `git diff 035ff1a c126337 -- conformance/` в jregtab даёт ровно 8 файлов из двух новых
  каталогов, так что синк исчерпывающий. Все файлы: UTF-8 без BOM, LF, завершающий
  перевод строки.
- `tests/test_semantic_conformance.py` находит кейсы листингом каталогов, регистрация
  не нужна; по умолчанию `expectedHasHeader: False`, оба новых кейса ставят `true` —
  именно это делает их способными поймать баг.
- `CHANGELOG.md` в проекте нет — не заводился.
- Сборка/прогон: `.venv\Scripts\python.exe`, `python -m maturin develop --release`
  обязателен перед pytest (иначе гоняется старое ядро); `cargo test --no-default-features`
  (фича `python` включена по умолчанию и тянет pyo3-линковку).

## 3. Ядро

`src/spec.rs`, `apply_anchor_at_position`: границы, проверка `position < 0` и построение
`reordered` — без изменений. После `reordered` добавляется перестановка схемы:

```rust
let new_attrs: Vec<String> = reordered.iter().map(|&i| attrs[i].clone()).collect();
let schema = Schema::new(new_attrs)?;
let records = /* как было */;
Ok(RecordsetCore { schema, records })
```

Doc-комментарий функции переписывается по образцу javadoc эталона: перемещается атрибут,
имя едет со значениями, анонимные имена не перенумеровываются, граничные случаи
(позиция 0, ≥ len, схема из одного атрибута) возвращают recordset как есть.

## 4. Юнит-тесты ядра (Rust)

Новый `#[cfg(test)] mod tests` в конце `src/spec.rs`; помощник строит `RecordsetCore`
из списка атрибутов и строк значений (поля `pub`, конструкторов нет). Кейсы — порт
`AnchorAttributeAtPositionTest`:

1. `named_attributes_keep_their_values` — `Lokaler,Dato,Tid` + `ANCH(2)` → схема
   `Dato,Tid,Lokaler`, значения `20.05.2019, 08.30-11.30, AU`.
2. `named_attributes_at_position_one` — `ANCH(1)` → `Dato,Lokaler,Tid`.
3. `anonymous_attributes_are_not_renumbered` — `$a_1..$a_4` + `ANCH(2)` →
   `$a_2,$a_3,$a_1,$a_4`, при этом `rs.get(0, "$a_1") == Some("anchor")`.
4. `anonymous_value_order_matches_legacy_behaviour` — **регресс-гарантия**: позиционно
   `v2, v3, anchor, v4` (как до правки). Тест обязан быть зелёным и на старом коде.
5. `mixed_schema_keeps_every_name` — `Lokaler,$a_2,Klasse` + `ANCH(1)` →
   `$a_2,Lokaler,Klasse`.
6. `degenerate_cases_return_the_input` — позиции 0, `len`, `> len`, схема из одного
   атрибута: результат равен входу (в Rust — `assert_eq!` с клоном, `assertSame`
   неприменим).
7. `negative_position_is_rejected` — специфика Rust-сигнатуры `i64`: `Err`.

## 5. Тесты уровня RTL (Python)

Новый `tests/test_rtl_anchor_forms.py` (порт `RtlAnchorPositionFormsTest`) с локальным
помощником `make_table` (как в `tests/test_api.py`). Таблица `Dato,Lokaler,Klasse`
(шапка + 2 строки, якорь — средний столбец):

- `test_settings_prefix_moves_the_anchor_attribute` — префикс `<ANCH(1)>` над
  `[[ATTR]+] [COL->AVP [VAL] [VAL: ROW*->REC] [VAL]]+` даёт схему
  `["Dato","Lokaler","Klasse"]` и значения `20.05,AU,0` / `11.06,A2.1,1` под своими именами.
- `test_all_three_forms_agree` — префикс `<ANCH(1)>`, inline `[VAL: ROW*->REC(1)]`
  и inline `[(VAL: ROW*->REC(1)){','}]` дают идентичный дамп (схема + значения записей).
- `test_inline_rec_inside_delimited_specification` — ячейка `"AU, C1.1"` даёт 3 записи;
  вторая — `20.05`, `" C1.1"`, `0`, с **ведущим пробелом** (токены сырые, без trim —
  правило `S_delim` из 0.5.0).

Пайплайн как в `tests/task_runner.py`: `AtpMatcher.match` → `TableInterpreter()
.with_strategy(SchemaConstructionStrategy.RECORD_FIRST).interpret(itm)` →
`pattern.transform(rs)`.

Плюс в `tests/test_api.py::test_transformations` добавляется `AnchorAttributeAtPosition`
(единственная трансформация без покрытия на уровне Python-объектов): перестановка схемы
и сохранение связки имя ↔ значение.

## 6. Синк conformance-корпуса

Байт-в-байт из локального jregtab (Git Bash `cp -r`, без EOL-конверсии):

    conformance/semantic/anch_named_attrs/        {pattern.rtl, input.csv, expected.csv, options.json}
    conformance/semantic/anch_named_inline_delim/ {pattern.rtl, input.csv, expected.csv, options.json}

Проверка после копирования: `diff -r ../jregtab/conformance conformance` — расхождений
быть не должно; `git ls-files --eol conformance/semantic/anch_*` — всюду `w/lf`.

`conformance/UPSTREAM`: пин `035ff1a` / `v0.5.0` →
`c12633763b309fd00f65d8b236a4ab91795303b4` / `v0.5.1`. `conformance/VERSION`
(`generated: 2026-08-26`) не меняется.

## 7. Документация

- `docs/rtl-reference.md`, таблица «Settings prefix»: строка `ANCH(n)` —
  «Use position *n* in the first record as the attribute name for all records» →
  «Move the anchor attribute to 0-based position *n* in the schema». Ниже примера —
  два абзаца (формулировки из jregtab): перемещается атрибут, имя едет со значениями,
  граничные случаи; анонимные имена не перенумеровываются (`$a_1,$a_2,$a_3` →
  `$a_2,$a_3,$a_1`).
- Там же во врезке «Inline equivalents»: позиция inline-формы в паттерне не важна,
  `REC(n)` подхватывается где угодно, включая delimited-спецификацию
  `[(VAL: ROW*->REC(1)){','}]`, и всегда даёт ту же трансформацию.
- `docs/rtl-reference.md`, таблица операций: строка `REC(n)` — «Same + use attribute at
  position *n* as the record's attribute name» → «Same + move the anchor attribute to
  position *n*».
- `docs/model/atp.md` — «adds `AnchorAttributeAtPosition` post-step» → «… — moves the
  anchor attribute (name with its values) to position *n*».
- `docs/model/itm.md` — в описании `ActionSpec.rec(int anchorPos, …)` и в строке таблицы
  трансформаций: «moves the anchor attribute, name and values together, preserving every
  attribute-value binding».
- `docs/api.md`, «Recordset transformations» — уточнение про `AnchorAttributeAtPosition(pos)`
  (перемещает атрибут, не переименовывает).

## 8. Версия

0.5.0 → 0.5.1 в `Cargo.toml`, `Cargo.lock`, `pyproject.toml`,
`python/pyregtab/__init__.py`, `README.md` («pyRegTab 0.5.1 ≙ jRegTab 0.5.1»),
`docs/index.md`. Публикация на PyPI в этот PR не входит.

## 9. Результат

```
cargo test --no-default-features    21 passed   (было 14; +7 юнит-тестов §4)
pytest tests -q                   1925 passed   (было 1918; +3 RTL-теста §5,
                                                 +4 от двух conformance-кейсов)
tests/fixtures/                     не тронуты — 0 изменённых expected_*.csv
diff -r -x UPSTREAM ../jregtab/conformance conformance   — расхождений нет
```

Исходный репродьюсер из отчёта (`<ANCH(4)>` на шапке
`Dato,Tid,Eksamen,Fagkode,Lokaler,Klasse`, ячейка `"AU, C1.1"`) даёт схему
`Dato,Tid,Eksamen,Fagkode,Lokaler,Klasse` и три записи с правильной связкой
имя ↔ значение, включая `" C1.1"` с ведущим пробелом.

**Дискриминирующая проверка (§10):** при временно возвращённом `schema: rs.schema`
падают 4 юнит-теста ядра (`named_attributes_keep_their_values`,
`named_attributes_at_position_one`, `anonymous_attributes_are_not_renumbered`,
`mixed_schema_keeps_every_name`) и 5 тестов Python (`test_transformations`,
`test_settings_prefix_moves_the_anchor_attribute`,
`test_inline_rec_inside_delimited_specification` и оба conformance-кейса) — 9 падений
против 8 в jregtab, лишнее приходится на добавленную проверку в `test_transformations`.
`anonymous_value_order_matches_legacy_behaviour`, `degenerate_cases_return_the_input`
и `negative_position_is_rejected` при этом зелёные, как и задумано.
`test_all_three_forms_agree` на сломанном ядре тоже проходит: он фиксирует
эквивалентность трёх форм, а не корректность самой перестановки.

**Отклонения от плана (несущественные):**

1. В §8 дополнительно обновлены `README.md` (число тестов 1 908 → 1 925) и `Cargo.lock`
   (перегенерирован `cargo check`).
2. Строка `README.md` про differential-тестирование («zero mismatches against jRegTab
   v0.5.0») оставлена как есть: прогон против v0.5.1 в эту работу не входил.

## 10. Проверка качества тестов

Дискриминирующая проверка (как в §4 плана S_DELIM): временно вернуть `schema: rs.schema`
и убедиться, что падают новые юнит-тесты §4 (кроме №4 и №7), RTL-тесты §5 и оба
conformance-кейса, а `anonymous_value_order_matches_legacy_behaviour` — проходит.
В jregtab так и вышло: 8 падений, регресс-тест зелёный.

## 11. Что осознанно не трогается

- `src/py.rs`, грамматика, лексер, парсер, `build.rs`, matcher, сериализатор — поведение
  трёх форм уже единое, меняется только пост-трансформация.
- Эталоны `tests/fixtures/` (задачи с ANCH/REC(n)) — header-less, сравниваются позиционно.
- Асимметрия `extract_inline_transformations` (ATP-путь) и `collect_rec_params` (RTL-путь)
  по унаследованным action-спекам на уровнях table/subtable/row/subrow — существующее
  расхождение, к этому багу отношения не имеет.
- `conformance/VERSION`, differential-тесты против Java, публикация на PyPI.
