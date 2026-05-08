//! WXF self-roundtrip tests: export → import → equal.

use wolfram_expr::{
    Association, ByteArray, Complex32, Complex64, Expr, NumericArray, NumericArrayDataType,
    PackedArray, PackedArrayDataType, Symbol,
};
use wolfram_serializer::{export, import, Format};

fn roundtrip(expr: Expr) {
    let bytes = export(&expr, Format::Wxf).expect("export Wxf");
    let parsed = import(&bytes, Format::Wxf).expect("import Wxf");
    assert_eq!(parsed, expr, "roundtrip mismatch");
}

#[test]
fn integer_widths() {
    roundtrip(Expr::from(0i64));
    roundtrip(Expr::from(127i64)); // fits Int8
    roundtrip(Expr::from(-128i64));
    roundtrip(Expr::from(32_000i64)); // fits Int16
    roundtrip(Expr::from(2_000_000_000i64)); // fits Int32
    roundtrip(Expr::from(i64::MAX));
    roundtrip(Expr::from(i64::MIN));
}

#[test]
fn real_basic() {
    roundtrip(Expr::real(3.14159));
    roundtrip(Expr::real(0.0));
    roundtrip(Expr::real(-1.5e100));
}

#[test]
fn string_unicode() {
    roundtrip(Expr::from("hello"));
    roundtrip(Expr::from(""));
    roundtrip(Expr::from("ünîcödé 🚀"));
}

#[test]
fn symbol_roundtrip() {
    roundtrip(Expr::symbol(Symbol::new("System`Plus")));
    roundtrip(Expr::symbol(Symbol::new("Global`x")));
}

#[test]
fn function_nested() {
    // Plus[1, Times[2, 3], "x"]
    let times = Expr::normal(Symbol::new("System`Times"), vec![Expr::from(2), Expr::from(3)]);
    let plus = Expr::normal(
        Symbol::new("System`Plus"),
        vec![Expr::from(1), times, Expr::from("x")],
    );
    roundtrip(plus);
}

#[test]
fn function_curried_head() {
    // f[1, 2][3, 4] — head is itself a Normal
    let inner = Expr::normal(Expr::symbol(Symbol::new("Global`f")), vec![Expr::from(1), Expr::from(2)]);
    let outer = Expr::normal(inner, vec![Expr::from(3), Expr::from(4)]);
    roundtrip(outer);
}

#[test]
fn byte_array_roundtrip() {
    roundtrip(Expr::from(ByteArray::new()));
    roundtrip(Expr::from(ByteArray::from(vec![0u8, 1, 2, 3, 0xff])));
    roundtrip(Expr::from((0..=255u8).collect::<ByteArray>()));
}

#[test]
fn association_rule_and_delayed() {
    let mut a = Association::new();
    a.insert(Expr::from("eager"), Expr::from(1));
    a.insert_delayed(Expr::from("lazy"), Expr::from(2));
    roundtrip(Expr::from(a));
}

#[test]
fn numeric_array_typed() {
    let arr = NumericArray::from_slice::<i32>(vec![2, 3], &[1, 2, 3, 4, 5, 6]);
    roundtrip(Expr::from(arr));
}

#[test]
fn numeric_array_real64() {
    let arr = NumericArray::from_slice::<f64>(vec![4], &[1.0, 2.0, 3.5, -7.0]);
    roundtrip(Expr::from(arr));
}

#[test]
fn numeric_array_unsigned_3d() {
    let arr = NumericArray::new(
        NumericArrayDataType::UBit8,
        vec![2, 2, 2],
        vec![1u8, 2, 3, 4, 5, 6, 7, 8],
    );
    roundtrip(Expr::from(arr));
}

#[test]
fn packed_array_real64() {
    let arr = PackedArray::from_slice::<f64>(vec![3], &[1.0, 2.0, 3.0]);
    roundtrip(Expr::from(arr));
}

#[test]
fn packed_array_int32_2d() {
    // Build the byte buffer from a typed Vec<i32>:
    let v: Vec<i32> = vec![1, 2, 3, 4];
    let bytes: Vec<u8> = unsafe {
        std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(&v[..]))
    }
    .to_vec();
    let arr = PackedArray::new(PackedArrayDataType::Integer32, vec![2, 2], bytes);
    roundtrip(Expr::from(arr));
}

#[test]
fn empty_function() {
    roundtrip(Expr::list(vec![]));
}

// `Vec<T>` direct serialization: numeric T → NumericArray; u8 → ByteArray.

#[test]
fn vec_u8_serializes_as_byte_array() {
    let bytes = wolfram_serializer::export(&vec![1u8, 2, 3, 0xff], wolfram_serializer::Format::Wxf)
        .unwrap();
    let parsed = wolfram_serializer::import(&bytes, wolfram_serializer::Format::Wxf).unwrap();
    // Importing brings it back as ExprKind::ByteArray:
    assert!(matches!(parsed.kind(), wolfram_expr::ExprKind::ByteArray(_)));
    assert_eq!(parsed.try_as_byte_array().unwrap().as_slice(), &[1u8, 2, 3, 0xff]);
}

#[test]
fn vec_i32_serializes_as_numeric_array() {
    let bytes = wolfram_serializer::export(&vec![10i32, 20, 30, 40], wolfram_serializer::Format::Wxf)
        .unwrap();
    let parsed = wolfram_serializer::import(&bytes, wolfram_serializer::Format::Wxf).unwrap();
    let arr = parsed.try_as_numeric_array().expect("expected NumericArray");
    assert_eq!(arr.data_type(), NumericArrayDataType::Bit32);
    assert_eq!(arr.dimensions(), &[4]);
    assert_eq!(arr.try_as_slice::<i32>(), Some([10, 20, 30, 40].as_slice()));
}

#[test]
fn numeric_array_complex64() {
    let arr = NumericArray::from_slice::<Complex64>(
        vec![3],
        &[
            Complex64::new(1.0, 2.0),
            Complex64::new(0.0, -1.0),
            Complex64::new(-3.5, 4.5),
        ],
    );
    roundtrip(Expr::from(arr));
}

#[test]
fn numeric_array_complex32() {
    let arr = NumericArray::from_slice::<Complex32>(
        vec![2],
        &[Complex32::new(1.0, 2.0), Complex32::new(3.0, 4.0)],
    );
    roundtrip(Expr::from(arr));
}

#[test]
fn packed_array_complex64() {
    let arr = PackedArray::from_slice::<Complex64>(
        vec![2],
        &[Complex64::new(1.5, -2.5), Complex64::new(0.0, 1.0)],
    );
    roundtrip(Expr::from(arr));
}

#[test]
fn vec_f64_serializes_as_numeric_array() {
    let bytes = wolfram_serializer::export(&vec![1.5f64, 2.5, 3.5], wolfram_serializer::Format::Wxf)
        .unwrap();
    let parsed = wolfram_serializer::import(&bytes, wolfram_serializer::Format::Wxf).unwrap();
    let arr = parsed.try_as_numeric_array().expect("expected NumericArray");
    assert_eq!(arr.data_type(), NumericArrayDataType::Real64);
    assert_eq!(arr.try_as_slice::<f64>(), Some([1.5, 2.5, 3.5].as_slice()));
}

#[test]
fn empty_association() {
    roundtrip(Expr::from(Association::new()));
}

#[test]
fn rejects_truncated_header() {
    assert!(import(b"", Format::Wxf).is_err());
    assert!(import(b"8", Format::Wxf).is_err());
}

#[test]
fn rejects_wrong_version() {
    assert!(import(b"7:", Format::Wxf).is_err());
}

#[cfg(feature = "bignum")]
#[test]
fn big_integer_roundtrip() {
    use wolfram_expr::BigInteger;
    let huge = BigInteger::parse("99999999999999999999999999999999999999999").unwrap();
    roundtrip(Expr::from(huge));
}

#[cfg(feature = "bignum")]
#[test]
fn big_real_roundtrip() {
    use wolfram_expr::BigReal;
    let r = BigReal::new("3.14159265358979323846`50.");
    roundtrip(Expr::from(r));
}
