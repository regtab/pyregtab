# План: производительность на крупных таблицах (0.7.2)

**Статус:** В РАБОТЕ (2026-09-08)
**Дата:** 2026-09-08
**Исходные данные:** эксперимент `D:\YandexDisk\code2\comp-on-large-tables` (README, `results/`):
10 самых больших входов ATBench (98 тыс. – 1,19 млн ячеек), одни и те же RTL-решения
для jRegTab 0.7.1 и pyRegTab 0.7.1, pandas для сравнения; медианы 5 запусков, пик RSS.
**Характер:** только производительность и память. Семантика RTL, паритет с jRegTab 0.7.1,
публичный API (только дополняется) и контракт CLI-раннера не меняются. Версия 0.7.2.

---

## 1. Контекст и узкие места (из эксперимента)

pyRegTab быстрее jRegTab по wall на 9 кейсах из 10 (0,5–0,8×), но проигрывает на самом
большом (`stack_test11`, 1,19 млн ячеек, 1,19 млн записей: 6,13 с против 5,12 с) и
отстаёт всё сильнее при масштабировании (1,2× → 1,7× при k = 4). По фазам:

1. **`read`** (CSV → `TableSyntax`): посимвольный цикл на Python + `cell(r, c).set_text()`
   на каждую ячейку — 1,10 с против 0,17 с у Java на `stack_test11`; 25–45 % времени
   на кейсах < 200 тыс. ячеек. Приоритет 1.
2. **`transform`** (интерпретация + recordset): 3,83 с против 3,36 с на 1,19 млн записей,
   рост ×2,1–2,2 на удвоение против ×1,8–2,1 у Java. Приоритет 2.
3. **`write`** (`Recordset.to_csv`): вся CSV собирается в одну `String` — на 40 МБ выхода
   выбросы 2,6 с вместо 0,16 с (2 из 6 запусков), на 160 МБ — 5,4 с. Приоритет 3.
4. **Память**: ≈ 1,4 ГБ на миллион ячеек (`CellData` с полным форматированием на каждую
   ячейку + 3–4 копии каждой строки текста по слоям). Приоритет 2.
5. `match` быстрее Java в 1,6–4× — не трогать; `import`/`compile` пренебрежимы — не ухудшать.

## 2. Проверки (перед каждым коммитом)

- `pytest tests -q` (1 981 теста в 0.7.1 + новые), `cargo test`, `cargo test --no-default-features`,
  `cargo clippy`, `ruff check python tools tests` (2 ошибки в `_core.pyi` — были до работы, не новые).
- `python tools/differential.py <java_dump.jsonl>` — 750 вариантов задач против дампа
  jRegTab 0.7.1 (дамп собран до изменений: `tools/RecordsetDumpMain.java` +
  `CsvTableLoader.java` из тестов jregtab, classpath = shaded `regtab-runner.jar` + jackson
  2.18.2 + commons-csv 1.11.0 + commons-io 2.18.0 + commons-codec 1.17.1).
- `python tools/atbench_bytecmp.py` (новый) — побайтная сверка `output.csv` и кодов возврата
  раннера pyRegTab с Java-раннером на всех 244 решениях regtab-eval-on-atbench; выходы Java
  кэшируются в `<atbench>/.bytecmp-java`. До изменений: 244/244 равны.
- Замер: `bench.py` из comp-on-large-tables (от сети, свободная память); базовые результаты
  0.7.1 сохранены в `results-baseline-0.7.1/`.

## 3. Шаги

### 3.1. Загрузка таблицы одним вызовом — СДЕЛАНО

- `src/csv.rs`: `parse_csv(text)` — байтовый сканер, дословный порт `RtlRunner.parseCsv`
  (все управляющие символы ASCII, поэтому границы полей на байтах совпадают с посимвольными
  циклами Java/Python); `universal_newlines(text)` — перевод `\r\n`/`\r` → `\n`, как
  `Path.read_text` (прежний `load_table` читал файл именно так; для файла поведение
  сохранено в точности, включая CRLF внутри поля в кавычках).
- `SyntaxCore::from_rows(rows, num_cols)` — построение таблицы за один проход с
  дополнением рваных строк пустыми ячейками.
- Python: `TableSyntax.from_rows(rows, *, num_cols=None)`, `TableSyntax.from_csv(path)`
  (файл, UTF-8, universal newlines), `TableSyntax.from_csv_text(text)` (текст как есть),
  `pyregtab._core.parse_csv(text)`. `pyregtab.runner.parse_csv`/`load_table` сохранены как
  публичные функции; `load_table` → `TableSyntax.from_csv`, `parse_csv` → нативная.
- Тесты: `tests/test_runner.py` (нативный парсер против прежнего Python-парсера на 22
  граничных входах, `from_rows`/`from_csv`, контракт раннера, коды возврата).
- Результат (`stack_test11`, 1,19 млн ячеек): `read` 1,10 с → 0,11 с (Java 0,17 с);
  wall 6,13 с → 5,21 с. Проверки: pytest 2012 passed, cargo test 49+49, differential
  750/750, bytecmp 244/244.

### 3.2. `transform`: рабочее состояние без хеш-карт и копий строк — СДЕЛАНО

Профиль (`examples/perf_runner.rs` — пофазный раннер над чистым Rust-ядром,
`interp::interpret_timed` — время четырёх фаз интерпретации) на `stack_test11` до изменений:
match 0,59 с, инициализация 0,15 с, завершение рабочего состояния 1,39 с, извлечение
recordset 1,56 с; итого 3,21 с в Rust против 3,83 с в фазе `transform` Python — разница
0,6 с уходила в биндинги.

Что было найдено и сделано:
- `TableInterpreter.interpret` (py.rs) **клонировал весь `SyntaxCore` и `SemanticsCore`**,
  чтобы отпустить GIL; `TablePattern.transform` клонировал весь recordset. Теперь
  интерпретация заимствует таблицу (ссылка `Send`), `PyRecordset` хранит `Arc<RecordsetCore>`,
  и `transform` без трансформаций разделяет recordset вместо копии.
- `WorkingState`: `val`/`attr`/`avp` были `IndexMap<ItemId, String>` (≈10 хеш-поисков и
  4–5 аллокаций на запись), `rec` — `IndexMap<usize, Vec<Vec<ItemId>>>`, `J`/`C` —
  `FxHashSet`. Теперь `ItemMap<V>` (плотные векторы по индексу элемента, отдельно cell/ctx),
  `AnchorMap<V>` (плотные слоты + список порядка вставки — порядок записей сохранён),
  `AnchorSet` (битовый вектор). Имена атрибутов интернированы (`u32`), `avp = (id, Text)`;
  сравнение пар в `CONCAT`/`JOIN` — по id и содержимому, что эквивалентно сравнению строк.
- Строки: `Text = Arc<str>` (`util.rs`) для `CellItem.s`, `CtxItem`, значений рабочего
  состояния и `RecordCore.values` — значение разделяется между элементом, состоянием и
  записью вместо 3–4 копий. Публичные Python-типы не изменились (`Record` возвращает `str`).
- `provide_into` — провайдеры пишут в один буфер на всю интерпретацию; `construct_schema`
  и `generate_records` работают по id атрибутов (без `to_string()` на каждую тройку и без
  хеширования строки схемы на каждую запись).
- Результат (`stack_test11`): `transform` 3,83 с → 0,73 с (jRegTab 3,36 с); в Rust:
  инициализация 0,03 с, завершение 0,44 с, извлечение 0,14 с; wall 6,13 с → 2,03 с;
  пик RSS 1656 → 1238 МБ. `explode_test22`: `transform` 0,49 → 0,12 с, wall 1,00 → 0,39 с.
  Проверки: pytest 2012, cargo test 49+49, clippy 0, differential 750/750, bytecmp 244/244.

### 3.3. Потоковый `to_csv` — СДЕЛАНО

- `RecordsetCore::write_csv(w, sep, missing, quote_all, newline)` — запись записей по одной
  в любой `Write`, удвоение кавычек без промежуточной строки; `to_csv_string` — то же в
  строку. `Recordset.to_csv(path)` пишет через `BufWriter<File>` (1 МБ) с отпущенным GIL;
  `to_csv()` без пути по-прежнему возвращает `str`. Формат байт-в-байт прежний
  (bytecmp 244/244, тест `test_to_csv_file_equals_string`).
- Результат: `write` на `stack_test11` 0,17 с → 0,05 с (jRegTab 0,22 с), без выбросов в
  6 запусках подряд (max 1,94 с wall при медиане 1,92 с); при k = 4 0,70 с → 0,17 с
  (jRegTab 0,91 с). Пик RSS не изменился — он достигается во время интерпретации.

### 3.4. Память — ПЛАН

## 4. Результат

(заполняется по завершении)
