package ru.icc.regtab;

import com.fasterxml.jackson.databind.ObjectMapper;
import ru.icc.regtab.atp.AtpMatcher;
import ru.icc.regtab.interpret.SchemaConstructionStrategy;
import ru.icc.regtab.interpret.TableInterpreter;
import ru.icc.regtab.itm.InterpretableTable;
import ru.icc.regtab.itm.syntax.TableSyntax;
import ru.icc.regtab.recordset.Record;
import ru.icc.regtab.recordset.Recordset;
import ru.icc.regtab.rtl.RtlCompiler;
import ru.icc.regtab.tasks.CsvTableLoader;

import java.io.BufferedWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * One-off dumper for differential testing against pyRegTab (pyregtab plan
 * §7.3): for every task/variant, compiles the conformance-corpus RTL,
 * matches, interprets, and writes the resulting recordset as JSONL.
 *
 * Usage: RecordsetDumpMain <tasksRoot> <conformancePositive> <out.jsonl>
 */
public final class RecordsetDumpMain {

    public static void main(String[] args) throws Exception {
        Path tasksRoot = Path.of(args[0]);
        Path corpus = Path.of(args[1]);
        Path out = Path.of(args[2]);
        ObjectMapper mapper = new ObjectMapper();

        List<String> taskIds = new ArrayList<>();
        try (var stream = Files.list(tasksRoot)) {
            stream.filter(Files::isDirectory)
                    .map(p -> p.getFileName().toString())
                    .filter(n -> n.startsWith("task_"))
                    .map(n -> n.substring(5))
                    .sorted()
                    .forEach(taskIds::add);
        }

        try (BufferedWriter w = Files.newBufferedWriter(out, StandardCharsets.UTF_8)) {
            for (String id : taskIds) {
                String rtl = new String(
                        Files.readAllBytes(corpus.resolve("task_" + id + ".rtl")),
                        StandardCharsets.UTF_8);
                var pattern = RtlCompiler.compile(rtl);
                Path taskDir = tasksRoot.resolve("task_" + id);
                for (int v = 1; v <= 9; v++) {
                    Path input = taskDir.resolve("input_" + v + ".csv");
                    if (!Files.exists(input)) continue;
                    TableSyntax syntax = CsvTableLoader.load(input);
                    Map<String, Object> row = new LinkedHashMap<>();
                    row.put("task", id);
                    row.put("variant", v);
                    InterpretableTable itm = AtpMatcher.match(pattern, syntax).orElse(null);
                    if (itm == null) {
                        row.put("match", false);
                    } else {
                        row.put("match", true);
                        Recordset rs = pattern.transform(new TableInterpreter()
                                .withStrategy(SchemaConstructionStrategy.RECORD_FIRST)
                                .interpret(itm));
                        row.put("schema", rs.schema().attributes());
                        List<List<String>> records = new ArrayList<>();
                        for (Record r : rs.records()) {
                            List<String> vals = new ArrayList<>();
                            for (String a : rs.schema().attributes()) {
                                vals.add(r.get(a));
                            }
                            records.add(vals);
                        }
                        row.put("records", records);
                    }
                    w.write(mapper.writeValueAsString(row));
                    w.write("\n");
                }
            }
        }
        System.out.println("dump written: " + out);
    }
}
