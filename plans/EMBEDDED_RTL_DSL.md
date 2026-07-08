# План: Embedded RTL как встроенный DSL для pyRegTab (`pyregtab.dsl`)

## Контекст

Сегодня pyRegTab позволяет задавать паттерны извлечения двумя способами: лаконичной
**RTL-строкой** (`RtlCompiler.compile("...")`) и многословным **ATP API**
(`TablePattern.of(SubtablePattern.of(...))` — см.
[tests/atp_patterns.py](d:/YandexDisk/code2/pyregtab/tests/atp_patterns.py)). В jRegTab есть третий,
объединяющий достоинства обоих слой — **Embedded RTL** (`ru.icc.regtab.dsl.Rtl.*`): краткие
fluent-фабрики (`table/subtable/row/cell/rec/skip/...`), которые читаются почти как RTL, но при этом
являются обычным кодом на языке-хозяине. Это даёт автодополнение в IDE, структурную проверку типов,
композицию паттернов через обычные переменные и escape-hatch на лямбдах. В pyRegTab эквивалента нет.
План миграции pyRegTab
([plans/PYREGTAB_MIGRATION_PLAN.md:453-459](d:/YandexDisk/code2/pyregtab/plans/PYREGTAB_MIGRATION_PLAN.md#L453))
уже зафиксировал решение (2026-07-06) добавить это как пост-0.1 модуль `pyregtab.dsl`.

Данный план реализует этот DSL. Ключевые факты, установленные при исследовании:

- **Чистый Python, без изменений в Rust.** Каждый узел ATP достижим через видимые из Python,
  `frozen` (иммутабельные, «клонируй и верни») фабрики нативного ядра. DSL — тонкий сахар,
  вызывающий их. Примитивы escape-hatch уже существуют и принимают Python-callable:
  `CellPredicate.custom`/`external`, `FilterTerm.custom`/`external`,
  `ItemFilterConditionSpec.custom`, `StringExtractor.custom` и `Bindings`.
- **Эквивалентность напрямую проверяема.** `TablePattern.__eq__` — глубокое структурное сравнение
  ([src/py.rs:2389](d:/YandexDisk/code2/pyregtab/src/py.rs#L2389)); `AtpToRtlSerializer.serialize`
  делает round-trip. Инвариант, который нужно гарантировать: `dsl_pattern == RtlCompiler.compile(rtl)`
  для паттернов без лямбд (зеркало `DslSpikeTest.assertMirrors` из jRegTab).

### Зафиксированные решения

- **Именование: snake_case, единая каноническая поверхность** (соответствует pandas/polars/PEP 8 и
  уже существующему snake_case API pyRegTab; camelCase-алиасов в v1 нет — их тривиально добавить
  позже при необходимости).
- **Операторы: только методы в v1.** Канонические `.one_or_more()/.exactly(n)/.and_()/.or_()`.
  Пока без перегрузки `+x`/`x*n`/`&`/`|` (отложено, тонкий будущий слой).
- **Полный паритет словаря в одном релизе (0.2.0).** Все уровни, квантификаторы, контент-спеки,
  действия, провайдеры, item-констрейнты, traversal, трансформации, наследуемые действия уровней
  `acts(...)`, условия уровней и escape-hatch `where(...)`.
- **`subtable(...)` — единственная фабрика уровня** (яснее, чем `sub(...)` из реализации jRegTab;
  совпадает с именем класса и публичной документацией jRegTab). Короткий алиас `sub` не добавляем.
- Поскольку `and`/`or` — ключевые слова Python, конъюнкция/дизъюнкция провайдеров записываются как
  `.and_()/.or_()` (согласуется с существующими `ItemFilterConditionSpec.and_/or_`).

## Размещение модуля

Новый чисто-Python модуль в дереве maturin `python-source` (автоматически попадает в wheel, правок
конфигурации сборки не требуется):

- `python/pyregtab/dsl.py` — весь DSL в одном файле (оценка ~500-700 строк). Полностью
  аннотирован типами (в пакете есть `py.typed`). Разбивать на пакет `dsl/` — только если перерастёт
  один файл.
- Подключение в [python/pyregtab/__init__.py](d:/YandexDisk/code2/pyregtab/python/pyregtab/__init__.py):
  добавить `from pyregtab import dsl`, чтобы `pyregtab.dsl` был импортируемым, и добавить `"dsl"` в
  `__all__`. **Не** поднимать имена DSL (`VAL`, `ST`, `C`, ...) в верхнеуровневое пространство имён
  `pyregtab` — они живут в `pyregtab.dsl` и используются через `from pyregtab.dsl import *` (модуль
  задаёт собственный `__all__`). Это исключает засорение и коллизии основного namespace.

## Ядро дизайна — единственная нетривиальная часть: билдер `Prov`

Всё остальное — прямое оборачивание. Тонкость — в провайдерах. В RTL провайдер записывается как
`template(constraints){cardinality}` с необязательным traversal, а его **kind** (VAL/ATTR/
UNRESTRICTED) выводится из охватывающего действия (`rec`/`join`→VAL, `avp`→ATTR,
`fill`/`prefix`/`suffix`→UNRESTRICTED). jRegTab делает это через билдер `Prov` + резолюцию
`kindFor(op)`. Зеркалируем:

- Класс `Prov` накапливает **OR-список AND-групп из `FilterTerm`-ов**, плюс `cardinality`
  (по умолчанию 1) и `traversal_order` (по умолчанию row-major). Иммутабелен; каждый постфиксный
  метод возвращает новый `Prov`.
  - `.and_(other)` — конъюнкция с **дистрибуцией по вложенным OR** ровно как в компиляторе
    (`A & (B|C)` ⇒ `(A&B)|(A&C)`).
  - `.or_(other)` — добавление OR-групп.
  - `.card(n)` / `.unbounded()` (→ `ProviderSpec.UNBOUNDED`).
  - `.reversed()` / `.col_major()` / `.reversed_col_major()` → установка `TraversalOrder`.
  - `.where(desc, callable)` — `.and_` к `FilterTerm.custom(desc, callable)` (escape hatch).
- `Prov.resolve(kind) -> ProviderSpec`: собрать условие через `ItemFilterConditionSpec`
  (один терм → `bare(term)`; одна группа → `and_(*terms)`; несколько групп →
  `or_(*[and_(...)...])`), затем диспатч по kind: VAL→`ProviderSpec.val(cond, cardinality, traversal)`,
  ATTR→`ProviderSpec.attr(cond, traversal)` (cardinality фиксирована 1),
  UNRESTRICTED→`ProviderSpec.any(cond, cardinality, traversal)`.
- **Критично для `==`:** вложенность результирующего `ItemFilterConditionSpec` должна совпадать с
  канонической формой компилятора байт-в-байт, иначе структурное равенство не пройдёт. Это главный
  риск корректности; его закрывают корпусные/ad-hoc тесты паритета (ниже). Если компилятор
  нормализует дистрибуцию OR иначе — подстроить нормализацию `.and_` под него.

Собственные **функции действий** DSL (`rec/avp/join/fill/prefix/suffix`) — а не «сырые»
`ActionSpec.*` — берут на себя диспатч kind: они резолвят каждый `Prov`/контекстный аргумент в
полный `ProviderSpec` нужного kind, затем вызывают существующий `ActionSpec.rec(*providers,
anchor_pos=, split_delimiter=)` и т. д. Это сохраняет пофакторную cardinality/traversal каждого
провайдера (что невозможно при коэрции «голого» условия внутри `ActionSpec.rec`).

Контекстные аргументы резолвятся через небольшой маркер `Ctx`/`CtxAvp`: `lit(text)` →
`ProviderSpec.ctx_val` под REC/JOIN, иначе `ctx_attr`; `ctx_avp(a, v)` → `ProviderSpec.ctx_avp`.

## Полный словарь (RTL → `pyregtab.dsl` → нижележащая ATP-фабрика)

Уровни и квантификаторы:
- `{...}`/`[...]` → `table(...)`, `subtable(...)`, `row(...)`, `subrow(...)` →
  `TablePattern.of` / `SubtablePattern.of` / `RowPattern.of` / `SubrowPattern.of`. `row(cell...)`
  автозаворачивает в один неявный subrow (фабрика `of` это уже делает); `row(subrow...)` — для явных
  subrow.
- Квантификаторы `+ * ? {n}` → постфиксные `.one_or_more() .zero_or_more() .zero_or_one()
  .exactly(n)` (**уже есть** на всех четырёх pattern-классах — DSL просто их возвращает).

Ячейки и контент:
- `[]` → `skip()` → `CellPattern.skip()`. `[VAL : acts]` → `cell(VAL, rec(...))`. `[cond ? VAL]`
  → `cell(not_blank(), VAL, ...)`. `[BLANK]` (только условие) → `cell(blank())`.
- `val(*acts) / attr(...) / aux(...)` → `AtomicContentSpec.val/attr/aux`; константы
  `VAL ATTR AUX SKIP` = `ItemDerivationDirective.*`. `cell(idd, *acts)` внутри строит
  `AtomicContentSpec.<idd>(*acts)`; `cell(content_spec)` пропускает compound/conditional
  контент-спек.
- `.tagged("H")`, `.extract(NORM)`, `.split_by(",")`, `.then(" ", ...)` → **уже есть** на
  `AtomicContentSpec`/контент-спеках.
- Условный `BLANK ? _ | VAL` → `when(blank(), SKIP, VAL)` → `ConditionalContentSpec` (4 перегрузки,
  принимающие директиву или контент-спек в каждой ветви).

Предикаты (совпадение ячейки) → `CellPredicate` в обёртке `CellMatchCondition`:
- `blank() not_blank() re(s) not_re(s) contains(s) not_contains(s)`;
  `where(desc, callable)` → `CellPredicate.custom`.

Провайдеры:
- Константы `LT RT AV BW ROW COL SR SC ST NCL CL STR` → `Prov` над
  `FilterTerm.left_of/right_of/above/below/same_row/same_col/same_subrow/same_subcol/
  same_subtable/not_same_cell/same_cell/same_str`.
- Позиционные: `C(n)`→`col_exact`, `C(lo,hi)`→`col_absolute_range`, `Crel(d)`→`col_offset`,
  `Crel(lo,hi)`→`col_range`, `CrelFrom(lo)`→`col_range(lo, UNBOUNDED)`; `R(...)`/`Rrel(d)` и
  `P(...)`/`Prel(d)` аналогично (у row/pos есть offset, но нет относительного диапазона/открытой
  формы, как в jRegTab).
- Контент-констрейнты: `tag(t) not_tag(t) item_re(s) item_not_re(s) item_blank() item_not_blank()
  item_contains(s) item_not_contains(s)` → соответствующие `FilterTerm.*`.
- Постфиксы `.and_ .or_ .card .unbounded .reversed .col_major .reversed_col_major .where` (см. выше).

Действия → `ActionSpec` (через DSL-обёртки с диспатчем kind):
- `rec(*prov) / rec(anchor_pos, *prov) / rec_split(delim, *prov)`; `avp(prov) / avp("NAME")`;
  `join(*prov) / join(key, *prov)`; `fill(delim, *prov) / prefix(...) / suffix(...)`.
- Контекст: `lit("EUR")`, `ctx_avp("K","V")`.

Экстракторы и трансформации:
- Константы `NORM TRIM UC LC`; `repl(rx, rep) substr(b, e) chain(*steps)` →
  `StringExtractor.replaced/substring/chain`.
- Настройки `<NORM, ANCH(n), SPLIT("s")>` → `table(...).with_transformations(norm(), anch(n),
  split(","))`, где `norm()/anch(n)/split(s)` → `WhitespaceNormalization/
  AnchorAttributeAtPosition/DelimitedFieldSplit` (**`.with_transformations` уже есть**).

Наследуемые действия и условия уровней:
- Маркер `acts(*actions)` → «спускается» вниз (merge-down); `table/subtable/row/subrow/cell`
  принимают необязательный ведущий `CellPredicate` (условие уровня) и/или `acts(...)` перед
  полезной нагрузкой, зеркаля перегрузки jRegTab и merge-down компилятора (использует
  `ActionSpec.as_inherited`).

## Escape-hatch

- Ячейка: `cell(where("isTotal", lambda c: c.text.startswith("Total")), VAL, ...)` →
  `CellPredicate.custom`.
- Провайдер: `ROW.where("isNum", lambda a, c: ...).unbounded()` → `FilterTerm.custom`.
- **Ограничение (как в jRegTab):** паттерны с `where(...)` нельзя сериализовать в RTL
  (`custom`-предикаты бросают исключение в `to_rtl`) и они сравниваются неравными при разных лямбдах
  (равенство callable — по идентичности указателя). Для **сериализуемой** именованной альтернативы
  используется существующий маршрут `EXT('name')` + `Bindings` в строковом компиляторе RTL (уже
  поддержан). Документировать оба.

## Версия, упаковка и попутный фикс существующей рассинхронизации

- Поднять до **0.2.0** в [Cargo.toml](d:/YandexDisk/code2/pyregtab/Cargo.toml),
  [Cargo.lock](d:/YandexDisk/code2/pyregtab/Cargo.lock),
  [pyproject.toml](d:/YandexDisk/code2/pyregtab/pyproject.toml) **и**
  [python/pyregtab/__init__.py](d:/YandexDisk/code2/pyregtab/python/pyregtab/__init__.py)
  `__version__` — который сейчас устарел на `"0.1.0"` (рассинхрон с релизом 0.1.1); выровнять все
  четыре.

## Тесты / верификация

Зеркалировать `DslSpikeTest` из jRegTab как `tests/test_dsl.py` с хелпером
`assert_mirrors(rtl, dsl_pattern)`, проверяющим **и**
`AtpToRtlSerializer.serialize(compile(rtl)) == serialize(dsl_pattern)`, **и**
`compile(rtl) == dsl_pattern` (структурно). Покрыть тот же набор, что проверил jRegTab:

- 20 задач-зеркал: 001, 002, 006, 009, 013, 015, 016, 022, 023, 025, 029, 045, 052, 068, 069, 070,
  071, 074, 107, 116 (107/116 также демонстрируют «фрагменты = обычные переменные Python»).
- 6 ad-hoc: OR-дизъюнкция, дистрибуция OR, настройки (`with_transformations`), cell-level `acts`,
  table-level условие+`acts` и один escape-hatch `where(...)` (только сборка, без serialize — так как
  `custom` несериализуем, проверять его поведенчески через раннер фикстур, а не через `==`).

Переиспользовать существующую инфраструктуру: `task_rtl(id)`/`run_task_variant` из
[tests/task_runner.py](d:/YandexDisk/code2/pyregtab/tests/task_runner.py) для поведенческих проверок.

**Корпусный структурный паритет (stretch, рекомендуется):** адаптировать
[tools/translate_atp.py](d:/YandexDisk/code2/pyregtab/tools/translate_atp.py), добавив режим `--dsl`,
который перенацеливает эмиттеры (`constructor`/`qualified_call`/`provider_call`/`action_call`) на
поверхность DSL, генерируя `tests/dsl_patterns.py` для всего бенчмарк-корпуса; параметризованный тест
затем проверяет `dsl_pattern_NNN() == RtlCompiler.compile(task_rtl(NNN))` для каждой задачи без лямбд,
давая байт-в-байт паритет по всем ~150 задачам вместо 20. Если перенацеливание эмиттеров окажется
трудоёмким — откатиться к 26 ручным зеркалам (планка самого jRegTab).

Шаги сквозной верификации:
1. `maturin develop --release` (изменений Rust нет, но пересобирает mixed-wheel, чтобы `pyregtab.dsl`
   импортировался).
2. `pytest tests/test_dsl.py -q` — тесты паритета + escape-hatch зелёные.
3. `pytest tests -q` — полный набор (1142+ существующих тестов) по-прежнему зелёный; подтверждает
   отсутствие регрессий и чистый импорт модуля DSL.
4. Smoke: `python -c "from pyregtab.dsl import *; from pyregtab import AtpMatcher; ..."` —
   собрать задачу 001 и сопоставить её с фикстурой, чтобы убедиться, что путь через DSL реально
   даёт совпадение.

## Документация

- Новый `docs/embedded-rtl.md` (порт из jRegTab, snake_case, Python-идиомы —
  переменные-как-фрагменты, escape-hatch `where(...)`, таблица сравнения трёх API), добавить в
  nav mkdocs.
- Добавить раздел «Embedded RTL» в [README.md](d:/YandexDisk/code2/pyregtab/README.md) и
  [docs/api.md](d:/YandexDisk/code2/pyregtab/docs/api.md); упомянуть в
  [docs/getting-started.md](d:/YandexDisk/code2/pyregtab/docs/getting-started.md).
- Обновить пункт-follow-up в плане миграции на «выполнено».

## Сводка изменений по файлам

| Файл | Изменение |
|---|---|
| `python/pyregtab/dsl.py` | **новый** — DSL (фабрики, билдер `Prov`, диспатч kind в действиях, escape-hatch) |
| `python/pyregtab/__init__.py` | ре-экспорт подмодуля `dsl`; добавить в `__all__`; поднять `__version__` до 0.2.0 |
| `Cargo.toml`, `Cargo.lock`, `pyproject.toml` | версия → 0.2.0 |
| `tests/test_dsl.py` | **новый** — `assert_mirrors` + 20 задач-зеркал + 6 ad-hoc |
| `tests/dsl_patterns.py` | **новый (stretch)** — сгенерированные корпусные зеркала DSL |
| `tools/translate_atp.py` | **(stretch)** — режим эмиттера `--dsl` |
| `docs/embedded-rtl.md` | **новый**; плюс правки README, api.md, getting-started, nav mkdocs, план миграции |

## Риски

- **Расхождение дистрибуции OR / канонизации условий** → структурное `==` не проходит. Смягчение:
  26 тестов паритета (в т. ч. два OR ad-hoc) плюс опциональный корпусный тест фиксируют точную
  каноническую форму; подстроить нормализацию `Prov.and_` под компилятор.
- **Дрейф DSL от RTL при эволюции языка.** Смягчение: инвариант `serialize(dsl)==canonical(rtl)`
  живёт в тестовом наборе; новые конструкции RTL обязаны расширять DSL в том же изменении.
- **Путаница «третьего диалекта» в документации.** Смягчение: документация всегда показывает RTL и
  DSL рядом со строгой таблицей соответствия 1:1 (как в jRegTab).
