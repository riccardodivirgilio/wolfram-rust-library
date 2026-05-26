(* Tests for the legacy_wstp example library.
   Run with: wolframscript -file Example.wl
   Requires: cargo wl build --example legacy_wstp -p wolfram-examples *)

$libName = "liblegacy_wstp";

$manifestPath = FileNameJoin[{
    DirectoryName[If[$TestFileName =!= "", $TestFileName, $InputFileName]],
    "..", "..",
    "target", "debug", "examples",
    $libName,
    $SystemID,
    $libName,
    "manifest.wl"
}];

$w = Get[$manifestPath];

If[!AssociationQ[$w],
    Print["SKIP: manifest not found or failed to load: ", $manifestPath];
    Exit[1]
];

Print["loaded ", $libName, " (", Length[$w], " functions)"];

ok[label_, got_, expected_] := If[got === expected,
    Print["  PASS  ", label],
    Print["  FAIL  ", label, "\n    got:      ", InputForm[got], "\n    expected: ", InputForm[expected]]
];

(* ---- integer / real atoms ---- *)
Print["\n-- echo atoms --"];
ok["echo 42",       $w["echo_expr"][42],    42];
ok["echo -7",       $w["echo_expr"][-7],   -7];
ok["echo 1.5",      $w["echo_expr"][1.5],  1.5];

(* ---- BigInteger / BigReal ---- *)
Print["\n-- big numbers --"];
ok["echo 2^200",    $w["echo_expr"][2^200],       2^200];
ok["echo -(2^200)", $w["echo_expr"][-(2^200)],    -(2^200)];
ok["echo N[Pi,50]", $w["echo_expr"][N[Pi, 50]],  N[Pi, 50]];
ok["echo N[E,30]",  $w["echo_expr"][N[E,  30]],  N[E,  30]];

(* ---- ByteArray / NumericArray ---- *)
Print["\n-- packed types --"];
ok["ByteArray {1,2,3}",        $w["make_byte_array"][],            ByteArray[{1, 2, 3}]];
ok["NumericArray Real64 1D",   $w["make_numeric_array_r64"][],     NumericArray[{1., 2., 3.}, "Real64"]];
ok["NumericArray Integer32 2D",$w["make_numeric_array_i32_2d"][], NumericArray[{{1,2},{3,4}}, "Integer32"]];

(* ---- ExprKind tags ---- *)
Print["\n-- kind tags --"];
ok["kind Integer",    $w["expr_kind_tag"][42],        "Integer"];
ok["kind Real",       $w["expr_kind_tag"][1.5],       "Real"];
ok["kind BigInteger", $w["expr_kind_tag"][2^200],     "BigInteger"];
ok["kind BigReal",    $w["expr_kind_tag"][N[Pi, 50]], "BigReal"];
ok["kind String",     $w["expr_kind_tag"]["hello"],   "String"];
ok["kind Symbol",     $w["expr_kind_tag"][Pi],        "Symbol"];

Print["\ndone"];
