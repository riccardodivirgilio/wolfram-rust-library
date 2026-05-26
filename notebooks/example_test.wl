(* Test script for legacy_native and legacy_wstp examples.
   Run with: wolframscript -file notebooks/example_test.wl
   from the wolfram-rust-library workspace root. *)

$root = ParentDirectory[DirectoryName[$InputFileName]];
$nativeLib = FileNameJoin[{$root, "target/debug/examples/liblegacy_native.dylib"}];
$wstpLib   = FileNameJoin[{$root, "target/debug/examples/liblegacy_wstp.dylib"}];

(* ── Helpers ─────────────────────────────────────────────────────────────── *)

$passed = 0;
$failed = 0;

check[label_String, got_, expected_] := If[got === expected,
    $passed++;
    Print["  PASS  ", label],
    $failed++;
    Print["  FAIL  ", label, "  got: ", got, "  expected: ", expected]
];

(* Wraps a call with a before/after print so hangs are visible immediately. *)
timed[label_, expr_] := (
    Print["  >> ", label];
    expr
);

(* ── Load via loaders ────────────────────────────────────────────────────── *)

Print["\n=== Loading native functions ==="];
loadNative = LibraryFunctionLoad[$nativeLib, "load_legacy_native_functions", LinkObject, LinkObject];
$n = loadNative[$nativeLib];
Print["Loaded: ", Keys[$n]];

Print["\n=== Loading WSTP functions ==="];
loadWstp = LibraryFunctionLoad[$wstpLib, "load_legacy_wstp_functions", LinkObject, LinkObject];
$w = loadWstp[$wstpLib];
Print["Loaded: ", Keys[$w]];

(* ══════════════════════════════════════════════════════════════════════════ *)
Print["\n=== Native tests ==="];

check["square[7]",           timed["square[7]",           $n["square"][7]],             49];
check["square[-3]",          timed["square[-3]",          $n["square"][-3]],            9];
check["add[3.0, 4.0]",       timed["add[3.0,4.0]",        $n["add"][3.0, 4.0]],        7.0];
check["add[0.5, 0.5]",       timed["add[0.5,0.5]",        $n["add"][0.5, 0.5]],        1.0];
check["reverse_string",      timed["reverse_string",      $n["reverse_string"]["hello"]], "olleh"];
check["reverse_string empty",timed["reverse_string empty",$n["reverse_string"][""]],   ""];
check["total_i64",           timed["total_i64",           $n["total_i64"][NumericArray[{1,2,3,4},"Integer64"]]], 10];
check["dot_f64",             timed["dot_f64",             $n["dot_f64"][NumericArray[{1.,2.,3.},"Real64"],NumericArray[{4.,5.,6.},"Real64"]]], 32.0];
check["scale_f64",           timed["scale_f64",           Normal@$n["scale_f64"][NumericArray[{1.,2.,3.},"Real64"],2.0]], {2.,4.,6.}];
check["positive_i64",        timed["positive_i64",        Normal@$n["positive_i64"][NumericArray[{-2,0,3,-1,5},"Integer64"]]], {0,0,1,0,1}];

(* ══════════════════════════════════════════════════════════════════════════ *)
Print["\n=== WSTP scalar / string tests ==="];

check["square_wstp[5]",     timed["square_wstp[5]",     $w["square_wstp"][5]],        25];
check["square_wstp[-4]",    timed["square_wstp[-4]",    $w["square_wstp"][-4]],       16];
check["count_args 0",       timed["count_args 0",       $w["count_args"][]],          0];
check["count_args 3",       timed["count_args 3",       $w["count_args"][a,b,c]],     3];
check["total_args_i64",     timed["total_args_i64",     $w["total_args_i64"][1,2,3,4,5]], 15];
check["string_join",        timed["string_join",        $w["string_join"]["foo","bar","baz"]], "foobarbaz"];
check["string_join empty",  timed["string_join empty",  $w["string_join"][]],         ""];
check["link_expr_identity", timed["link_expr_identity", $w["link_expr_identity"][1,2,3]], {1,2,3}];
check["total integers",     timed["total integers",     $w["total"][1,2,3,4]],        10];
check["total reals",        timed["total reals",        $w["total"][1.,2.,3.]],       6.];
check["expr_string_join",   timed["expr_string_join",   $w["expr_string_join"]["x","y","z"]], "xyz"];

(* ══════════════════════════════════════════════════════════════════════════ *)
Print["\n=== ExprKind / BigInteger / BigReal assertions ==="];

check["kind Integer 1",        timed["kind Integer 1",        $w["expr_kind_tag"][1]],         "Integer"];
check["kind Real 2.",          timed["kind Real 2.",           $w["expr_kind_tag"][2.]],        "Real"];
check["kind BigInteger 2^200", timed["kind BigInteger 2^200",  $w["expr_kind_tag"][2^200]],     "BigInteger"];
check["kind BigReal N[Pi,50]", timed["kind BigReal N[Pi,50]",  $w["expr_kind_tag"][N[Pi,50]]], "BigReal"];

check["echo Integer 1",        timed["echo Integer 1",        $w["echo_expr"][1]],             1];
check["echo Real 2.",          timed["echo Real 2.",          $w["echo_expr"][2.]],            2.];
check["echo BigInteger 2^200", timed["echo BigInteger 2^200", $w["echo_expr"][2^200]],         2^200];
check["echo ByteArray",        timed["echo ByteArray",        $w["echo_expr"][ByteArray[{1,2,3}]]],               ByteArray[{1,2,3}]];
check["echo NumericArray R64", timed["echo NumericArray R64", $w["echo_expr"][NumericArray[{1.,2.},"Real64"]]],   NumericArray[{1.,2.},"Real64"]];
check["echo NumericArray I64", timed["echo NumericArray I64", $w["echo_expr"][NumericArray[{10,20},"Integer64"]]], NumericArray[{10,20},"Integer64"]];

(* ══════════════════════════════════════════════════════════════════════════ *)
Print["\n=== ExprKind probe — what arrives on the Rust side over WSTP ==="];

probe[label_, value_] := Module[{kind, echo},
    Print["  >> probe kind: ", label];
    kind = $w["expr_kind_tag"][value];
    Print["  >> probe echo: ", label];
    echo = $w["echo_expr"][value];
    Print["  ", label, "  =>  kind=", kind, "  echo=", echo]
];

probe["Integer 42",                  42];
probe["Real 3.14",                   3.14];
probe["String \"hello\"",            "hello"];
probe["Symbol Pi",                   Pi];
probe["List {1,2,3}",                {1, 2, 3}];
probe["Association <|a->1, b->2|>",  <|"a" -> 1, "b" -> 2|>];
probe["ByteArray {1,2,3}",           ByteArray[{1, 2, 3}]];
probe["NumericArray Real64",         NumericArray[{1.0, 2.0, 3.0}, "Real64"]];
probe["NumericArray Integer64",      NumericArray[{10, 20, 30}, "Integer64"]];
probe["PackedArray (dense list)",    {1.0, 2.0, 3.0}];
probe["BigInteger 2^200",            2^200];
probe["BigReal N[Pi, 50]",           N[Pi, 50]];

(* ══════════════════════════════════════════════════════════════════════════ *)
Print["\n=== Panic handling ==="];

Module[{result = (Print["  >> force_panic"]; $w["force_panic"]["boom"])},
    check["panic returns Failure", MatchQ[result, _Failure], True];
    Print["  panic result: ", result]
];

(* ── Summary ─────────────────────────────────────────────────────────────── *)

Print["\n=== Summary: ", $passed, " passed, ", $failed, " failed ===\n"];
If[$failed > 0, Exit[1]];
