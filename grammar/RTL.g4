// Regular Table Language (RTL) Grammar
grammar RTL ;

options { caseInsensitive = true ; }

// Quantifier
quantifier : zeroOrOne | zeroOrMore | oneOrMore | exactly ;

zeroOrOne  : QUESTION ;
zeroOrMore : MULT ;
oneOrMore  : PLUS ;
exactly    : LCURLY INT RCURLY ;

// Named fragment definitions (preamble): $name=[body] or $name={body}
fragmentDef : FRAGMENT_ID ASSIGN LSQUARE cellPatternBody? RSQUARE    // cell:     $N=[cellBody]
            | FRAGMENT_ID ASSIGN LSQUARE rowPatternBody   RSQUARE    // row:      $N=[ [sub]+ ]
            | FRAGMENT_ID ASSIGN LCURLY  subrowPatternBody  RCURLY   // subrow:   $N={ [cell]+ }
            | FRAGMENT_ID ASSIGN LCURLY  subtablePatternBody RCURLY  // subtable: $N={ [row]+ }
            ;

// Table pattern
tablePattern : fragmentDef* (cellMatchCond QUESTION)? settings? actSpecs? subtablePattern+ ;

// Optional settings prefix <NORM,ANCH(n),SPLIT("s")>
settings     : LANGLE setting (COMMA setting)* RANGLE ;
setting      : normSetting | anchSetting | splitSetting ;
normSetting  : 'NORM' ;
anchSetting  : 'ANCH' LPAREN INT RPAREN ;
splitSetting : 'SPLIT' LPAREN STRING RPAREN ;

// Subtable pattern: implicit, explicit, or fragment reference
subtablePattern : implSubtablePattern
                | explSubtablePattern
                | LCURLY FRAGMENT_ID RCURLY quantifier?   // subtable fragment ref {$N}
                ;

// Implicit subtable pattern
implSubtablePattern : rowPattern+ ;

// Explicit subtable pattern
explSubtablePattern : LCURLY subtablePatternBody RCURLY quantifier? ;
subtablePatternBody : (cellMatchCond QUESTION)? (actSpecs)? rowPattern+ ;

// Row pattern
rowPattern : LSQUARE rowPatternBody RSQUARE quantifier?   // regular
           | LSQUARE FRAGMENT_ID    RSQUARE quantifier?   // row fragment ref [$N]
           ;
rowPatternBody : (cellMatchCond QUESTION)? (actSpecs)? subrowPattern+ ;

// Subrow pattern: implicit, explicit, or fragment reference
subrowPattern : implSubrowPattern
              | explSubrowPattern
              | LCURLY FRAGMENT_ID RCURLY quantifier?     // subrow fragment ref {$N}
              ;

// Implicit subrow pattern
implSubrowPattern : cellPattern+ ;

// Explicit subrow pattern
explSubrowPattern : LCURLY subrowPatternBody RCURLY quantifier? ;
subrowPatternBody : (cellMatchCond QUESTION)? (actSpecs)? cellPattern+ ;

// Cell pattern
cellPattern : LSQUARE cellPatternBody? RSQUARE quantifier?   // regular
            | LSQUARE FRAGMENT_ID      RSQUARE quantifier?   // fragment reference [$name]
            ;
cellPatternBody : cellMatchCond QUESTION actSpecs? contSpec
               | cellMatchCond
               | actSpecs? contSpec
               ;

// Content specification: atomic, delimited, compound, conditional
contSpec : atomContSpec | delimContSpec | compContSpec | condContSpec ;

// Atomic content specification
atomContSpec : itemDerivDir tags? (ASSIGN strExtr)? (COLON actSpecs)? ;

// Item derivation directive
itemDerivDir  : ATTRIBUTE | VALUE | AUXILIARY | SKIPPED ;
ATTRIBUTE : 'ATTR' ;
VALUE     : 'VAL'  ;
AUXILIARY : 'AUX'  ;
SKIPPED   : 'SKIP' | '_' ;

// User-defined tags
tags    : tagItem+ ;
tagItem : HASH STRING ;

// Item string extractor (supports chains: =REPL("x","").TRIM)
strExtr     : strExtrStep ('.' strExtrStep)* ;
strExtrStep : substr | replace | norm | upperCase | lowerCase | trim ;

// String processing
substr    : 'SUBSTR' LPAREN INT COMMA INT RPAREN ;
replace   : 'REPL'   LPAREN STRING COMMA STRING RPAREN ;
norm      : 'NORM' ;
upperCase : 'UC' ;
lowerCase : 'LC' ;
trim      : 'TRIM' ;

// Interpretation action specifications
actSpecs : actSpec (COMMA actSpec)* ;

// Interpretation action specification
actSpec : provSpecs RIGHT_ARROW op ;
op : fillOp | prefixOp | suffixOp | AVP | recOp | joinOp ;
fillOp   : FILL   (LPAREN STRING RPAREN)? ;
prefixOp : PREFIX (LPAREN STRING RPAREN)? ;
suffixOp : SUFFIX (LPAREN STRING RPAREN)? ;
recOp    : REC    (LPAREN (INT | STRING) RPAREN)? ;
joinOp   : JOIN   (LPAREN INT (COMMA INT)* RPAREN)? ;
FILL   : 'FILL'   ;
PREFIX : 'PREFIX' ;
SUFFIX : 'SUFFIX' ;
AVP    : 'AVP'    ;
REC    : 'REC'    ;
JOIN   : 'JOIN'   ;

provSpecs : provSpec | (LPAREN provSpec (COMMA provSpec)* RPAREN) | LPAREN RPAREN ;

// Delimited content specification
delimContSpec : LPAREN atomContSpec RPAREN LCURLY separator RCURLY;

separator  : STRING ;  // Separator

// Compound content specification
compContSpec : openDelim? compSeg (separator compSeg)* closeDelim? ;
compSeg      : atomContSpec | delimContSpec ;

openDelim  : STRING ;  // Opening delimiter
closeDelim : STRING ;  // Closing delimiter

// Conditional content specification
condContSpec : cellMatchCond QUESTION (xContSpec VBAR xContSpec) ;
xContSpec    : atomContSpec | delimContSpec | compContSpec ;

// Cell match condition
cellMatchCond : cellMatchConstr ;
cellMatchConstr : regex | blank | contains | ext ;

// Item provider specification
provSpec : tblProvSpec | ctxProvSpec | ctxAvpSpec ;

// Cell derived item provider specification
// Single bare form: spatConstr only (avoids ambiguity with ctxProvSpec STRING).
// Bare & -conjunction starting with spatConstr is also unambiguous (keywords, not STRING).
// Disjunctions (|) and content-only constraints still require parentheses.
tblProvSpec : traversalOrderMark? (spatConstr | LPAREN constraints RPAREN | bareConjConstraints) cardinality? ;

// Bare & -conjunction of constraints starting with a spatConstr keyword
bareConjConstraints : spatConstr (AMP baseConstr)+ ;

// Traversal order mark (absence = ROW_MAJOR)
traversalOrderMark : reverseRowMajor | columnMajor | reverseColumnMajor ;
reverseRowMajor    : MINUS ;
columnMajor        : CARET ;
reverseColumnMajor : MINUS CARET ;

// Context derived item provider specification
ctxProvSpec : STRING ;

// Constant attribute-value pair provider: @'ATTR'='VALUE'
ctxAvpSpec : AT STRING ASSIGN STRING ;

// Cardinality k: {n} = at most n; * = UNBOUNDED (0..*); absent = at most 1 (default)
cardinality : LCURLY INT RCURLY | MULT ;

// Constraints (| has lower precedence than &; parentheses for explicit grouping)
constraints : orGroup (VBAR orGroup)* ;
orGroup     : baseConstr (AMP baseConstr)* ;
baseConstr  : constr | LPAREN constraints RPAREN ;
constr      : spatConstr | contConstr ;

// Spatial constraints
spatConstr : LEFT_OF | RIGHT_OF | ABOVE | BELOW | SAME_ROW | SAME_COLUMN
           | SAME_SUBROW | SAME_SUBCOLUMN | SAME_SUBTABLE | NOT_SAME_CELL | SAME_CELL
           | col | row | pos ;

LEFT_OF   : 'LT'  ;   // sameSubrow(a) && col < col(a)
RIGHT_OF  : 'RT'  ;   // sameSubrow(a) && col > col(a)
ABOVE     : 'AV'  ;   // sameSubcol(a) && row < row(a)
BELOW     : 'BW'  ;   // sameSubcol(a) && row > row(a)
SAME_ROW       : 'ROW' ;   // sameRow(a) && !sameCell(a)
SAME_COLUMN    : 'COL' ;   // sameCol(a) && !sameCell(a)
SAME_SUBROW    : 'SR'  ;   // sameSubrow(a) && !sameCell(a)
SAME_SUBCOLUMN : 'SC'  ;   // sameSubcol(a) && !sameCell(a)
SAME_SUBTABLE  : 'ST'  ;   // sameSubtable(a) && !sameCell(a)
NOT_SAME_CELL  : 'NCL' ;   // !sameCell(a)
SAME_CELL      : 'CL'  ;   // sameCell(a)

// Positional constraints
row : 'R' (range | offset | INT) ;
col : 'C' (range | offset | INT) ;
pos : 'P' (range | offset | INT) ;

range : start DOUBLE_PERIOD end? ;
start : offset | INT ;
end   : offset | INT ;

offset : (MINUS INT) | (PLUS INT) ;

// Content constraints
contConstr : regex | blank | tag | sameStr | contains | ext ;

// External Java binding: EXT('name') — resolved against Bindings at compile time
ext : EXT LPAREN STRING RPAREN ;
EXT : 'EXT' ;

// Contains constraint
contains : EXCLAMATION? TILDA STRING ;
TILDA : '~' ;

tag : EXCLAMATION? tagItem ;

sameStr : STR ;
STR : 'STR' ;

regex : EXCLAMATION? STRING ;

blank : EXCLAMATION? 'BLANK' ;

PLUS  : '+' ;
MINUS : '-' ;
CARET : '^' ;
MULT  : '*' ;
AMP   : '&' ;

LPAREN  : '(' ;
RPAREN  : ')' ;
LCURLY  : '{' ;
RCURLY  : '}' ;
LSQUARE : '[' ;
RSQUARE : ']' ;
LANGLE  : '<' ;
RANGLE  : '>' ;

COLON       : ':' ;
COMMA       : ',' ;
QUESTION    : '?' ;
VBAR        : '|' ;
EXCLAMATION : '!' ;

DOUBLE_PERIOD : '..' ;
ASSIGN        : '=' ;

RIGHT_ARROW : '->' ;

HASH : '#' ;
AT   : '@' ;

// Fragment identifier: $name (single token avoids keyword conflicts with caseInsensitive=true)
FRAGMENT_ID : '$' [A-Z][A-Z0-9_]* ;

INT : [0-9]+ ;

STRING
    : '"'      (ESC | '""'   | ~["])* '"'
    | '\''     (ESC | '\'\'' | ~['])* '\''
    | '“' (ESC | .)*? ('”' | '″')   // smart quotes
    ;

fragment ESC
    : '`\''    // backtick single-quote
    | '`"'     // backtick double-quote
    ;

WS : [ \r\t\n]+ -> channel(HIDDEN) ;

ZWNBSP : [﻿]+ -> channel(HIDDEN) ; // Remove UTF8 BOM character

LineComment
    : '//' ~[\r\n]* -> channel(HIDDEN) ;
