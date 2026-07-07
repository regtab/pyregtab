# План миграции jRegTab → pyRegTab

**Статус:** ЧЕРНОВИК v2 (ANTLR4-требование ослаблено; ядро — Rust, альтернатива — C++)
**Дата:** 2026-07-06 (v2 — того же дня; v1 с обязательным ANTLR4 заменена целиком)
**Исходный проект:** `d:\YandexDisk\code2\jregtab` (Java 21, v0.1.2-SNAPSHOT)
**Целевой проект:** pyRegTab — Python-пакет с нативным ядром, публикуемый на PyPI

---

## 1. Цель и требования

Перенести библиотеку jRegTab в экосистему Python в виде пакета **pyRegTab**:

1. **Нативное ядро** (Rust — рекомендация, C++ — альтернатива; сравнение в §3) для
   эффективного сопоставления паттернов и интерпретации таблиц.
2. **`RTL.g4` — нормативная спецификация языка RTL.** Генерация парсера из грамматики
   не требуется; парсер ядра — рукописный. Соответствие грамматике проверяется
   **общим conformance-корпусом** (§5.3), прогоняемым в CI обоих проектов
   (jregtab и pyregtab).
3. **Python API, повторяющее текущее API jRegTab** — те же классы, фабрики и пайплайн
   `TableSyntax → RtlCompiler/TablePattern → AtpMatcher → TableInterpreter → Recordset`.
4. Установка одной командой `pip install pyregtab` (бинарные wheels для Windows,
   Linux, macOS; без требования JDK/компилятора у пользователя).
5. Прохождение всего эталонного набора: **150 задач (Foofah, RegTab, Baikal),
   1 500 тестовых вариантов** — фикстуры переиспользуются напрямую.

> Изменение против v1: требование «обязательно ANTLR4» ослаблено до п. 2. Это устраняет
> главный аргумент против Rust (у ANTLR4 нет живого Rust-runtime) и убирает самую
> капризную зависимость сборки (ANTLR C++ runtime). Разбор RTL — одноразовая операция
> на паттерн и не является горячим путём; рукописный рекурсивный спуск при этом ещё и
> быстрее ANTLR и даёт лучшие сообщения об ошибках.

---

## 2. Анализ исходной кодовой базы

### 2.1 Объём и состав

- **~5 400 строк** Java в **83 файлах** — компактная, полностью переносимая вручную база.
- Единственная runtime-зависимость — `antlr4-runtime 4.13.2` (в pyRegTab не переносится).
- Грамматика `RTL.g4` — **207 строк**; простой LL-язык без неоднозначностей.
- Самые крупные файлы (алгоритмическое ядро): `ATPBuilder` (392), `SyntaxMatcher` (309),
  `ProviderTemplateResolver` (281), `FilterTerm` (271), `AtpToRtlSerializer` (267),
  `WorkingState` (253), `SemanticConstructor` (224), `TableInterpreter` (216).

### 2.2 Публичная API-поверхность (то, что должно повториться в Python)

| Компонент | Классы |
|---|---|
| Вход | `RtlCompiler`, `AtpMatcher`, `TableInterpreter`, `AtpToRtlSerializer` |
| ATP spec | `TablePattern`, `SubtablePattern`, `RowPattern`, `SubrowPattern`, `CellPattern`, `AtomicContentSpec`, `DelimitedContentSpec`, `CompoundContentSpec`, `ConditionalContentSpec`, `ActionSpec`, `ProviderSpec`, `ItemFilterConditionSpec`, `Quantifier`, `CellMatchCondition`, `CellPredicate`, `StringExtractor`, `ItemDerivationDirective` |
| ITM syntax | `TableSyntax`, `Cell`, `Row`, `Subrow`, `Subtable`, `BoundingBox`, `GridPosition`, `CellColor`, enum'ы выравнивания/шрифтов |
| ITM semantics | `InterpretableTable`, `TableSemantics`, items, providers, operations (в основном внутренние, но `ContextDerivedItem` — публичный параметр `AtpMatcher.match`) |
| Interpret | `SchemaConstructionStrategy`, `ActionApplicationStrategy`, `MissingValueHandler`, `RecordsetTransformation` (+ `WhitespaceNormalization`, `FieldSplitting`, `SchemaReordering`, `AnchorAttributeAtPosition`) |
| Результат | `Recordset`, `Record`, `Schema` |

### 2.3 Особенности Java-кода, влияющие на перенос

| Особенность | Где | Последствие для порта на Rust |
|---|---|---|
| `Optional<InterpretableTable>` | `AtpMatcher.match` | `Option<…>` в ядре → `InterpretableTable \| None` в Python |
| Sealed-интерфейсы (`ContentSpec`, `FilterTerm`, `ItemFilterConditionSpec`) | atp.spec | Идеально ложатся на Rust-enum'ы с данными |
| Java-лямбды: `CellPredicate.Custom`, `ItemFilterConditionSpec.Custom` | atp.spec | Python-callable, вызываемый из ядра (§4.4) |
| Java-регулярки (`java.util.regex.Pattern`) | `FilterTerm`, `CellPredicate`, `StringExtractorFactory`, RTL-ограничения `"regex"` | **Риск совместимости**: аудит фикстур → crate `regex` либо `fancy-regex` (§4.3) |
| Records / иммутабельность spec-классов | повсеместно | Иммутабельные структуры, `Arc<…>` для разделения |
| ANTLR4 visitor (`RTLBaseVisitor` → `ATPBuilder`) | rtl.internal | Заменяется рукописным парсером; логика `ATPBuilder` + `ProviderTemplateResolver` портируется как построитель ATP из AST (§5) |
| Строки UTF-16 (Java) | `StringExtractor`, split, индексы | В Rust — UTF-8; все смещения по code points или литеральным разделителям; тесты с кириллицей |

### 2.4 Профиль производительности

Горячий путь — `AtpMatcher.match` (перебор разбиений с квантификаторами) и
`TableInterpreter.interpret` (4 фазы, провайдеры перебирают items по фильтрам).
Компиляция RTL — одноразовая на паттерн, не критична по скорости (рукописный парсер
при этом заведомо быстрее ANTLR). Типичный сценарий, ради которого нужно нативное ядро:
**пакетная обработка тысяч таблиц одним паттерном** — важно освобождать GIL в
`match`/`interpret`.

---

## 3. Выбор нативного ядра: Rust vs C++

После ослабления ANTLR4-требования оба языка полноценны: парсер в обоих случаях
рукописный, по производительности горячего пути они эквивалентны (оба через LLVM,
без GC; разница — в пределах шума). Выбор определяется инженерными критериями.

### Вариант A — Rust + PyO3 + maturin ★ РЕКОМЕНДУЕТСЯ

- **Парсер RTL**: рукописный лексер (при желании — crate `logos`, генерирующий очень
  быстрые лексеры) + рекурсивный спуск. Sealed-иерархии ATP из Java выражаются
  Rust-enum'ами один в один.
- **Биндинги**: PyO3 — зрелые, с поддержкой Python-callable, GIL-management,
  `abi3`-wheels (одна сборка на все CPython ≥ 3.10).
- **Сборка/дистрибуция**: **maturin** — лучший в отрасли конвейер для нативных
  Python-пакетов: `maturin build` даёт wheels без CMake и без ручной возни с toolchain;
  готовый GitHub Action `PyO3/maturin-action` собирает матрицу win/linux/macos ×
  x86-64/arm64, включая кросс-компиляцию aarch64.
- **Плюсы**: безопасность памяти (нет класса ошибок висячих ссылок/UAF — существенно
  для объектного графа Cell↔Row↔Subtable, разделяемого с Python); enum'ы + `match`
  делают порт sealed-типов и `FilterTerm` механическим; cargo — единый инструмент
  зависимостей/тестов/бенчмарков; crate `regex` — один из самых быстрых движков.
- **Минусы**: borrow checker потребует явной модели владения (решается `Arc`/арена,
  §4.2); если аудит покажет lookaround в регулярках фикстур — придётся брать
  `fancy-regex` (медленнее, но совместимее); команда должна владеть Rust.

### Вариант B — C++17/20 + nanobind + scikit-build-core

- Тот же рукописный рекурсивный спуск; биндинги nanobind; сборка CMake + cibuildwheel.
- **Плюсы**: nanobind чуть дешевле PyO3 на вызовах через границу (несущественно при
  гранулярности «вызов на таблицу»); возможно, ближе командам с C++-опытом.
- **Минусы**: ручное управление памятью при объектном графе, разделяемом с Python
  (нужны shared_ptr-holder'ы, keep_alive, ASan в CI); CMake-инфраструктура тяжелее
  maturin; sealed-иерархии выражаются многословнее (std::variant или виртуальные
  иерархии); отдельные инструменты для тестов/бенчмарков/санитайзеров.
- После отказа от ANTLR C++ runtime единственный весомый аргумент за C++ —
  предпочтения команды. Технических преимуществ перед Rust не остаётся.

### Отвергнутые варианты (кратко, обоснование в v1 плана)

- **Java-ядро через JPype/Py4J** — требует JVM у пользователя; остаётся только как
  мост для дифференциального тестирования (§7.3).
- **GraalVM native-image** — экспорт богатого объектного API через плоский C-интерфейс
  и тяжёлая сборка на 3 ОС.
- **Чистый Python** — не выполняет требование нативной эффективности; возможен позже
  как fallback-реализация для экзотических платформ (§10, вопрос 5).

### Сравнительная таблица

| Критерий | A: Rust | B: C++ |
|---|---|---|
| Производительность matcher/interpreter | ✅✅ | ✅✅ (паритет) |
| Скорость и качество парсера RTL | ✅✅ рукописный (+logos) | ✅✅ рукописный |
| Упаковка wheels | ✅✅ maturin, abi3 | ✅ cibuildwheel + CMake |
| Безопасность памяти при разделяемом объектном графе | ✅✅ гарантии компилятора | ⚠️ shared_ptr + ASan |
| Порт sealed-типов / `FilterTerm` | ✅✅ enum + match | ⚠️ variant/иерархии |
| Python-callbacks (Custom-предикаты) | ✅ PyO3 | ✅ nanobind |
| Регулярки, совместимые с Java | ⚠️ `regex`/`fancy-regex` (§4.3) | ⚠️ RE2/PCRE2 |
| Единая инфраструктура (deps/test/bench) | ✅✅ cargo | ⚠️ CMake + внешние |
| Риск сопровождения | низкий | средний |

**Рекомендация: вариант A (Rust + PyO3 + maturin).** Точка выхода в фазе 0: если
прототип выявит блокер (например, непреодолимую несовместимость регулярок), откат на B —
архитектура §4 переносится на C++ без структурных изменений.

---

## 4. Целевая архитектура (Rust)

### 4.1 Слои

```
pyregtab (репозиторий)
├── python/pyregtab/           — чистый Python: публичное API-зеркало jRegTab
│   ├── __init__.py            — реэкспорт: RtlCompiler, AtpMatcher, TableInterpreter, …
│   ├── syntax.py              — TableSyntax, Cell, … (тонкие обёртки над _core)
│   ├── spec.py                — TablePattern, …, Quantifier (обёртки/фабрики)
│   ├── interpret.py           — TableInterpreter, стратегии, трансформации
│   ├── recordset.py           — Recordset, Record, Schema (+ to_pandas(), to_csv())
│   └── rtl.py                 — RtlCompiler.compile, AtpToRtlSerializer, RtlCompileError
├── src/                       — Rust-ядро (cdylib → модуль pyregtab._core)
│   ├── itm/                   — syntax + semantics (порт ru.icc.regtab.itm)
│   ├── atp/                   — spec (enum'ы) + matcher (порт ru.icc.regtab.atp)
│   ├── interpret/             — интерпретатор (порт ru.icc.regtab.interpret)
│   ├── rtl/                   — лексер (logos) + рекурсивный спуск + ATP-построитель
│   │                            (порт логики ATPBuilder + ProviderTemplateResolver)
│   │                            + сериализатор ATP→RTL
│   └── bindings/              — #[pyclass]/#[pymethods] (PyO3)
├── grammar/RTL.g4             — нормативная спецификация (копия из jregtab,
│                                CI-проверка синхронности по hash)
├── conformance/               — общий conformance-корпус RTL (§5.3)
├── tests/                     — pytest; фикстуры задач 001–150 из jregtab
├── Cargo.toml, pyproject.toml (maturin)
└── .github/workflows/         — maturin-action, публикация на PyPI
```

Принцип: **вся логика — в Rust**, Python-слой не содержит алгоритмов, только
идиоматичную обёртку (докстринги, типы `typing`, `__repr__`, интеграцию с pandas).

### 4.2 Владение объектами

- Модель Java (граф с обратными ссылками Cell → Row/Subtable) в Rust заменяется
  **аренной**: `TableSyntax` владеет плоскими `Vec<Cell>`, `Vec<Row>`, …; связи —
  индексы, а не ссылки. Это идиоматично, снимает вопросы borrow checker'а и быстрее
  (локальность данных).
- Через границу PyO3 наружу отдаются лёгкие handle-объекты (`CellRef { table: Py<TableSyntax>, idx }`),
  так что Python-ссылка на ячейку держит таблицу живой — семантика Java-API сохраняется.
- Все spec-классы (ATP) — иммутабельные, разделяемые `Arc<…>`; фабрики `of(...)`
  возвращают готовые значения, как в Java.
- `InterpretableTable` = syntax + semantics; создаётся matcher'ом, отдаётся в Python
  как владелец.

### 4.3 Регулярные выражения — отдельное решение

RTL-ограничения `"regex"` сегодня исполняются `java.util.regex`.

**План**: в фазе 0 провести аудит всех регулярок в 150 фикстурах и RTL/ATP-тестах;
- если lookaround/backreferences **не используются** → crate **`regex`** (линейное
  время, один из самых быстрых движков вообще);
- если используются → **`fancy-regex`** (тот же API, поддержка lookaround, медленнее
  на этих конструкциях).
Зафиксировать выбранный диалект в документации pyRegTab как контракт; расхождения
с Java-семантикой (например, `\d` и Unicode) — задокументировать и покрыть тестами.

### 4.4 Python-callbacks (Custom-предикаты и Bindings)

`CellPredicate.Custom` и `ItemFilterConditionSpec.Custom` в Java принимают лямбды.
В pyRegTab они принимают Python-callable.

Кроме того, jRegTab (с 2026-07-06, ветка `feature/rtl-ext-bindings`) поддерживает
именованные привязки `EXT('name')` в RTL: `RtlCompiler.compile(rtl, Bindings)`,
sealed-варианты `CellPredicate.External` / `FilterTerm.External`, сериализуемые
обратно в `EXT('name')`. pyRegTab обязан зеркалировать: `compile(rtl, bindings)`
с Python-callable (`bindings.cell(name, fn)` / `bindings.filter(name, fn)`),
поддержка `EXT` в рукописном парсере (фаза 5) и в сериализаторе:

- В ядре предикат — enum-вариант `Custom(PyObject)`; вызов через PyO3 с захватом GIL.
- Документировать: паттерны с Custom-предикатами не сериализуются в RTL (как и в Java)
  и **блокируют освобождение GIL** в `match` — ожидаемое ограничение медленного пути.
- Все встроенные предикаты (Blank, regex, теги, spatial) — чисто нативные.

### 4.5 GIL и пакетная обработка

- `AtpMatcher.match` и `TableInterpreter.interpret` без Custom-предикатов выполняются
  под `py.allow_threads(...)` → свободная многопоточность из Python
  (`ThreadPoolExecutor` по таблицам).
- Дополнительно (пост-1.0): `AtpMatcher.match_many(pattern, tables)` с внутренним
  пулом (rayon).

### 4.6 Python API — соответствие Java API

API повторяется 1:1 по классам и семантике; имена методов — **snake_case** (PEP 8),
с сохранением всех фабрик:

```python
from pyregtab import TableSyntax, RtlCompiler, AtpMatcher, TableInterpreter

syntax = TableSyntax(3, 3)
syntax.cell(0, 1).set_text("CA");  syntax.cell(0, 2).set_text("HU")
syntax.cell(1, 0).set_text("IKT"); syntax.cell(1, 1).set_text("0 Jan"); syntax.cell(1, 2).set_text("8 Feb")
syntax.cell(2, 0).set_text("SVO"); syntax.cell(2, 1).set_text("31 Jan"); syntax.cell(2, 2).set_text("40 Feb")

pattern = RtlCompiler.compile("""
    [ [] [VAL : 'AIRLINE'->AVP]+ ]
    [ [VAL : 'AIRPORT'->AVP]
      [VAL : (COL, ROW, CL)->REC, 'ND'->AVP " " VAL : 'MON'->AVP]+ ]+
""")

itm = AtpMatcher.match(pattern, syntax)          # InterpretableTable | None (вместо Optional)
rs = TableInterpreter().interpret(itm)            # Recordset
rs.schema.attributes                              # ['ND', 'AIRLINE', 'AIRPORT', 'MON']
rs[0]["ND"]                                       # '0'
df = rs.to_pandas()                               # бонус-интеграция (extras: pyregtab[pandas])
```

Таблица соответствий (фрагмент, полная — в docs pyRegTab):

| Java | Python |
|---|---|
| `RtlCompiler.compile(String)` → `TablePattern` | `RtlCompiler.compile(str)` → `TablePattern` (модульная функция `pyregtab.compile()` — алиас) |
| `AtpMatcher.match(p, s)` → `Optional<InterpretableTable>` | `AtpMatcher.match(p, s)` → `InterpretableTable \| None` |
| `TablePattern.of(sub1, sub2)` | `TablePattern.of(sub1, sub2)` (varargs → `*args`) |
| `Quantifier.oneOrMore()` | `Quantifier.one_or_more()` |
| `AtomicContentSpec.val(ActionSpec...)` | `AtomicContentSpec.val(*actions)` |
| `new TableInterpreter().withStrategy(s).interpret(itm)` | `TableInterpreter().with_strategy(s).interpret(itm)` (fluent сохраняется) |
| `rs.records().get(0).get("Name")` | `rs[0]["Name"]`; также `rs.records`, `record.get("Name")` |
| `cell.text()` / `cell.setText(t)` | property `cell.text` (getter/setter) — питонично; либо `set_text()` для симметрии с Java (решить в фазе 0) |
| `RtlCompileException` | `RtlCompileError(Exception)` |

Открытый вопрос именования (решить до фазы 2): properties (`cell.text = "x"`) vs
Java-стиль (`cell.set_text("x")`). Рекомендация — properties для геттеров/сеттеров
ячеек, фабрики и fluent-методы — как в Java (snake_case).

---

## 5. RTL: рукописный парсер и нормативная грамматика

### 5.1 Парсер

1. **Лексер**: crate `logos` (или рукописный) — токены RTL (скобки, квантификаторы,
   `->`, литералы, regex-строки, идентификаторы шаблонов провайдеров, `#теги`, …).
2. **Парсер**: рекурсивный спуск, структурно следующий правилам `RTL.g4`
   (каждому правилу грамматики — функция парсера с комментарием-ссылкой на правило).
   Грамматика LL-простая, 207 строк — ожидаемый объём парсера ~1–1.5 тыс. строк Rust,
   сопоставимо с портируемым в любом случае `ATPBuilder` (392) + `ProviderTemplateResolver` (281).
3. **Построение ATP**: парсер строит AST, затем построитель (порт логики `ATPBuilder`)
   и резолвер шаблонов провайдеров (порт `ProviderTemplateResolver`) дают `TablePattern` —
   включая все расширения ветки: ST, STR, TAG-OR, bare `&`-конъюнкции, bare condContSpec,
   автоматический вывод `CellDerivedProviderKind`, именованные фрагменты `$name=[…]`.
4. **Ошибки**: `RtlCompileError` с позицией line:col, ожидаемыми токенами и фрагментом
   строки — качество диагностики должно быть **не хуже** ANTLR-версии (у рукописного
   парсера это обычно проще сделать лучше).
5. **Сериализатор** `AtpToRtlSerializer` портируется в ядро; round-trip тест обязателен.

### 5.2 Статус RTL.g4

- `RTL.g4` остаётся **единственной нормативной спецификацией** языка RTL; живёт
  в jregtab, в pyregtab — копия с CI-проверкой синхронности (job сравнивает hash
  с upstream-версией, зафиксированной в файле `grammar/UPSTREAM`).
- Любое изменение языка проходит цикл: правка `RTL.g4` в jregtab → расширение
  conformance-корпуса → реализация в обоих парсерах (ANTLR-генерация в jregtab,
  рукописная в pyregtab) → зелёный корпус в обоих CI.

### 5.3 Общий conformance-корпус

Страховка от расхождения двух реализаций языка:

1. **Позитивные кейсы**: все ~750 RTL-строк из тестов задач 001–150 + RTL-примеры
   из docs. Формат: `conformance/positive/NNN.rtl` + эталонная каноническая форма
   `NNN.expected.rtl` (результат `serialize(compile(rtl))` — канонизация через
   round-trip). Обе реализации обязаны: (а) успешно компилировать, (б) давать
   идентичную каноническую форму.
2. **Негативные кейсы**: `conformance/negative/*.rtl` — синтаксически/семантически
   ошибочные строки (собрать при портировании: каждая ветка ошибки парсера получает
   кейс); обе реализации обязаны отвергать (позиция ошибки — рекомендуется, но не
   нормируется).
3. Корпус хранится в отдельной директории, пригодной для выноса в общий репозиторий
   (`regtab/rtl-conformance`) или синхронизации копированием, как фикстуры.
4. В CI jregtab добавляется job, прогоняющий корпус через `RtlCompiler` +
   `AtpToRtlSerializer` — доработка на стороне jregtab, детальный план:
   **`jregtab/plans/RTL_CONFORMANCE_CI.md`** (генератор корпуса из `RtlTaskNNNTest`,
   негативные кейсы, тест свежести, workflow `ci.yml`).
5. Финальная страховка — те же 1 500 тестовых вариантов: расхождение парсеров, влияющее
   на результат, неизбежно проявится в Recordset'ах.

---

## 6. Структура репозитория и упаковка

- Новый репозиторий `regtab/pyregtab` (GitHub), зеркальный по духу jregtab:
  README, mkdocs-документация (переиспользование docs jregtab с заменой примеров на Python).
- `pyproject.toml`: `build-backend = maturin`, `requires-python >= 3.10`;
  abi3-wheels (`abi3-py310`) — одна сборка на все поддерживаемые CPython.
- Wheels: `PyO3/maturin-action` — `win_amd64`, `manylinux_x86_64/aarch64`,
  `macosx_x86_64/arm64`.
- `sdist` собирается из исходников (пользователю нужен Rust-toolchain — редкий случай).
- Версионирование: pyRegTab стартует с `0.1.0`; соответствие функциональности версии
  jRegTab фиксируется в README («pyRegTab 0.1.x ≙ jRegTab 0.1.2»).

---

## 7. Стратегия тестирования

1. **Перенос фикстур как есть**: `src/test/resources/tasks/task_001…150/` (CSV UTF-8
   без BOM, `task_match_options.json`) копируются в `tests/fixtures/`. Формат не меняется.
2. **Порт тестовой инфраструктуры**: аналог `RtlTaskBase` на pytest —
   `@pytest.mark.parametrize` по 150 задачам × 5 вариантов × {ATP, RTL}.
   ATP-эталоны (`AtpTaskNNTest`) переписываются на Python-фабрики spec-классов —
   заодно приёмочный тест полноты Python API.
3. **Дифференциальное тестирование против jRegTab**: одноразовый скрипт (JPype или
   CLI-обёртка над jar) прогоняет обе реализации на всех фикстурах и сравнивает
   Recordset'ы побайтно. Отдельный CI-job до релиза 1.0.
4. **Conformance-корпус RTL** (§5.3) — в CI обоих проектов.
5. **Round-trip** `compile(serialize(p)) == p` для задач 001–050 (аналог `AtpRtlRoundTripTest`).
6. Юнит-тесты ядра на Rust (`cargo test`) — matcher/interpreter/парсер в отрыве от
   Python; property-based тесты парсера (proptest): генерация паттернов → serialize →
   compile → сравнение.
7. `cargo clippy` + `cargo miri` (выборочно) вместо санитайзеров C++.

Критерий готовности: **1 500/1 500 вариантов зелёные + дифф с jRegTab пуст +
conformance-корпус зелёный в обоих проектах**.

---

## 8. Риски

| Риск | Вероятность | Смягчение |
|---|---|---|
| Расхождение рукописного парсера с нормативной `RTL.g4` | средняя | conformance-корпус (§5.3): позитив+негатив+каноническая форма; структура парсера 1:1 с правилами грамматики |
| Дрейф языка RTL между jregtab и pyregtab при будущих изменениях | средняя | процесс §5.2: сначала грамматика и корпус, потом обе реализации; CI-hash-проверка |
| Расхождение семантики регулярок Java ↔ Rust | средняя | аудит фикстур в фазе 0; `regex`/`fancy-regex`; дифф-тесты (§7.3) |
| Расхождение поведения matcher'а (порядок перебора квантификаторов, жадность) | средняя | порт «построчно», дифф-тесты на 1 500 вариантах |
| Unicode: Java UTF-16 vs Rust UTF-8 (индексы в StringExtractor, split) | средняя | смещения по code points либо литеральным разделителям; тесты с кириллицей |
| Модель владения (граф Java → арена+индексы) потребует переработки алгоритмов | низкая–средняя | арена проектируется в фазе 1 до порта matcher'а; handle-объекты для Python |
| Недостаток Rust-экспертизы | зависит от команды | точка выхода в фазе 0 → вариант B (C++), архитектура переносится без структурных изменений |

---

## 9. Поэтапный план работ

Оценки — в неделях чистой работы одного разработчика; фазы 2–5 частично параллелизуемы.

### Фаза 0 — Решения и прототип (1 нед.)
- [ ] Аудит регулярок во всех фикстурах/тестах → выбор `regex` vs `fancy-regex`.
- [ ] Прототип парсера: лексер (logos) + рекурсивный спуск для 2–3 правил `RTL.g4`,
      разбор 5–10 реальных RTL-строк.
- [ ] Прототип wheel: maturin + PyO3, пустой `_core` с одним классом, сборка
      wheels на Windows и Linux (GitHub Actions, maturin-action).
- [ ] Утвердить стиль API (properties vs `set_*`; §4.6) и имя пакета на PyPI.
- [ ] **Точка выхода**: при блокере — переключение на C++ (вариант B), §3.B.
- Результат: утверждённые решения, репозиторий-скелет `pyregtab`.

### Фаза 1 — ITM syntax + Recordset (1–1.5 нед.)
- [ ] Порт `itm.syntax` (11 классов) на Rust: арена + индексы (§4.2).
- [ ] Порт `recordset` (3 класса).
- [ ] PyO3-биндинги (handle-объекты) + Python-обёртки `syntax.py`, `recordset.py`.
- [ ] Юнит-тесты (создание таблиц, subtables/subrows, форматирование ячеек).

### Фаза 2 — ATP spec (1–1.5 нед.)
- [ ] Порт `atp.spec` (18 классов) на Rust-enum'ы/структуры: паттерны, content specs,
      `ActionSpec`, `ProviderSpec`, `FilterTerm`, `Quantifier`, `StringExtractor`, предикаты.
- [ ] Custom-предикаты с Python-callable (§4.4).
- [ ] Python-фабрики, повторяющие Java-фабрики; docstrings из Javadoc.

### Фаза 3 — Matcher + ITM semantics (2 нед., самая сложная)
- [ ] Порт `itm.semantics` (items, providers, operations, `WorkingState`).
- [ ] Порт `atp.match` (`SyntaxMatcher`, `SemanticConstructor`, `AtpMatcher`).
- [ ] Первые сквозные тесты: задачи 001–020 через ATP-паттерны (без RTL).

### Фаза 4 — Interpreter (1 нед.)
- [ ] Порт `interpret` (4 фазы, стратегии, `MissingValueHandler`, трансформации
      `WhitespaceNormalization` / `FieldSplitting` / `SchemaReordering` / `AnchorAttributeAtPosition`).
- [ ] Сквозные ATP-тесты для всех 150 задач зелёные.

### Фаза 5 — RTL: парсер, построитель ATP, сериализатор (1.5–2 нед.)
- [ ] Полный лексер + рекурсивный спуск по всем правилам `RTL.g4` (§5.1).
- [ ] Порт логики `ATPBuilder`, `ProviderTemplateResolver`, `StringExtractorFactory`;
      `RtlCompiler` / `RtlCompileError` с диагностикой line:col.
- [ ] Порт `AtpToRtlSerializer`; round-trip тест.
- [ ] Сборка conformance-корпуса (позитив из тестов jregtab, негатив — по веткам
      ошибок парсера); job для jregtab-CI.
- [ ] RTL-тесты для всех 150 задач зелёные (итого 1 500 вариантов).

### Фаза 6 — Дифференциальная проверка и полировка API (1 нед.)
- [ ] Дифф-прогон против jRegTab (§7.3), устранение расхождений.
- [ ] GIL-release (`allow_threads`), `match_many`-бенчмарк, сравнение скорости
      с jRegTab (criterion ↔ JMH).
- [ ] `to_pandas()`, `__repr__`, типизация (`py.typed`, stubs для `_core`).

### Фаза 7 — CI/CD, документация, релиз (1 нед.)
- [ ] maturin-action матрица wheels, публикация на TestPyPI → PyPI (`pyregtab 0.1.0`).
- [ ] mkdocs-сайт (адаптация docs jregtab: getting-started, RTL reference — общий,
      API reference — Python).
- [ ] README с примером из §4.6; документ о процессе эволюции RTL (§5.2) — в оба репо.

**Итого: ~8.5–10 недель.**

---

## 10. Открытые вопросы (требуют решения пользователя)

1. **Ядро**: утвердить Rust (вариант A); C++ остаётся точкой выхода фазы 0.
2. **Стиль API**: properties (`cell.text = "..."`) или Java-зеркальный (`cell.set_text(...)`)?
3. **Имя пакета на PyPI**: `pyregtab` (рекомендуется) — проверить доступность имени.
4. **Conformance-корпус**: отдельный репозиторий `regtab/rtl-conformance` или
   директории-копии в обоих проектах с CI-проверкой синхронности?
5. **Минимальная версия Python**: 3.10 (рекомендуется) или ниже/выше?
6. **Судьба чистого Python-фолбэка**: делать ли параллельную pure-Python реализацию
   для платформ без wheels, или только нативное ядро?
7. **Доработка jregtab** — РЕАЛИЗОВАНО 2026-07-07 (`jregtab/plans/RTL_CONFORMANCE_CI.md`,
   ветка `feature/rtl-conformance-ci`): корпус `jregtab/conformance/` (151 позитивная
   пара с каноническими формами + 15 негативных кейсов, включая `EXT`), контракт
   в `conformance/README.md`, CI-job `conformance`. Для pyRegTab: пин на commit
   jregtab, копия корпуса, pytest-аналог `RtlConformanceTest` (фаза 5).
8. **Embedded DSL в pyRegTab** — РЕШЕНО (2026-07-06): после релиза pyregtab 0.1
   добавить зеркальный embedded-DSL (аналог jregtab `plans/RTL_EMBEDDED_DSL.md`,
   пакет `pyregtab.dsl`); эскейп-хэтч — Python-callable; перегрузка операторов
   в Python позволяет синтаксис ещё ближе к RTL (например, настоящие `+`/`*`
   квантификаторы) — отдельная дизайн-фаза. Учесть также расширение RTL
   привязками `EXT('name')` (этап A плана RTL_EMBEDDED_DSL) в рукописном парсере
   и в conformance-корпусе.
