# Provenance: Test Tasks

Test tasks come from two collections.

---

## Collection 1: Foofah (tasks 01–50)

Test data sourced from the [Foofah](https://github.com/umich-dbgroup/foofah) benchmark collection.

**Source:** [https://github.com/umich-dbgroup/foofah/tree/master/tests/data](https://github.com/umich-dbgroup/foofah/tree/master/tests/data)

**License:** [MIT](https://github.com/umich-dbgroup/foofah/blob/master/LICENSE)

**Structure:** 50 tasks × 5 variants = 250 test cases. Each variant has an input table (CSV) and optionally an expected recordset (CSV).

**Local conversion:** The original Foofah data (`.txt` in `tests/data`) was converted to CSV format with comma delimiter and stored in `foofah-csv-with-comma`. Each variant folder contains `TestingTable.csv`.

**Expected (ground truth):** `D:\YandexDisk\data\foofah-benchmarks\gt` — flat CSV files `{foofah_id}_{variant}.csv` (no header; header `$a_0`,`$a_1`,… is added when copying to `expected_Y.csv`).

### Task mapping: task_XX ↔ foofah_id

| task_id | foofah_id |
|---------|-----------|
| 01 | exp0_2 |
| 02 | exp0_3 |
| 03 | exp0_4 |
| 04 | exp0_5 |
| 05 | exp0_6 |
| 06 | exp0_7 |
| 07 | exp0_8 |
| 08 | exp0_10 |
| 09 | exp0_11 |
| 10 | exp0_12 |
| 11 | exp0_13 |
| 12 | exp0_15 |
| 13 | exp0_17 |
| 14 | exp0_18 |
| 15 | exp0_19 |
| 16 | exp0_22 |
| 17 | exp0_24 |
| 18 | exp0_25 |
| 19 | exp0_26 |
| 20 | exp0_27 |
| 21 | exp0_28 |
| 22 | exp0_29 |
| 23 | exp0_30 |
| 24 | exp0_33 |
| 25 | exp0_34 |
| 26 | exp0_36 |
| 27 | exp0_37 |
| 28 | exp0_40 |
| 29 | exp0_41 |
| 30 | exp0_43 |
| 31 | exp0_44 |
| 32 | exp0_45 |
| 33 | exp0_46 |
| 34 | exp0_47 |
| 35 | exp0_48 |
| 36 | exp0_49 |
| 37 | exp0_51 |
| 38 | exp0_agriculture |
| 39 | exp0_craigslist_data_wrangler |
| 40 | exp0_crime_data_wrangler |
| 41 | exp0_potters_wheel_divide |
| 42 | exp0_potters_wheel_fold |
| 43 | exp0_potters_wheel_fold_2 |
| 44 | exp0_potters_wheel_merge_split |
| 45 | exp0_potters_wheel_split_fold |
| 46 | exp0_potters_wheel_unfold |
| 47 | exp0_potters_wheel_unfold2 |
| 48 | exp0_proactive_wrangling_complex |
| 49 | exp0_proactive_wrangling_fold |
| 50 | exp0_reshape_table_structure_data_wrangler |

**Note:** `exp0_potters_wheel_fold` and `exp0_potters_wheel_fold_2` are distinct tasks. The latter uses variant folders `exp0_potters_wheel_fold_2_1` … `exp0_potters_wheel_fold_2_5`.

---

## Collection 2: RegTab (tasks 51–110)

Test data created as part of the jRegTab project to cover table patterns not present in Foofah.

**Source:** authored by the jRegTab team.

**Structure:** 60 tasks × 5 variants = 300 test cases. Each variant has an input table (CSV) and an expected recordset (CSV) with a header row.

### Task mapping: task_XX ↔ regtab_id

| task_id | regtab_id |
|---------|-----------|
| 51 | synthetic_0001 |
| 52 | synthetic_0002 |
| 53 | synthetic_0003 |
| 54 | synthetic_0004 |
| 55 | synthetic_0005 |
| 56 | synthetic_0006 |
| 57 | synthetic_0007 |
| 58 | synthetic_0008 |
| 59 | synthetic_0009 |
| 60 | synthetic_0010 |
| 61 | synthetic_0011 |
| 62 | synthetic_0012 |
| 63 | synthetic_0013 |
| 64 | synthetic_0014 |
| 65 | synthetic_0015 |
| 66 | synthetic_0016 |
| 67 | synthetic_0017 |
| 68 | synthetic_0018 |
| 69 | synthetic_0019 |
| 70 | synthetic_0020 |
| 71 | synthetic_0021 |
| 72 | synthetic_0022 |
| 73 | synthetic_0023 |
| 74 | synthetic_0024 |
| 75 | synthetic_0025 |
| 76 | synthetic_0026 |
| 77 | synthetic_0027 |
| 78 | synthetic_0028 |
| 79 | synthetic_0029 |
| 80 | synthetic_0030 |
| 81 | synthetic_0031 |
| 82 | synthetic_0032 |
| 83 | synthetic_0033 |
| 84 | synthetic_0034 |
| 85 | synthetic_0035 |
| 86 | synthetic_0036 |
| 87 | synthetic_0037 |
| 88 | synthetic_0038 |
| 89 | synthetic_0039 |
| 90 | synthetic_0040 |
| 91 | synthetic_0041 |
| 92 | synthetic_0042 |
| 93 | synthetic_0043 |
| 94 | synthetic_0044 |
| 95 | synthetic_0045 |
| 96 | synthetic_0046 |
| 97 | synthetic_0047 |
| 98 | synthetic_0048 |
| 99 | synthetic_0049 |
| 100 | synthetic_0050 |
| 101 | synthetic_0051 |
| 102 | synthetic_0052 |
| 103 | synthetic_0053 |
| 104 | synthetic_0054 |
| 105 | synthetic_0055 |
| 106 | synthetic_0056 |
| 107 | synthetic_0057 |
| 108 | synthetic_0058 |
| 109 | synthetic_0059 |
| 110 | synthetic_0060 |

---

## Collection 3: Baikal benchmark (tasks 111–150)

Test data based on real tourism and environmental monitoring tables from the Lake Baikal region.

**Source:** authored by the jRegTab team; derived from public ecological and tourism reports.

**Structure:** 40 tasks × 5 variants = 200 test cases. Each variant has an input table (CSV) and an expected recordset (CSV) with a header row.

### Task mapping: task_XX ↔ regtab_id (Baikal)

| task_id | regtab_id |
|---------|-----------|
| 111 | real_baikal_0001 |
| 112 | real_baikal_0002 |
| 113 | real_baikal_0003 |
| 114 | real_baikal_0004 |
| 115 | real_baikal_0005 |
| 116 | real_baikal_0006 |
| 117 | real_baikal_0007 |
| 118 | real_baikal_0008 |
| 119 | real_baikal_0009 |
| 120 | real_baikal_0010 |
| 121 | real_baikal_0011 |
| 122 | real_baikal_0012 |
| 123 | real_baikal_0013 |
| 124 | real_baikal_0014 |
| 125 | real_baikal_0015 |
| 126 | real_baikal_0016 |
| 127 | real_baikal_0017 |
| 128 | real_baikal_0018 |
| 129 | real_baikal_0019 |
| 130 | real_baikal_0020 |
| 131 | real_baikal_0021 |
| 132 | real_baikal_0022 |
| 133 | real_baikal_0023 |
| 134 | real_baikal_0024 |
| 135 | real_baikal_0025 |
| 136 | real_baikal_0026 |
| 137 | real_baikal_0027 |
| 138 | real_baikal_0028 |
| 139 | real_baikal_0029 |
| 140 | real_baikal_0030 |
| 141 | real_baikal_0031 |
| 142 | real_baikal_0032 |
| 143 | real_baikal_0033 |
| 144 | real_baikal_0034 |
| 145 | real_baikal_0035 |
| 146 | real_baikal_0036 |
| 147 | real_baikal_0037 |
| 148 | real_baikal_0038 |
| 149 | real_baikal_0039 |
| 150 | real_baikal_0040 |
